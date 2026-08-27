/**
 * TypeScript type definitions matching Rust models
 * These types ensure type safety across the Tauri bridge
 */

/**
 * Represents a keyboard shortcut from the skhd config
 */
export interface Shortcut {
  /** Unique identifier for tracking during editing */
  id: string;

  /** Modifier keys (cmd, alt, shift, ctrl, fn) - order-independent */
  modifiers: string[];

  /** Primary key being pressed */
  key: string;

  /** Shell command to execute */
  command: string;

  /** Optional mode name for modal shortcuts */
  mode?: string;

  /** Optional inline comment */
  comment?: string;

  /** Original line number from config file */
  line_number: number;
}

/**
 * Represents a parse error encountered during config parsing
 */
export interface ParseError {
  /** Line number where error occurred */
  line_number: number;

  /** Column number (if available) */
  column?: number;

  /** Type of error */
  error_type: string;

  /** Human-readable error message */
  message: string;

  /** Content of the line that caused the error */
  line_content: string;
}

/**
 * Represents the complete skhd configuration file
 */
export interface ConfigFile {
  /** Absolute path to config file */
  file_path: string;

  /** List of keyboard shortcuts (ordered by line number) */
  shortcuts: Shortcut[];

  /** Global comment lines not associated with shortcuts */
  global_comments: string[];

  /** Last modification timestamp (ISO 8601) */
  last_modified: string;

  /** Whether in-memory state differs from file */
  is_modified: boolean;

  /** Path to latest backup (if any) */
  backup_path?: string;

  /** Parse errors encountered (if any) */
  parse_errors: ParseError[];

  /** Tracks the currently active file path (where saves will write) */
  current_file_path: string;
}

/**
 * Represents a backup of a configuration file
 */
export interface Backup {
  /** Absolute path to the backup file */
  file_path: string;

  /** Path to the original config file */
  original_path: string;

  /** Timestamp when backup was created (ISO 8601) */
  created_at: string;

  /** SHA-256 checksum of backup content for integrity verification */
  checksum: string;

  /** Human-readable description of backup reason */
  description?: string;

  /** File size in bytes */
  size_bytes: number;
}

/**
 * Validation result containing errors and warnings
 */
export interface ValidationResult {
  /** Whether the validation passed (no errors) */
  is_valid: boolean;

  /** List of validation errors */
  errors: string[];

  /** List of validation warnings */
  warnings: string[];
}

/**
 * Request to create a new shortcut
 */
export interface CreateShortcutRequest {
  modifiers: string[];
  key: string;
  command: string;
  mode?: string;
  comment?: string;
}

/**
 * Request to update an existing shortcut
 */
export interface UpdateShortcutRequest {
  id: string;
  modifiers: string[];
  key: string;
  command: string;
  mode?: string;
  comment?: string;
}

/**
 * Result from testing a shortcut
 */
export interface TestResult {
  /** ID of the shortcut that was tested */
  shortcut_id: string;

  /** The command that was tested */
  command: string;

  /** Whether the command syntax is valid */
  syntax_valid: boolean;

  /** Syntax error message if invalid */
  syntax_error?: string;

  /** Preview of what the command would do */
  preview: string;

  /** Timestamp when the test was executed */
  timestamp: string;

  // ===== NEW EXECUTION FIELDS =====

  /** Whether this was an actual execution (true) or syntax validation only (false) */
  executed: boolean;

  /** Exit code from command execution (undefined for syntax-only tests) */
  exit_code?: number;

  /** Standard output from command execution (undefined for syntax-only tests) */
  stdout?: string;

  /** Standard error from command execution (undefined for syntax-only tests) */
  stderr?: string;

  /** Execution duration in milliseconds (undefined for syntax-only tests) */
  execution_duration_ms?: number;

  /** Whether the command was cancelled by the user */
  cancelled: boolean;

  /** Whether the command timed out */
  timed_out: boolean;

  /** Whether output was truncated due to size limit */
  output_truncated: boolean;
}

/**
 * Execution status for UI state management
 */
