/// Settings-related Tauri commands
use crate::models::{Settings, SkhdVariantSetting};
use crate::services::settings::{effective_variant_async, EffectiveVariantResult, SettingsManager};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

/// Response type for effective variant computation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectiveVariantResponse {
    /// The effective variant to use: 'original' | 'zig'
    pub variant: String,
    /// Whether the variant was auto-detected
    pub is_auto_detected: bool,
    /// Optional warning message if chosen variant is not installed
    pub warning: Option<String>,
    /// The detection source and metadata (if auto mode)
    pub detected: Option<crate::models::DetectedVariant>,
}

impl From<EffectiveVariantResult> for EffectiveVariantResponse {
    fn from(result: EffectiveVariantResult) -> Self {
        Self {
            variant: match result.variant {
                crate::models::SkhdVariant::Original => "original".to_string(),
                crate::models::SkhdVariant::Zig => "zig".to_string(),
            },
            is_auto_detected: result.is_auto_detected,
            warning: result.warning,
            detected: result.detected,
        }
    }
}

/// State for settings management
pub struct SettingsState {
    manager: Mutex<SettingsManager>,
}

impl SettingsState {
    pub fn new() -> Self {
        Self {
            manager: Mutex::new(SettingsManager::new()),
        }
    }
}

impl Default for SettingsState {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the current skhd variant setting
///
/// Returns the user's preference: 'auto', 'original', or 'zig'
#[tauri::command]
pub fn get_skhd_variant_setting(state: State<'_, SettingsState>) -> Result<String, String> {
    let manager = state.manager.lock().map_err(|e| e.to_string())?;
    let setting = manager.get_skhd_variant_setting();

    let value = match setting {
        SkhdVariantSetting::Auto => "auto",
        SkhdVariantSetting::Original => "original",
        SkhdVariantSetting::Zig => "zig",
    };

    Ok(value.to_string())
}

/// Set the skhd variant setting
///
/// # Arguments
/// * `value` - One of: 'auto', 'original', 'zig'
#[tauri::command]
pub fn set_skhd_variant_setting(
    value: String,
    state: State<'_, SettingsState>,
) -> Result<(), String> {
    let setting = match value.as_str() {
        "auto" => SkhdVariantSetting::Auto,
        "original" => SkhdVariantSetting::Original,
        "zig" => SkhdVariantSetting::Zig,
        _ => {
            return Err(format!(
                "Invalid variant setting: {}. Must be 'auto', 'original', or 'zig'",
                value
            ))
        }
    };

    let manager = state.manager.lock().map_err(|e| e.to_string())?;
    manager
        .set_skhd_variant_setting(setting)
        .map_err(|e| format!("Failed to save settings: {}", e))
}

/// Get the effective variant based on settings and detection
///
/// This command computes the actual variant to use, considering:
/// - If setting is 'auto': detects installed variant
/// - If setting is a specific variant: returns that variant with a warning if not installed
#[tauri::command]
pub async fn get_effective_variant() -> Result<EffectiveVariantResponse, String> {
    let result = effective_variant_async().await;
    Ok(result.into())
}

/// Get all settings
///
/// Returns the complete settings object
#[tauri::command]
pub fn get_settings(state: State<'_, SettingsState>) -> Result<Settings, String> {
    let manager = state.manager.lock().map_err(|e| e.to_string())?;
    let settings = Settings {
        skhd_variant: manager.get_skhd_variant_setting(),
    };
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effective_variant_response_from_result() {
        let result = EffectiveVariantResult {
            variant: crate::models::SkhdVariant::Original,
            is_auto_detected: true,
            warning: None,
            detected: Some(crate::models::DetectedVariant::none()),
        };

        let response: EffectiveVariantResponse = result.into();
        assert_eq!(response.variant, "original");
        assert!(response.is_auto_detected);
        assert!(response.warning.is_none());
    }

    #[test]
    fn test_effective_variant_response_zig() {
        let result = EffectiveVariantResult {
            variant: crate::models::SkhdVariant::Zig,
            is_auto_detected: false,
            warning: Some("Not installed".to_string()),
            detected: Some(crate::models::DetectedVariant::none()),
        };

        let response: EffectiveVariantResponse = result.into();
        assert_eq!(response.variant, "zig");
        assert!(!response.is_auto_detected);
        assert_eq!(response.warning, Some("Not installed".to_string()));
    }
}
