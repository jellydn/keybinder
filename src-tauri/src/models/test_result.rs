/// Test result for shortcut execution
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// ID of the shortcut that was tested
    pub shortcut_id: String,

    /// The command that was tested
    pub command: String,

    /// Whether the command syntax is valid
    pub syntax_valid: bool,

    /// Syntax error message if invalid
    pub syntax_error: Option<String>,

    /// Preview of what the command would do
    pub preview: String,

    /// Timestamp when the test was executed
    pub timestamp: String,

    // ===== NEW EXECUTION FIELDS =====
    /// Whether this was an actual execution (true) or syntax validation only (false)
    pub executed: bool,

    /// Exit code from command execution (None for syntax-only tests)
    pub exit_code: Option<i32>,

    /// Standard output from command execution (None for syntax-only tests)
    pub stdout: Option<String>,

    /// Standard error from command execution (None for syntax-only tests)
    pub stderr: Option<String>,

    /// Execution duration in milliseconds (None for syntax-only tests)
    pub execution_duration_ms: Option<u64>,

    /// Whether the command was cancelled by the user
    pub cancelled: bool,

    /// Whether the command timed out
    pub timed_out: bool,

    /// Whether output was truncated due to size limit
    pub output_truncated: bool,
}
