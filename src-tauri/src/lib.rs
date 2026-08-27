// Allow unexpected cfgs from objc crate macros
// The objc crate uses cfg conditions that clippy doesn't recognize
#![allow(unexpected_cfgs)]

// Modules
pub mod commands;
pub mod models;
pub mod parser;
pub mod services;
pub mod utils;

use commands::config::ConfigState;
use commands::logs::LogStreamState;
use commands::settings::SettingsState;
use commands::testing::ExecutionState;
use services::{ServiceManager, ThemeMonitorState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(ConfigState::new())
        .manage(ExecutionState::default())
        .manage(LogStreamState::default())
        .manage(ServiceManager::new())
        .manage(ThemeMonitorState::new())
        .manage(SettingsState::new())
        .invoke_handler(tauri::generate_handler![
            commands::applications::get_installed_applications,
            commands::config::detect_active_config,
            commands::config::get_active_config_path,
            commands::config::load_config,
            commands::config::save_config,
            commands::config::save_as_config,
            commands::config::reload_config,
            commands::config::import_config,
            commands::config::export_config,
            commands::shortcuts::create_shortcut,
            commands::shortcuts::update_shortcut,
            commands::shortcuts::delete_shortcut,
            commands::validation::validate_shortcut,
            commands::validation::validate_config,
            commands::backups::create_backup,
            commands::backups::list_backups,
            commands::backups::restore_backup,
            commands::testing::test_shortcut,
            commands::testing::execute_test_command,
            commands::testing::execute_shortcut_command,
            commands::testing::cancel_shortcut_execution,
            commands::testing::get_execution_config,
            commands::logs::start_log_stream,
            commands::logs::stop_log_stream,
            commands::logs::is_log_stream_running,
            commands::logs::get_recent_logs,
            commands::service::get_service_status,
            commands::service::start_service,
            commands::service::stop_service,
            commands::service::restart_service,
            commands::service::reload_service,
            commands::service::install_service,
            commands::service::uninstall_service,
            commands::templates::get_command_templates,
            commands::templates::get_command_categories,
            commands::templates::generate_command_from_template,
            commands::file_picker::open_file_picker,
            commands::file_picker::check_file_executable,
            commands::file_picker::escape_path_for_shell,
            commands::file_picker::detect_script_interpreter,
            commands::theme::get_system_theme,
            commands::theme::start_theme_monitor,
            commands::theme::stop_theme_monitor,
            commands::variant::detect_skhd_variant,
            commands::settings::get_skhd_variant_setting,
            commands::settings::set_skhd_variant_setting,
            commands::settings::get_effective_variant,
            commands::settings::get_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
