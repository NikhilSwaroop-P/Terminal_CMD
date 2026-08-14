//! High-performance ANSI escape sequence filter and LLM token optimizer.
//!
//! Strips CSI formatting, SGR color codes, OSC sequences, and control artifacts
//! to minimize token consumption when streaming terminal output to AI models.

use std::sync::LazyLock;
use regex::Regex;

static ANSI_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?x)
        \x1b\][^\x07\x1b]*(\x07|\x1b\\) |
        \x1b\[[0-9;?]*[a-zA-Z]          |
        \x1b[PX^_][^\x1b]*\x1b\\        |
        \x1b[@-Z\\-_]                   |
        [\x00-\x06\x07\x08\x0b\x0c\x0e-\x1a\x1c-\x1f]
    ").expect("valid ANSI regex")
});

/// Strips all ANSI escape codes, OSC sequences, and non-printable control characters from text.
pub fn strip_ansi(input: &str) -> String {
    let cleaned = ANSI_REGEX.replace_all(input, "");
    cleaned.replace("\r\n", "\n").replace('\r', "\n")
}

/// Tokenizer helper returning clean plaintext along with raw and stripped byte counts.
pub fn optimize_tokens(input: &str) -> (String, usize, usize) {
    let raw_len = input.len();
    let stripped = strip_ansi(input);
    let stripped_len = stripped.len();
    (stripped, raw_len, stripped_len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_sgr_color_codes() {
        let colored = "\x1b[31mRed Text\x1b[0m \x1b[1;34mBold Blue\x1b[0m";
        assert_eq!(strip_ansi(colored), "Red Text Bold Blue");
    }

    #[test]
    fn test_strip_osc_sequences() {
        let osc_text = "\x1b]133;A\x07prompt$ \x1b]7;file://host/path\x07ls\x1b]133;D;0\x07";
        assert_eq!(strip_ansi(osc_text), "prompt$ ls");
    }

    #[test]
    fn test_strip_csi_cursor_and_clear() {
        let csi_text = "\x1b[2J\x1b[H\x1b[?25hReady\x1b[K";
        assert_eq!(strip_ansi(csi_text), "Ready");
    }

    #[test]
    fn test_token_efficiency_measurement() {
        let noisy = "\x1b[38;2;255;100;0mTruecolor\x1b[0m\x1b[1m Text\x1b[0m\r\n";
        let (clean, raw_len, stripped_len) = optimize_tokens(noisy);
        assert_eq!(clean, "Truecolor Text\n");
        assert!(stripped_len < raw_len);
    }
}
