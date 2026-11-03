//! Text helpers: search normalisation (English and Arabic), file names, and the page file format.

/// Words that carry no meaning for search. Small on purpose.
const STOP: &[&str] = &[
    "the", "a", "an", "of", "to", "in", "on", "for", "and", "or", "is", "are", "was", "were", "be", "do", "does", "did", "what", "which", "who", "when", "where", "how", "why",
    "that", "this", "it", "its", "with", "as", "at", "by", "from", "my", "me", "i", "you", "your", "we", "our", "they", "their", "user", "any", "about", "has", "have", "had",
    "there", "than", "then", "so", "if", "not", "no", "can", "will", "would", "should",
    "في", "من", "على", "الى", "عن", "مع", "هل", "ما", "ماذا", "كيف", "متى", "اين", "لماذا", "هذا", "هذه", "ذلك", "التي", "الذي", "كان", "كانت", "يكون", "هو", "هي", "نحن", "انا", "انت", "او", "ان", "لا",
];

fn arabic_fold(c: char) -> Option<char> {
    match c {
        '\u{064b}'..='\u{065f}' | '\u{0670}' | '\u{0640}' => None, // diacritics, tatweel
        'أ' | 'إ' | 'آ' => Some('ا'),
        'ى' => Some('ي'),
        'ة' => Some('ه'),
        '٠'..='٩' => char::from_digit(c as u32 - '٠' as u32, 10),
        c => Some(c),
    }
}

fn strip_arabic_prefix(w: &str) -> &str {
    if !w.chars().next().is_some_and(|c| ('\u{0600}'..='\u{06ff}').contains(&c)) {
        return w;
    }
    for p in ["وال", "بال", "كال", "فال", "لل", "ال", "و"] {
        if let Some(rest) = w.strip_prefix(p) {
            if rest.chars().count() >= 3 {
                return rest;
            }
        }
    }
    w
}

/// Light English stemming so "flights" finds "flight" and "lives" finds "live". Applied to the
/// index and to queries alike, so over- or under-stripping cannot make them disagree.
fn stem_en(w: &str) -> String {
    if !w.is_ascii() || w.len() <= 3 || w.chars().any(|c| c.is_ascii_digit()) {
        return w.to_string();
    }
    let strip = |suffix: &str, min_left: usize| w.strip_suffix(suffix).filter(|r| r.len() >= min_left).map(str::to_string);
    if let Some(r) = strip("ies", 2) {
        return r + "y";
    }
    for (suffix, left) in [("ing", 4), ("ed", 4)] {
        if let Some(r) = strip(suffix, left) {
            return r;
        }
    }
    // boxes, wishes, buses: drop "es" only after a hissing sound; "lives" just loses its "s"
    if let Some(r) = strip("es", 3) {
        if r.ends_with(['s', 'x', 'z']) || r.ends_with("ch") || r.ends_with("sh") {
            return r;
        }
    }
    if !w.ends_with("ss") {
        if let Some(r) = strip("s", 3) {
            return r;
        }
    }
    w.to_string()
}

/// Words for the keyword index and for queries: lower case, Arabic folded and de-prefixed,
/// stop words dropped. The same function runs on both sides, so they always agree.
pub fn tokens(text: &str) -> Vec<String> {
    let folded: String = text.chars().flat_map(|c| c.to_lowercase()).filter_map(arabic_fold).collect();
    folded
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(strip_arabic_prefix)
        .filter(|w| !STOP.contains(w))
        .map(stem_en)
        .collect()
}

pub fn search_text(title: &str, body: &str) -> String {
    format!("{} {}", tokens(title).join(" "), tokens(body).join(" "))
}

/// File name stem from a title: letters and digits of any script, lower case, joined by `-`.
pub fn slugify(title: &str) -> String {
    let s: String = title.chars().flat_map(|c| c.to_lowercase()).map(|c| if c.is_alphanumeric() { c } else { '-' }).collect();
    let s = s.split('-').filter(|w| !w.is_empty()).collect::<Vec<_>>().join("-");
    let s: String = s.chars().take(60).collect::<String>().trim_end_matches('-').to_string();
    if s.is_empty() || s == "index" { "page".into() } else { s }
}

/// A page file: a small header between `---` lines, then `# Title`, then the text.
#[derive(Debug, Default, PartialEq)]
pub struct FileParts {
    pub header: Vec<(String, String)>,
    pub title: String,
    pub body: String,
}

impl FileParts {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.header.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
}

