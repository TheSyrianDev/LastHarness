//! What the extra Settings pages show and send, without drawing anything: Tools and MCP, Channels, Web and phone
//! (devices), Your data, and Activity. Each page has its own file here; the window code that draws them is in the
//! app. Everything here is plain data in, plain words out, so it is tested without a window.

pub mod activity;
pub mod channels;
pub mod data;
pub mod hooks;
pub mod providers;
pub mod schedules;
pub mod service;
pub mod tools;
pub mod web;

use serde_json::Value;

/// The text under a name in a JSON object, or "".
pub fn s(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or("").to_string()
}

/// Split a typed command into the program and its arguments. Single and double quotes keep words together.
/// Nothing else is interpreted: no variables, no pipes, so what is shown is exactly what runs.
pub fn split_command(line: &str) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = vec![];
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut started = false;
    for ch in line.chars() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => cur.push(c),
            (None, '\'' | '"') => {
                quote = Some(ch);
                started = true;
            }
            (None, c) if c.is_whitespace() => {
                if started || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                    started = false;
                }
            }
            (None, c) => cur.push(c),
        }
    }
    if quote.is_some() {
        return Err("A quote is not closed.".into());
    }
    if started || !cur.is_empty() {
        out.push(cur);
    }
    Ok(out)
}

/// Days since 1970-01-01 to (year, month, day), proleptic Gregorian.
fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// A unix time as `2026-10-16 14:05` (UTC), the way the log lists it.
pub fn stamp(at: i64) -> String {
    let (y, m, d) = civil(at.div_euclid(86_400));
    let secs = at.rem_euclid(86_400);
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}", secs / 3600, secs % 3600 / 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_is_split_the_way_it_will_run() {
        assert_eq!(split_command("npx -y server").unwrap(), ["npx", "-y", "server"]);
        assert_eq!(split_command("python3 'my server.py' --flag \"a b\"").unwrap(), ["python3", "my server.py", "--flag", "a b"]);
        assert_eq!(split_command("  a   b ").unwrap(), ["a", "b"]);
        assert_eq!(split_command("a ''").unwrap(), ["a", ""]);
        assert!(split_command("").unwrap().is_empty());
        assert!(split_command("a 'b").is_err());
        assert_eq!(split_command("echo $HOME | cat").unwrap(), ["echo", "$HOME", "|", "cat"]);
    }

    #[test]
    fn times_read_as_dates() {
        assert_eq!(stamp(0), "1970-01-01 00:00");
        assert_eq!(stamp(1_791_590_400 + 4000), "2026-10-10 01:06");
        assert_eq!(stamp(1_709_164_800 + 5), "2024-02-29 00:00");
    }
}
