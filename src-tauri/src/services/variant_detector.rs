/// Variant detection service for skhd
use std::process::Command;

use crate::models::skhd_variant::{DetectedVariant, DetectionSource, SkhdVariant};

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
            None,
            Some("com.jackielii.skhd".to_string()),
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
            None,
            Some("com.koekeishiya.skhd".to_string()),
            DetectionSource::Running,
        ));
    }

    None
}

/// Detect from Homebrew installation
fn detect_from_homebrew() -> Option<DetectedVariant> {
    // Check for skhd.zig first (jackielii/tap/skhd-zig)
    let output = Command::new("brew")
        .args(["list", "skhd-zig"])
        .output()
        .ok()?;

    if output.status.success() {
        // Get the binary path from brew --prefix
        let prefix_output = Command::new("brew")
            .args(["--prefix", "skhd-zig"])
            .output()
            .ok()?;

        let prefix = String::from_utf8_lossy(&prefix_output.stdout).trim().to_string();
        let binary_path = format!("{}/bin/skhd", prefix);

        return Some(DetectedVariant::new(
            Some(SkhdVariant::Zig),
            Some(binary_path),
            Some("com.jackielii.skhd".to_string()),
            DetectionSource::Homebrew,
        ));
    }

    // Check for original skhd
    let output = Command::new("brew")
        .args(["list", "skhd"])
        .output()
        .ok()?;

    if output.status.success() {
        // Get the binary path from brew --prefix
        let prefix_output = Command::new("brew")
            .args(["--prefix", "skhd"])
            .output()
            .ok()?;

        let prefix = String::from_utf8_lossy(&prefix_output.stdout).trim().to_string();
        let binary_path = format!("{}/bin/skhd", prefix);

        return Some(DetectedVariant::new(
            Some(SkhdVariant::Original),
            Some(binary_path),
            Some("com.koekeishiya.skhd".to_string()),
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

    let binary_path = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();

    if binary_path.is_empty() {
        return None;
    }

    // Get version output to fingerprint
    let version_output = Command::new(&binary_path)
        .arg("--version")
        .output()
        .ok()?;

    let version_str = String::from_utf8_lossy(&version_output.stdout).to_lowercase();

    // skhd.zig version output contains "zig" or "skhd.zig"
    if version_str.contains("zig") || version_str.contains("skhd.zig") {
        return Some(DetectedVariant::new(
            Some(SkhdVariant::Zig),
            Some(binary_path),
            Some("com.jackielii.skhd".to_string()),
            DetectionSource::Path,
        ));
    }

    // Original skhd (check if it's NOT zig)
    // If we found skhd and it doesn't have zig markers, assume original
    Some(DetectedVariant::new(
        Some(SkhdVariant::Original),
        Some(binary_path),
        Some("com.koekeishiya.skhd".to_string()),
        DetectionSource::Path,
    ))
}

/// Detect from .app bundle
fn detect_from_app_bundle() -> Option<DetectedVariant> {
    let app_bundle_path =
        "/Applications/skhd.app/Contents/MacOS/skhd".to_string();

    if std::path::Path::new(&app_bundle_path).exists() {
        // Treat .app bundle as skhd.zig variant
        return Some(DetectedVariant::new(
            Some(SkhdVariant::Zig),
            Some(app_bundle_path),
            Some("com.jackielii.skhd".to_string()),
            DetectionSource::AppBundle,
        ));
    }

    None
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
