/// Variant detection service for skhd
use std::process::Command;

use crate::models::skhd_variant::{DetectedVariant, DetectionSource, SkhdVariant};
use crate::services::homebrew;

fn classify_version_output(output: &str) -> SkhdVariant {
    if output.trim_start().to_lowercase().starts_with("skhd.zig v") {
        SkhdVariant::Zig
    } else {
        SkhdVariant::Original
    }
}

/// Detects which skhd variant is installed/active
///
/// Detection order:
/// 1. Check running launchd jobs (com.jackielii.skhd first, then com.koekeishiya.skhd)
/// 2. Check Homebrew formulae (brew list skhd-zig, then brew list skhd)
/// 3. Check PATH for skhd binary and fingerprint version output
/// 4. Check .app bundle at /Applications/skhd.app/Contents/MacOS/skhd
/// 5. Return None if not detected
pub fn detect_variant() -> DetectedVariant {
    // 1. Check running launchd jobs first
    if let Some(variant) = detect_from_launchd() {
        return variant;
    }

    // 2. Check Homebrew
    if let Some(variant) = detect_from_homebrew() {
        return variant;
    }

    // 3. Check PATH with version fingerprint
    if let Some(variant) = detect_from_path() {
        return variant;
    }

    // 4. Check .app bundle
    if let Some(variant) = detect_from_app_bundle() {
        return variant;
    }

    // 5. Not detected
    DetectedVariant::none()
}

/// Check one variant without allowing another installed variant to mask it.
pub fn is_variant_installed(variant: SkhdVariant) -> bool {
    if Command::new("launchctl")
        .args(["list", variant.service_label()])
        .output()
        .is_ok_and(|output| output.status.success())
    {
        return true;
    }

    let brew_args: &[&str] = match variant {
        SkhdVariant::Original => &["list", "skhd"],
        SkhdVariant::Zig => &["list", "--cask", "skhd-zig"],
    };
    if homebrew::command()
        .and_then(|mut command| command.args(brew_args).output())
        .is_ok_and(|output| output.status.success())
    {
        return true;
    }

    if variant == SkhdVariant::Zig && std::path::Path::new(&app_bundle_binary()).exists() {
        return true;
    }

    find_binary(variant).is_some()
}

/// Detect from running launchd jobs
fn detect_from_launchd() -> Option<DetectedVariant> {
    // Check for skhd.zig first (jackielii)
    let output = Command::new("launchctl")
        .args(["list", "com.jackielii.skhd"])
        .output()
        .ok()?;

    if output.status.success() {
        return Some(DetectedVariant::new(
            Some(SkhdVariant::Zig),
            find_binary(SkhdVariant::Zig),
            Some(SkhdVariant::Zig.service_label().to_string()),
            DetectionSource::Running,
        ));
    }

    // Check for original skhd (koekeishiya)
    let output = Command::new("launchctl")
        .args(["list", "com.koekeishiya.skhd"])
        .output()
        .ok()?;

    if output.status.success() {
        return Some(DetectedVariant::new(
            Some(SkhdVariant::Original),
            find_binary(SkhdVariant::Original),
            Some(SkhdVariant::Original.service_label().to_string()),
            DetectionSource::Running,
        ));
    }

    None
}

/// Detect from Homebrew installation
fn detect_from_homebrew() -> Option<DetectedVariant> {
    // skhd.zig is distributed as a cask. Its app may use a custom Homebrew appdir,
    // so prefer the executable path reported by `brew list --cask`.
    let mut brew = homebrew::command().ok()?;
    let output = brew.args(["list", "--cask", "skhd-zig"]).output().ok()?;

    if output.status.success() {
        let listed_files = String::from_utf8_lossy(&output.stdout);
        let binary_path = cask_binary_from_listing(&listed_files).or_else(|| {
            Some(app_bundle_binary()).filter(|path| std::path::Path::new(path).exists())
        });

        return Some(DetectedVariant::new(
            Some(SkhdVariant::Zig),
            binary_path,
            Some(SkhdVariant::Zig.service_label().to_string()),
            DetectionSource::Homebrew,
        ));
    }

    // Check for original skhd
    let output = homebrew::command()
        .ok()?
        .args(["list", "skhd"])
        .output()
        .ok()?;

    if output.status.success() {
        // Get the binary path from brew --prefix
        let binary_path = original_brew_binary()?;

        return Some(DetectedVariant::new(
            Some(SkhdVariant::Original),
            Some(binary_path),
            Some(SkhdVariant::Original.service_label().to_string()),
            DetectionSource::Homebrew,
        ));
    }

    None
}

