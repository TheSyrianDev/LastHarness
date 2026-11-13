//! Client-side fallback for tool calls the server left as text. Some Qwen3 chat templates make the model
//! write `<function=NAME><parameter=KEY>VALUE</parameter></function>` in the reply, and the llama-server
//! parser does not turn that into `tool_calls`. When a reply has no tool calls and holds such blocks,
//! they are turned into calls and removed from the text. Only complete blocks are taken; anything
//! else is left as the model wrote it.

use serde_json::{Map, Value};

use crate::types::{Response, ToolCall};

/// Turn raw `<function=...>` blocks in a reply with no tool calls into calls. Changes nothing otherwise.
pub fn recover(resp: &mut Response) {
    if !resp.tool_calls.is_empty() || !resp.text.contains("<function=") {
        return;
    }
    let (calls, rest) = parse(&resp.text);
    if calls.is_empty() {
        return;
    }
    resp.tool_calls = calls;
    resp.text = rest.trim().to_string();
}

/// Parse every complete `<function=NAME> ... </function>` block. Returns the calls and the text left over.
pub fn parse(text: &str) -> (Vec<ToolCall>, String) {
    let mut calls = Vec::new();
    let mut rest = String::new();
    let mut s = text;
    while let Some(start) = s.find("<function=") {
        let after = &s[start + "<function=".len()..];
        let Some(name_end) = after.find('>') else { break };
        let name = after[..name_end].trim().to_string();
        let body_start = &after[name_end + 1..];
        let Some(end) = body_start.find("</function>") else { break };
        let body = &body_start[..end];
        match params(body) {
            Some(args) if !name.is_empty() => {
                rest.push_str(&s[..start]);
                let idx = calls.len();
                calls.push(ToolCall {
                    id: format!("raw_call_{idx}"),
                    name,
                    arguments: Value::Object(args).to_string(),
                    repaired: false,
                });
            }
            _ => rest.push_str(&s[..start + "<function=".len() + name_end + 1 + end + "</function>".len()]),
        }
        s = &body_start[end + "</function>".len()..];
    }
    rest.push_str(s);
    (calls, rest)
}

/// Read `<parameter=KEY>VALUE</parameter>` pairs. Values stay strings, trimmed of one leading and one
/// trailing newline. Returns None when the body has no parameter, so the block is kept as text.
fn params(body: &str) -> Option<Map<String, Value>> {
    let mut out = Map::new();
    let mut s = body;
    while let Some(p) = s.find("<parameter=") {
        let after = &s[p + "<parameter=".len()..];
        let key_end = after.find('>')?;
        let key = after[..key_end].trim().to_string();
        let value_part = &after[key_end + 1..];
        let close = value_part.find("</parameter>")?;
        let value = value_part[..close].strip_prefix('\n').unwrap_or(&value_part[..close]);
        let value = value.strip_suffix('\n').unwrap_or(value);
        if key.is_empty() {
            return None;
        }
        out.insert(key, Value::String(value.to_string()));
        s = &value_part[close + "</parameter>".len()..];
    }
    if out.is_empty() { None } else { Some(out) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turns_a_complete_raw_block_into_a_call_and_keeps_other_text() {
        let mut r = Response { text: "Checking.\n<function=weather>\n<parameter=city>Paris</parameter>\n<parameter=unit>c</parameter>\n</function>".into(), ..Default::default() };
        recover(&mut r);
        assert_eq!(r.tool_calls.len(), 1);
        assert_eq!(r.tool_calls[0].name, "weather");
        assert_eq!(r.tool_calls[0].arguments, r#"{"city":"Paris","unit":"c"}"#);
        assert_eq!(r.text, "Checking.");
    }

    #[test]
    fn leaves_incomplete_or_real_calls_alone() {
        let mut cut = Response { text: "<function=calc>\n<parameter=expr>2^10</parameter>".into(), ..Default::default() };
        recover(&mut cut);
        assert!(cut.tool_calls.is_empty());
        assert!(cut.text.starts_with("<function=calc>"));

        let mut real = Response { text: "<function=calc>\n<parameter=expr>1</parameter>\n</function>".into(), tool_calls: vec![ToolCall { id: "c".into(), name: "x".into(), arguments: "{}".into(), repaired: false }], ..Default::default() };
        recover(&mut real);
        assert_eq!(real.tool_calls.len(), 1);
        assert_eq!(real.tool_calls[0].name, "x");
    }

    #[test]
    fn two_calls_in_one_reply() {
        let (calls, rest) = parse("<function=a><parameter=x>1</parameter></function>and<function=b><parameter=y>2</parameter></function>");
        assert_eq!(calls.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        assert_eq!(rest, "and");
    }
}

#[cfg(test)]
mod sample_tests {
    use super::*;

    /// The 20-case run from ai-8c (commit 1d0ffb9): every MISSED_RAW reply is a complete raw call and must
    /// now come back as one call with the same name. The SPURIOUS case shows the cost: a call the case did not want.
    #[test]
    fn every_missed_raw_sample_becomes_a_call() {
        let v: Value = serde_json::from_str(include_str!("../tests/fixtures/qwen3-tooltest-improved-20case.json")).unwrap();
        let mut seen = 0;
        for row in v["rows"].as_array().unwrap() {
            if row["verdict"] != "MISSED_RAW" {
                continue;
            }
            seen += 1;
            let text = row["content_head"].as_str().unwrap();
            let mut r = Response { text: text.into(), ..Default::default() };
            recover(&mut r);
            let want = row["case"].as_str().unwrap().split('_').next().unwrap();
            assert_eq!(r.tool_calls.len(), 1, "case {}", row["case"]);
            assert_eq!(r.tool_calls[0].name, want, "case {}", row["case"]);
            assert!(serde_json::from_str::<Value>(&r.tool_calls[0].arguments).is_ok());
        }
        assert_eq!(seen, 8);
    }
}
