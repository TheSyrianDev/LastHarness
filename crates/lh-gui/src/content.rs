//! Something the assistant (or the user) opened to look at properly, and which viewer suits it.

/// A piece of content for the side panel or a window of its own.
#[derive(Clone, Debug, PartialEq)]
pub struct Content {
    pub title: String,
    pub kind: ContentKind,
    pub body: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentKind {
    Markdown,
    Code,
    Text,
    /// `body` is a file path.
    Image,
}

impl Content {
    /// What the assistant's `show` tool sent: the kind it named, the text, or for an image a path.
    pub fn from_show(title: &str, kind: &str, content: &str, path: Option<&str>) -> Content {
        match (kind, path) {
            ("image", Some(p)) => Content { title: title.into(), kind: ContentKind::Image, body: p.into() },
            ("code", _) => Content { title: title.into(), kind: ContentKind::Code, body: content.into() },
            ("text", _) => Content { title: title.into(), kind: ContentKind::Text, body: content.into() },
            ("markdown", _) => Content { title: title.into(), kind: ContentKind::Markdown, body: content.into() },
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
        Content { title: title.to_string(), kind, body }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

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
}
