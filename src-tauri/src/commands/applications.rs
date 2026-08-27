use crate::models::application::Application;
use crate::services::app_discovery;

/// Tauri command to get all installed macOS applications
#[tauri::command]
pub async fn get_installed_applications() -> Result<Vec<Application>, String> {
    // Return apps without icon_path to avoid loading issues
    // Icons will be handled separately or shown as placeholders
    let mut apps = app_discovery::discover_applications()?;

    // Clear icon paths since .icns files can't be loaded directly in HTML
    for app in &mut apps {
        app.icon_path = None;
    }

    Ok(apps)
}

/// Tauri command to get an application icon as base64-encoded PNG
/// This is optional and can be implemented later for better UX
#[tauri::command]
pub async fn get_app_icon(_icon_path: String) -> Result<Option<String>, String> {
    // For now, return None - icons will use emoji placeholder
    // TODO: Implement .icns to PNG conversion using sips or image crate
    Ok(None)
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_installed_applications() {
        let result = get_installed_applications().await;
        assert!(result.is_ok());
        let apps = result.unwrap();
        assert!(apps.len() > 0, "Should find at least one application");
    }
}
