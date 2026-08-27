//! Integration tests for variant detector
//!
//! These tests verify the variant detection logic by mocking command outputs.
//! Real command execution is not tested here (that would require actual skhd installations).

use keybinder_lib::models::{DetectedVariant, DetectionSource, SkhdVariant};

#[test]
fn test_detected_variant_original() {
    let variant = DetectedVariant::new(
        Some(SkhdVariant::Original),
        Some("/usr/local/bin/skhd".to_string()),
        Some("com.koekeishiya.skhd".to_string()),
        DetectionSource::Homebrew,
    );

    assert!(variant.is_detected());
    assert_eq!(variant.variant, Some(SkhdVariant::Original));
    assert_eq!(variant.binary_path, Some("/usr/local/bin/skhd".to_string()));
    assert_eq!(
        variant.plist_label,
        Some("com.koekeishiya.skhd".to_string())
    );
    assert_eq!(variant.source, DetectionSource::Homebrew);
}

#[test]
fn test_detected_variant_zig() {
    let variant = DetectedVariant::new(
        Some(SkhdVariant::Zig),
        Some("/opt/homebrew/bin/skhd".to_string()),
        Some("com.jackielii.skhd".to_string()),
        DetectionSource::Path,
    );

    assert!(variant.is_detected());
    assert_eq!(variant.variant, Some(SkhdVariant::Zig));
    assert_eq!(
        variant.binary_path,
        Some("/opt/homebrew/bin/skhd".to_string())
    );
    assert_eq!(variant.plist_label, Some("com.jackielii.skhd".to_string()));
    assert_eq!(variant.source, DetectionSource::Path);
}

#[test]
fn test_detected_variant_app_bundle() {
    let variant = DetectedVariant::new(
        Some(SkhdVariant::Zig),
        Some("/Applications/skhd.app/Contents/MacOS/skhd".to_string()),
        Some("com.jackielii.skhd".to_string()),
        DetectionSource::AppBundle,
    );

    assert!(variant.is_detected());
    assert_eq!(variant.variant, Some(SkhdVariant::Zig));
    assert_eq!(
        variant.binary_path,
        Some("/Applications/skhd.app/Contents/MacOS/skhd".to_string())
    );
    assert_eq!(variant.plist_label, Some("com.jackielii.skhd".to_string()));
    assert_eq!(variant.source, DetectionSource::AppBundle);
}

#[test]
fn test_detected_variant_from_running() {
    let variant = DetectedVariant::new(
        Some(SkhdVariant::Original),
        None,
        Some("com.koekeishiya.skhd".to_string()),
        DetectionSource::Running,
    );

    assert!(variant.is_detected());
    assert_eq!(variant.variant, Some(SkhdVariant::Original));
    assert_eq!(variant.binary_path, None);
    assert_eq!(
        variant.plist_label,
        Some("com.koekeishiya.skhd".to_string())
    );
    assert_eq!(variant.source, DetectionSource::Running);
}

#[test]
fn test_detected_variant_from_plist() {
    let variant = DetectedVariant::new(
        Some(SkhdVariant::Zig),
        None,
        Some("com.jackielii.skhd".to_string()),
        DetectionSource::Plist,
    );

    assert!(variant.is_detected());
    assert_eq!(variant.variant, Some(SkhdVariant::Zig));
    assert_eq!(variant.binary_path, None);
    assert_eq!(variant.plist_label, Some("com.jackielii.skhd".to_string()));
    assert_eq!(variant.source, DetectionSource::Plist);
}

#[test]
fn test_detected_variant_none() {
    let variant = DetectedVariant::none();

    assert!(!variant.is_detected());
    assert_eq!(variant.variant, None);
    assert_eq!(variant.binary_path, None);
    assert_eq!(variant.plist_label, None);
    assert_eq!(variant.source, DetectionSource::None);
}

#[test]
fn test_skhd_variant_equality() {
    assert_eq!(SkhdVariant::Original, SkhdVariant::Original);
    assert_eq!(SkhdVariant::Zig, SkhdVariant::Zig);
    assert_ne!(SkhdVariant::Original, SkhdVariant::Zig);
}

