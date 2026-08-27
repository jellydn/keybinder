/// Settings management service
use crate::models::{DetectedVariant, Settings, SkhdVariant, SkhdVariantSetting};
use crate::services::file_io::write_config_atomic;
use crate::services::variant_detector::detect_variant;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Result from effective_variant computation
#[derive(Debug, Clone)]
pub struct EffectiveVariantResult {
    /// The effective variant to use
    pub variant: SkhdVariant,
    /// Whether the variant was auto-detected (true) or forced by setting (false)
    pub is_auto_detected: bool,
    /// Optional warning if the chosen variant is not installed
    pub warning: Option<String>,
    /// The actual detected variant info (if auto mode)
    pub detected: Option<DetectedVariant>,
}

/// Settings manager that handles persistence
pub struct SettingsManager {
    settings: Mutex<Settings>,
    config_dir: PathBuf,
}

impl SettingsManager {
    /// Create a new settings manager
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("keybinder");

        let settings = Self::load_settings(&config_dir);

        Self {
            settings: Mutex::new(settings),
            config_dir,
        }
    }

    /// Get the settings file path
    fn settings_path(&self) -> PathBuf {
        self.config_dir.join("settings.json")
    }

    /// Load settings from disk
    fn load_settings(config_dir: &Path) -> Settings {
        let settings_path = config_dir.join("settings.json");

        if let Ok(content) = fs::read_to_string(&settings_path) {
            if let Ok(settings) = serde_json::from_str::<Settings>(&content) {
                return settings;
            }
        }

        // Return defaults if file doesn't exist or is corrupted
        Settings::default()
    }

    /// Save settings to disk
    fn save_settings(&self, settings: &Settings) -> io::Result<()> {
        // Ensure config directory exists
        fs::create_dir_all(&self.config_dir)?;

        let settings_path = self.settings_path();
        let content = serde_json::to_string_pretty(settings)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        write_config_atomic(settings_path, &content)
    }

    /// Get the current skhd variant setting
    pub fn get_skhd_variant_setting(&self) -> SkhdVariantSetting {
        self.settings.lock().unwrap().skhd_variant
    }

    /// Set the skhd variant setting and persist it
    pub fn set_skhd_variant_setting(&self, setting: SkhdVariantSetting) -> io::Result<()> {
        let mut settings = self.settings.lock().unwrap();
        let mut updated_settings = settings.clone();
        updated_settings.skhd_variant = setting;
        self.save_settings(&updated_settings)?;
        *settings = updated_settings;

        Ok(())
    }

    /// Compute the effective variant based on setting and detection
    ///
    /// If the setting is Auto, this will run detection.
    /// If the setting is a specific variant, it returns that variant but
    /// includes a warning if that variant is not actually installed.
    pub fn effective_variant(&self) -> EffectiveVariantResult {
        let setting = self.get_skhd_variant_setting();

        match setting {
            SkhdVariantSetting::Auto => {
                let detected = detect_variant();

                if let Some(variant) = detected.variant {
                    EffectiveVariantResult {
                        variant,
                        is_auto_detected: true,
                        warning: None,
                        detected: Some(detected),
                    }
                } else {
                    // No variant detected, default to Original for backwards compatibility
                    EffectiveVariantResult {
                        variant: SkhdVariant::Original,
                        is_auto_detected: true,
                        warning: Some(
                            "No skhd variant detected. Install skhd or skhd.zig.".to_string(),
                        ),
                        detected: Some(detected),
                    }
                }
            }
            SkhdVariantSetting::Original => {
                let detected = detect_variant();
                let is_installed = detected
                    .variant
                    .map(|v| v == SkhdVariant::Original)
                    .unwrap_or(false);

                EffectiveVariantResult {
                    variant: SkhdVariant::Original,
                    is_auto_detected: false,
                    warning: if !is_installed {
                        Some(
                            "Original skhd is selected but not detected. Install with: brew install koekeishiya/formulae/skhd".to_string()
                        )
                    } else {
                        None
                    },
                    detected: Some(detected),
                }
            }
            SkhdVariantSetting::Zig => {
                let detected = detect_variant();
                let is_installed = detected
                    .variant
                    .map(|v| v == SkhdVariant::Zig)
                    .unwrap_or(false);

                EffectiveVariantResult {
                    variant: SkhdVariant::Zig,
                    is_auto_detected: false,
                    warning: if !is_installed {
                        Some(
                            "skhd.zig is selected but not detected. Install with: brew install jackielii/tap/skhd-zig".to_string()
                        )
                    } else {
                        None
                    },
                    detected: Some(detected),
                }
            }
        }
    }
}

impl Default for SettingsManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Async version for Tauri commands
pub async fn effective_variant_async() -> EffectiveVariantResult {
    // Run detection in a blocking task
    tokio::task::spawn_blocking(move || {
        let manager = SettingsManager::new();
        manager.effective_variant()
    })
    .await
    .unwrap_or_else(|_| EffectiveVariantResult {
        variant: SkhdVariant::Original,
        is_auto_detected: true,
        warning: Some("Failed to compute effective variant".to_string()),
        detected: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_settings_manager_load_default() {
        // Use a temp directory to ensure no existing settings interfere
        let temp_dir = TempDir::new().unwrap();
        let manager = SettingsManager {
            settings: Mutex::new(Settings::default()),
            config_dir: temp_dir.path().to_path_buf(),
        };

        let setting = manager.get_skhd_variant_setting();
        assert!(matches!(setting, SkhdVariantSetting::Auto));
    }

    #[test]
    fn test_settings_manager_save_and_load() {
        let temp_dir = TempDir::new().unwrap();

        // Create manager and change setting
        let manager = SettingsManager {
            settings: Mutex::new(Settings::default()),
            config_dir: temp_dir.path().to_path_buf(),
        };

        manager
            .set_skhd_variant_setting(SkhdVariantSetting::Zig)
            .unwrap();

        // Verify file was created
        let settings_path = temp_dir.path().join("settings.json");
        assert!(settings_path.exists());

        // Load the file and verify content
        let content = fs::read_to_string(&settings_path).unwrap();
        assert!(content.contains("\"zig\""));

        // Create new manager pointing to same directory and verify it loads
        let loaded_settings = SettingsManager::load_settings(temp_dir.path());
        assert!(matches!(
            loaded_settings.skhd_variant,
            SkhdVariantSetting::Zig
        ));
    }

    #[test]
    fn test_settings_manager_all_variant_values() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SettingsManager {
            settings: Mutex::new(Settings::default()),
            config_dir: temp_dir.path().to_path_buf(),
        };

        // Test Auto
        manager
            .set_skhd_variant_setting(SkhdVariantSetting::Auto)
            .unwrap();
        let setting = manager.get_skhd_variant_setting();
        assert!(matches!(setting, SkhdVariantSetting::Auto));

        // Test Original
        manager
            .set_skhd_variant_setting(SkhdVariantSetting::Original)
            .unwrap();
        let setting = manager.get_skhd_variant_setting();
        assert!(matches!(setting, SkhdVariantSetting::Original));

        // Test Zig
        manager
            .set_skhd_variant_setting(SkhdVariantSetting::Zig)
            .unwrap();
        let setting = manager.get_skhd_variant_setting();
        assert!(matches!(setting, SkhdVariantSetting::Zig));
    }
}
