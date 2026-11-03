//! Something the assistant (or the user) opened to look at properly, and which viewer suits it.

/// A piece of content for the side panel or a window of its own.
#[derive(Clone, Debug, PartialEq)]
pub struct Content {
    pub title: String,
    pub kind: ContentKind,
    pub body: String,
    /// For a web page: the file the core approved, so it can be opened in the person's browser.
    pub path: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentKind {
    Markdown,
    Code,
    Text,
    /// `body` is a file path.
    Image,
    /// A web page or a small app: `body` is its source; `path` is the file to open in a browser, if any.
    Html,
    /// Columns and rows: `body` is the JSON the core checked (see `Table::parse`).
    Table,
    /// Numbers to draw: `body` is the JSON the core checked (see `Chart::parse`).
    Chart,
}

/// A table the assistant made: plain values only, so every front end can draw it.
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub caption: Option<String>,
    /// For each column: are all its values numbers? Those are lined up on the right.
    pub numeric: Vec<bool>,
}

fn plain(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

impl Table {
    pub fn parse(body: &str) -> Option<Table> {
        let v: serde_json::Value = serde_json::from_str(body).ok()?;
        let columns: Vec<String> = v["columns"].as_array()?.iter().map(plain).collect();
        let raw = v["rows"].as_array()?;
        let rows: Vec<Vec<String>> = raw.iter().filter_map(|r| r.as_array().map(|r| r.iter().map(plain).collect())).collect();
        let numeric = (0..columns.len()).map(|c| !raw.is_empty() && raw.iter().all(|r| r.get(c).is_some_and(|x| x.is_number() || x.is_null()))).collect();
        Some(Table { columns, rows, caption: v["caption"].as_str().map(str::to_string), numeric })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChartKind {
    Bar,
    Line,
    Pie,
}

/// A chart the assistant made: labels, and one or more series of numbers, one per label.
#[derive(Clone, Debug, PartialEq)]
pub struct Chart {
    pub kind: ChartKind,
    pub labels: Vec<String>,
    pub series: Vec<(String, Vec<f64>)>,
    pub caption: Option<String>,
}

impl Chart {
    pub fn parse(body: &str) -> Option<Chart> {
        let v: serde_json::Value = serde_json::from_str(body).ok()?;
        let kind = match v["type"].as_str()? {
            "bar" => ChartKind::Bar,
            "line" => ChartKind::Line,
            "pie" => ChartKind::Pie,
            _ => return None,
        };
        let labels: Vec<String> = v["labels"].as_array()?.iter().map(plain).collect();
        let series = v["series"].as_array()?.iter().enumerate().map(|(i, s)| {
            let name = s["name"].as_str().map(str::to_string).unwrap_or_else(|| format!("Series {}", i + 1));
            (name, s["values"].as_array().map(|a| a.iter().map(|x| x.as_f64().unwrap_or(0.)).collect()).unwrap_or_default())
        }).collect();
        Some(Chart { kind, labels, series, caption: v["caption"].as_str().map(str::to_string) })
    }

    /// The value axis: from zero (or the lowest value, if below zero) to a round top, and the step between ticks.
    pub fn axis(&self) -> (f64, f64, f64) {
        let all = self.series.iter().flat_map(|s| s.1.iter().copied());
        let (lo, hi) = all.fold((0f64, 0f64), |(a, b), v| (a.min(v), b.max(v)));
        let span = (hi - lo).max(1e-9);
        let raw = span / 4.;
        let mag = 10f64.powf(raw.log10().floor());
        let step = [1., 2., 2.5, 5., 10.].iter().map(|m| m * mag).find(|s| *s >= raw).unwrap_or(10. * mag);
        ((lo / step).floor() * step, (hi / step).ceil() * step, step)
    }

    /// Where a value sits between the bottom (0) and the top (1) of the plot.
    pub fn level(&self, v: f64) -> f32 {
        let (lo, hi, _) = self.axis();
        (((v - lo) / (hi - lo).max(1e-9)) as f32).clamp(0., 1.)
    }

    /// For a pie: each slice's start and end angle, in turns (0 to 1), clockwise from the top.
    pub fn slices(&self) -> Vec<(f32, f32)> {
        let vals: Vec<f64> = self.series.first().map(|s| s.1.iter().map(|v| v.max(0.)).collect()).unwrap_or_default();
        let total: f64 = vals.iter().sum();
        let mut at = 0f32;
        vals.iter().map(|v| {
            let w = if total > 0. { (*v / total) as f32 } else { 0. };
            let s = (at, at + w);
            at += w;
            s
        }).collect()
    }
}

/// A number as a person reads it on an axis or in a table: no trailing zeros, thousands grouped.
pub fn short_number(v: f64) -> String {
    if v.abs() >= 1e6 {
        return format!("{}M", trim((v / 1e6 * 10.).round() / 10.));
    }
    if v.abs() >= 1e4 {
        return format!("{}k", trim((v / 1e3 * 10.).round() / 10.));
    }
    trim((v * 100.).round() / 100.)
}

fn trim(v: f64) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

impl Content {
    /// What the assistant's `show` tool sent: the kind it named, the text, or for an image a path.
    pub fn from_show(title: &str, kind: &str, content: &str, path: Option<&str>) -> Content {
        // a file's own extension wins over what the model called it: a .md file shown as "text" still reads formatted
        let named = path.unwrap_or(title).to_lowercase();
        let kind = if named.ends_with(".md") || named.ends_with(".markdown") { "markdown" } else if (named.ends_with(".html") || named.ends_with(".htm")) && kind != "image" { "html" } else { kind };
        match (kind, path) {
            ("image", Some(p)) => Content { title: title.into(), kind: ContentKind::Image, body: p.into(), path: None },
            ("table", _) => Content { title: title.into(), kind: ContentKind::Table, body: content.into(), path: None },
            ("chart", _) => Content { title: title.into(), kind: ContentKind::Chart, body: content.into(), path: None },
            ("html" | "app", _) => Content { title: title.into(), kind: ContentKind::Html, body: content.into(), path: path.map(str::to_string) },
            ("code", _) => Content { title: title.into(), kind: ContentKind::Code, body: content.into(), path: None },
            ("text", _) => Content { title: title.into(), kind: ContentKind::Text, body: content.into(), path: None },
            ("markdown", _) => Content { title: title.into(), kind: ContentKind::Markdown, body: content.into(), path: None },
            _ => Content::detect(title, content),
        }
    }


    /// Choose a viewer for a piece of text or a path.
    pub fn detect(title: &str, body: &str) -> Content {
        let t = body.trim();
        let kind = if !t.contains('\n') && [".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp"].iter().any(|e| t.to_lowercase().ends_with(e)) && std::path::Path::new(t).is_file() {
            ContentKind::Image
        } else if t.starts_with("```") && t.trim_end().ends_with("```") && t.matches("```").count() == 2 {
            ContentKind::Code
        } else if t.lines().any(|l| l.starts_with('#') || l.starts_with("- ") || l.starts_with("* ") || l.starts_with("> ") || l.contains("](") || l.starts_with("|") || l.contains("**") || l.starts_with("```")) {
            ContentKind::Markdown
        } else {
            ContentKind::Text
        };
        let body = if kind == ContentKind::Code { t.trim_start_matches("```").lines().skip(1).collect::<Vec<_>>().join("\n").trim_end_matches("```").trim_end().to_string() } else { body.to_string() };
        Content { title: title.to_string(), kind, body, path: None }
    }

    /// Should a reply offer "Open in panel"? Long replies, and any reply that holds a code block or a table
    /// whatever its length, since those read better larger and apart from the chat.
    pub fn worth_opening(reply: &str) -> bool {
        reply.chars().count() > 400 || reply.contains("```") || has_table(reply)
    }

    /// The local file a browser may be asked to open for this page, or nothing. Only a web page that came with a
    /// path the core approved, that is an absolute path to an existing `.html` or `.htm` file; never a web address.
    pub fn browser_target(&self) -> Option<std::path::PathBuf> {
        if self.kind != ContentKind::Html {
            return None;
        }
        let p = self.path.as_deref()?;
        if p.contains("://") || p.starts_with("file:") {
            return None;
        }
        let path = std::path::PathBuf::from(p);
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        (path.is_absolute() && path.is_file() && (ext == "html" || ext == "htm")).then_some(path)
    }
}

/// Keep the model's single line breaks. Markdown joins two lines that have no blank line between them, but models
/// write short lines on purpose (addresses, poems, steps without list marks). Each such line gets a hard break
/// (two trailing spaces). Code blocks, tables, headings and lines next to a blank line or a new block are left alone.
pub fn keep_line_breaks(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out = String::with_capacity(text.len() + lines.len() * 2);
    let mut fence: Option<&str> = None;
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim_start();
        let marker = if t.starts_with("```") { Some("```") } else if t.starts_with("~~~") { Some("~~~") } else { None };
        let mut hard = false;
        match (fence, marker) {
            (Some(f), Some(m)) if f == m => fence = None,
            (Some(_), _) => {}
            (None, Some(m)) => fence = Some(m),
            (None, None) => {
                let next = lines.get(i + 1).map(|n| n.trim_start());
                hard = matches!(next, Some(n) if !n.trim().is_empty() && !starts_block(n))
                    && !line.trim().is_empty()
                    && !t.starts_with('#')
                    && !t.starts_with('|')
                    && !line.ends_with("  ")
                    && !line.ends_with('\\');
            }
        }
        out.push_str(line.trim_end_matches('\r'));
        if hard {
            out.push_str("  ");
        }
        if i + 1 < lines.len() {
            out.push('\n');
        }
    }
    out
}

/// Does this line open a block of its own (so the line before it needs no break)?
fn starts_block(t: &str) -> bool {
    let list = t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ ")
        || t.split_once(['.', ')']).is_some_and(|(n, rest)| !n.is_empty() && n.len() <= 9 && n.chars().all(|c| c.is_ascii_digit()) && (rest.is_empty() || rest.starts_with(' ')));
    let rule = t.len() >= 3 && (t.chars().all(|c| c == '-' || c == ' ') || t.chars().all(|c| c == '=' || c == ' ') || t.chars().all(|c| c == '*' || c == ' '));
    list || rule || t.starts_with('#') || t.starts_with('>') || t.starts_with('|') || t.starts_with("```") || t.starts_with("~~~")
}

/// A markdown table: a row of cells followed by the divider row (`|---|---|`, with optional colons).
fn has_table(text: &str) -> bool {
    let mut lines = text.lines().map(str::trim);
    let mut prev = lines.next().unwrap_or("");
    for line in lines {
        let divider = line.contains('-') && line.contains('|') && line.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '));
        if divider && prev.contains('|') {
            return true;
        }
        prev = line;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_extension_wins_over_the_kind() {
        assert_eq!(Content::from_show("digest.md", "text", "# Hi", None).kind, ContentKind::Markdown);
        assert_eq!(Content::from_show("x", "text", "# Hi", Some("/a/b/notes.markdown")).kind, ContentKind::Markdown);
        assert_eq!(Content::from_show("page.html", "text", "<p>x</p>", None).kind, ContentKind::Html);
        assert_eq!(Content::from_show("plain", "text", "# Hi", None).kind, ContentKind::Text);
    }

    #[test]
    fn single_line_breaks_are_kept() {
        assert_eq!(keep_line_breaks("one\ntwo\nthree"), "one  \ntwo  \nthree");
        assert_eq!(keep_line_breaks("para\n\nnext"), "para\n\nnext", "blank lines stay paragraph breaks");
        assert_eq!(keep_line_breaks("intro\n- a\n- b"), "intro\n- a\n- b", "a list needs no break before it");
        assert_eq!(keep_line_breaks("Steps:\n1. go\n2. stop"), "Steps:\n1. go\n2. stop");
        assert_eq!(keep_line_breaks("```\na\nb\n```"), "```\na\nb\n```", "code is left alone");
        assert_eq!(keep_line_breaks("| a | b |\n|---|---|\n| 1 | 2 |"), "| a | b |\n|---|---|\n| 1 | 2 |");
        assert_eq!(keep_line_breaks("# Title\ntext"), "# Title\ntext");
        assert_eq!(keep_line_breaks("Title\n---"), "Title\n---", "a setext heading stays a heading");
        assert_eq!(keep_line_breaks("done  \nnext"), "done  \nnext", "an existing break is not doubled");
        assert_eq!(keep_line_breaks("سطر\nسطر"), "سطر  \nسطر");
    }

    #[test]
    fn content_gets_the_viewer_that_suits_it() {
        assert_eq!(Content::detect("t", "plain words\nmore words").kind, ContentKind::Text);
        assert_eq!(Content::detect("t", "# Title\n\ntext").kind, ContentKind::Markdown);
        assert_eq!(Content::detect("t", "- one\n- two").kind, ContentKind::Markdown);
        assert_eq!(Content::detect("t", "| a | b |\n|---|---|\n| 1 | 2 |").kind, ContentKind::Markdown, "tables go to the markdown viewer");
        let c = Content::detect("t", "```rust\nfn main() {}\n```");
        assert_eq!(c.kind, ContentKind::Code);
        assert_eq!(c.body, "fn main() {}");
        assert_eq!(Content::detect("t", "intro\n```x\ncode\n```\noutro").kind, ContentKind::Markdown, "text around code is markdown");
        assert_eq!(Content::detect("t", "/no/such/file.png").kind, ContentKind::Text, "an image path that does not exist is just text");
    }

    #[test]
    fn a_reply_offers_the_panel_when_it_is_long_or_holds_code_or_a_table() {
        assert!(!Content::worth_opening("Short and plain."));
        assert!(!Content::worth_opening("a - b | c"), "a dash and a bar are not a table");
        assert!(Content::worth_opening(&"word ".repeat(100)), "over 400 characters");
        assert!(!Content::worth_opening(&"x".repeat(400)), "400 is not over");
        assert!(Content::worth_opening("Try:\n```rust\nfn main() {}\n```"), "a code block, however short");
        assert!(Content::worth_opening("| a | b |\n|---|---|\n| 1 | 2 |"), "a table, however short");
        assert!(Content::worth_opening("Name | Age\n:--- | ---:\nAda | 36"), "a table without outer bars");
        assert!(Content::worth_opening("intro\n\n| a | b |\n| :-: | --- |\n| 1 | 2 |\n\nafter"), "a table in the middle");
        assert!(!Content::worth_opening("| only a row |"), "a row with no divider is not a table");
        assert!(Content::worth_opening(&"ن".repeat(401)), "counted in characters, not bytes");
        assert!(!Content::worth_opening(&"ن".repeat(300)), "300 Arabic letters are 600 bytes but short");
    }

    #[test]
    fn an_existing_image_file_opens_in_the_image_viewer() {
        let p = std::env::temp_dir().join(format!("lh-gui-img-{}.png", std::process::id()));
        std::fs::write(&p, b"x").unwrap();
        let c = Content::detect("pic", &p.to_string_lossy());
        assert_eq!(c.kind, ContentKind::Image);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn what_the_assistant_shows_keeps_the_kind_it_named() {
        assert_eq!(Content::from_show("t", "code", "x = 1", None).kind, ContentKind::Code);
        assert_eq!(Content::from_show("t", "text", "# not markdown", None).kind, ContentKind::Text);
        assert_eq!(Content::from_show("t", "markdown", "plain", None).kind, ContentKind::Markdown);
        let i = Content::from_show("pic", "image", "", Some("/p/a.png"));
        assert_eq!((i.kind, i.body.as_str()), (ContentKind::Image, "/p/a.png"));
        assert_eq!(Content::from_show("t", "something new", "# Title", None).kind, ContentKind::Markdown, "an unknown kind is guessed from the text");
    }

    #[test]
    fn a_web_page_keeps_its_source_and_only_a_local_html_file_may_go_to_the_browser() {
        let d = std::env::temp_dir().join(format!("lh-gui-html-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let page = d.join("index.html");
        std::fs::write(&page, "<h1>x</h1>").unwrap();
        std::fs::write(d.join("notes.txt"), "x").unwrap();
        let p = page.to_string_lossy().to_string();
        let c = Content::from_show("Page", "html", "<h1>x</h1>", Some(&p));
        assert_eq!((c.kind, c.body.as_str()), (ContentKind::Html, "<h1>x</h1>"));
        assert_eq!(c.browser_target(), Some(page.clone()));
        // no path: source only
        assert_eq!(Content::from_show("Page", "html", "<p>", None).browser_target(), None);
        // never a web address, never another kind of file, never a file that is gone
        for bad in ["https://example.com/a.html", "http://x/a.html", "file:///etc/x.html", "relative/index.html", &d.join("notes.txt").to_string_lossy(), &d.join("gone.html").to_string_lossy()] {
            assert_eq!(Content::from_show("Page", "html", "", Some(bad)).browser_target(), None, "{bad}");
        }
        // only a web page has a browser target, even when it carries a path
        let mut other = c.clone();
        other.kind = ContentKind::Code;
        assert_eq!(other.browser_target(), None);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn tables_and_charts_are_read_from_the_checked_json() {
        let t = Table::parse(r#"{"columns":["Plan","Price"],"rows":[["Starter",10],["Pro",30.5]],"caption":"per month"}"#).unwrap();
        assert_eq!(t.rows[1], vec!["Pro".to_string(), "30.5".into()]);
        assert_eq!(t.numeric, vec![false, true]);
        assert_eq!(Content::from_show("T", "table", "{}", None).kind, ContentKind::Table);
        assert_eq!(Content::from_show("A", "app", "<p>", None).kind, ContentKind::Html);
        let c = Chart::parse(r#"{"type":"bar","labels":["a","b"],"series":[{"name":"x","values":[2.4,5]}]}"#).unwrap();
        assert_eq!(c.axis(), (0., 6., 2.));
        assert_eq!(c.level(6.), 1.);
        assert_eq!(c.level(0.), 0.);
        let pie = Chart::parse(r#"{"type":"pie","labels":["a","b"],"series":[{"values":[1,3]}]}"#).unwrap();
        assert_eq!(pie.slices(), vec![(0., 0.25), (0.25, 1.)]);
        assert_eq!(pie.series[0].0, "Series 1");
        assert_eq!((short_number(2.40), short_number(12500.), short_number(3.0)), ("2.4".into(), "12.5k".into(), "3".into()));
    }
}
