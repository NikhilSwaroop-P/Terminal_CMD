//! Zero-allocation and streaming OSC 133 / OSC 7 ANSI escape sequence parser.
//!
//! Provides semantic state extraction for terminal command boundaries,
//! exit codes, and dynamic working directory tracking.

use std::path::PathBuf;

/// Semantic event emitted by terminal OSC sequence state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OscEvent {
    /// OSC 133;A - Terminal prompt started, waiting for user input.
    PromptStart,
    /// OSC 133;B - Command line entered, prompt finished.
    CommandStart,
    /// OSC 133;C - Command execution and output started.
    OutputStart,
    /// OSC 133;D;[exit_code] - Command execution completed with status.
    CommandFinished { exit_code: i32 },
    /// OSC 7;file://[host]/[path] - Current working directory changed.
    CwdChanged(PathBuf),
}

/// Streaming parser for ANSI OSC 133 and OSC 7 sequences.
#[derive(Debug, Default)]
pub struct OscParser {
    state: ParserState,
    osc_buffer: Vec<u8>,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum ParserState {
    #[default]
    Ground,
    Escape,
    OscPayload,
    OscStringTerminator,
}

impl OscParser {
    /// Creates a new streaming OSC parser instance.
    pub fn new() -> Self {
        Self {
            state: ParserState::Ground,
            osc_buffer: Vec::with_capacity(256),
        }
    }

    /// Feeds a chunk of streaming bytes into the parser and extracts detected OSC events.
    pub fn parse_chunk(&mut self, input: &[u8]) -> Vec<OscEvent> {
        let mut events = Vec::new();

        for &byte in input {
            match self.state {
                ParserState::Ground => {
                    if byte == 0x1B {
                        self.state = ParserState::Escape;
                    }
                }
                ParserState::Escape => {
                    if byte == b']' {
                        self.state = ParserState::OscPayload;
                        self.osc_buffer.clear();
                    } else if byte == 0x1B {
                        self.state = ParserState::Escape;
                    } else {
                        self.state = ParserState::Ground;
                    }
                }
                ParserState::OscPayload => {
                    if byte == 0x07 {
                        if let Some(event) = self.dispatch_osc_payload() {
                            events.push(event);
                        }
                        self.state = ParserState::Ground;
                        self.osc_buffer.clear();
                    } else if byte == 0x1B {
                        self.state = ParserState::OscStringTerminator;
                    } else {
                        if self.osc_buffer.len() < 4096 {
                            self.osc_buffer.push(byte);
                        } else {
                            self.state = ParserState::Ground;
                            self.osc_buffer.clear();
                        }
                    }
                }
                ParserState::OscStringTerminator => {
                    if byte == b'\\' {
                        if let Some(event) = self.dispatch_osc_payload() {
                            events.push(event);
                        }
                        self.state = ParserState::Ground;
                        self.osc_buffer.clear();
                    } else if byte == 0x1B {
                        self.state = ParserState::OscStringTerminator;
                    } else {
                        self.state = ParserState::Ground;
                        self.osc_buffer.clear();
                    }
                }
            }
        }

        events
    }

    fn dispatch_osc_payload(&self) -> Option<OscEvent> {
        let payload = std::str::from_utf8(&self.osc_buffer).ok()?;

        if let Some(rest) = payload.strip_prefix("133;") {
            return Self::parse_osc_133(rest);
        }

        if let Some(rest) = payload.strip_prefix("7;") {
            return Self::parse_osc_7(rest);
        }

        None
    }

    fn parse_osc_133(payload: &str) -> Option<OscEvent> {
        let mut parts = payload.split(';');
        let action = parts.next()?;

        match action {
            "A" => Some(OscEvent::PromptStart),
            "B" => Some(OscEvent::CommandStart),
            "C" => Some(OscEvent::OutputStart),
            "D" => {
                let exit_code = parts.next().and_then(|c| c.parse::<i32>().ok()).unwrap_or(0);
                Some(OscEvent::CommandFinished { exit_code })
            }
            _ => None,
        }
    }

    fn parse_osc_7(payload: &str) -> Option<OscEvent> {
        let url_str = payload.trim();
        let path_str = if let Some(stripped) = url_str.strip_prefix("file://") {
            if let Some(first_slash) = stripped.find('/') {
                &stripped[first_slash..]
            } else {
                stripped
            }
        } else {
            url_str
        };

        if path_str.is_empty() {
            return None;
        }

        let decoded = Self::percent_decode(path_str);
        Some(OscEvent::CwdChanged(PathBuf::from(decoded)))
    }

    fn percent_decode(input: &str) -> String {
        let mut bytes_out = Vec::with_capacity(input.len());
        let bytes_in = input.as_bytes();
        let mut i = 0;

        while i < bytes_in.len() {
            if bytes_in[i] == b'%' && i + 2 < bytes_in.len() {
                if let Ok(val) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                    bytes_out.push(val);
                    i += 3;
                    continue;
                }
            }
            bytes_out.push(bytes_in[i]);
            i += 1;
        }

        String::from_utf8_lossy(&bytes_out).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc133_markers_bel() {
        let mut parser = OscParser::new();

        let events_a = parser.parse_chunk(b"\x1b]133;A\x07");
        assert_eq!(events_a, vec![OscEvent::PromptStart]);

        let events_b = parser.parse_chunk(b"\x1b]133;B\x07");
        assert_eq!(events_b, vec![OscEvent::CommandStart]);

        let events_c = parser.parse_chunk(b"\x1b]133;C\x07");
        assert_eq!(events_c, vec![OscEvent::OutputStart]);

        let events_d = parser.parse_chunk(b"\x1b]133;D;42\x07");
        assert_eq!(events_d, vec![OscEvent::CommandFinished { exit_code: 42 }]);

        let events_d0 = parser.parse_chunk(b"\x1b]133;D\x07");
        assert_eq!(events_d0, vec![OscEvent::CommandFinished { exit_code: 0 }]);
    }

    #[test]
    fn test_osc133_markers_st() {
        let mut parser = OscParser::new();

        let events = parser.parse_chunk(b"\x1b]133;A\x1b\\hello\x1b]133;D;1\x1b\\");
        assert_eq!(
            events,
            vec![
                OscEvent::PromptStart,
                OscEvent::CommandFinished { exit_code: 1 }
            ]
        );
    }

    #[test]
    fn test_osc7_cwd_parsing() {
        let mut parser = OscParser::new();

        let events = parser.parse_chunk(b"\x1b]7;file://localhost/home/user/project%20folder\x07");
        assert_eq!(
            events,
            vec![OscEvent::CwdChanged(PathBuf::from("/home/user/project folder"))]
        );
    }

    #[test]
    fn test_osc7_utf8_percent_decoding() {
        let mut parser = OscParser::new();

        let events = parser.parse_chunk(b"\x1b]7;file://localhost/home/user/%E2%9C%A8%20star%20%C3%A9tude\x07");
        assert_eq!(
            events,
            vec![OscEvent::CwdChanged(PathBuf::from("/home/user/✨ star étude"))]
        );
    }

    #[test]
    fn test_split_across_chunks() {
        let mut parser = OscParser::new();

        let e1 = parser.parse_chunk(b"\x1b]133;");
        assert!(e1.is_empty());

        let e2 = parser.parse_chunk(b"D;130\x07");
        assert_eq!(e2, vec![OscEvent::CommandFinished { exit_code: 130 }]);
    }
}
