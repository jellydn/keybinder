use serde::{Deserialize, Serialize};
use std::process::Command;
use tokio::sync::Mutex;

use crate::models::{ServiceState, ServiceStatus, SkhdVariant};
use crate::services::settings::{effective_variant_async, EffectiveVariantResult};

/// Error type for service operations
#[derive(Debug, Clone)]
pub struct ServiceError {
    pub variant: SkhdVariant,
    pub message: String,
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let variant_name = match self.variant {
            SkhdVariant::Original => "skhd",
            SkhdVariant::Zig => "skhd.zig",
        };
        write!(f, "{}: {}", variant_name, self.message)
    }
}

impl std::error::Error for ServiceError {}

/// Service manager that dispatches based on the effective skhd variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceManager {
    #[serde(skip)]
    reload_lock: std::sync::Arc<Mutex<()>>,
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceManager {
    pub fn new() -> Self {
        Self {
            reload_lock: std::sync::Arc::new(Mutex::new(())),
        }
    }

    /// Get the current status of the skhd service
    pub async fn get_status(&self) -> Result<ServiceStatus, String> {
        let effective = effective_variant_async().await;
        match effective.variant {
            SkhdVariant::Original => self.get_status_original().await,
            SkhdVariant::Zig => self.get_status_zig(&effective).await,
        }
    }

    /// Get status for original skhd (koekeishiya)
    async fn get_status_original(&self) -> Result<ServiceStatus, String> {
        self.get_status_from_launchctl("com.koekeishiya.skhd").await
    }

    /// Get status for skhd.zig (jackielii)
    async fn get_status_zig(&self, effective: &EffectiveVariantResult) -> Result<ServiceStatus, String> {
        // First check launchctl list for com.jackielii.skhd
        let launchctl_status = self.get_status_from_launchctl("com.jackielii.skhd").await?;
        
        // If we got a valid state, return it
        if !matches!(launchctl_status.state, ServiceState::Unknown) {
            return Ok(launchctl_status);
        }

        // If launchctl doesn't show the service, try `skhd --status`
        if let Some(ref detected) = effective.detected {
            if let Some(ref binary_path) = detected.binary_path {
                return self.get_status_from_skhd_command(binary_path).await;
            }
        }

        // Check PATH for skhd binary
        if let Ok(binary_path) = self.get_skhd_binary_path_from_path().await {
            return self.get_status_from_skhd_command(&binary_path).await;
        }

        // Service not found
        Ok(ServiceStatus {
            state: ServiceState::Unknown,
            pid: None,
            last_updated: chrono::Utc::now(),
            config_path: self.get_active_config_path_zig().await.ok(),
            error_message: Some(
                "skhd.zig service not found. Install skhd.zig and register the service with: \
                 skhd --install-service && skhd --start-service"
                    .to_string(),
            ),
        })
    }

    /// Get status from launchctl list output for a specific label
    async fn get_status_from_launchctl(&self, label: &str) -> Result<ServiceStatus, String> {
        let output = Command::new("launchctl")
            .arg("list")
            .output()
            .map_err(|e| {
                format!(
                    "Failed to execute launchctl command: {}. \
                     Make sure you're running on macOS and launchctl is available.",
                    e
                )
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Look for the service in the output
        for line in stdout.lines() {
            if line.contains(label) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let pid_str = parts[0];
                    let status_code = parts[1];

                    let (state, pid, error_message) = if pid_str == "-" {
                        (ServiceState::Stopped, None, None)
                    } else if status_code != "0" {
                        (
                            ServiceState::Error,
                            pid_str.parse().ok(),
                            Some(format!("Service exited with code {}", status_code)),
                        )
                    } else {
                        (ServiceState::Running, pid_str.parse().ok(), None)
                    };

                    let config_path = match label {
                        "com.koekeishiya.skhd" => self.get_active_config_path_original().await.ok(),
                        _ => self.get_active_config_path_zig().await.ok(),
                    };

                    return Ok(ServiceStatus {
                        state,
                        pid,
                        last_updated: chrono::Utc::now(),
                        config_path,
                        error_message,
                    });
                }
            }
        }

        // Service not found in launchctl list
        Ok(ServiceStatus {
            state: ServiceState::Unknown,
            pid: None,
            last_updated: chrono::Utc::now(),
            config_path: None,
            error_message: Some(format!(
                "skhd service not found in launchctl list (label: {}).",
                label
            )),
        })
    }

    /// Get status from skhd --status command (for skhd.zig)
    async fn get_status_from_skhd_command(&self, binary_path: &str) -> Result<ServiceStatus, String> {
        let output = Command::new(binary_path)
            .arg("--status")
            .output()
            .ok();

        let (state, error_message) = if let Some(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Check if the status indicates running
                if stdout.contains("running") || stdout.contains("active") || stdout.contains("started") {
                    (ServiceState::Running, None)
                } else if stdout.contains("stopped") || stdout.contains("inactive") {
                    (ServiceState::Stopped, None)
                } else {
                    (ServiceState::Unknown, Some(format!("Status: {}", stdout.trim())))
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                (ServiceState::Error, Some(format!("skhd --status failed: {}", stderr.trim())))
            }
        } else {
            (ServiceState::Unknown, Some("Failed to run skhd --status".to_string()))
        };

        Ok(ServiceStatus {
            state,
            pid: None, // skhd.zig doesn't expose PID via --status
            last_updated: chrono::Utc::now(),
            config_path: self.get_active_config_path_zig().await.ok(),
            error_message,
        })
    }

    /// Find skhd binary in PATH
    async fn get_skhd_binary_path_from_path(&self) -> Result<String, String> {
        let output = Command::new("which")
            .arg("skhd")
            .output()
            .map_err(|e| format!("Failed to find skhd in PATH: {}", e))?;

        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                Ok(path)
            } else {
                Err("skhd not found in PATH".to_string())
            }
        } else {
            Err("skhd not found in PATH".to_string())
        }
    }

    /// Stop the skhd service
    pub async fn stop_service(&self) -> Result<(), String> {
        let effective = effective_variant_async().await;
        match effective.variant {
            SkhdVariant::Original => self.stop_service_original().await,
            SkhdVariant::Zig => self.stop_service_zig(&effective).await,
        }
    }

    /// Stop service for original skhd
    async fn stop_service_original(&self) -> Result<(), String> {
        let plist_path = self.get_plist_path_original()?;

        let output = Command::new("launchctl")
            .arg("bootout")
            .arg("gui/$(id - u)")
            .arg(&plist_path)
            .output()
            .map_err(|e| {
                format!(
                    "skhd: Failed to execute launchctl bootout: {}. \
                     Check that you have permission to control launchd services.",
                    e
                )
            })?;

        // Also try the older unload command for backwards compatibility
        if !output.status.success() {
            let _ = Command::new("launchctl")
                .arg("unload")
                .arg(&plist_path)
                .output();
        }

        // Verify the service was stopped
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let status = self.get_status_original().await?;
        
        if matches!(status.state, ServiceState::Running) {
            return Err("skhd: Failed to stop service. The service is still running.".to_string());
        }

        Ok(())
    }

    /// Stop service for skhd.zig
    async fn stop_service_zig(&self, effective: &EffectiveVariantResult) -> Result<(), String> {
        let binary_path = self.get_skhd_binary_path(effective).await?;

        let output = Command::new(&binary_path)
            .arg("--stop-service")
            .output()
            .map_err(|e| {
                format!(
                    "skhd.zig: Failed to execute skhd --stop-service: {}. \
                     Make sure skhd.zig is installed and available in PATH.",
                    e
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "skhd.zig: Failed to stop service: {}. \
                 The service may not be running.",
                stderr.trim()
            ));
        }

        // Verify the service was stopped
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let status = self.get_status_zig(effective).await?;
        
        if matches!(status.state, ServiceState::Running) {
            return Err("skhd.zig: Failed to stop service. The service is still running.".to_string());
        }

        Ok(())
    }

    /// Start the skhd service
    pub async fn start_service(&self) -> Result<(), String> {
        let effective = effective_variant_async().await;
        match effective.variant {
            SkhdVariant::Original => self.start_service_original().await,
            SkhdVariant::Zig => self.start_service_zig(&effective).await,
        }
    }

    /// Start service for original skhd
    async fn start_service_original(&self) -> Result<(), String> {
        let plist_path = self.get_plist_path_original()?;

        let output = Command::new("launchctl")
            .arg("bootstrap")
            .arg("gui/$(id - u)")
            .arg(&plist_path)
            .output()
            .map_err(|e| {
                format!(
                    "skhd: Failed to execute launchctl bootstrap: {}. \
                     Check that you have permission to control launchd services.",
                    e
                )
            })?;

        // Also try the older load command for backwards compatibility
        if !output.status.success() {
            let _ = Command::new("launchctl")
                .arg("load")
                .arg(&plist_path)
                .output();
        }

        // Wait for service to start
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Verify service started
        let status = self.get_status_original().await?;
        match status.state {
            ServiceState::Running => Ok(()),
            ServiceState::Error => Err(status.error_message.unwrap_or_else(|| {
                "skhd: Service failed to start. Check your skhd configuration for syntax errors.".to_string()
            })),
            _ => Err(format!(
                "skhd: Service in unexpected state: {:?}. Try restarting the service manually with: \
                 brew services restart skhd",
                status.state
            )),
        }
    }

    /// Start service for skhd.zig
    async fn start_service_zig(&self, effective: &EffectiveVariantResult) -> Result<(), String> {
        let binary_path = self.get_skhd_binary_path(effective).await?;

        let output = Command::new(&binary_path)
            .arg("--start-service")
            .output()
            .map_err(|e| {
                format!(
                    "skhd.zig: Failed to execute skhd --start-service: {}. \
                     Make sure skhd.zig is installed and available in PATH.",
                    e
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "skhd.zig: Failed to start service: {}. \
                 Check that skhd.zig is installed and the service is registered.",
                stderr.trim()
            ));
        }

        // Wait for service to start
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Verify service started
        let status = self.get_status_zig(effective).await?;
        match status.state {
            ServiceState::Running => Ok(()),
            ServiceState::Error => Err(status.error_message.unwrap_or_else(|| {
                "skhd.zig: Service failed to start. Check your skhd configuration for syntax errors.".to_string()
            })),
            _ => Err(format!(
                "skhd.zig: Service in unexpected state: {:?}. Try restarting the service manually with: \
                 skhd --restart-service",
                status.state
            )),
        }
    }

    /// Restart the skhd service
    pub async fn restart_service(&self) -> Result<(), String> {
        let effective = effective_variant_async().await;
        match effective.variant {
            SkhdVariant::Original => self.restart_service_original().await,
            SkhdVariant::Zig => self.restart_service_zig(&effective).await,
        }
    }

    /// Restart service for original skhd
    async fn restart_service_original(&self) -> Result<(), String> {
        let output = Command::new("brew")
            .args(["services", "restart", "skhd"])
            .output()
            .map_err(|e| {
                format!(
                    "skhd: Failed to execute brew services restart: {}. \
                     Make sure Homebrew is installed and skhd was installed via Homebrew.",
                    e
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // If brew services fails, try manual stop/start
            if self.stop_service_original().await.is_ok() {
                return self.start_service_original().await;
            }
            return Err(format!(
                "skhd: Failed to restart service: {}.",
                stderr.trim()
            ));
        }

        // Wait for service to restart
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Verify service is running
        let status = self.get_status_original().await?;
        if !matches!(status.state, ServiceState::Running) {
            return Err("skhd: Service failed to restart. Check the service status.".to_string());
        }

        Ok(())
    }

    /// Restart service for skhd.zig
    async fn restart_service_zig(&self, effective: &EffectiveVariantResult) -> Result<(), String> {
        let binary_path = self.get_skhd_binary_path(effective).await?;

        let output = Command::new(&binary_path)
            .arg("--restart-service")
            .output()
            .map_err(|e| {
                format!(
                    "skhd.zig: Failed to execute skhd --restart-service: {}. \
                     Make sure skhd.zig is installed and available in PATH.",
                    e
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "skhd.zig: Failed to restart service: {}.",
                stderr.trim()
            ));
        }

        // Wait for service to restart
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Verify service is running
        let status = self.get_status_zig(effective).await?;
        if !matches!(status.state, ServiceState::Running) {
            return Err("skhd.zig: Service failed to restart. Check the service status.".to_string());
        }

        Ok(())
    }

    /// Reload the skhd service configuration
    ///
    /// This method acquires a lock to prevent concurrent reloads.
    /// The lock is automatically released when the function returns (RAII pattern),
    /// even in case of errors or panics.
    pub async fn reload_service(&self) -> Result<(), String> {
        // Acquire lock to prevent concurrent reloads
        let _lock = self.reload_lock.lock().await;

        let effective = effective_variant_async().await;
        let result = match effective.variant {
            SkhdVariant::Original => self.reload_service_original().await,
            SkhdVariant::Zig => self.reload_service_zig(&effective).await,
        };

        // Lock is automatically released here when _lock goes out of scope
        result
    }

    /// Reload service for original skhd
    async fn reload_service_original(&self) -> Result<(), String> {
        let output = Command::new("skhd")
            .arg("--reload")
            .output()
            .map_err(|e| {
                format!(
                    "skhd: Failed to execute skhd --reload: {}. \
                     Make sure skhd is installed and available in PATH.",
                    e
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "skhd: Failed to reload service: {}. \
                 The service may not be running. Try starting it first.",
                stderr.trim()
            ));
        }

        Ok(())
    }

    /// Reload service for skhd.zig
    async fn reload_service_zig(&self, effective: &EffectiveVariantResult) -> Result<(), String> {
        let binary_path = self.get_skhd_binary_path(effective).await?;

        let output = Command::new(&binary_path)
            .arg("--reload")
            .output()
            .map_err(|e| {
                format!(
                    "skhd.zig: Failed to execute skhd --reload: {}. \
                     Make sure skhd.zig is installed and available in PATH.",
                    e
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "skhd.zig: Failed to reload service: {}. \
                 The service may not be running. Try starting it first.",
                stderr.trim()
            ));
        }

        Ok(())
    }

    /// Install the skhd.zig service
    /// This is only applicable for skhd.zig variant
    pub async fn install_service(&self) -> Result<(), String> {
        let effective = effective_variant_async().await;
        if matches!(effective.variant, SkhdVariant::Original) {
            return Err(
                "skhd: Service installation is handled via the plist file. \
                 Install skhd via Homebrew and use 'brew services start skhd'."
                    .to_string(),
            );
        }

        let binary_path = self.get_skhd_binary_path(&effective).await?;

        let output = Command::new(&binary_path)
            .arg("--install-service")
            .output()
            .map_err(|e| {
                format!(
                    "skhd.zig: Failed to execute skhd --install-service: {}. \
                     Make sure skhd.zig is installed.",
                    e
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "skhd.zig: Failed to install service: {}.",
                stderr.trim()
            ));
        }

        Ok(())
    }

    /// Uninstall the skhd.zig service
    /// This is only applicable for skhd.zig variant
    pub async fn uninstall_service(&self) -> Result<(), String> {
        let effective = effective_variant_async().await;
        if matches!(effective.variant, SkhdVariant::Original) {
            return Err(
                "skhd: Service uninstallation is handled via the plist file. \
                 Stop skhd with 'brew services stop skhd'."
                    .to_string(),
            );
        }

        let binary_path = self.get_skhd_binary_path(&effective).await?;

        let output = Command::new(&binary_path)
            .arg("--uninstall-service")
            .output()
            .map_err(|e| {
                format!(
                    "skhd.zig: Failed to execute skhd --uninstall-service: {}. \
                     Make sure skhd.zig is installed.",
                    e
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "skhd.zig: Failed to uninstall service: {}.",
                stderr.trim()
            ));
        }

        Ok(())
    }

    /// Get the path to the skhd binary
    async fn get_skhd_binary_path(&self, effective: &EffectiveVariantResult) -> Result<String, String> {
        // First, check if we have a detected binary path
        if let Some(ref detected) = effective.detected {
            if let Some(ref binary_path) = detected.binary_path {
                return Ok(binary_path.clone());
            }
        }

        // Try to find skhd in PATH
        self.get_skhd_binary_path_from_path().await
    }

    /// Get the path to the skhd launchd plist file for original skhd
    fn get_plist_path_original(&self) -> Result<String, String> {
        let home = std::env::var("HOME").map_err(|_| {
            "Failed to get HOME environment variable. \
             This is required to locate the skhd plist file."
                .to_string()
        })?;

        Ok(format!(
            "{}/Library/LaunchAgents/com.koekeishiya.skhd.plist",
            home
        ))
    }

    /// Get the active skhd configuration path for original skhd
    async fn get_active_config_path_original(&self) -> Result<String, String> {
        let home = std::env::var("HOME").map_err(|_| {
            "Failed to get HOME environment variable. \
             This is required to locate the skhd configuration."
                .to_string()
        })?;

        let config_paths = vec![
            format!("{}/.config/skhd/skhdrc", home),
            format!("{}/.skhdrc", home),
        ];

        for path in config_paths {
            if std::path::Path::new(&path).exists() {
                return Ok(path);
            }
        }

        Err(format!(
            "No skhd configuration file found in standard locations:\n\
             - {}/.config/skhd/skhdrc\n\
             - {}/.skhdrc\n\
             Create a configuration file in one of these locations.",
            home, home
        ))
    }

    /// Get the active skhd configuration path for skhd.zig
    /// Checks XDG-style paths first, then falls back to standard locations
    async fn get_active_config_path_zig(&self) -> Result<String, String> {
        let home = std::env::var("HOME").map_err(|_| {
            "Failed to get HOME environment variable. \
             This is required to locate the skhd configuration."
                .to_string()
        })?;

        // XDG-style paths (skhd.zig uses these)
        let xdg_config_home = std::env::var("XDG_CONFIG_HOME")
            .unwrap_or_else(|_| format!("{}/.config", home));

        let config_paths = vec![
            format!("{}/skhd/skhdrc", xdg_config_home),
            format!("{}/.config/skhd/skhdrc", home),
            format!("{}/.skhdrc", home),
        ];

        for path in &config_paths {
            if std::path::Path::new(&path).exists() {
                return Ok(path.clone());
            }
        }

        Err(format!(
            "No skhd configuration file found. Searched in:\n\
             - {}/skhd/skhdrc (XDG_CONFIG_HOME)\n\
             - {}/.config/skhd/skhdrc\n\
             - {}/.skhdrc\n\
             Create a configuration file in one of these locations.",
            xdg_config_home, home, home
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SkhdVariant;

    #[test]
    fn test_service_manager_new() {
        let manager = ServiceManager::new();
        // Just verify it creates without error
        assert!(manager.reload_lock.try_lock().is_ok());
    }

    #[test]
    fn test_service_error_display() {
        let err = ServiceError {
            variant: SkhdVariant::Original,
            message: "Failed to start".to_string(),
        };
        assert_eq!(err.to_string(), "skhd: Failed to start");

        let err_zig = ServiceError {
            variant: SkhdVariant::Zig,
            message: "Binary not found".to_string(),
        };
        assert_eq!(err_zig.to_string(), "skhd.zig: Binary not found");
    }

    #[test]
    fn test_plist_path_original() {
        let manager = ServiceManager::new();
        std::env::set_var("HOME", "/Users/test");
        let path = manager.get_plist_path_original();
        assert!(path.is_ok());
        assert!(path.unwrap().contains("com.koekeishiya.skhd.plist"));
    }

    #[tokio::test]
    async fn test_get_active_config_path_original_searches_correct_paths() {
        let manager = ServiceManager::new();
        std::env::set_var("HOME", "/nonexistent_home");

        // This should fail since paths don't exist
        let result = manager.get_active_config_path_original().await;
        assert!(result.is_err());

        let err_msg = result.unwrap_err();
        assert!(err_msg.contains(".config/skhd/skhdrc"));
        assert!(err_msg.contains(".skhdrc"));
    }

    #[tokio::test]
    async fn test_get_active_config_path_zig_searches_xdg_paths() {
        let manager = ServiceManager::new();
        std::env::set_var("HOME", "/nonexistent_home");
        std::env::remove_var("XDG_CONFIG_HOME");

        // This should fail since paths don't exist
        let result = manager.get_active_config_path_zig().await;
        assert!(result.is_err());

        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("XDG_CONFIG_HOME"));
        assert!(err_msg.contains(".config/skhd/skhdrc"));
        assert!(err_msg.contains(".skhdrc"));
    }
}
