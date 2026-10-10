use crate::text;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Layer {
    /// How you like things done.
    Preferences,
    /// Facts about you.
    Facts,
    /// Projects and decisions.
    Projects,
}

impl Layer {
    pub const ALL: [Layer; 3] = [Layer::Preferences, Layer::Facts, Layer::Projects];

    pub fn as_str(self) -> &'static str {
        match self {
            Layer::Preferences => "preferences",
            Layer::Facts => "facts",
            Layer::Projects => "projects",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.as_str() == s)
    }
}

/// Where a page may be sent. `Sensitive` pages stay on this machine: they go only to local models.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Privacy {
    Normal,
    Sensitive,
}

impl Privacy {
    pub fn as_str(self) -> &'static str {
        match self {
            Privacy::Normal => "normal",
            Privacy::Sensitive => "sensitive",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "normal" => Some(Privacy::Normal),
            "sensitive" => Some(Privacy::Sensitive),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    pub id: String,
    pub layer: Layer,
    pub slug: String,
    pub title: String,
    pub body: String,
    /// One line, written once when the page is accepted. Feeds the "about you" block.
    pub gist: String,
    pub privacy: Privacy,
    pub privacy_reason: String,
    pub pinned: bool,
    /// The project folder this page belongs to; `None` means it applies everywhere.
    pub project: Option<String>,
    pub source: String,
    pub version: i64,
    pub archived_at: Option<i64>,
    pub created: i64,
    pub updated: i64,
}

impl Page {
    pub fn file_text(&self) -> String {
        let mut h: Vec<(&str, String)> = vec![
            ("id", self.id.clone()),
            ("layer", self.layer.as_str().into()),
            ("privacy", self.privacy.as_str().into()),
            ("pinned", self.pinned.to_string()),
            ("source", self.source.clone()),
            ("version", self.version.to_string()),
            ("created", self.created.to_string()),
            ("updated", self.updated.to_string()),
        ];
        if let Some(p) = &self.project {
            h.push(("project", p.clone()));
        }
        if !self.privacy_reason.is_empty() {
            h.push(("privacy_reason", self.privacy_reason.clone()));
        }
        h.push(("gist", self.gist.clone()));
        text::render_file(&h, &self.title, &self.body)
    }
}

#[derive(Clone, Debug)]
pub struct NewPage {
    pub layer: Layer,
    pub title: String,
    pub body: String,
    pub gist: String,
    pub privacy: Privacy,
    pub privacy_reason: String,
    pub pinned: bool,
    pub project: Option<String>,
    pub source: String,
}

impl NewPage {
    pub fn new(layer: Layer, title: impl Into<String>, body: impl Into<String>) -> Self {
        Self { layer, title: title.into(), body: body.into(), gist: String::new(), privacy: Privacy::Normal, privacy_reason: String::new(), pinned: false, project: None, source: "user".into() }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Edit {
    pub title: Option<String>,
    pub body: Option<String>,
    pub gist: Option<String>,
}

/// Who made a change; recorded in the history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum By {
    User,
    Assistant,
    HandEdit,
    Maintenance,
    Import,
}

impl By {
    pub fn as_str(self) -> &'static str {
        match self {
            By::User => "user",
            By::Assistant => "assistant",
            By::HandEdit => "hand_edit",
            By::Maintenance => "maintenance",
            By::Import => "import",
        }
    }
}
