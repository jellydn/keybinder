/// Service modules
pub mod app_discovery;
pub mod backup;
pub mod file_io;
pub mod log_tailer;
pub mod path_validator;
pub mod service_manager;
pub mod template_loader;
pub mod theme_monitor;
pub mod validation;
pub mod variant_detector;

pub use app_discovery::{discover_applications, parse_app_bundle};
pub use backup::{create_backup, list_backups, restore_backup};
pub use file_io::{read_config_safe, write_config_atomic};
pub use log_tailer::{parse_log_line, LogTailer};
pub use path_validator::{detect_interpreter, escape_shell_path, validate_file_executable};
pub use service_manager::ServiceManager;
pub use template_loader::{get_categories, get_templates};
pub use theme_monitor::ThemeMonitorState;
pub use validation::{validate_config, validate_shortcut};
pub use variant_detector::{detect_variant, detect_variant_async};