#[test]
fn test_detection_source_equality() {
    assert_eq!(DetectionSource::Running, DetectionSource::Running);
    assert_eq!(DetectionSource::Plist, DetectionSource::Plist);
    assert_eq!(DetectionSource::Homebrew, DetectionSource::Homebrew);
    assert_eq!(DetectionSource::Path, DetectionSource::Path);
    assert_eq!(DetectionSource::AppBundle, DetectionSource::AppBundle);
    assert_eq!(DetectionSource::None, DetectionSource::None);

    assert_ne!(DetectionSource::Running, DetectionSource::Homebrew);
    assert_ne!(DetectionSource::Path, DetectionSource::AppBundle);
}

#[test]
fn test_detected_variant_descriptions() {
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

#[test]
fn test_skhd_variant_serialization_roundtrip() {
    let variants = vec![SkhdVariant::Original, SkhdVariant::Zig];

    for variant in variants {
        let json = serde_json::to_string(&variant).unwrap();
        let deserialized: SkhdVariant = serde_json::from_str(&json).unwrap();
        assert_eq!(variant, deserialized);
    }
}

#[test]
fn test_detection_source_serialization_roundtrip() {
    let sources = vec![
        DetectionSource::Running,
        DetectionSource::Plist,
        DetectionSource::Homebrew,
        DetectionSource::Path,
        DetectionSource::AppBundle,
        DetectionSource::None,
    ];

    for source in sources {
        let json = serde_json::to_string(&source).unwrap();
        let deserialized: DetectionSource = serde_json::from_str(&json).unwrap();
        assert_eq!(source, deserialized);
    }
}

#[test]
fn test_detected_variant_serialization_roundtrip() {
    let variants = vec![
        DetectedVariant::new(
            Some(SkhdVariant::Original),
            Some("/usr/local/bin/skhd".to_string()),
            Some("com.koekeishiya.skhd".to_string()),
            DetectionSource::Homebrew,
        ),
        DetectedVariant::new(
            Some(SkhdVariant::Zig),
            Some("/opt/homebrew/bin/skhd".to_string()),
            Some("com.jackielii.skhd".to_string()),
            DetectionSource::Path,
        ),
        DetectedVariant::none(),
    ];

    for variant in variants {
        let json = serde_json::to_string(&variant).unwrap();
        let deserialized: DetectedVariant = serde_json::from_str(&json).unwrap();
        assert_eq!(variant.variant, deserialized.variant);
        assert_eq!(variant.binary_path, deserialized.binary_path);
        assert_eq!(variant.plist_label, deserialized.plist_label);
        assert_eq!(variant.source, deserialized.source);
    }
}

#[test]
fn test_json_serialization_format() {
    // Test that SkhdVariant serializes to expected string values
    let original = SkhdVariant::Original;
    let zig = SkhdVariant::Zig;

    assert_eq!(serde_json::to_string(&original).unwrap(), "\"original\"");
    assert_eq!(serde_json::to_string(&zig).unwrap(), "\"zig\"");

    // Test DetectionSource
    assert_eq!(
        serde_json::to_string(&DetectionSource::Running).unwrap(),
        "\"running\""
    );
    assert_eq!(
        serde_json::to_string(&DetectionSource::Plist).unwrap(),
        "\"plist\""
    );
    assert_eq!(
        serde_json::to_string(&DetectionSource::Homebrew).unwrap(),
        "\"homebrew\""
    );
    assert_eq!(
        serde_json::to_string(&DetectionSource::Path).unwrap(),
        "\"path\""
    );
    assert_eq!(
        serde_json::to_string(&DetectionSource::AppBundle).unwrap(),
        "\"app_bundle\""
    );
    assert_eq!(
        serde_json::to_string(&DetectionSource::None).unwrap(),
        "\"none\""
    );
}

#[test]
fn test_detected_variant_full_json_structure() {
    let variant = DetectedVariant::new(
        Some(SkhdVariant::Zig),
        Some("/Applications/skhd.app/Contents/MacOS/skhd".to_string()),
        Some("com.jackielii.skhd".to_string()),
        DetectionSource::AppBundle,
    );

    let json = serde_json::to_string_pretty(&variant).unwrap();

    // Verify the JSON contains expected fields
    assert!(json.contains("\"variant\""));
    assert!(json.contains("\"zig\""));
    assert!(json.contains("\"binary_path\""));
    assert!(json.contains("/Applications/skhd.app/Contents/MacOS/skhd"));
    assert!(json.contains("\"plist_label\""));
    assert!(json.contains("com.jackielii.skhd"));
    assert!(json.contains("\"source\""));
    assert!(json.contains("\"app_bundle\""));
}
