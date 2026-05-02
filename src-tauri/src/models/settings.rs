/// App settings model
use serde::{Deserialize, Serialize};

/// User's preferred skhd variant setting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkhdVariantSetting {
    /// Automatically detect the variant (default)
    #[serde(rename = "auto")]
    Auto,
    /// Force use of original skhd
    #[serde(rename = "original")]
    Original,
    /// Force use of skhd.zig
    #[serde(rename = "zig")]
    Zig,
}

impl Default for SkhdVariantSetting {
    fn default() -> Self {
        Self::Auto
    }
}

impl SkhdVariantSetting {
    /// Check if this setting is Auto
    pub fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }

    /// Convert to a specific SkhdVariant if not Auto
    pub fn to_variant(&self) -> Option<super::SkhdVariant> {
        match self {
            Self::Auto => None,
            Self::Original => Some(super::SkhdVariant::Original),
            Self::Zig => Some(super::SkhdVariant::Zig),
        }
    }
}

/// App settings that persist between launches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// User's preferred skhd variant
    #[serde(default)]
    pub skhd_variant: SkhdVariantSetting,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            skhd_variant: SkhdVariantSetting::default(),
        }
    }
}

impl Settings {
    /// Create new settings with defaults
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{SkhdVariant, SkhdVariantSetting};

    #[test]
    fn test_skhd_variant_setting_default() {
        let default = SkhdVariantSetting::default();
        assert!(matches!(default, SkhdVariantSetting::Auto));
    }

    #[test]
    fn test_skhd_variant_setting_is_auto() {
        assert!(SkhdVariantSetting::Auto.is_auto());
        assert!(!SkhdVariantSetting::Original.is_auto());
        assert!(!SkhdVariantSetting::Zig.is_auto());
    }

    #[test]
    fn test_skhd_variant_setting_to_variant() {
        assert_eq!(SkhdVariantSetting::Auto.to_variant(), None);
        assert_eq!(
            SkhdVariantSetting::Original.to_variant(),
            Some(SkhdVariant::Original)
        );
        assert_eq!(SkhdVariantSetting::Zig.to_variant(), Some(SkhdVariant::Zig));
    }

    #[test]
    fn test_skhd_variant_setting_serialization() {
        let auto_json = serde_json::to_string(&SkhdVariantSetting::Auto).unwrap();
        let original_json = serde_json::to_string(&SkhdVariantSetting::Original).unwrap();
        let zig_json = serde_json::to_string(&SkhdVariantSetting::Zig).unwrap();

        assert_eq!(auto_json, "\"auto\"");
        assert_eq!(original_json, "\"original\"");
        assert_eq!(zig_json, "\"zig\"");
    }

    #[test]
    fn test_skhd_variant_setting_deserialization() {
        let auto: SkhdVariantSetting = serde_json::from_str("\"auto\"").unwrap();
        let original: SkhdVariantSetting = serde_json::from_str("\"original\"").unwrap();
        let zig: SkhdVariantSetting = serde_json::from_str("\"zig\"").unwrap();

        assert!(matches!(auto, SkhdVariantSetting::Auto));
        assert!(matches!(original, SkhdVariantSetting::Original));
        assert!(matches!(zig, SkhdVariantSetting::Zig));
    }

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert!(matches!(settings.skhd_variant, SkhdVariantSetting::Auto));
    }

    #[test]
    fn test_settings_new() {
        let settings = Settings::new();
        assert!(matches!(settings.skhd_variant, SkhdVariantSetting::Auto));
    }

    #[test]
    fn test_settings_serialization() {
        let settings = Settings::new();
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"skhd_variant\""));
        assert!(json.contains("\"auto\""));
    }

    #[test]
    fn test_settings_deserialization() {
        let json = r#"{"skhd_variant": "zig"}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert!(matches!(settings.skhd_variant, SkhdVariantSetting::Zig));
    }

    #[test]
    fn test_settings_deserialization_default() {
        // Missing field should use default
        let json = r#"{}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert!(matches!(settings.skhd_variant, SkhdVariantSetting::Auto));
    }
}
