/// skhd configuration file parser using pest
pub mod ast;

use pest::Parser;
use pest_derive::Parser;
use std::error::Error;
use std::fmt;

use ast::{ConfigLine, ParsedComment, ParsedConfig, ParsedShortcut, ZigDirective};

#[derive(Parser)]
#[grammar = "parser/grammar.pest"]
pub struct SkhdParser;

/// Parse errors
#[derive(Debug, Clone)]
pub struct ParseError {
    pub line_number: usize,
    pub column: Option<usize>,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(col) = self.column {
            write!(
                f,
                "Parse error at line {}, column {}: {}",
                self.line_number, col, self.message
            )
        } else {
            write!(
                f,
                "Parse error at line {}: {}",
                self.line_number, self.message
            )
        }
    }
}

impl Error for ParseError {}

/// Parse an skhd configuration file
pub fn parse_config(content: &str) -> Result<ParsedConfig, Vec<ParseError>> {
    let mut parsed_config = ParsedConfig::new();
    let mut errors = Vec::new();

    // Parse the entire file
    let pairs = match SkhdParser::parse(Rule::config_file, content) {
        Ok(pairs) => pairs,
        Err(e) => {
            let (line, col) = match e.line_col {
                pest::error::LineColLocation::Pos((l, c)) => (l, Some(c)),
                pest::error::LineColLocation::Span((l, _), _) => (l, None),
            };
            errors.push(ParseError {
                line_number: line,
                column: col,
                message: format!("Syntax error: {}", e.variant),
            });
            return Err(errors);
        }
    };

    for pair in pairs {
        if pair.as_rule() == Rule::config_file {
            for line_pair in pair.into_inner() {
                let line_num = line_pair.as_span().start_pos().line_col().0;
                match line_pair.as_rule() {
                    Rule::comment => {
                        let text = line_pair
                            .as_str()
                            .trim_end_matches('\n')
                            .trim_end_matches('\r')
                            .trim_start_matches('#')
                            .trim();
                        parsed_config.lines.push(ConfigLine::Comment(ParsedComment {
                            line_number: line_num,
                            text: text.to_string(),
                        }));
                    }
                    Rule::shortcut => match parse_shortcut(&line_pair, line_num) {
                        Ok(shortcut) => {
                            parsed_config.lines.push(ConfigLine::Shortcut(shortcut));
                        }
                        Err(e) => {
                            errors.push(e);
                        }
                    },
                    Rule::directive => match parse_directive(&line_pair, line_num) {
                        Ok(directive) => {
                            parsed_config.lines.push(ConfigLine::Directive(directive));
                        }
                        Err(e) => {
                            errors.push(e);
                        }
                    },
                    Rule::empty_line => {
                        parsed_config.lines.push(ConfigLine::Empty(line_num));
                    }
                    Rule::EOI => break,
                    _ => {}
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(parsed_config)
    } else {
        Err(errors)
    }
}

/// Parse a single shortcut line
fn parse_shortcut(
    pair: &pest::iterators::Pair<Rule>,
    line_num: usize,
) -> Result<ParsedShortcut, ParseError> {
    let mut modifiers = Vec::new();
    let mut key = String::new();
    let mut command = String::new();

    for inner_pair in pair.clone().into_inner() {
        match inner_pair.as_rule() {
            Rule::modifiers => {
                for modifier_pair in inner_pair.into_inner() {
                    if modifier_pair.as_rule() == Rule::modifier {
                        modifiers.push(modifier_pair.as_str().to_string());
                    }
                }
            }
            Rule::key => {
                key = inner_pair.as_str().to_string();
            }
            Rule::command => {
                command = inner_pair.as_str().trim().to_string();
            }
            _ => {}
        }
    }

    if key.is_empty() {
        return Err(ParseError {
            line_number: line_num,
            column: None,
            message: "Missing key specification".to_string(),
        });
    }

    if command.is_empty() {
        return Err(ParseError {
            line_number: line_num,
            column: None,
            message: "Missing command specification".to_string(),
        });
    }

    Ok(ParsedShortcut {
        line_number: line_num,
        modifiers,
        key,
        command,
        comment: None,
    })
}

/// Parse a directive line (.alias, .define, .path, etc.)
fn parse_directive(
    pair: &pest::iterators::Pair<Rule>,
    line_num: usize,
) -> Result<ZigDirective, ParseError> {
    // Get the directive text (e.g., ".alias $hyper cmd + alt + ctrl + shift\n")
    let directive_text = pair.as_str();

    // Remove the leading dot and trailing newline
    let trimmed = directive_text.trim_start_matches('.').trim_end();

    // Split into keyword and content while accepting spaces or tabs.
    let keyword_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    let keyword = &trimmed[..keyword_end];
    let content = trimmed[keyword_end..].trim();

    if keyword.is_empty() {
        return Err(ParseError {
            line_number: line_num,
            column: None,
            message: "Directive missing keyword".to_string(),
        });
    }

    // Parse based on keyword
    match keyword {
        "alias" => parse_alias_content(content, line_num),
        "define" => parse_define_content(content, line_num),
        "path" => parse_path_content(content, line_num),
        "shell" => parse_shell_content(content, line_num),
        "blacklist" => parse_blacklist_content(content, line_num),
        "load" => parse_load_content(content, line_num),
        _ => Ok(ZigDirective::Generic {
            line_number: line_num,
            keyword: keyword.to_string(),
            content: content.to_string(),
        }),
    }
}

/// Parse .alias directive content: $name <value>
/// Examples: $hyper cmd + alt + ctrl + shift
///           $grave 0x32
fn parse_alias_content(content: &str, line_num: usize) -> Result<ZigDirective, ParseError> {
    // Parse: $name <value>
    let mut parts = content.splitn(2, ' ');
    let name = parts.next().unwrap_or("").trim().to_string();
    let value = parts.next().unwrap_or("").trim().to_string();

    if name.is_empty() || !name.starts_with('$') {
        return Err(ParseError {
            line_number: line_num,
            column: None,
            message: "Alias directive must have a name starting with $".to_string(),
        });
    }

    Ok(ZigDirective::Alias {
        line_number: line_num,
        name,
        value,
    })
}

/// Parse .define directive content
/// Two forms:
/// 1. Process group: name [ "app1", "app2" ]
/// 2. Command template: name : command_template
fn parse_define_content(content: &str, line_num: usize) -> Result<ZigDirective, ParseError> {
    let trimmed = content.trim();

    // Check if it's group form (contains [ and ])
    if trimmed.contains('[') && trimmed.contains(']') {
        // Group form: name [ "app1", "app2" ]
        let bracket_idx = trimmed.find('[').unwrap();
        let name = trimmed[..bracket_idx].trim().to_string();
        let list_part = &trimmed[bracket_idx..];

        // Parse the list
        let applications = parse_string_list_content(list_part);

        Ok(ZigDirective::DefineGroup {
            line_number: line_num,
            name,
            applications,
        })
    } else if trimmed.contains(':') {
        // Command form: name : command_template
        let colon_idx = trimmed.find(':').unwrap();
        let name = trimmed[..colon_idx].trim().to_string();
        let template = trimmed[colon_idx + 1..].trim().to_string();

        Ok(ZigDirective::DefineCommand {
            line_number: line_num,
            name,
            template,
        })
    } else {
        Err(ParseError {
            line_number: line_num,
            column: None,
            message: "Define directive must be in group form [..] or command form : template"
                .to_string(),
        })
    }
}

/// Parse .path directive content
/// Forms: "<dir>" or [ "dir1", "dir2" ]
fn parse_path_content(content: &str, line_num: usize) -> Result<ZigDirective, ParseError> {
    let trimmed = content.trim();

    let paths = if trimmed.starts_with('[') && trimmed.ends_with(']') {
        // List form
        parse_string_list_content(trimmed)
    } else if trimmed.starts_with('"') && trimmed.ends_with('"') {
        // Single path form
        vec![trimmed.trim_matches('"').to_string()]
    } else {
        // Single path without quotes
        vec![trimmed.to_string()]
    };

    Ok(ZigDirective::Path {
        line_number: line_num,
        paths,
    })
}

/// Parse .shell directive content: "<shell_path>"
fn parse_shell_content(content: &str, line_num: usize) -> Result<ZigDirective, ParseError> {
    let shell_path = content.trim().trim_matches('"').to_string();

    Ok(ZigDirective::Shell {
        line_number: line_num,
        shell_path,
    })
}

/// Parse .blacklist directive content: [ "app1", "app2" ]
fn parse_blacklist_content(content: &str, line_num: usize) -> Result<ZigDirective, ParseError> {
    let trimmed = content.trim();
    let applications = parse_string_list_content(trimmed);

    Ok(ZigDirective::Blacklist {
        line_number: line_num,
        applications,
    })
}

/// Parse .load directive content: "<file_path>"
fn parse_load_content(content: &str, line_num: usize) -> Result<ZigDirective, ParseError> {
    let file_path = content.trim().trim_matches('"').to_string();

    Ok(ZigDirective::Load {
        line_number: line_num,
        file_path,
    })
}

/// Helper: parse a string list like [ "a", "b", "c" ]
fn parse_string_list_content(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let trimmed = s.trim();

    // Extract content between brackets
    let inner = if trimmed.starts_with('[') && trimmed.ends_with(']') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };

    // Lists may be comma-delimited, newline-delimited, or use both forms.
    for part in inner.split([',', '\n', '\r']) {
        let p = part.trim();
        if !p.is_empty() {
            // Remove quotes if present
            result.push(p.trim_matches('"').to_string());
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_shortcut() {
        let config = "cmd - return : open -a Terminal\n";
        let result = parse_config(config);
        if let Err(ref errors) = result {
            for e in errors {
                eprintln!("Parse error: {:?}", e);
            }
        }
        assert!(result.is_ok());

        let parsed = result.unwrap();
        let shortcuts = parsed.shortcuts();
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].modifiers, vec!["cmd"]);
        assert_eq!(shortcuts[0].key, "return");
        assert_eq!(shortcuts[0].command, "open -a Terminal");
    }

    #[test]
    fn test_parse_multiple_modifiers() {
        let config = "cmd + shift - f : open ~\n";
        let result = parse_config(config);
        assert!(result.is_ok());

        let parsed = result.unwrap();
        let shortcuts = parsed.shortcuts();
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].modifiers, vec!["cmd", "shift"]);
        assert_eq!(shortcuts[0].key, "f");
    }

    #[test]
    fn test_parse_with_comments() {
        let config = "# This is a comment\ncmd - return : open -a Terminal\n";
        let result = parse_config(config);
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.comments().len(), 1);
        assert_eq!(parsed.shortcuts().len(), 1);
    }

    // ============================================================
    // skhd.zig Directive Tests (US-005)
    // ============================================================

    #[test]
    fn test_parse_alias_directive_modifier() {
        let config = ".alias $hyper cmd + alt + ctrl + shift\n";

        let result = parse_config(config);
        assert!(
            result.is_ok(),
            "Failed to parse alias directive: {:?}",
            result.err()
        );

        let parsed = result.unwrap();
        let directives = parsed.directives();
        assert_eq!(directives.len(), 1);

        match &directives[0] {
            ZigDirective::Alias { name, value, .. } => {
                assert_eq!(name, "$hyper");
                assert_eq!(value, "cmd + alt + ctrl + shift");
            }
            _ => panic!("Expected Alias directive"),
        }
    }

    #[test]
    fn test_parse_alias_directive_keycode() {
        let config = ".alias $grave 0x32\n";
        let result = parse_config(config);
        assert!(
            result.is_ok(),
            "Failed to parse keycode alias: {:?}",
            result.err()
        );

        let parsed = result.unwrap();
        let directives = parsed.directives();
        assert_eq!(directives.len(), 1);

        match &directives[0] {
            ZigDirective::Alias { name, value, .. } => {
                assert_eq!(name, "$grave");
                assert_eq!(value, "0x32");
            }
            _ => panic!("Expected Alias directive"),
        }
    }

    #[test]
    fn test_parse_define_group_directive() {
        let config = ".define terminal_apps [\"kitty\", \"wezterm\", \"terminal\"]\n";
        let result = parse_config(config);
        assert!(
            result.is_ok(),
            "Failed to parse define group: {:?}",
            result.err()
        );

        let parsed = result.unwrap();
        let directives = parsed.directives();
        assert_eq!(directives.len(), 1);

        match &directives[0] {
            ZigDirective::DefineGroup {
                name, applications, ..
            } => {
                assert_eq!(name, "terminal_apps");
                assert_eq!(applications, &["kitty", "wezterm", "terminal"]);
            }
            _ => panic!("Expected DefineGroup directive"),
        }
    }

    #[test]
    fn test_parse_define_command_directive() {
        let config = ".define yabai_focus : yabai -m window --focus {{1}}\n";
        let result = parse_config(config);
        assert!(
            result.is_ok(),
            "Failed to parse define command: {:?}",
            result.err()
        );

        let parsed = result.unwrap();
        let directives = parsed.directives();
        assert_eq!(directives.len(), 1);

        match &directives[0] {
            ZigDirective::DefineCommand { name, template, .. } => {
                assert_eq!(name, "yabai_focus");
                assert_eq!(template, "yabai -m window --focus {{1}}");
            }
            _ => panic!("Expected DefineCommand directive"),
        }
    }

    #[test]
    fn test_parse_path_directive_single() {
        let config = ".path \"/usr/local/bin\"\n";
        let result = parse_config(config);
        assert!(
            result.is_ok(),
            "Failed to parse path directive: {:?}",
            result.err()
        );

        let parsed = result.unwrap();
        let directives = parsed.directives();
        assert_eq!(directives.len(), 1);

        match &directives[0] {
            ZigDirective::Path { paths, .. } => {
                assert_eq!(paths, &["/usr/local/bin"]);
            }
            _ => panic!("Expected Path directive"),
        }
    }

    #[test]
    fn test_parse_path_directive_list() {
        let config = ".path [\"/opt/custom/bin\", \"$HOME/bin\"]\n";
        let result = parse_config(config);
        assert!(
            result.is_ok(),
            "Failed to parse path list: {:?}",
            result.err()
        );

        let parsed = result.unwrap();
        let directives = parsed.directives();
        assert_eq!(directives.len(), 1);

        match &directives[0] {
            ZigDirective::Path { paths, .. } => {
                assert_eq!(paths, &["/opt/custom/bin", "$HOME/bin"]);
            }
            _ => panic!("Expected Path directive"),
        }
    }

    #[test]
    fn test_parse_shell_directive() {
        let config = ".shell \"/bin/zsh\"\n";
        let result = parse_config(config);
        assert!(
            result.is_ok(),
            "Failed to parse shell directive: {:?}",
            result.err()
        );

        let parsed = result.unwrap();
        let directives = parsed.directives();
        assert_eq!(directives.len(), 1);

        match &directives[0] {
            ZigDirective::Shell { shell_path, .. } => {
                assert_eq!(shell_path, "/bin/zsh");
            }
            _ => panic!("Expected Shell directive"),
        }
    }

    #[test]
    fn test_parse_blacklist_directive() {
        let config = ".blacklist [\"loginwindow\", \"screensaver\"]\n";
        let result = parse_config(config);
        assert!(
            result.is_ok(),
            "Failed to parse blacklist directive: {:?}",
            result.err()
        );

        let parsed = result.unwrap();
        let directives = parsed.directives();
        assert_eq!(directives.len(), 1);

        match &directives[0] {
            ZigDirective::Blacklist { applications, .. } => {
                assert_eq!(applications, &["loginwindow", "screensaver"]);
            }
            _ => panic!("Expected Blacklist directive"),
        }
    }

    #[test]
    fn test_parse_load_directive() {
        let config = ".load \"~/.config/skhd/extra.skhdrc\"\n";
        let result = parse_config(config);
        assert!(
            result.is_ok(),
            "Failed to parse load directive: {:?}",
            result.err()
        );

        let parsed = result.unwrap();
        let directives = parsed.directives();
        assert_eq!(directives.len(), 1);

        match &directives[0] {
            ZigDirective::Load { file_path, .. } => {
                assert_eq!(file_path, "~/.config/skhd/extra.skhdrc");
            }
            _ => panic!("Expected Load directive"),
        }
    }

    #[test]
    fn test_parse_mixed_skhdrc_with_directives() {
        let config = r#"# skhd.zig config
.shell "/bin/zsh"
.alias $hyper cmd + alt + ctrl + shift
.define terminal_apps ["kitty", "wezterm", "terminal"]
.path "~/.cargo/bin"

# Standard shortcuts
cmd - return : open -a Terminal
hyper - h : echo "hyper-h"
"#;

        let result = parse_config(config);
        assert!(
            result.is_ok(),
            "Failed to parse mixed config: {:?}",
            result.err()
        );

        let parsed = result.unwrap();

        // Should have shortcuts
        let shortcuts = parsed.shortcuts();
        assert_eq!(shortcuts.len(), 2, "Expected 2 shortcuts");

        // Should have directives
        let directives = parsed.directives();
        assert_eq!(directives.len(), 4, "Expected 4 directives");

        // Should detect zig directives
        assert!(parsed.has_zig_directives());

        // Check directive counts
        let counts = parsed.directive_counts();
        assert_eq!(counts.get("shell"), Some(&1));
        assert_eq!(counts.get("alias"), Some(&1));
        assert_eq!(counts.get("define"), Some(&1));
        assert_eq!(counts.get("path"), Some(&1));
    }

    #[test]
    fn test_parse_multiline_directive_lists() {
        let config = ".blacklist [\n  \"loginwindow\"\n  \"screensaver\"\n]\n.path [\n  \"/opt/bin\",\n  \"$HOME/bin\"\n]\ncmd - a : echo ready\n";
        let parsed = parse_config(config).unwrap();
        let directives = parsed.directives();

        assert!(matches!(
            directives[0],
            ZigDirective::Blacklist { applications, .. }
                if applications == &["loginwindow", "screensaver"]
        ));
        assert!(matches!(
            directives[1],
            ZigDirective::Path { paths, .. }
                if paths == &["/opt/bin", "$HOME/bin"]
        ));
        assert_eq!(parsed.shortcuts()[0].line_number, 9);
    }

    #[test]
    fn test_parse_current_and_future_directives_losslessly() {
        let config = ".device builtin { vendor: 0x05AC, product: 0x0342 }\n.sequence_timeout 500ms\n.remap caps_lock [device builtin] {\n  tap: escape\n  hold: lctrl\n}\n";
        let parsed = parse_config(config).unwrap();
        let directives = parsed.directives();

        assert!(matches!(
            directives[0],
            ZigDirective::Generic { keyword, content, .. }
                if keyword == "device" && content.contains("vendor: 0x05AC")
        ));
        assert!(matches!(
            directives[1],
            ZigDirective::Generic { keyword, content, .. }
                if keyword == "sequence_timeout" && content == "500ms"
        ));
        assert!(matches!(
            directives[2],
            ZigDirective::Generic { keyword, content, .. }
                if keyword == "remap" && content.contains("hold: lctrl")
        ));
        assert_eq!(directives[2].line_number(), 3);
    }

    #[test]
    fn test_parse_mouse_keys() {
        let config = "cmd - mouse1 : echo 'cmd-click'\nmeh - mouse3 : open -a 'Mission Control'\n";
        let result = parse_config(config);
        assert!(
            result.is_ok(),
            "Failed to parse mouse keys: {:?}",
            result.err()
        );

        let parsed = result.unwrap();
        let shortcuts = parsed.shortcuts();
        assert_eq!(shortcuts.len(), 2);
        assert_eq!(shortcuts[0].key, "mouse1");
        assert_eq!(shortcuts[1].key, "mouse3");
    }

    #[test]
    fn test_parse_backtick_key() {
        let config = "cmd - backtick : echo 'backtick pressed'\n";
        let result = parse_config(config);
        assert!(
            result.is_ok(),
            "Failed to parse backtick key: {:?}",
            result.err()
        );

        let parsed = result.unwrap();
        let shortcuts = parsed.shortcuts();
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].key, "backtick");
    }

    #[test]
    fn test_parse_extended_modifiers() {
        let config =
            "hyper - h : echo 'hyper-h'\nmeh - t : echo 'meh-t'\nlcmd - a : echo 'left cmd'\n";
        let result = parse_config(config);
        assert!(
            result.is_ok(),
            "Failed to parse extended modifiers: {:?}",
            result.err()
        );

        let parsed = result.unwrap();
        let shortcuts = parsed.shortcuts();
        assert_eq!(shortcuts.len(), 3);
        assert_eq!(shortcuts[0].modifiers, vec!["hyper"]);
        assert_eq!(shortcuts[1].modifiers, vec!["meh"]);
        assert_eq!(shortcuts[2].modifiers, vec!["lcmd"]);
    }
}
