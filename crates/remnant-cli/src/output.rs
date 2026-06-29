//! Output escaping helpers.
//!
//! Terminal output may include attacker-controlled package metadata, archive
//! paths, script names, or user-provided filesystem paths. Keep escaping small,
//! deterministic, and centralized so human-readable output does not emit raw
//! control characters.
//!
//! The npm registry proxy uses a separate byte-level escaping helper for
//! bounded raw upstream bytes that have not been decoded as UTF-8; see
//! `integrations/npm-registry-proxy/src/output.rs`.

use std::path::Path;

/// Escapes text for deterministic human-readable terminal output.
///
/// This does not add surrounding quotes. Control characters, backslashes, and
/// other debug-escaped characters are rendered as visible escape sequences.
pub fn escape_terminal_text(value: &str) -> String {
    value.escape_debug().to_string()
}

/// Escapes a filesystem or archive path for deterministic terminal output.
///
/// Paths that are not valid Unicode are rendered with Rust's deterministic
/// lossy conversion before terminal escaping. Archive path validation rejects
/// non-UTF-8 accepted paths, but rejected paths and user-provided filesystem
/// paths can still reach error output.
pub fn escape_terminal_path(path: &Path) -> String {
    escape_terminal_text(&path.as_os_str().to_string_lossy())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn escapes_control_characters_in_terminal_text() {
        assert_eq!(
            escape_terminal_text("demo\npackage\tname\r"),
            r"demo\npackage\tname\r"
        );
    }

    #[test]
    fn escapes_backslashes_in_terminal_text() {
        assert_eq!(escape_terminal_text(r"package\file"), r"package\\file");
    }

    #[test]
    fn escapes_control_characters_in_terminal_paths() {
        assert_eq!(
            escape_terminal_path(Path::new("package/file\nname.js")),
            r"package/file\nname.js"
        );
    }
}
