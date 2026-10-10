/// Incremental Server-Sent Events parser. Feed it bytes as they arrive; it returns the `data`
/// payload of each complete event. Handles events split across chunks, `\r\n` line ends,
/// multi-line data, comments, and multi-byte characters cut in half.
#[derive(Default)]
pub struct SseParser {
    buf: Vec<u8>,
}

impl SseParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, bytes: &[u8]) -> Vec<String> {
        self.buf.extend_from_slice(bytes);
        let mut out = Vec::new();
        while let Some((end, skip)) = find_event_end(&self.buf) {
            let raw: Vec<u8> = self.buf.drain(..end + skip).collect();
            let text = String::from_utf8_lossy(&raw[..end]);
            let mut data: Vec<&str> = Vec::new();
            for line in text.lines() {
                if let Some(rest) = line.strip_prefix("data:") {
                    data.push(rest.strip_prefix(' ').unwrap_or(rest));
                }
            }
            if !data.is_empty() {
                out.push(data.join("\n"));
            }
        }
        out
    }
}

/// Position of the first blank line (end of an event) and how many bytes it takes.
fn find_event_end(b: &[u8]) -> Option<(usize, usize)> {
    let mut i = 0;
    while i < b.len() {
        if b[i..].starts_with(b"\n\n") {
            return Some((i, 2));
        }
        if b[i..].starts_with(b"\r\n\r\n") {
            return Some((i, 4));
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_across_chunks_and_crlf() {
        let mut p = SseParser::new();
        assert!(p.push(b"data: {\"a\":").is_empty());
        assert_eq!(p.push(b"1}\r\n\r\ndata: [DONE]\n\n"), vec!["{\"a\":1}", "[DONE]"]);
    }

    #[test]
    fn multibyte_split_and_comments() {
        let mut p = SseParser::new();
        let s = "data: مرحبا\n\n".as_bytes();
        assert!(p.push(&s[..8]).is_empty());
        assert_eq!(p.push(&s[8..]), vec!["مرحبا"]);
        assert!(p.push(b": keep-alive\n\n").is_empty());
    }
}
