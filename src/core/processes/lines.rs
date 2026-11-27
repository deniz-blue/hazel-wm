#[derive(Default)]
pub struct LineBuffer {
    pending: Vec<u8>,
}

impl LineBuffer {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        self.pending.extend_from_slice(bytes);
        let mut lines = Vec::new();
        while let Some(end) = self.pending.iter().position(|&byte| byte == b'\n') {
            let mut line: Vec<u8> = self.pending.drain(..=end).collect();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            lines.push(line);
        }
        lines
    }

    pub fn finish(&mut self) -> Option<Vec<u8>> {
        (!self.pending.is_empty()).then(|| std::mem::take(&mut self.pending))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_split_across_pushes() {
        let mut buffer = LineBuffer::default();
        assert_eq!(buffer.push(b"one\ntw"), [b"one".to_vec()]);
        assert_eq!(buffer.push(b"o\r\nthree\n"), [b"two".to_vec(), b"three".to_vec()]);
        assert_eq!(buffer.finish(), None);
    }

    #[test]
    fn a_partial_last_line_is_returned_on_finish() {
        let mut buffer = LineBuffer::default();
        assert!(buffer.push(b"tail").is_empty());
        assert_eq!(buffer.finish(), Some(b"tail".to_vec()));
        assert_eq!(buffer.finish(), None);
    }

    #[test]
    fn empty_lines_are_kept() {
        let mut buffer = LineBuffer::default();
        assert_eq!(buffer.push(b"\n\na\n"), [b"".to_vec(), b"".to_vec(), b"a".to_vec()]);
    }
}
