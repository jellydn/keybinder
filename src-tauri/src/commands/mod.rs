/// Tauri command modules
pub mod applications;
pub mod backups;
pub mod config;
pub mod file_picker;
pub mod logs;
pub mod service;
pub mod settings;
pub mod shortcuts;
pub mod templates;
pub mod testing;
pub mod theme;
pub mod validation;
pub mod variant;

// Re-export commands for easy access
pub use applications::get_installed_applications;
pub use backups::{create_backup, list_backups, restore_backup};
pub use config::{detect_active_config, export_config, import_config, load_config, reload_config, save_config};
pub use file_picker::{check_file_executable, detect_script_interpreter, escape_path_for_shell, open_file_picker};
pub use logs::{is_log_stream_running, start_log_stream, stop_log_stream};
pub use service::{get_service_status, reload_service};
pub use settings::{get_effective_variant, get_settings, get_skhd_variant_setting, set_skhd_variant_setting};
pub use shortcuts::{create_shortcut, delete_shortcut, update_shortcut};
pub use templates::{generate_command_from_template, get_command_categories, get_command_templates};
pub use testing::{cancel_shortcut_execution, execute_shortcut_command, execute_test_command, test_shortcut};
pub use theme::{get_system_theme, start_theme_monitor, stop_theme_monitor};
pub use validation::{validate_config, validate_shortcut};
pub use variant::detect_skhd_variant;
