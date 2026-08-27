use crate::models::DetectedVariant;
use crate::services::detect_variant_async;

/// Detect which skhd variant is installed and active
///
/// Returns a DetectedVariant containing:
/// - variant: The detected variant (Original, Zig, or None)
/// - binary_path: Path to the binary if found
/// - plist_label: Launchd label (com.koekeishiya.skhd or com.jackielii.skhd)
/// - source: How the variant was detected (Running, Homebrew, Path, AppBundle, None)
#[tauri::command]
pub async fn detect_skhd_variant() -> Result<DetectedVariant, String> {
    let detected = detect_variant_async().await;
    Ok(detected)
}
