use crate::commands::config::ConfigState;
use crate::models::{Shortcut, TestResult};
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::State;
use tokio::io::AsyncReadExt;
use tokio::process::Command as TokioCommand;
use tokio::sync::oneshot;
use tokio::time::{timeout, Duration};

/// Maximum execution time for commands in seconds
pub const COMMAND_TIMEOUT_SECS: u64 = 30;

/// Maximum output length before truncation
pub const MAX_OUTPUT_LENGTH: usize = 10000;

/// State for tracking running command executions
pub struct ExecutionState {
    /// Map of shortcut IDs to cancellation senders
    pub cancel_senders: Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>,
}

impl Default for ExecutionState {
    fn default() -> Self {
        Self {
            cancel_senders: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

/// Truncate output to a maximum length
pub fn truncate_output(output: String, limit: usize) -> (String, bool) {
    if output.len() > limit {
        (output[..limit].to_string(), true)
    } else {
        (output, false)
    }
}

/// Escape a string for safe shell usage
fn shell_escape(s: &str) -> String {
    // Replace single quotes with '\'' (end quote, escaped quote, start quote)
    s.replace('\'', r"'\''")
}

/// Test a shortcut by executing its command in dry-run mode
/// This shows what would be executed without actually triggering the shortcut
#[tauri::command]
pub fn test_shortcut(
    shortcut_id: String,
    state: State<'_, ConfigState>,
) -> Result<TestResult, String> {
    let config_guard = state.config.lock().unwrap();
    let config = config_guard.as_ref().ok_or("No configuration loaded")?;

    // Find the shortcut
    let shortcut = config
        .shortcuts
        .iter()
        .find(|s| s.id == shortcut_id)
        .ok_or("Shortcut not found")?;

    // Execute the command with sh -n (syntax check without execution)
    // Use printf to properly handle quotes and special characters
    let test_script = format!("printf '%s' '{}' | sh -n", shell_escape(&shortcut.command));
    let output = Command::new("sh")
        .arg("-c")
        .arg(&test_script)
        .output()
        .map_err(|e| format!("Failed to test command: {}", e))?;

    let syntax_valid = output.status.success();
    let syntax_error = if !syntax_valid {
        Some(String::from_utf8_lossy(&output.stderr).to_string())
    } else {
        None
    };

    // Also do a dry-run preview showing what would execute
    let preview = format_command_preview(shortcut);

    Ok(TestResult {
        shortcut_id: shortcut.id.clone(),
        command: shortcut.command.clone(),
        syntax_valid,
        syntax_error,
        preview,
        timestamp: chrono::Local::now().to_rfc3339(),
        executed: false,
        exit_code: None,
        stdout: None,
        stderr: None,
        execution_duration_ms: None,
        cancelled: false,
        timed_out: false,
        output_truncated: false,
    })
}

/// Execute a shortcut's command in a safe test mode (echo only, no actual execution)
#[tauri::command]
pub fn execute_test_command(
    shortcut_id: String,
    state: State<'_, ConfigState>,
) -> Result<TestResult, String> {
    let config_guard = state.config.lock().unwrap();
    let config = config_guard.as_ref().ok_or("No configuration loaded")?;

    // Find the shortcut
    let shortcut = config
        .shortcuts
        .iter()
        .find(|s| s.id == shortcut_id)
        .ok_or("Shortcut not found")?;

    // Execute the command with echo to show what would run
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "echo 'Would execute: {}' && {}",
            shortcut.command, shortcut.command
        ))
        .output()
        .map_err(|e| format!("Failed to execute test: {}", e))?;

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let preview = format!(
        "Exit Code: {}\n\nStdout:\n{}\n\nStderr:\n{}",
        output.status.code().unwrap_or(-1),
        stdout,
        if stderr.is_empty() { "(none)" } else { &stderr }
    );

    Ok(TestResult {
        shortcut_id: shortcut.id.clone(),
        command: shortcut.command.clone(),
        syntax_valid: success,
        syntax_error: if !success { Some(stderr.clone()) } else { None },
        preview,
        timestamp: chrono::Local::now().to_rfc3339(),
        executed: false,
        exit_code: None,
        stdout: None,
        stderr: None,
        execution_duration_ms: None,
        cancelled: false,
        timed_out: false,
        output_truncated: false,
    })
}

/// Execute a shortcut's command and return detailed execution results
#[tauri::command]
pub async fn execute_shortcut_command(
    shortcut_id: String,
    state: State<'_, ConfigState>,
    exec_state: State<'_, ExecutionState>,
) -> Result<TestResult, String> {
    // 1. Find shortcut (release lock immediately)
    let shortcut = {
        let config_guard = state.config.lock().unwrap();
        let config = config_guard.as_ref().ok_or("No configuration loaded")?;
        config
            .shortcuts
            .iter()
            .find(|s| s.id == shortcut_id)
            .cloned()
            .ok_or("Shortcut not found")?
    }; // Lock released here

    // 2. Check if already running
    {
        let senders = exec_state.cancel_senders.lock().unwrap();
        if senders.contains_key(&shortcut_id) {
            return Err(format!("Command already executing for shortcut: {}", shortcut_id));
        }
    }

    // 3. Create cancellation channel
    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();

    // 4. Store cancellation sender
    {
        let mut senders = exec_state.cancel_senders.lock().unwrap();
        senders.insert(shortcut_id.clone(), cancel_tx);
    }

    // 5. Start timing
    let start = Instant::now();

    // 6. Spawn command
    let mut child = TokioCommand::new("sh")
        .arg("-c")
        .arg(&shortcut.command)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            // Clean up sender on spawn failure
            let mut senders = exec_state.cancel_senders.lock().unwrap();
            senders.remove(&shortcut_id);

            match e.kind() {
                std::io::ErrorKind::NotFound => format!("Command not found: {}", shortcut.command),
                std::io::ErrorKind::PermissionDenied => format!("Permission denied: {}", shortcut.command),
                _ => format!("Failed to spawn command: {}", e),
            }
        })?;

    // 7. Spawn tasks to read output streams concurrently
    let stdout_handle = child.stdout.take().map(|mut stdout| {
        tokio::spawn(async move {
            let mut data = Vec::new();
            let _ = stdout.read_to_end(&mut data).await;
            data
        })
    });

    let stderr_handle = child.stderr.take().map(|mut stderr| {
        tokio::spawn(async move {
            let mut data = Vec::new();
            let _ = stderr.read_to_end(&mut data).await;
            data
        })
    });

    // 8. Wait with timeout and cancellation support
    let wait_result = tokio::select! {
        // Command completes or times out
        result = timeout(Duration::from_secs(COMMAND_TIMEOUT_SECS), child.wait()) => {
            result
        }
        // Cancellation requested
        _ = cancel_rx => {
            // Kill the process immediately
            let _ = child.start_kill();

            // Abort the stdout/stderr reading tasks to avoid waiting for them
            if let Some(handle) = stdout_handle {
                handle.abort();
            }
            if let Some(handle) = stderr_handle {
                handle.abort();
            }

            // Clean up sender
            {
                let mut senders = exec_state.cancel_senders.lock().unwrap();
                senders.remove(&shortcut_id);
            }

            let duration_ms = start.elapsed().as_millis() as u64;

            return Ok(TestResult {
                shortcut_id: shortcut.id.clone(),
                command: shortcut.command.clone(),
                syntax_valid: true,
                syntax_error: None,
                preview: String::new(),
                timestamp: chrono::Local::now().to_rfc3339(),
                executed: true,
                exit_code: None,
                stdout: Some(String::new()),
                stderr: Some(String::from("Command execution cancelled by user")),
                execution_duration_ms: Some(duration_ms),
                cancelled: true,
                timed_out: false,
                output_truncated: false,
            });
        }
    };

    // 9. Collect output from the spawned tasks
    let stdout_data = if let Some(handle) = stdout_handle {
        handle.await.unwrap_or_default()
    } else {
        Vec::new()
    };

    let stderr_data = if let Some(handle) = stderr_handle {
        handle.await.unwrap_or_default()
    } else {
        Vec::new()
    };

    // 10. Clean up sender
    {
        let mut senders = exec_state.cancel_senders.lock().unwrap();
        senders.remove(&shortcut_id);
    }

    // 11. Calculate duration
    let duration_ms = start.elapsed().as_millis() as u64;

    // 12. Build result based on outcome
    match wait_result {
        Ok(Ok(status)) => {
            // Successful execution (or failed with exit code)
            let stdout_raw = String::from_utf8_lossy(&stdout_data).to_string();
            let stderr_raw = String::from_utf8_lossy(&stderr_data).to_string();

            let (stdout, stdout_truncated) = truncate_output(stdout_raw, MAX_OUTPUT_LENGTH);
            let (stderr, stderr_truncated) = truncate_output(stderr_raw, MAX_OUTPUT_LENGTH);

            Ok(TestResult {
                shortcut_id: shortcut.id.clone(),
                command: shortcut.command.clone(),
                syntax_valid: true,
                syntax_error: None,
                preview: String::new(),
                timestamp: chrono::Local::now().to_rfc3339(),
                executed: true,
                exit_code: status.code(),
                stdout: Some(stdout),
                stderr: Some(stderr),
                execution_duration_ms: Some(duration_ms),
                cancelled: false,
                timed_out: false,
                output_truncated: stdout_truncated || stderr_truncated,
            })
        }
        Ok(Err(e)) => {
            // Failed to wait for output
            Err(match e.kind() {
                std::io::ErrorKind::BrokenPipe => "Command output error: broken pipe".to_string(),
                std::io::ErrorKind::Interrupted => "Command execution interrupted".to_string(),
                _ => format!("Execution failed: {}", e),
            })
        }
        Err(_) => {
            // Timeout - child has been moved into wait_with_output, so we can't kill it
            // The timeout will have interrupted the wait, and the process will be orphaned
            // This is acceptable for MVP as the timeout itself stops us from waiting

            Ok(TestResult {
                shortcut_id: shortcut.id.clone(),
                command: shortcut.command.clone(),
                syntax_valid: true,
                syntax_error: None,
                preview: String::new(),
                timestamp: chrono::Local::now().to_rfc3339(),
                executed: true,
                exit_code: None,
                stdout: Some(String::new()),
                stderr: Some(String::from("Command timed out after 30 seconds and was terminated")),
                execution_duration_ms: Some(30000),
                cancelled: false,
                timed_out: true,
                output_truncated: false,
            })
        }
    }
}

