/// Close a JSON text that was cut short (a stream that ended early, or a model that stopped
/// mid-argument). Returns `None` if the text is not a recoverable object or array.
/// Only closes what is open; it does not invent missing values.
pub fn repair_json(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() {
        return Some("{}".into());
    }
    let mut out = String::from(s);
    let mut stack: Vec<char> = Vec::new();
    let (mut in_str, mut esc) = (false, false);
    for c in s.chars() {
        if in_str {
            if esc { esc = false } else if c == '\\' { esc = true } else if c == '"' { in_str = false }
            continue;
        }
        match c {
            '"' => in_str = true,
            '{' => stack.push('}'),
            '[' => stack.push(']'),
            '}' | ']' if stack.pop() != Some(c) => return None,
            _ => {}
        }
    }
    if esc {
        out.pop();
    }
    if in_str {
        out.push('"');
    }
    // A dangling `"key":` or `,` cannot be completed; drop it back to the last full member.
    let trimmed = out.trim_end();
    if trimmed.ends_with(':') {
        let cut = trimmed.rfind(',').or_else(|| trimmed.rfind(['{', '[']).map(|i| i + 1))?;
        out.truncate(cut);
    } else if trimmed.ends_with(',') {
        out.truncate(trimmed.len() - 1);
    }
    while let Some(c) = stack.pop() {
        out.push(c);
    }
    serde_json::from_str::<serde_json::Value>(&out).ok().map(|_| out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repairs_common_cuts() {
        assert_eq!(repair_json("{\"city\":\"Par").unwrap(), "{\"city\":\"Par\"}");
        assert_eq!(repair_json("{\"a\":1,").unwrap(), "{\"a\":1}");
        assert_eq!(repair_json("{\"a\":1,\"b\":").unwrap(), "{\"a\":1}");
        assert_eq!(repair_json("{\"a\":[1,2").unwrap(), "{\"a\":[1,2]}");
        assert_eq!(repair_json("").unwrap(), "{}");
    }

    #[test]
    fn refuses_garbage() {
        assert!(repair_json("{\"a\":1}}").is_none());
        assert!(repair_json("not json").is_none());
    }
}
