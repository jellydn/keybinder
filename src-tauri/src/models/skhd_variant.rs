/// skhd variant detection and types
use serde::{Deserialize, Serialize};

/// Represents which skhd variant is installed/active
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkhdVariant {
    /// Original koekeishiya/skhd
    #[serde(rename = "original")]
    Original,
    /// jackielii/skhd.zig fork
    #[serde(rename = "zig")]
    Zig,
}

/// Represents how the variant was detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetectionSource {
    /// Detected from a running launchd job
    #[serde(rename = "running")]
    Running,
    /// Detected from a plist file
    #[serde(rename = "plist")]
    Plist,
    /// Detected via Homebrew
    #[serde(rename = "homebrew")]
    Homebrew,
    /// Detected from PATH (binary found)
    #[serde(rename = "path")]
    Path,
    /// Detected from .app bundle
    #[serde(rename = "app_bundle")]
    AppBundle,
    /// No variant detected
    #[serde(rename = "none")]
    None,
}

/// Represents a detected skhd variant with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedVariant {
    /// Which variant was detected (None if not detected)
    pub variant: Option<SkhdVariant>,
    /// Path to the binary if found
    pub binary_path: Option<String>,
    /// Launchd plist label (com.koekeishiya.skhd or com.jackielii.skhd)
    pub plist_label: Option<String>,
    /// How the variant was detected
    pub source: DetectionSource,
}

impl DetectedVariant {
    /// Create a new detected variant
    pub fn new(
        variant: Option<SkhdVariant>,
        binary_path: Option<String>,
        plist_label: Option<String>,
        source: DetectionSource,
    ) -> Self {
        Self {
            variant,
            binary_path,
            plist_label,
            source,
        }
    }

    /// Create a "none detected" result
    pub fn none() -> Self {
        Self {
            variant: None,
            binary_path: None,
            plist_label: None,
            source: DetectionSource::None,
        }
    }

    /// Check if a variant was detected
    pub fn is_detected(&self) -> bool {
        self.variant.is_some()
    }

    /// Get a human-readable description
    pub fn description(&self) -> String {
        match self.variant {
            Some(SkhdVariant::Original) => "skhd (original)".to_string(),
            Some(SkhdVariant::Zig) => "skhd.zig".to_string(),
            None => "Not detected".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skhd_variant_serialization() {
        let original = SkhdVariant::Original;
        let zig = SkhdVariant::Zig;

        let original_json = serde_json::to_string(&original).unwrap();
        let zig_json = serde_json::to_string(&zig).unwrap();

        assert_eq!(original_json, "\"original\"");
        assert_eq!(zig_json, "\"zig\"");
    }

    #[test]
    fn test_detection_source_serialization() {
        let sources = vec![
            DetectionSource::Running,
            DetectionSource::Plist,
            DetectionSource::Homebrew,
            DetectionSource::Path,
            DetectionSource::AppBundle,
            DetectionSource::None,
        ];

        let expected = vec![
            "\"running\"",
            "\"plist\"",
            "\"homebrew\"",
            "\"path\"",
            "\"app_bundle\"",
            "\"none\"",
        ];

        for (source, exp) in sources.iter().zip(expected.iter()) {
            let json = serde_json::to_string(source).unwrap();
            assert_eq!(json, *exp);
        }
    }

    #[test]
    fn test_detected_variant_detected() {
        let detected = DetectedVariant::new(
            Some(SkhdVariant::Original),
            Some("/usr/local/bin/skhd".to_string()),
            Some("com.koekeishiya.skhd".to_string()),
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
            DetectionSource::Homebrew,
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
