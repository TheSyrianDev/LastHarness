//! A small reader for the YAML subset that skill headers use: plain and quoted scalars, folded (`>`)
//! and literal (`|`) block scalars, one level of nested map (`metadata:`), and `- item` lists.
//! Not general YAML, on purpose: anything it does not understand is an error, not a guess.

use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Str(String),
    Map(BTreeMap<String, String>),
    List(Vec<String>),
}

fn indent(l: &str) -> usize {
    l.len() - l.trim_start_matches(' ').len()
}

fn unquote(v: &str) -> Result<String, String> {
    let v = v.trim();
    if let Some(inner) = v.strip_prefix('"').and_then(|x| x.strip_suffix('"')).filter(|_| v.len() >= 2) {
        let mut out = String::new();
        let mut it = inner.chars();
        while let Some(c) = it.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            match it.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('/') => out.push('/'),
                Some(o) => return Err(format!("unknown escape \\{o}")),
                None => return Err("a quoted value ends with a backslash".into()),
            }
        }
        return Ok(out);
    }
    if let Some(inner) = v.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')).filter(|_| v.len() >= 2) {
        return Ok(inner.replace("''", "'"));
    }
    if v.starts_with('"') || v.starts_with('\'') {
        return Err(format!("a quoted value is not closed: {v}"));
    }
    // plain: drop a trailing comment
    Ok(match v.find(" #") {
        Some(i) => v[..i].trim_end().to_string(),
        None => v.to_string(),
    })
}

fn block(lines: &[&str], folded: bool, strip: bool) -> String {
    let min = lines.iter().filter(|l| !l.trim().is_empty()).map(|l| indent(l)).min().unwrap_or(0);
    let body: Vec<&str> = lines.iter().map(|l| if l.len() >= min { &l[min..] } else { l.trim() }).collect();
    let text = if folded {
        let mut out = String::new();
        let mut blank_run = 0;
        for l in &body {
            if l.trim().is_empty() {
                blank_run += 1;
                continue;
            }
            if !out.is_empty() {
                out.push_str(&"\n".repeat(blank_run));
                if blank_run == 0 {
                    out.push(' ');
                }
            }
            blank_run = 0;
            out.push_str(l.trim_end());
        }
        out
    } else {
        body.join("\n").trim_end_matches('\n').to_string()
    };
    if strip { text } else { text.trim_end().to_string() }
}

pub fn parse(src: &str) -> Result<Vec<(String, Value)>, String> {
    let lines: Vec<&str> = src.lines().collect();
    let mut out = vec![];
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        if l.trim().is_empty() || l.trim_start().starts_with('#') {
            i += 1;
            continue;
        }
        if indent(l) != 0 {
            return Err(format!("unexpected indented line: {}", l.trim()));
        }
        let (key, rest) = l.split_once(':').ok_or_else(|| format!("not a key: {l}"))?;
        let key = key.trim().to_string();
        if key.is_empty() || key.contains(' ') && !key.starts_with('"') {
            return Err(format!("bad key: {key}"));
        }
        let rest = rest.trim();
        i += 1;
        // the lines that belong to this key: more indented, or blank
        let start = i;
        while i < lines.len() && (lines[i].trim().is_empty() || indent(lines[i]) > 0) {
            i += 1;
        }
        let child = &lines[start..i];
        let value = if matches!(rest, ">" | ">-" | ">+" | "|" | "|-" | "|+") {
            Value::Str(block(child, rest.starts_with('>'), rest.ends_with('-')))
        } else if rest.is_empty() {
            let real: Vec<&&str> = child.iter().filter(|c| !c.trim().is_empty() && !c.trim_start().starts_with('#')).collect();
            if real.is_empty() {
                Value::Str(String::new())
            } else if real.iter().all(|c| c.trim_start().starts_with("- ") || c.trim() == "-") {
                Value::List(real.iter().map(|c| unquote(c.trim_start().trim_start_matches('-'))).collect::<Result<_, _>>()?)
            } else {
                let mut m = BTreeMap::new();
                for c in real {
                    let (k, v) = c.trim().split_once(':').ok_or_else(|| format!("not a key: {}", c.trim()))?;
                    m.insert(k.trim().to_string(), unquote(v)?);
                }
                Value::Map(m)
            }
        } else {
            if !child.iter().all(|c| c.trim().is_empty()) {
                return Err(format!("{key}: a value on the same line cannot be followed by indented lines"));
            }
            Value::Str(unquote(rest)?)
        };
        out.push((key, value));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get<'a>(v: &'a [(String, Value)], k: &str) -> &'a Value {
        &v.iter().find(|(x, _)| x == k).unwrap().1
    }

    #[test]
    fn reads_the_header_shapes_real_skills_use() {
        let v = parse("name: diagnose-crash\ndescription: >\n  Diagnose why a program crashed on this machine, from a core dump.\n  Use when a process has segfaulted.\n  Triggers: crash, \"why did X crash\".\nlicense: Apache-2.0\n").unwrap();
        assert_eq!(get(&v, "name"), &Value::Str("diagnose-crash".into()));
        assert_eq!(get(&v, "description"), &Value::Str("Diagnose why a program crashed on this machine, from a core dump. Use when a process has segfaulted. Triggers: crash, \"why did X crash\".".into()));
        let v = parse("name: a\ndescription: \"Hugging Face Hub CLI (`hf`) for \\\"models\\\", and more.\"\n").unwrap();
        assert_eq!(get(&v, "description"), &Value::Str("Hugging Face Hub CLI (`hf`) for \"models\", and more.".into()));
        let v = parse("name: 'it''s'\ndescription: plain words # a comment\n").unwrap();
        assert_eq!((get(&v, "name"), get(&v, "description")), (&Value::Str("it's".into()), &Value::Str("plain words".into())));
    }

    #[test]
    fn maps_lists_and_literal_blocks() {
        let v = parse("name: x\nmetadata:\n  author: alex\n  version: \"1.0\"\nallowed-tools:\n  - Bash\n  - Read\nnotes: |\n  line one\n  line two\n").unwrap();
        assert_eq!(get(&v, "metadata"), &Value::Map([("author".to_string(), "alex".to_string()), ("version".to_string(), "1.0".to_string())].into()));
        assert_eq!(get(&v, "allowed-tools"), &Value::List(vec!["Bash".into(), "Read".into()]));
        assert_eq!(get(&v, "notes"), &Value::Str("line one\nline two".into()));
    }

    #[test]
    fn folded_blocks_keep_paragraph_breaks() {
        let v = parse("description: >\n  First paragraph\n  continues here.\n\n  Second paragraph.\n").unwrap();
        assert_eq!(get(&v, "description"), &Value::Str("First paragraph continues here.\nSecond paragraph.".into()));
    }

    #[test]
    fn what_it_does_not_understand_is_an_error() {
        assert!(parse("name: \"unclosed\n").is_err());
        assert!(parse("  indented: first\n").is_err());
        assert!(parse("no colon here\n").is_err());
        assert!(parse("key: value\n  stray: indent\n").is_err());
        assert!(parse("a: \"bad \\q escape\"\n").is_err());
    }
}