export type ExecutionStatus =
  | 'idle'
  | 'confirming'
  | 'executing'
  | 'success'
  | 'error'
  | 'cancelled'
  | 'timeout';

/**
 * Log level categorization for visual distinction
 */
export type LogLevel = 'ERROR' | 'WARN' | 'INFO' | 'DEBUG';

/**
 * Represents a single log line from skhd service
 */
export interface LogEntry {
  /** Unique identifier for deduplication and tracking */
  id: string;

  /** When the log entry was generated (ISO 8601) */
  timestamp: string;

  /** Severity level of the log entry */
  level: LogLevel;

  /** The actual log message content */
  message: string;

  /** Original unparsed log line (fallback for display) */
  raw: string;
}

/**
 * skhd service lifecycle states
 */
export type ServiceState =
  | 'Stopped'
  | 'Starting'
  | 'Running'
  | 'Stopping'
  | 'Reloading'
  | 'Error'
  | 'Unknown';

/**
 * Represents the current state of the skhd service
 */
export interface ServiceStatus {
  /** Current lifecycle state */
  state: ServiceState;

  /** Process ID if running, null if stopped */
  pid: number | null;

  /** When status was last checked (ISO 8601) */
  last_updated: string;

  /** Path to active configuration file */
  config_path: string | null;

  /** Error details if state is Error */
  error_message: string | null;
}

/**
 * Reference to a stored configuration file
 */
export interface ConfigurationReference {
  /** Unique identifier */
  id: string;

  /** Human-readable configuration name */
  name: string;

  /** Absolute path to configuration file */
  path: string;

  /** Whether this config is currently loaded in skhd */
  is_active: boolean;

  /** File modification timestamp (ISO 8601) */
  last_modified: string;

  /** Whether configuration passes syntax validation */
  valid: boolean;
}

/**
 * Represents an installed macOS application
 */
export interface Application {
  display_name: string;
  app_path: string;
  bundle_id: string;
  executable_path: string;
  icon_path?: string;
  version?: string;
}

/**
 * Represents a command template with parameters
 */
export interface CommandTemplate {
  id: string;
  name: string;
  description: string;
  category_id: string;
  command_pattern: string;
  parameters: CommandParameter[];
  requires_admin?: boolean;
}

/**
 * Represents a parameter for a command template
 */
export interface CommandParameter {
  name: string;
  description: string;
  data_type: 'string' | 'integer' | 'float' | 'boolean' | 'enum';
  default_value: string;
  validation_regex?: string;
  min_value?: number;
  max_value?: number;
  enum_values?: string[];
}

/**
 * Represents a category for organizing command templates
 */
export interface CommandCategory {
  id: string;
  name: string;
  description: string;
  icon?: string;
  display_order?: number;
}

/**
 * skhd variant types
 */
export type SkhdVariant = 'original' | 'zig';

/**
 * How the skhd variant was detected
 */
export type DetectionSource = 'running' | 'plist' | 'homebrew' | 'path' | 'app_bundle' | 'none';

/**
 * Represents a detected skhd variant with metadata
 */
export interface DetectedVariant {
  /** Which variant was detected (null if not detected) */
  variant: SkhdVariant | null;
  /** Path to the binary if found */
  binary_path: string | null;
  /** Launchd plist label (com.koekeishiya.skhd or com.jackielii.skhd) */
  plist_label: string | null;
  /** How the variant was detected */
  source: DetectionSource;
}

/**
 * User's preferred skhd variant setting
 */
export type SkhdVariantSetting = 'auto' | 'original' | 'zig';

/**
 * App settings that persist between launches
 */
export interface Settings {
  /** User's preferred skhd variant */
  skhd_variant: SkhdVariantSetting;
}

/**
 * Result from effective variant computation
 */
export interface EffectiveVariantResponse {
  /** The effective variant to use: 'original' | 'zig' */
  variant: SkhdVariant;
  /** Whether the variant was auto-detected */
  is_auto_detected: boolean;
  /** Optional warning message if chosen variant is not installed */
  warning: string | null;
  /** The detection source and metadata (if auto mode) */
  detected: DetectedVariant | null;
}
