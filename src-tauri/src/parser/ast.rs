/// Abstract Syntax Tree structures for skhd configuration parsing
use serde::{Deserialize, Serialize};

// ============================================================
// Original skhd types (unchanged)
// ============================================================

/// Represents a parsed keyboard shortcut from the skhd config
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedShortcut {
    /// Line number in the original config file
    pub line_number: usize,

    /// Modifier keys (cmd, alt, shift, ctrl, fn, hyper, meh, lcmd, rcmd, etc.)
    pub modifiers: Vec<String>,

    /// Primary key being pressed
    pub key: String,

    /// Shell command to execute
    pub command: String,

    /// Optional inline comment
    pub comment: Option<String>,
}

/// Represents a comment line in the config
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedComment {
    /// Line number in the original config file
    pub line_number: usize,

    /// Comment text (without the # prefix)
    pub text: String,
}

// ============================================================
// skhd.zig Directive Types (US-005)
// ============================================================

/// Represents a parsed directive from skhd.zig
/// These are parsed but treated as read-only (not executed by Keybinder)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ZigDirective {
    /// .alias directive: .alias $name <modifier_combo|keycode>
    /// Examples: .alias $hyper cmd + alt + ctrl + shift
    ///           .alias $grave 0x32
    Alias {
        line_number: usize,
        name: String,  // e.g., "$hyper"
        value: String, // e.g., "cmd + alt + ctrl + shift" or "0x32"
    },

    /// .define directive - process group form
    /// .define <name> [ "app1", "app2" ]
    DefineGroup {
        line_number: usize,
        name: String,              // e.g., "terminal_apps"
        applications: Vec<String>, // e.g., ["kitty", "wezterm", "terminal"]
    },

    /// .define directive - command template form
    /// .define <name> : <command_template>
    /// Command templates can contain {{1}}, {{2}}, etc. placeholders
    DefineCommand {
        line_number: usize,
        name: String,     // e.g., "yabai_focus"
        template: String, // e.g., "yabai -m window --focus {{1}}"
    },

    /// .path directive: .path "<dir>" or .path [ "dir1", "dir2" ]
    Path {
        line_number: usize,
        paths: Vec<String>, // One or more paths to add to PATH
    },

    /// .shell directive: .shell "<shell_path>"
    Shell {
        line_number: usize,
        shell_path: String, // e.g., "/bin/zsh"
    },

    /// .blacklist directive: .blacklist [ "app1", "app2" ]
    Blacklist {
        line_number: usize,
        applications: Vec<String>, // Apps to blacklist
    },

    /// .load directive: .load "<file_path>"
    Load {
        line_number: usize,
        file_path: String, // Path to additional config file
    },

    /// Any other skhd.zig directive, preserved for read-only handling.
    Generic {
        line_number: usize,
        keyword: String,
        content: String,
    },
}

impl ZigDirective {
    /// Get the line number for this directive
    pub fn line_number(&self) -> usize {
        match self {
            ZigDirective::Alias { line_number, .. } => *line_number,
            ZigDirective::DefineGroup { line_number, .. } => *line_number,
            ZigDirective::DefineCommand { line_number, .. } => *line_number,
            ZigDirective::Path { line_number, .. } => *line_number,
            ZigDirective::Shell { line_number, .. } => *line_number,
            ZigDirective::Blacklist { line_number, .. } => *line_number,
            ZigDirective::Load { line_number, .. } => *line_number,
            ZigDirective::Generic { line_number, .. } => *line_number,
        }
    }

    /// Get the directive name/type as a string
    pub fn directive_name(&self) -> &str {
        match self {
            ZigDirective::Alias { .. } => "alias",
            ZigDirective::DefineGroup { .. } => "define",
            ZigDirective::DefineCommand { .. } => "define",
            ZigDirective::Path { .. } => "path",
            ZigDirective::Shell { .. } => "shell",
            ZigDirective::Blacklist { .. } => "blacklist",
            ZigDirective::Load { .. } => "load",
            ZigDirective::Generic { keyword, .. } => keyword,
        }
    }
}

// ============================================================
// Config Line Types
// ============================================================

/// Represents a line in the skhd config file
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConfigLine {
    /// A keyboard shortcut mapping
    Shortcut(ParsedShortcut),

    /// A comment line
    Comment(ParsedComment),

    /// An empty line
    Empty(usize), // line number

    /// A skhd.zig directive (read-only support)
    Directive(ZigDirective),
}

