use std::ffi::OsStr;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const STANDARD_BIN_DIRS: [&str; 2] = ["/opt/homebrew/bin", "/usr/local/bin"];

fn resolve_brew(standard_dirs: &[&Path], path: Option<&OsStr>) -> Option<PathBuf> {
    // Preserve an explicit installation preference; GUI apps can fall back to
    // standard locations when their PATH does not contain Homebrew.
    path.into_iter()
        .flat_map(std::env::split_paths)
        .chain(standard_dirs.iter().map(|dir| dir.to_path_buf()))
        // Do not execute a program from the app's working directory.
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join("brew"))
        .find(|candidate| {
            candidate.metadata().is_ok_and(|metadata| {
                metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
            })
        })
}

/// Resolve Homebrew without requiring a login shell or its PATH configuration.
pub(crate) fn command() -> io::Result<Command> {
    let path = std::env::var_os("PATH");
    let standard_dirs = STANDARD_BIN_DIRS.map(Path::new);
    command_with_paths(&standard_dirs, path.as_deref())
}

fn command_with_paths(standard_dirs: &[&Path], path: Option<&OsStr>) -> io::Result<Command> {
    let binary = resolve_brew(standard_dirs, path).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "No executable Homebrew found in /opt/homebrew/bin, /usr/local/bin, or PATH. \
             For a custom Homebrew install, make its bin directory available to Keybinder",
        )
    })?;
    let mut command = Command::new(&binary);
    // brew is a script and invokes tools itself. Give only this child a usable PATH.
    let mut dirs = vec![binary.parent().unwrap().to_path_buf()];
    dirs.extend(["/usr/bin", "/bin", "/usr/sbin", "/sbin"].map(PathBuf::from));
    if let Some(path) = path {
        dirs.extend(std::env::split_paths(path).filter(|dir| dir.is_absolute()));
    }
    command.env(
        "PATH",
        std::env::join_paths(dirs)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?,
    );
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_path_order_when_standard_and_custom_installations_coexist() {
        let root = tempfile::tempdir().unwrap();
        let standard = root.path().join("standard");
        let first = root.path().join("first");
        let second = root.path().join("second");
        for dir in [&standard, &first, &second] {
            std::fs::create_dir(dir).unwrap();
            let binary = dir.join("brew");
            std::fs::write(&binary, "#!/bin/sh\nexit 0\n").unwrap();
            std::fs::set_permissions(binary, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        for ordered in [[&first, &second], [&second, &first]] {
            let path = std::env::join_paths(ordered).unwrap();
            let command = command_with_paths(&[&standard], Some(&path)).unwrap();
            assert_eq!(command.get_program(), ordered[0].join("brew"));
            let child_path = command
                .get_envs()
                .find(|(key, _)| *key == "PATH")
                .unwrap()
                .1
                .unwrap();
            assert_eq!(
                std::env::split_paths(child_path).next().unwrap(),
                *ordered[0]
            );
        }
        std::fs::remove_file(first.join("brew")).unwrap();
        assert_eq!(
            resolve_brew(&[&standard], Some(first.as_os_str())),
            Some(standard.join("brew"))
        );
    }

    #[test]
    fn runs_brew_script_with_arguments_and_child_tools_without_inherited_path() {
        let root = tempfile::tempdir().unwrap();
        let binary = root.path().join("brew");
        std::fs::write(
            &binary,
            "#!/usr/bin/env bash\nprintf '%s\\n' \"$@\"\ncommand -v bash\n",
        )
        .unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut command = command_with_paths(&[root.path()], None).unwrap();
        assert_eq!(command.get_program(), binary.as_os_str());
        let output = command
            .args(["services", "restart", "skhd"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.starts_with("services\nrestart\nskhd\n/"), "{stdout}");
    }

    #[test]
    fn missing_brew_error_lists_search_locations_and_custom_install_guidance() {
        let error = command_with_paths(&[], None).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        let message = error.to_string();
        assert!(message.contains("/opt/homebrew/bin"));
        assert!(message.contains("/usr/local/bin"));
        assert!(message.contains("custom Homebrew install"));
    }

    #[test]
    fn resolves_both_standard_locations_with_gui_path() {
        let root = tempfile::tempdir().unwrap();
        let arm = root.path().join("opt/homebrew/bin");
        let intel = root.path().join("usr/local/bin");
        for dir in [&arm, &intel] {
            std::fs::create_dir_all(dir).unwrap();
        }
        let dirs = [arm.as_path(), intel.as_path()];
        let gui_path = Some(OsStr::new("/usr/bin:/bin:/usr/sbin:/sbin"));
        assert_eq!(resolve_brew(&dirs, gui_path), None);
        for dir in [&intel, &arm] {
            let binary = dir.join("brew");
            std::fs::write(&binary, "#!/bin/sh\nexit 0\n").unwrap();
            std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
            assert_eq!(resolve_brew(&dirs, gui_path), Some(binary));
        }
        assert_eq!(resolve_brew(&dirs, None), Some(arm.join("brew")));
    }

    #[test]
    fn supports_custom_absolute_path_but_rejects_unusable_candidates() {
        let root = tempfile::tempdir().unwrap();
        let binary = root.path().join("brew");
        let path = Some(root.path().as_os_str());
        std::fs::create_dir(&binary).unwrap();
        assert_eq!(resolve_brew(&[], path), None);
        std::fs::remove_dir(&binary).unwrap();
        std::fs::write(&binary, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(resolve_brew(&[], path), None);
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(resolve_brew(&[], path), Some(binary));
        assert_eq!(resolve_brew(&[], Some(OsStr::new(".:relative:"))), None);
    }
}
