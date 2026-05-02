/// Data models module
pub mod application;
pub mod backup;
pub mod command_category;
pub mod command_template;
pub mod config;
pub mod log_entry;
pub mod service_status;
pub mod shortcut;
pub mod skhd_variant;
pub mod test_result;

// Re-export commonly used types
pub use application::Application;
pub use backup::Backup;
pub use command_category::CommandCategory;
pub use command_template::{CommandParameter, CommandTemplate};
pub use config::{ConfigFile, ParseError};
pub use log_entry::{LogEntry, LogLevel};
pub use service_status::{ServiceState, ServiceStatus};
pub use shortcut::Shortcut;
pub use skhd_variant::{DetectedVariant, DetectionSource, SkhdVariant};
pub use test_result::TestResult;
