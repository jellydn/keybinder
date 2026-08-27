use std::env;
/// Path resolution utilities for skhd configuration files
use std::path::{Path, PathBuf};

use crate::models::SkhdVariant;

/// Expand ~ in path to user's home directory
///
/// # Examples
/// ```
/// use keybinder_lib::utils::path::expand_path;
///
/// let path = expand_path("~/.config/skhd/skhdrc");
/// assert!(path.starts_with("/Users/") || path.starts_with("/home/"));
/// ```
pub fn expand_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let path_str = path.as_ref().to_string_lossy();

    if let Some(remainder) = path_str.strip_prefix("~/") {
        if let Ok(home) = env::var("HOME") {
            let home_path = PathBuf::from(home);
            return home_path.join(remainder);
        }
    } else if path_str == "~" {
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home);
        }
    }

    // Return as-is if no expansion needed or HOME not available
    PathBuf::from(path.as_ref())
}

/// Get the default skhd configuration file path
///
/// Checks in order:
/// 1. $XDG_CONFIG_HOME/skhd/skhdrc
/// 2. ~/.config/skhd/skhdrc
/// 3. ~/.skhdrc
///
/// Returns first existing path, or ~/.config/skhd/skhdrc as default for new configs
pub fn get_default_config_path() -> PathBuf {
    // Check XDG_CONFIG_HOME first if set
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        let xdg_path = PathBuf::from(xdg).join("skhd/skhdrc");
        if xdg_path.exists() {
            return xdg_path;
        }
    }

    // Check ~/.config/skhd/skhdrc
    let config_path = expand_path("~/.config/skhd/skhdrc");
    if config_path.exists() {
        return config_path;
    }

    // Check ~/.skhdrc
    let home_path = expand_path("~/.skhdrc");
    if home_path.exists() {
        return home_path;
    }

    // Default to ~/.config/skhd/skhdrc for new configs
    config_path
}

/// Get the config path for a specific skhd variant
///
/// # Arguments
/// * `variant` - The skhd variant to get the config path for
///
/// # Returns
/// * `Ok(String)` - Path to the first existing config file for the variant
/// * `Err(String)` - Error message listing all searched paths if none found
///
/// Both current variants use this search order:
/// 1. $XDG_CONFIG_HOME/skhd/skhdrc
/// 2. ~/.config/skhd/skhdrc
/// 3. ~/.skhdrc
pub fn get_config_path_for_variant(variant: SkhdVariant) -> Result<String, String> {
    let home = env::var("HOME").map_err(|_| {
        "Failed to get HOME environment variable. \
         This is required to locate the skhd configuration."
            .to_string()
    })?;

    let paths = config_paths(&home, env::var("XDG_CONFIG_HOME").ok().as_deref());
    first_existing_config_path(&paths).ok_or_else(|| config_path_error(variant, &paths))
}

fn config_paths(home: &str, xdg_config_home: Option<&str>) -> Vec<String> {
    let default_config_home = format!("{}/.config", home);
    let config_home = xdg_config_home
        .filter(|path| !path.is_empty())
        .unwrap_or(&default_config_home);
    let mut paths = vec![
        format!("{}/skhd/skhdrc", config_home),
        format!("{}/.config/skhd/skhdrc", home),
        format!("{}/.skhdrc", home),
    ];
    paths.dedup();
    paths
}

fn first_existing_config_path(paths: &[String]) -> Option<String> {
    paths.iter().find(|path| Path::new(path).exists()).cloned()
}

fn config_path_error(variant: SkhdVariant, paths: &[String]) -> String {
    let variant_name = match variant {
        SkhdVariant::Original => "skhd",
        SkhdVariant::Zig => "skhd.zig",
    };
    let searched_paths = paths
        .iter()
        .map(|path| format!("- {}", path))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "No {} configuration file found. Searched in:\n{}\n\
         Create a configuration file in one of these locations.",
        variant_name, searched_paths
    )
}

/// Get all config paths that would be searched for a variant
///
/// Both variants currently use the same XDG-aware search order. The variant
/// argument is retained because callers select a path as part of variant dispatch.
pub fn get_config_paths_for_variant(_variant: SkhdVariant) -> Vec<String> {
    let home = env::var("HOME").unwrap_or_else(|_| String::from("~"));
    config_paths(&home, env::var("XDG_CONFIG_HOME").ok().as_deref())
}

