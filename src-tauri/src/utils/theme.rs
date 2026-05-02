/// macOS system theme detection utilities
///
/// This module provides shared utilities for detecting the macOS system theme
/// (light or dark mode) via NSUserDefaults and command-line fallback.
use objc::rc::autoreleasepool;
use objc::runtime::{Class, Object};
use objc::{msg_send, sel, sel_impl};
use std::ffi::{c_char, CStr};

/// Detect the current macOS system theme preference
///
/// Tries objc/NSUserDefaults first, falls back to `defaults` command if needed.
/// Returns "dark" or "light". Defaults to "dark" if all detection methods fail.
///
/// # Returns
/// - `Ok("light")` if system is in light mode
/// - `Ok("dark")` if system is in dark mode or detection fails
pub fn detect_system_theme() -> Result<String, String> {
    // Try using objc crate to access NSUserDefaults (primary method)
    match detect_theme_via_objc() {
        Ok(theme) => Ok(theme),
        Err(_) => {
            // Fallback to defaults command-line tool
            match detect_theme_via_defaults() {
                Ok(theme) => Ok(theme),
                Err(_) => {
                    // Ultimate fallback: default to dark mode
                    Ok("dark".to_string())
                }
            }
        }
    }
}

/// Detect theme using objc crate and NSUserDefaults (primary method)
///
/// # Safety
/// This function uses `unsafe` blocks to interact with Objective-C runtime APIs:
/// - `msg_send!` macro calls are inherently unsafe due to dynamic dispatch
/// - We verify that class lookups succeed before use
/// - All pointers are null-checked before dereferencing
/// - String conversion uses safe CStr wrapper for C string handling
/// - Runs within an autorelease pool to properly manage Objective-C memory
fn detect_theme_via_objc() -> Result<String, String> {
    autoreleasepool(|| unsafe {
        // Get NSUserDefaults class
        // Safety: Class::get is safe - it either returns Some(class) or None
        let user_defaults_class =
            Class::get("NSUserDefaults").ok_or("NSUserDefaults class not available")?;

        // Get standard user defaults instance
        // Safety: msg_send! calls Objective-C method; we null-check the result
        #[allow(unexpected_cfgs)]
        let standard_defaults: *mut Object = msg_send![user_defaults_class, standardUserDefaults];
        if standard_defaults.is_null() {
            return Err("Failed to get standard user defaults".to_string());
        }

        // Create NSString for the key "AppleInterfaceStyle"
        // Safety: Class::get is safe - checked for None
        let nsstring_class = Class::get("NSString").ok_or("NSString class not available")?;

        // Safety: c-string literal is null-terminated; msg_send! result is null-checked
        #[allow(unexpected_cfgs)]
        let key: *mut Object =
            msg_send![nsstring_class, stringWithUTF8String: c"AppleInterfaceStyle".as_ptr()];
        if key.is_null() {
            return Err("Failed to create NSString key".to_string());
        }

        // Read AppleInterfaceStyle value
        // Safety: msg_send! result is null-checked; null means key not set (light mode)
        #[allow(unexpected_cfgs)]
        let style_obj: *mut Object = msg_send![standard_defaults, objectForKey: key];
        if style_obj.is_null() {
            // Key not set means light mode
            return Ok("light".to_string());
        }

        // Convert NSString to Rust string
        // Safety: msg_send! returns C string pointer; we null-check before using
        #[allow(unexpected_cfgs)]
        let utf8_string: *const c_char = msg_send![style_obj, UTF8String];
        if utf8_string.is_null() {
            return Ok("light".to_string());
        }

        // Safety: CStr::from_ptr requires valid null-terminated C string
        // The UTF8String method guarantees this for NSString objects
        let c_str = CStr::from_ptr(utf8_string);
        let style_str = c_str
            .to_str()
            .map_err(|_| "Failed to convert NSString to UTF-8")?;

        // "Dark" means dark mode, anything else (or missing) means light mode
        if style_str == "Dark" {
            Ok("dark".to_string())
        } else {
            Ok("light".to_string())
        }
    })
}

/// Detect theme using defaults command-line tool (fallback method)
///
/// Executes `defaults read -g AppleInterfaceStyle` to read system preference.
/// Command failure typically indicates light mode (key doesn't exist).
fn detect_theme_via_defaults() -> Result<String, String> {
    let output = std::process::Command::new("defaults")
        .arg("read")
        .arg("-g")
        .arg("AppleInterfaceStyle")
        .output()
        .map_err(|e| format!("Failed to execute defaults command: {}", e))?;

    if !output.status.success() {
        // Command failed (likely means light mode - key doesn't exist)
        return Ok("light".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();

    if trimmed == "Dark" {
        Ok("dark".to_string())
    } else {
        Ok("light".to_string())
    }
}