pub fn render_file(header: &[(&str, String)], title: &str, body: &str) -> String {
    let mut s = String::from("---\n");
    for (k, v) in header {
        s.push_str(&format!("{k}: {}\n", v.replace(['\n', '\r'], " ")));
    }
    s.push_str(&format!("---\n# {title}\n\n{}\n", body.trim_end()));
    s
}

/// Reads a page file. Files without a header are accepted (a note someone just dropped in): the
/// first `# ` line is the title, or the file name is used by the caller.
pub fn parse_file(text: &str) -> FileParts {
    let text = text.replace("\r\n", "\n");
    let mut parts = FileParts::default();
    let mut rest = text.as_str();
    if let Some(after) = rest.strip_prefix("---\n") {
        if let Some(end) = after.find("\n---\n").map(|i| (i, 5)).or_else(|| after.strip_suffix("\n---").map(|a| (a.len(), 4))) {
            for line in after[..end.0].lines() {
                if let Some((k, v)) = line.split_once(':') {
                    parts.header.push((k.trim().to_string(), v.trim().to_string()));
                }
            }
            rest = after.get(end.0 + end.1..).unwrap_or("");
        }
    }
    let rest = rest.trim_start_matches('\n');
    match rest.strip_prefix("# ") {
        Some(r) => {
            let (title, body) = r.split_once('\n').unwrap_or((r, ""));
            parts.title = title.trim().to_string();
            parts.body = body.trim().to_string();
        }
        None => parts.body = rest.trim().to_string(),
    }
    parts
}

/// `[[slug]]` links in a body.
pub fn links(body: &str) -> Vec<String> {
    let mut out = vec![];
    let mut rest = body;
    while let Some(i) = rest.find("[[") {
        let after = &rest[i + 2..];
        match after.find("]]") {
            Some(j) => {
                out.push(after[..j].trim().to_string());
                rest = &after[j + 2..];
            }
            None => break,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_and_arabic_tokens_line_up_across_forms() {
        assert_eq!(tokens("What is the user's allergy?"), ["s", "allergy"]);
        assert_eq!(tokens("الإيجار"), tokens("والإيجار"));
        assert_eq!(tokens("إيجار"), tokens("ايجار"));
        assert_eq!(tokens("مَدْرَسَة"), tokens("مدرسه"));
        assert_eq!(tokens("راتب ٤٢٠٠"), ["راتب", "4200"]);
        assert!(tokens("هل هذا في").is_empty());
    }

    #[test]
    fn plurals_and_endings_meet_in_the_middle() {
        assert_eq!(tokens("flights"), tokens("flight"));
        assert_eq!(tokens("lives"), tokens("live"));
        assert_eq!(tokens("boxes wishes"), ["box", "wish"]);
        assert_eq!(tokens("cities"), tokens("city"));
        assert_eq!(tokens("walking walked walks"), ["walk", "walk", "walk"]);
        assert_eq!(tokens("business class"), ["business", "class"]);
        assert_eq!(tokens("RTX 3050 GPUs"), ["rtx", "3050", "gpu"]);
    }

    #[test]
    fn slugs() {
        assert_eq!(slugify("Salary & rent (2026)!"), "salary-rent-2026");
        assert_eq!(slugify("الراتب والإيجار"), "الراتب-والإيجار");
        assert_eq!(slugify("../../etc"), "etc");
        assert_eq!(slugify("index"), "page");
        assert_eq!(slugify("???"), "page");
    }

    #[test]
    fn page_files_roundtrip_and_tolerate_plain_notes() {
        let t = render_file(&[("id", "p_1".into()), ("privacy", "normal".into())], "Allergy", "Penicillin.\nSee [[doctor]] and [[ pharmacy ]].");
        let p = parse_file(&t);
        assert_eq!((p.get("id"), p.get("privacy"), p.title.as_str()), (Some("p_1"), Some("normal"), "Allergy"));
        assert_eq!(p.body, "Penicillin.\nSee [[doctor]] and [[ pharmacy ]].");
        assert_eq!(links(&p.body), ["doctor", "pharmacy"]);
        let plain = parse_file("# Just a note\n\nhello");
        assert_eq!((plain.header.len(), plain.title.as_str(), plain.body.as_str()), (0, "Just a note", "hello"));
        assert_eq!(parse_file("no title at all").body, "no title at all");
        assert_eq!(parse_file("---\r\nid: p_9\r\n---\r\n# T\r\n\r\nb").get("id"), Some("p_9"));
    }
}