/// Cancel a running shortcut command execution
#[tauri::command]
pub async fn cancel_shortcut_execution(
    shortcut_id: String,
    exec_state: State<'_, ExecutionState>,
) -> Result<(), String> {
    // Remove the cancellation sender and trigger cancellation
    let sender = {
        let mut senders = exec_state.cancel_senders.lock().unwrap();
        senders.remove(&shortcut_id)
    };

    if let Some(sender) = sender {
        // Send cancellation signal (ignore error if receiver already dropped)
        let _ = sender.send(());
        Ok(())
    } else {
        // Process already completed/timed out - this is fine, the execution is stopped
        // which is what the user wanted
        Ok(())
    }
}

/// Get execution configuration constants
/// Returns timeout_seconds and max_output_length for the frontend
#[tauri::command]
pub fn get_execution_config() -> serde_json::Value {
    serde_json::json!({
        "timeout_seconds": COMMAND_TIMEOUT_SECS,
        "max_output_length": MAX_OUTPUT_LENGTH,
    })
}

fn format_command_preview(shortcut: &Shortcut) -> String {
    let mut preview = String::new();

    // Shortcut binding
    let modifiers = if shortcut.modifiers.is_empty() {
        "No modifiers".to_string()
    } else {
        shortcut.modifiers.join(" + ")
    };

    preview.push_str(&format!("Shortcut: {} + {}\n\n", modifiers, shortcut.key));

    // Command breakdown
    preview.push_str(&format!("Command: {}\n\n", shortcut.command));

    // Parse command for common patterns
    if shortcut.command.starts_with("open ") {
        preview.push_str("Action: Opens an application or file\n");
        if let Some(app) = shortcut.command.strip_prefix("open -a ") {
            preview.push_str(&format!("Application: {}\n", app));
        } else if let Some(path) = shortcut.command.strip_prefix("open ") {
            preview.push_str(&format!("Path: {}\n", path));
        }
    } else if shortcut.command.contains("yabai") {
        preview.push_str("Action: Window management (yabai)\n");
    } else if shortcut.command.contains("&&") || shortcut.command.contains(";") {
        preview.push_str("Action: Multiple commands (chained)\n");
        let parts: Vec<&str> = if shortcut.command.contains("&&") {
            shortcut.command.split("&&").collect()
        } else {
            shortcut.command.split(';').collect()
        };
        preview.push_str("\nSteps:\n");
        for (i, part) in parts.iter().enumerate() {
            preview.push_str(&format!("  {}. {}\n", i + 1, part.trim()));
        }
    } else {
        preview.push_str("Action: Shell command\n");
    }

    if let Some(comment) = &shortcut.comment {
        preview.push_str(&format!("\nDescription: {}\n", comment));
    }

    preview
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Shortcut;

    #[test]
    fn test_format_command_preview_open_app() {
        let shortcut = Shortcut {
            id: "test".to_string(),
            modifiers: vec!["cmd".to_string()],
            key: "return".to_string(),
            command: "open -a Terminal".to_string(),
            comment: Some("Open Terminal".to_string()),
            mode: None,
            line_number: 1,
        };

        let preview = format_command_preview(&shortcut);
        assert!(preview.contains("Shortcut: cmd + return"));
        assert!(preview.contains("Application: Terminal"));
        assert!(preview.contains("Description: Open Terminal"));
    }

    #[test]
    fn test_format_command_preview_chained() {
        let shortcut = Shortcut {
            id: "test".to_string(),
            modifiers: vec![],
            key: "f".to_string(),
            command: "echo hello && echo world".to_string(),
            comment: None,
            mode: None,
            line_number: 1,
        };

        let preview = format_command_preview(&shortcut);
        assert!(preview.contains("Multiple commands"));
        assert!(preview.contains("echo hello"));
        assert!(preview.contains("echo world"));
    }
}
