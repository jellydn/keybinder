/// Utility modules
pub mod path;
pub mod theme;

pub use path::{
    expand_path, get_config_path_for_variant, get_config_paths_for_variant, get_default_config_path,
};
pub use theme::detect_system_theme;
