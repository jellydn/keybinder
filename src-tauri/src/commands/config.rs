/// Configuration management Tauri commands
use crate::models::{ConfigFile, Shortcut};
use crate::parser::parse_config;
use crate::services::file_io::{read_config_safe, write_config_atomic};
use crate::services::settings::effective_variant_async;
use crate::utils::path::{expand_path, get_config_path_for_variant, get_default_config_path};
use std::sync::Mutex;
use tauri::State;

/// Global state for the current configuration
pub struct ConfigState {
    pub config: Mutex<Option<ConfigFile>>,
}

impl Default for ConfigState {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigState {
    pub fn new() -> Self {
        Self {
            config: Mutex::new(None),
        }
    }
}

/// Helper: Load configuration from a specific path
///
/// This is the core loading logic used by load_config, import_config, and reload_config
fn load_config_from_path(
    path: &std::path::Path,
    state: &State<'_, ConfigState>,
) -> Result<ConfigFile, String> {
    // Read file
    let content =
        read_config_safe(path).map_err(|e| format!("Failed to read config file: {}", e))?;

    // Parse content
    let parsed = parse_config(&content)
        .map_err(|errors| format!("Failed to parse config: {} errors", errors.len()))?;

    // Convert parsed config to ConfigFile
    let path_str = path.to_string_lossy().to_string();
    let mut config = ConfigFile::new(path_str);

    for parsed_shortcut in parsed.shortcuts() {
        let shortcut = Shortcut::new(
            parsed_shortcut.modifiers.clone(),
            parsed_shortcut.key.clone(),
            parsed_shortcut.command.clone(),
            parsed_shortcut.line_number,
        );
        config.add_shortcut(shortcut);
    }

    // Extract comments
    for parsed_comment in parsed.comments() {
        config.global_comments.push(parsed_comment.text.clone());
    }

    // Reset modified flag since we just loaded
    config.is_modified = false;

    // Update state
    *state.config.lock().unwrap() = Some(config.clone());

    Ok(config)
}

/// Detect the active skhd configuration file path
///
/// Checks standard skhd config locations in order:
/// 1. $XDG_CONFIG_HOME/skhd/skhdrc
/// 2. ~/.config/skhd/skhdrc
/// 3. ~/.skhdrc
///
/// # Returns
/// * `Ok(String)` - Path to first existing config file
/// * `Err(String)` if no config file found
#[tauri::command]
pub fn detect_active_config() -> Result<String, String> {
    use std::env;
    use std::path::PathBuf;

    // Define config paths in order of precedence (matching skhd behavior)
    let config_paths: Vec<PathBuf> = vec![
        // 1. $XDG_CONFIG_HOME/skhd/skhdrc
        env::var("XDG_CONFIG_HOME")
            .ok()
            .map(|xdg| PathBuf::from(xdg).join("skhd/skhdrc"))
            .unwrap_or_else(|| expand_path("~/.config/skhd/skhdrc")),
        // 2. ~/.config/skhd/skhdrc
        expand_path("~/.config/skhd/skhdrc"),
        // 3. ~/.skhdrc
        expand_path("~/.skhdrc"),
    ];

    // Find first existing config file
    for path in config_paths {
        if path.exists() {
            return Ok(path.to_string_lossy().to_string());
        }
    }

    // No config file found in any standard location
    Err("No skhd configuration file found in standard locations".to_string())
}

/// Get the active skhd configuration file path based on effective variant
///
/// This variant-aware function checks config locations appropriate for the
/// currently active skhd variant (original or skhd.zig).
///
/// Both current variants use this search order:
/// 1. $XDG_CONFIG_HOME/skhd/skhdrc
/// 2. ~/.config/skhd/skhdrc
/// 3. ~/.skhdrc
///
/// # Returns
/// * `Ok(String)` - Path to first existing config file with detailed info
/// * `Err(String)` if no config file found (includes searched paths)
#[tauri::command]
pub async fn get_active_config_path() -> Result<ActiveConfigPathInfo, String> {
    let effective = effective_variant_async().await;
    let variant = effective.variant;

    match get_config_path_for_variant(variant) {
        Ok(path) => Ok(ActiveConfigPathInfo {
            path,
            variant: variant.to_string(),
            searched_paths: crate::utils::path::get_config_paths_for_variant(variant),
        }),
        Err(e) => Err(e),
    }
}

/// Information about the active config path
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActiveConfigPathInfo {
    /// The path to the active config file
    pub path: String,
    /// Which variant was used to find it
    pub variant: String,
    /// All paths that were searched
    pub searched_paths: Vec<String>,
}

/// Load skhd configuration from file
///
/// # Arguments
/// * `file_path` - Optional custom path (defaults to ~/.config/skhd/skhdrc)
/// * `state` - Application state
///
/// # Returns
/// * `Ok(ConfigFile)` on success
/// * `Err(String)` on failure
#[tauri::command]
pub fn load_config(
    file_path: Option<String>,
    state: State<'_, ConfigState>,
) -> Result<ConfigFile, String> {
    let path = if let Some(p) = file_path {
        expand_path(p)
    } else {
        get_default_config_path()
    };

    load_config_from_path(&path, &state)
}

/// Import configuration from custom file location via file picker
///
/// Opens a native macOS file dialog for user to select an skhd configuration file.
/// Loads and parses the selected file, updates the current configuration state.
///
/// # Arguments
/// * `state` - Application state
///
/// # Returns
/// * `Ok(ConfigFile)` - Loaded configuration with current_file_path set to selected path
/// * `Err(String)` - "Import cancelled" if user closes dialog, or parse/IO errors
#[tauri::command]
pub async fn import_config(state: State<'_, ConfigState>) -> Result<ConfigFile, String> {
    // Show file picker dialog
    // Note: skhd config files typically have no extension (just "skhdrc")
    // So we don't use extension filters - let users pick any file
    let file = rfd::AsyncFileDialog::new()
        .set_title("Import skhd Configuration")
        .set_directory(dirs::home_dir().unwrap_or_default().join(".config/skhd"))
        .pick_file()
        .await;

    // Handle user cancellation
    let file = match file {
        Some(f) => f,
        None => return Err("Import cancelled".to_string()),
    };

    let path = file.path().to_path_buf();

    // Load config from selected path
    load_config_from_path(&path, &state)
}

/// Export current configuration to custom file location via file picker
///
/// Opens a native macOS save dialog for user to choose export destination.
/// Serializes and validates the current configuration before writing.
///
/// # Arguments
/// * `state` - Application state
///
/// # Returns
/// * `Ok(String)` - Path where configuration was exported
/// * `Err(String)` - "Export cancelled" if user closes dialog, or validation/IO errors
#[tauri::command]
pub async fn export_config(state: State<'_, ConfigState>) -> Result<String, String> {
    // Get current config and serialize it (in separate scope to drop lock before await)
    let content = {
        let locked_config = state.config.lock().unwrap();
        let config = locked_config
            .as_ref()
            .ok_or("No configuration loaded")?
            .clone();
        drop(locked_config); // Release lock

        // Serialize configuration
        let serialized = serialize_config(&config);

        // Validate by attempting to parse
        parse_config(&serialized).map_err(|errors| {
            format!("Validation failed: {} syntax errors detected", errors.len())
        })?;

        serialized
    }; // Lock is definitely dropped here

    // Show save file dialog
    // Note: skhd config files typically have no extension (just "skhdrc")
    // So we don't use extension filters - just set default filename
    let file = rfd::AsyncFileDialog::new()
        .set_title("Export skhd Configuration")
        .set_file_name("skhdrc")
        .set_directory(dirs::home_dir().unwrap_or_default().join(".config/skhd"))
        .save_file()
        .await;

    // Handle user cancellation
    let file = match file {
        Some(f) => f,
        None => return Err("Export cancelled".to_string()),
    };

    let path = file.path();

    // Write atomically
    write_config_atomic(path, &content).map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(path.to_string_lossy().to_string())
}

/// Save configuration to file
///
/// # Arguments
/// * `config` - Configuration to save
/// * `state` - Application state
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(String)` on failure
#[tauri::command]
pub fn save_config(config: ConfigFile, state: State<'_, ConfigState>) -> Result<(), String> {
    // Serialize config back to skhd format
    let content = serialize_config(&config);

    // Write atomically
    write_config_atomic(&config.file_path, &content)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    // Update state with saved version
    let mut saved_config = config.clone();
    saved_config.is_modified = false;
    *state.config.lock().unwrap() = Some(saved_config);

    Ok(())
}

/// Save configuration to a user-selected file location via file picker
///
/// This is similar to export_config but updates the configuration's file_path
/// to the selected location for future saves.
///
/// # Arguments
/// * `config` - Configuration to save
/// * `state` - Application state
///
/// # Returns
/// * `Ok(ConfigFile)` - Updated config with new file_path on success
/// * `Err(String)` on failure or user cancellation
#[tauri::command]
pub async fn save_as_config(
    mut config: ConfigFile,
    state: State<'_, ConfigState>,
) -> Result<ConfigFile, String> {
    // Serialize config back to skhd format
    let content = serialize_config(&config);

    // Show save file dialog
    let file = rfd::AsyncFileDialog::new()
        .set_title("Save skhd Configuration")
        .set_file_name("skhdrc")
        .set_directory(dirs::home_dir().unwrap_or_default().join(".config/skhd"))
        .save_file()
        .await;

    // Handle user cancellation
    let file = match file {
        Some(f) => f,
        None => return Err("Save cancelled".to_string()),
    };

    let path = file.path();
    let path_str = path.to_string_lossy().to_string();

    // Write atomically
    write_config_atomic(path, &content).map_err(|e| format!("Failed to write config: {}", e))?;

    // Update config with new file path and mark as saved
    config.file_path = path_str.clone();
    config.current_file_path = path_str;
    config.is_modified = false;

    // Update state with saved version
    *state.config.lock().unwrap() = Some(config.clone());

    Ok(config)
}

/// Reload configuration from disk (discarding in-memory changes)
///
/// # Arguments
/// * `state` - Application state
///
/// # Returns
/// * `Ok(ConfigFile)` on success
/// * `Err(String)` on failure
#[tauri::command]
pub fn reload_config(state: State<'_, ConfigState>) -> Result<ConfigFile, String> {
    let current = state.config.lock().unwrap();
    let file_path = current
        .as_ref()
        .map(|c| c.file_path.clone())
        .ok_or("No config loaded")?;

    drop(current); // Release lock before calling load_config

    load_config(Some(file_path), state)
}

/// Serialize ConfigFile back to skhd configuration format
///
/// This ensures round-trip compatibility: parse → modify → serialize → parse
pub fn serialize_config(config: &ConfigFile) -> String {
    let mut output = String::new();

    // Add global comments at the top
    for comment in &config.global_comments {
        output.push_str("# ");
        output.push_str(comment);
        output.push('\n');
    }

    if !config.global_comments.is_empty() {
        output.push('\n');
    }

    // Sort shortcuts by line number to maintain order
    let mut shortcuts = config.shortcuts.clone();
    shortcuts.sort_by_key(|s| s.line_number);

    // Serialize each shortcut
    for shortcut in shortcuts {
        // Add comment if present
        if let Some(comment) = &shortcut.comment {
            output.push_str("# ");
            output.push_str(comment);
            output.push('\n');
        }

        // Build modifier string
        let modifier_str = if shortcut.modifiers.is_empty() {
            String::new()
        } else {
            let mut mods = shortcut.modifiers.clone();
            mods.sort(); // Ensure consistent ordering
            format!("{} ", mods.join(" + "))
        };

        // Write shortcut line: [modifiers] - key : command
        output.push_str(&format!(
            "{}- {} : {}\n",
            modifier_str, shortcut.key, shortcut.command
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_config() {
        let mut config = ConfigFile::new("/test/path".to_string());

        // Add global comment
        config.global_comments.push("Global config".to_string());

        // Add shortcuts
        let s1 = Shortcut::new(
            vec!["cmd".to_string()],
            "return".to_string(),
            "open -a Terminal".to_string(),
            1,
        );

        let s2 = Shortcut::new(
            vec!["cmd".to_string(), "shift".to_string()],
            "f".to_string(),
            "open ~".to_string(),
            2,
        );

        config.add_shortcut(s1);
        config.add_shortcut(s2);

        let serialized = serialize_config(&config);
        eprintln!("Serialized output:\n{}", serialized);

        // Verify format
        assert!(serialized.contains("# Global config"));
        assert!(serialized.contains("cmd - return : open -a Terminal"));
        assert!(serialized.contains("cmd + shift - f : open ~"));
    }

    #[test]
    fn test_serialize_config_no_modifiers() {
        let mut config = ConfigFile::new("/test/path".to_string());

        let shortcut = Shortcut::new(vec![], "f1".to_string(), "echo test".to_string(), 1);

        config.add_shortcut(shortcut);

        let serialized = serialize_config(&config);
        assert!(serialized.contains("- f1 : echo test"));
    }
}