// ============================================================
// Parsed Config
// ============================================================

/// Complete parsed configuration file
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedConfig {
    /// All lines from the config file in order
    pub lines: Vec<ConfigLine>,
}

impl ParsedConfig {
    /// Create a new empty parsed config
    pub fn new() -> Self {
        Self { lines: Vec::new() }
    }

    /// Get all shortcuts from the config
    pub fn shortcuts(&self) -> Vec<&ParsedShortcut> {
        self.lines
            .iter()
            .filter_map(|line| match line {
                ConfigLine::Shortcut(s) => Some(s),
                _ => None,
            })
            .collect()
    }

    /// Get all comments from the config
    pub fn comments(&self) -> Vec<&ParsedComment> {
        self.lines
            .iter()
            .filter_map(|line| match line {
                ConfigLine::Comment(c) => Some(c),
                _ => None,
            })
            .collect()
    }

    /// Get all skhd.zig directives from the config
    pub fn directives(&self) -> Vec<&ZigDirective> {
        self.lines
            .iter()
            .filter_map(|line| match line {
                ConfigLine::Directive(d) => Some(d),
                _ => None,
            })
            .collect()
    }

    /// Check if the config contains any skhd.zig-specific directives
    pub fn has_zig_directives(&self) -> bool {
        self.lines
            .iter()
            .any(|line| matches!(line, ConfigLine::Directive(_)))
    }

    /// Get count of each directive type
    pub fn directive_counts(&self) -> std::collections::HashMap<String, usize> {
        let mut counts = std::collections::HashMap::new();
        for line in &self.lines {
            if let ConfigLine::Directive(d) = line {
                let name = d.directive_name().to_string();
                *counts.entry(name).or_insert(0) += 1;
            }
        }
        counts
    }
}

impl Default for ParsedConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsed_shortcut_creation() {
        let shortcut = ParsedShortcut {
            line_number: 1,
            modifiers: vec!["cmd".to_string()],
            key: "return".to_string(),
            command: "open -a Terminal".to_string(),
            comment: None,
        };
        assert_eq!(shortcut.line_number, 1);
        assert_eq!(shortcut.modifiers, vec!["cmd"]);
    }

    #[test]
    fn test_zig_directive_alias() {
        let alias = ZigDirective::Alias {
            line_number: 1,
            name: "$hyper".to_string(),
            value: "cmd + alt + ctrl + shift".to_string(),
        };
        assert_eq!(alias.line_number(), 1);
        assert_eq!(alias.directive_name(), "alias");
    }

    #[test]
    fn test_zig_directive_define_group() {
        let define = ZigDirective::DefineGroup {
            line_number: 2,
            name: "terminal_apps".to_string(),
            applications: vec!["kitty".to_string(), "wezterm".to_string()],
        };
        assert_eq!(define.line_number(), 2);
        assert_eq!(define.directive_name(), "define");
    }

    #[test]
    fn test_zig_directive_define_command() {
        let define = ZigDirective::DefineCommand {
            line_number: 3,
            name: "yabai_focus".to_string(),
            template: "yabai -m window --focus {{1}}".to_string(),
        };
        assert_eq!(define.line_number(), 3);
        assert_eq!(define.directive_name(), "define");
    }

    #[test]
    fn test_zig_directive_path() {
        let path = ZigDirective::Path {
            line_number: 4,
            paths: vec!["/usr/local/bin".to_string()],
        };
        assert_eq!(path.line_number(), 4);
        assert_eq!(path.directive_name(), "path");
    }

    #[test]
    fn test_parsed_config_directives() {
        let mut config = ParsedConfig::new();
        config
            .lines
            .push(ConfigLine::Directive(ZigDirective::Alias {
                line_number: 1,
                name: "$hyper".to_string(),
                value: "cmd + shift + alt + ctrl".to_string(),
            }));
        config.lines.push(ConfigLine::Directive(ZigDirective::Path {
            line_number: 2,
            paths: vec!["~/.cargo/bin".to_string()],
        }));

        let directives = config.directives();
        assert_eq!(directives.len(), 2);
        assert!(config.has_zig_directives());

        let counts = config.directive_counts();
        assert_eq!(counts.get("alias"), Some(&1));
        assert_eq!(counts.get("path"), Some(&1));
    }
}
