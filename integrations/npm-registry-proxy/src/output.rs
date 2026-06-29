//! Terminal output escaping for attacker-influenced upstream response bytes.

// The proxy logs bounded raw upstream bytes, so escaping happens before UTF-8 decoding.
pub fn escape_for_terminal(bytes: &[u8]) -> String {
    let mut escaped = String::new();

    for byte in bytes {
        match byte {
            b'\\' => escaped.push_str(r"\\"),
            0x20..=0x7e => escaped.push(char::from(*byte)),
            _ => escaped.push_str(&format!(r"\x{byte:02x}")),
        }
    }

    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_terminal_control_sequences() {
        assert_eq!(
            escape_for_terminal(b"before\x1b[31mafter"),
            r"before\x1b[31mafter"
        );
    }

    #[test]
    fn escapes_backslashes() {
        assert_eq!(escape_for_terminal(b"a\\b"), r"a\\b");
    }
}