/// Detect from PATH with version fingerprint
fn detect_from_path() -> Option<DetectedVariant> {
    // Find skhd binary in PATH
    let output = Command::new("which").arg("skhd").output().ok()?;

    if !output.status.success() {
        return None;
    }

    let binary_path = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if binary_path.is_empty() {
        return None;
    }

    // Get version output to fingerprint
    // Older original builds may not support --version. Finding a binary on PATH
    // is still sufficient to detect original skhd unless output identifies Zig.
    let variant = Command::new(&binary_path)
        .arg("--version")
        .output()
        .ok()
        .map(|output| {
            let version = format!(
                "{} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            classify_version_output(&version)
        })
        .unwrap_or(SkhdVariant::Original);

    Some(DetectedVariant::new(
        Some(variant),
        Some(binary_path),
        Some(variant.service_label().to_string()),
        DetectionSource::Path,
    ))
}

/// Detect from .app bundle
fn detect_from_app_bundle() -> Option<DetectedVariant> {
    let app_bundle_path = app_bundle_binary();

    if std::path::Path::new(&app_bundle_path).exists() {
        // Treat .app bundle as skhd.zig variant
        return Some(DetectedVariant::new(
            Some(SkhdVariant::Zig),
            Some(app_bundle_path),
            Some(SkhdVariant::Zig.service_label().to_string()),
            DetectionSource::AppBundle,
        ));
    }

    None
}

fn app_bundle_binary() -> String {
    "/Applications/skhd.app/Contents/MacOS/skhd".to_string()
}

fn cask_binary_from_listing(listing: &str) -> Option<String> {
    listing.lines().map(str::trim).find_map(|path| {
        if path.ends_with("/skhd.app/Contents/MacOS/skhd") {
            Some(path.to_string())
        } else if path.ends_with("/skhd.app") {
            Some(format!("{path}/Contents/MacOS/skhd"))
        } else {
            None
        }
    })
}

fn zig_cask_binary() -> Option<String> {
    let output = homebrew::command()
        .ok()?
        .args(["list", "--cask", "skhd-zig"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| cask_binary_from_listing(&String::from_utf8_lossy(&output.stdout)))
        .flatten()
}

fn original_brew_binary() -> Option<String> {
    let output = homebrew::command()
        .ok()?
        .args(["--prefix", "skhd"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    // brew --prefix can report a formula's path even when it is not installed.
    // In that case, allow a manually installed binary on PATH to be selected.
    let prefix = String::from_utf8_lossy(&output.stdout);
    let binary = std::path::Path::new(prefix.trim()).join("bin/skhd");
    (binary.is_absolute() && binary.is_file()).then(|| binary.to_string_lossy().into_owned())
}

fn path_binary() -> Option<String> {
    let output = Command::new("which").arg("skhd").output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|path| !path.is_empty())
}

pub(crate) fn find_binary(variant: SkhdVariant) -> Option<String> {
    if variant == SkhdVariant::Zig {
        let app = app_bundle_binary();
        if std::path::Path::new(&app).exists() {
            return Some(app);
        }
        if let Some(cask_binary) = zig_cask_binary() {
            return Some(cask_binary);
        }
    } else if let Some(brew_binary) = original_brew_binary() {
        return Some(brew_binary);
    }

    let path = path_binary()?;
    let output = Command::new(&path).arg("--version").output().ok()?;
    let version = format!(
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (classify_version_output(&version) == variant).then_some(path)
}

/// Async version of detect_variant for Tauri commands
pub async fn detect_variant_async() -> DetectedVariant {
    // Run sync version in a blocking task
    tokio::task::spawn_blocking(detect_variant)
        .await
        .unwrap_or_else(|_| DetectedVariant::none())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_discovery_handles_missing_formula_and_coexisting_zig() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let scripts = [
            ("brew", "#!/bin/sh\n[ \"$*\" = '--prefix skhd' ] || exit 1\nprintf '%s/formula\\n' \"$HOME\"\n"),
            ("which", "#!/bin/sh\nprintf '%s/skhd\\n' \"$HOME\"\n"),
            ("skhd", "#!/bin/sh\necho 'skhd.zig v0.2.0'\n"),
        ];
        for (name, script) in scripts {
            let path = root.path().join(name);
            std::fs::write(&path, script).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "services::variant_detector::tests::original_discovery_child",
                "--nocapture",
            ])
            .env("KEYBINDER_DISCOVERY_TEST_CHILD", "1")
            .env("HOME", root.path())
            .env("PATH", root.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn original_discovery_child() {
        use std::os::unix::fs::PermissionsExt;
        if std::env::var_os("KEYBINDER_DISCOVERY_TEST_CHILD").is_none() {
            return;
        }
        let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
        // A missing formula must not hide a manual install, or select Zig as classic.
        assert_eq!(original_brew_binary(), None);
        assert_eq!(find_binary(SkhdVariant::Original), None);
        std::fs::write(home.join("skhd"), "#!/bin/sh\necho 'skhd 0.3.9'\n").unwrap();
        assert_eq!(
            find_binary(SkhdVariant::Original),
            Some(home.join("skhd").to_string_lossy().into_owned())
        );

        // An installed classic formula takes precedence over a Zig binary on PATH.
        let classic = home.join("formula/bin/skhd");
        std::fs::create_dir_all(classic.parent().unwrap()).unwrap();
        std::fs::write(&classic, "#!/bin/sh\necho 'skhd 0.3.9'\n").unwrap();
        std::fs::set_permissions(&classic, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::write(home.join("skhd"), "#!/bin/sh\necho 'skhd.zig v0.2.0'\n").unwrap();
        assert_eq!(
            find_binary(SkhdVariant::Original),
            Some(classic.to_string_lossy().into_owned())
        );
    }

    #[test]
    fn test_classify_version_output() {
        assert_eq!(classify_version_output("skhd.zig v0.2.0"), SkhdVariant::Zig);
        assert_eq!(classify_version_output("skhd 0.3.9"), SkhdVariant::Original);
        assert_eq!(
            classify_version_output("built with zig cc"),
            SkhdVariant::Original
        );
    }

    #[test]
    fn test_cask_binary_from_listing_uses_the_app_executable() {
        let listing = "/Applications/skhd.app\n/opt/homebrew/bin/skhd\n";
        assert_eq!(
            cask_binary_from_listing(listing).as_deref(),
            Some("/Applications/skhd.app/Contents/MacOS/skhd")
        );

        let executable_listing = "/custom/skhd.app/Contents/MacOS/skhd\n/opt/homebrew/bin/skhd\n";
        assert_eq!(
            cask_binary_from_listing(executable_listing).as_deref(),
            Some("/custom/skhd.app/Contents/MacOS/skhd")
        );
    }

    #[test]
    fn test_detected_variant_new() {
        let variant = DetectedVariant::new(
            Some(SkhdVariant::Zig),
            Some("/usr/local/bin/skhd".to_string()),
            Some("com.jackielii.skhd".to_string()),
            DetectionSource::Homebrew,
        );

        assert_eq!(variant.variant, Some(SkhdVariant::Zig));
        assert_eq!(variant.binary_path, Some("/usr/local/bin/skhd".to_string()));
        assert_eq!(variant.plist_label, Some("com.jackielii.skhd".to_string()));
        assert_eq!(variant.source, DetectionSource::Homebrew);
    }

    #[test]
    fn test_detected_variant_none() {
        let variant = DetectedVariant::none();

        assert_eq!(variant.variant, None);
        assert_eq!(variant.binary_path, None);
        assert_eq!(variant.plist_label, None);
        assert_eq!(variant.source, DetectionSource::None);
        assert!(!variant.is_detected());
    }

    #[test]
    fn test_detected_variant_is_detected() {
        let detected = DetectedVariant::new(
            Some(SkhdVariant::Original),
            None,
            None,
            DetectionSource::Path,
        );
        assert!(detected.is_detected());

        let not_detected = DetectedVariant::none();
        assert!(!not_detected.is_detected());
    }

    #[test]
    fn test_detected_variant_description() {
        let original = DetectedVariant::new(
            Some(SkhdVariant::Original),
            None,
            None,
            DetectionSource::Running,
        );
        assert_eq!(original.description(), "skhd (original)");

        let zig = DetectedVariant::new(
            Some(SkhdVariant::Zig),
            None,
            None,
            DetectionSource::AppBundle,
        );
        assert_eq!(zig.description(), "skhd.zig");

        let none = DetectedVariant::none();
        assert_eq!(none.description(), "Not detected");
    }
}
