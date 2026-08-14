//! Bounded circular ring buffer for terminal output history.
//!
//! Maintains a memory-safe fixed capacity (default 50,000 lines) with
//! automatic FIFO eviction and snapshot capabilities.

use std::collections::VecDeque;

/// Default line capacity for terminal scrollback buffer.
pub const DEFAULT_MAX_LINES: usize = 50_000;

/// Bounded circular line buffer for terminal scrollback history.
#[derive(Debug, Clone)]
pub struct RingBuffer {
    lines: VecDeque<String>,
    max_lines: usize,
    current_partial_line: String,
    total_lines_ingested: u64,
}

impl Default for RingBuffer {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_LINES)
    }
}

impl RingBuffer {
    /// Creates a new circular ring buffer with the specified line limit.
    pub fn new(max_lines: usize) -> Self {
        let capacity = max_lines.min(DEFAULT_MAX_LINES);
        Self {
            lines: VecDeque::with_capacity(capacity),
            max_lines,
            current_partial_line: String::with_capacity(256),
            total_lines_ingested: 0,
        }
    }

    /// Ingests a raw byte chunk from the PTY stream, assembling newlines and evicting old lines.
    pub fn push_chunk(&mut self, chunk: &[u8]) {
        let text = String::from_utf8_lossy(chunk);

        for ch in text.chars() {
            if ch == '\n' {
                let finished_line = std::mem::replace(&mut self.current_partial_line, String::with_capacity(256));
                self.push_completed_line(finished_line);
            } else if ch != '\r' {
                self.current_partial_line.push(ch);
                if self.current_partial_line.len() >= 1_000_000 {
                    let finished_line = std::mem::replace(&mut self.current_partial_line, String::with_capacity(256));
                    self.push_completed_line(finished_line);
                }
            }
        }
    }

    /// Appends a single completed line, evicting the oldest entry if max capacity is reached.
    pub fn push_line(&mut self, line: impl Into<String>) {
        self.push_completed_line(line.into());
    }

    fn push_completed_line(&mut self, line: String) {
        if self.lines.len() >= self.max_lines {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
        self.total_lines_ingested += 1;
    }

    /// Returns a snapshot clone of all stored lines, including any currently uncommitted partial line.
    pub fn get_snapshot(&self) -> Vec<String> {
        let mut snapshot: Vec<String> = self.lines.iter().cloned().collect();
        if !self.current_partial_line.is_empty() {
            snapshot.push(self.current_partial_line.clone());
        }
        snapshot
    }

    /// Returns up to the most recent `count` lines stored in the buffer.
    pub fn get_recent_lines(&self, count: usize) -> Vec<String> {
        let available = self.lines.len();
        let skip_count = available.saturating_sub(count);
        let mut result: Vec<String> = self.lines.iter().skip(skip_count).cloned().collect();
        if !self.current_partial_line.is_empty() && result.len() < count {
            result.push(self.current_partial_line.clone());
        }
        result
    }

    /// Returns the current number of completed lines in the buffer.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Returns true if the buffer contains no lines.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty() && self.current_partial_line.is_empty()
    }

    /// Clears all lines and partial buffer content.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.current_partial_line.clear();
    }

    /// Returns total lifetime count of lines ingested through this buffer.
    pub fn total_lines_ingested(&self) -> u64 {
        self.total_lines_ingested
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_fifo_eviction() {
        let mut buffer = RingBuffer::new(5);

        for i in 0..8 {
            buffer.push_line(format!("line {}", i));
        }

        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer.total_lines_ingested(), 8);
        assert_eq!(
            buffer.get_snapshot(),
            vec!["line 3", "line 4", "line 5", "line 6", "line 7"]
        );
    }

    #[test]
    fn test_push_chunk_newline_assembly() {
        let mut buffer = RingBuffer::new(10);
        buffer.push_chunk(b"hello world\nfoo bar\npartial");

        assert_eq!(buffer.len(), 2);
        assert_eq!(
            buffer.get_snapshot(),
            vec!["hello world", "foo bar", "partial"]
        );

        buffer.push_chunk(b" finished\n");
        assert_eq!(buffer.len(), 3);
        assert_eq!(
            buffer.get_snapshot(),
            vec!["hello world", "foo bar", "partial finished"]
        );
    }

    #[test]
    fn test_50000_line_stress_eviction() {
        let mut buffer = RingBuffer::new(50_000);

        for i in 0..60_000 {
            buffer.push_line(format!("line_{}", i));
        }

        assert_eq!(buffer.len(), 50_000);
        assert_eq!(buffer.total_lines_ingested(), 60_000);

        let recent = buffer.get_recent_lines(2);
        assert_eq!(recent, vec!["line_59998", "line_59999"]);
    }
}