/// Get the directory for skhd configuration files
pub fn get_config_dir() -> PathBuf {
    expand_path("~/.config/skhd")
}

/// Get the directory for application backups
pub fn get_backup_dir() -> PathBuf {
    expand_path("~/.config/skhd/backups")
}

/// Validate that a path is within the allowed skhd config directory
pub fn is_valid_config_path<P: AsRef<Path>>(path: P) -> bool {
    let expanded = expand_path(path);
    let config_dir = get_config_dir();

    expanded.starts_with(config_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_expand_tilde() {
        let expanded = expand_path("~/test/path");
        let home = env::var("HOME").unwrap();
        assert_eq!(expanded, PathBuf::from(home).join("test/path"));
    }

    #[test]
    fn test_expand_tilde_only() {
        let expanded = expand_path("~");
        let home = env::var("HOME").unwrap();
        assert_eq!(expanded, PathBuf::from(home));
    }

    #[test]
    fn test_no_expansion_needed() {
        let path = "/absolute/path";
        let expanded = expand_path(path);
        assert_eq!(expanded, PathBuf::from(path));
    }

    #[test]
    fn test_get_default_config_path() {
        let path = get_default_config_path();
        let home = env::var("HOME").unwrap();

        // The function returns the first existing path, or defaults to ~/.config/skhd/skhdrc
        // We need to check that the returned path is one of the valid options
        let valid_paths = [
            PathBuf::from(&home).join(".config/skhd/skhdrc"),
            PathBuf::from(&home).join(".skhdrc"),
        ];

        // If XDG_CONFIG_HOME is set, include that path too
        if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
            let xdg_path = PathBuf::from(xdg).join("skhd/skhdrc");
            assert!(
                valid_paths.contains(&path) || path == xdg_path,
                "Expected one of the valid config paths, got: {:?}",
                path
            );
        } else {
            assert!(
                valid_paths.contains(&path),
                "Expected one of the valid config paths, got: {:?}",
                path
            );
        }
    }

    #[test]
    fn test_get_config_dir() {
        let dir = get_config_dir();
        let home = env::var("HOME").unwrap();
        assert_eq!(dir, PathBuf::from(home).join(".config/skhd"));
    }

    #[test]
    fn test_is_valid_config_path() {
        // Valid path within config directory
        assert!(is_valid_config_path("~/.config/skhd/skhdrc"));
        assert!(is_valid_config_path("~/.config/skhd/custom.conf"));

        // Invalid paths outside config directory
        assert!(!is_valid_config_path("~/Documents/file.txt"));
        assert!(!is_valid_config_path("/etc/passwd"));
    }

    #[test]
    fn test_config_paths_without_xdg() {
        let paths = config_paths("/Users/test", None);

        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], "/Users/test/.config/skhd/skhdrc");
        assert_eq!(paths[1], "/Users/test/.skhdrc");
    }

    #[test]
    fn test_config_paths_with_xdg() {
        let paths = config_paths("/Users/test", Some("/Users/test/.xdg/config"));

        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0], "/Users/test/.xdg/config/skhd/skhdrc");
        assert_eq!(paths[1], "/Users/test/.config/skhd/skhdrc");
        assert_eq!(paths[2], "/Users/test/.skhdrc");
    }

    #[test]
    fn test_first_existing_config_path_uses_search_order() {
        let temp_dir = TempDir::new().unwrap();
        let first = temp_dir.path().join("first");
        let second = temp_dir.path().join("second");
        std::fs::write(&second, "config").unwrap();
        let paths = vec![
            first.to_string_lossy().into_owned(),
            second.to_string_lossy().into_owned(),
        ];

        assert_eq!(first_existing_config_path(&paths), Some(paths[1].clone()));
    }

    #[test]
    fn test_config_path_error_lists_all_paths() {
        let paths = config_paths("/nonexistent_home", Some("/nonexistent_xdg"));
        let error = config_path_error(SkhdVariant::Original, &paths);

        assert!(error.contains("/nonexistent_xdg/skhd/skhdrc"));
        assert!(error.contains("/nonexistent_home/.config/skhd/skhdrc"));
        assert!(error.contains("/nonexistent_home/.skhdrc"));
    }
}
