/// Tauri commands for log streaming and management
///
/// This module exposes the following commands to the frontend:
/// - `start_log_stream`: Begin streaming logs from skhd service
/// - `stop_log_stream`: Stop the log stream
/// - `is_log_stream_running`: Check if stream is active
/// - `get_recent_logs`: Retrieve historical logs from file
///
/// Events emitted:
/// - `log-entry`: Emitted for each new log entry (payload: LogEntry)
use crate::models::LogEntry;
use crate::services::{log_tailer::parse_log_line, LogTailer};
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
    sync::Mutex,
};

/// Shared state for LogTailer across all commands
pub struct LogStreamState {
    tailer: Arc<Mutex<Option<LogTailer>>>,
}

impl LogStreamState {
    /// Create new log stream state
    pub fn new() -> Self {
        Self {
            tailer: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for LogStreamState {
    fn default() -> Self {
        Self::new()
    }
}

/// Start streaming logs from the skhd service
///
/// This command:
/// - Spawns a background `log stream` process
/// - Begins emitting "log-entry" events to the frontend
/// - Returns immediately after starting the stream
///
/// # Arguments
/// * `app` - Tauri AppHandle for event emission
/// * `state` - Shared LogStreamState
///
/// # Returns
/// * `Ok(())` - Stream started successfully
/// * `Err(String)` - Failed to start (already running, spawn error)
///
/// # Frontend Usage
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// try {
///   await invoke('start_log_stream');
///   console.log('Log stream started');
/// } catch (error) {
///   console.error('Failed to start log stream:', error);
/// }
/// ```
#[tauri::command]
pub async fn start_log_stream(
    app: AppHandle,
    state: State<'_, LogStreamState>,
) -> Result<(), String> {
    let mut tailer_lock = state.tailer.lock().await;

    // Check if already initialized
    if tailer_lock.is_some() {
        return Err("Log stream is already running".to_string());
    }

    // Create new tailer and start stream
    let tailer = LogTailer::new(app);
    tailer.start_stream().await?;

    *tailer_lock = Some(tailer);

    Ok(())
}

/// Stop the log stream and clean up resources
///
/// This command:
/// - Kills the background `log stream` process
/// - Stops emitting log-entry events
/// - Releases all resources
///
/// # Arguments
/// * `state` - Shared LogStreamState
///
/// # Returns
/// * `Ok(())` - Stream stopped successfully
/// * `Err(String)` - Failed to stop (not running, kill error)
///
/// # Frontend Usage
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// try {
///   await invoke('stop_log_stream');
///   console.log('Log stream stopped');
/// } catch (error) {
///   console.error('Failed to stop log stream:', error);
/// }
/// ```
#[tauri::command]
pub async fn stop_log_stream(state: State<'_, LogStreamState>) -> Result<(), String> {
    let mut tailer_lock = state.tailer.lock().await;

    if let Some(tailer) = tailer_lock.take() {
        tailer.stop_stream().await?;
        Ok(())
    } else {
        Err("Log stream is not running".to_string())
    }
}

/// Check if the log stream is currently running
///
/// # Arguments
/// * `state` - Shared LogStreamState
///
/// # Returns
/// * `true` - Stream is active
/// * `false` - Stream is not running
///
/// # Frontend Usage
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const isRunning = await invoke<boolean>('is_log_stream_running');
/// console.log('Stream running:', isRunning);
/// ```
#[tauri::command]
pub async fn is_log_stream_running(state: State<'_, LogStreamState>) -> Result<bool, String> {
    let tailer_lock = state.tailer.lock().await;

    Ok(if let Some(tailer) = tailer_lock.as_ref() {
        tailer.is_running().await
    } else {
        false
    })
}

/// Get recent logs from the skhd log file
///
/// This command reads historical logs from `/tmp/skhd_<username>.err.log`
/// and returns them as an array of parsed LogEntry objects. This is useful
/// for loading logs that were generated before the stream started.
///
/// # Arguments
/// * `limit` - Maximum number of log lines to retrieve (default: 100)
///
/// # Returns
/// * `Ok(Vec<LogEntry>)` - Array of parsed log entries
/// * `Err(String)` - Failed to read log file
///
/// # Frontend Usage
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// try {
///   const logs = await invoke<LogEntry[]>('get_recent_logs', { limit: 100 });
///   console.log('Loaded', logs.length, 'historical logs');
/// } catch (error) {
///   console.error('Failed to load logs:', error);
/// }
/// ```
/// Read recent logs from a specific log file
async fn read_log_file(file_path: &str, limit: usize) -> Result<Vec<String>, String> {
    // Check if log file exists
    if tokio::fs::metadata(file_path).await.is_err() {
        // Return empty vec if file doesn't exist (not an error - file may not be created yet)
        return Ok(Vec::new());
    }

    // Open and read the log file
    let file = File::open(file_path).await.map_err(|e| {
        format!(
            "Failed to open log file {}: {}. \
             Check that you have permission to read the file.",
            file_path, e
        )
    })?;

    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut all_lines = Vec::new();

    // Read all lines from the file
    while let Ok(Some(line)) = lines.next_line().await {
        all_lines.push(line);
    }

    // Take the last N lines (most recent)
    let recent_lines: Vec<String> = all_lines
        .into_iter()
        .rev() // Reverse to get most recent first
        .take(limit)
        .rev() // Reverse back to chronological order
        .collect();

    Ok(recent_lines)
}

#[tauri::command]
pub async fn get_recent_logs(limit: Option<usize>) -> Result<Vec<LogEntry>, String> {
    let limit = limit.unwrap_or(100);
    let limit_per_file = limit / 2; // Split between stdout and stderr

    // Get current username for log file paths
    let username = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string());

    // Sanitize username (same logic as LogTailer)
    let sanitized_username = username
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect::<String>();

    let stdout_log_file = format!("/tmp/skhd_{}.out.log", sanitized_username);
    let stderr_log_file = format!("/tmp/skhd_{}.err.log", sanitized_username);

    // Read from both log files
    let stdout_lines = read_log_file(&stdout_log_file, limit_per_file).await?;
    let stderr_lines = read_log_file(&stderr_log_file, limit_per_file).await?;

    // Check if both files are empty
    if stdout_lines.is_empty() && stderr_lines.is_empty() {
        return Err(format!(
            "No log files found: {} and {}. \
             The skhd service may not have been started yet, or logs may not have been generated. \
             Start the skhd service to begin generating logs.",
            stdout_log_file, stderr_log_file
        ));
    }

    // Combine and parse lines into LogEntry objects
    let mut log_entries = Vec::new();

    // Parse stdout lines (INFO logs)
    for line in stdout_lines {
        if let Some(entry) = parse_log_line(&line, false) {
            log_entries.push(entry);
        }
    }

    // Parse stderr lines (ERROR logs)
    for line in stderr_lines {
        if let Some(entry) = parse_log_line(&line, true) {
            log_entries.push(entry);
        }
    }

    // Sort by timestamp (chronological order)
    log_entries.sort_by_key(|entry| entry.timestamp);

    Ok(log_entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_stream_state_creation() {
        let state = LogStreamState::new();
        // State should be created successfully
        assert!(state.tailer.try_lock().is_ok());
    }

    #[test]
    fn test_log_stream_state_default() {
        let state = LogStreamState::default();
        // Default implementation should work
        assert!(state.tailer.try_lock().is_ok());
    }
}
