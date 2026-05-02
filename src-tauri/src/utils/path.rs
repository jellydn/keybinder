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
/// ## Original variant search order:
/// 1. ~/.config/skhd/skhdrc
/// 2. ~/.skhdrc
///
/// ## Zig variant search order:
/// 1. $XDG_CONFIG_HOME/skhd/skhdrc
/// 2. ~/.config/skhd/skhdrc
/// 3. ~/.skhdrc
pub fn get_config_path_for_variant(variant: SkhdVariant) -> Result<String, String> {
    let home = env::var("HOME").map_err(|_| {
        "Failed to get HOME environment variable. \
         This is required to locate the skhd configuration."
            .to_string()
    })?;

    match variant {
        SkhdVariant::Original => get_config_path_original(&home),
        SkhdVariant::Zig => get_config_path_zig(&home),
    }
}

/// Get config path for original skhd variant
fn get_config_path_original(home: &str) -> Result<String, String> {
    let paths = vec![
        format!("{}/.config/skhd/skhdrc", home),
        format!("{}/.skhdrc", home),
    ];

    let mut searched_paths = Vec::new();

    for path in &paths {
        searched_paths.push(path.clone());
        if Path::new(path).exists() {
            return Ok(path.clone());
        }
    }

    Err(format!(
        "No skhd configuration file found. Searched in:\n\
         - {}\n\
         - {}\n\
         Create a configuration file in one of these locations.",
        searched_paths[0], searched_paths[1]
    ))
}

/// Get config path for skhd.zig variant
/// Checks XDG-style paths first, then falls back to standard locations
fn get_config_path_zig(home: &str) -> Result<String, String> {
    // XDG-style paths (skhd.zig uses these)
    let xdg_config_home =
        env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{}/.config", home));

    let paths = vec![
        format!("{}/skhd/skhdrc", xdg_config_home),
        format!("{}/.config/skhd/skhdrc", home),
        format!("{}/.skhdrc", home),
    ];

    let mut searched_paths = Vec::new();

    for path in &paths {
        searched_paths.push(path.clone());
        if Path::new(path).exists() {
            return Ok(path.clone());
        }
    }

    Err(format!(
        "No skhd configuration file found. Searched in:\n\
         - {} (XDG_CONFIG_HOME)\n\
         - {}\n\
         - {}\n\
         Create a configuration file in one of these locations.",
        searched_paths[0], searched_paths[1], searched_paths[2]
    ))
}

/// Get all config paths that would be searched for a variant
///
/// Returns the list of paths without checking if they exist
pub fn get_config_paths_for_variant(variant: SkhdVariant) -> Vec<String> {
    let home = env::var("HOME").unwrap_or_else(|_| String::from("~"));

    match variant {
        SkhdVariant::Original => {
            vec![
                format!("{}/.config/skhd/skhdrc", home),
                format!("{}/.skhdrc", home),
            ]
        }
        SkhdVariant::Zig => {
            let xdg_config_home =
                env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{}/.config", home));
            vec![
                format!("{}/skhd/skhdrc", xdg_config_home),
                format!("{}/.config/skhd/skhdrc", home),
                format!("{}/.skhdrc", home),
            ]
        }
    }
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
        let valid_paths = vec![
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

    // Variant-aware config path tests
    use crate::models::SkhdVariant;

    #[test]
    fn test_get_config_paths_for_variant_original() {
        env::set_var("HOME", "/Users/test");
        env::remove_var("XDG_CONFIG_HOME");

        let paths = get_config_paths_for_variant(SkhdVariant::Original);

        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], "/Users/test/.config/skhd/skhdrc");
        assert_eq!(paths[1], "/Users/test/.skhdrc");
    }

    #[test]
    fn test_get_config_paths_for_variant_zig_without_xdg() {
        env::set_var("HOME", "/Users/test");
        env::remove_var("XDG_CONFIG_HOME");

        let paths = get_config_paths_for_variant(SkhdVariant::Zig);

        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0], "/Users/test/.config/skhd/skhdrc"); // XDG fallback
        assert_eq!(paths[1], "/Users/test/.config/skhd/skhdrc");
        assert_eq!(paths[2], "/Users/test/.skhdrc");
    }

    #[test]
    fn test_get_config_paths_for_variant_zig_with_xdg() {
        env::set_var("HOME", "/Users/test");
        env::set_var("XDG_CONFIG_HOME", "/Users/test/.xdg/config");

        let paths = get_config_paths_for_variant(SkhdVariant::Zig);

        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0], "/Users/test/.xdg/config/skhd/skhdrc");
        assert_eq!(paths[1], "/Users/test/.config/skhd/skhdrc");
        assert_eq!(paths[2], "/Users/test/.skhdrc");

        env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    fn test_get_config_path_for_variant_original_error_lists_paths() {
        env::set_var("HOME", "/nonexistent_home_original");

        let result = get_config_path_for_variant(SkhdVariant::Original);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("/nonexistent_home_original/.config/skhd/skhdrc"));
        assert!(err.contains("/nonexistent_home_original/.skhdrc"));
    }

    #[test]
    fn test_get_config_path_for_variant_zig_error_lists_paths() {
        env::set_var("HOME", "/nonexistent_home_zig");
        env::remove_var("XDG_CONFIG_HOME");

        let result = get_config_path_for_variant(SkhdVariant::Zig);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("XDG_CONFIG_HOME"));
        assert!(err.contains("/nonexistent_home_zig/.config/skhd/skhdrc"));
        assert!(err.contains("/nonexistent_home_zig/.skhdrc"));
    }
}
