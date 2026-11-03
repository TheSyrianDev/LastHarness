use lh_store::now;
use serde_json::json;

use crate::{Error, Learn, Result};

/// The style profile is short on purpose: it is sent with every request.
pub const STYLE_MAX_CHARS: usize = 1500;

#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub text: String,
    pub version: i64,
    pub updated: i64,
}

impl Learn {
    pub fn style(&self) -> Result<Option<Style>> {
        use lh_store::rusqlite::OptionalExtension;
        Ok(self.store.with(|c| c.query_row("select text, version, updated from style_profile where id = 'style'", [], |r| Ok(Style { text: r.get(0)?, version: r.get(1)?, updated: r.get(2)? })).optional())?)
    }

    /// Replace the style profile. Returns the new version.
    pub fn set_style(&self, text: &str, by: &str) -> Result<Style> {
        let t = text.trim();
        if t.chars().count() > STYLE_MAX_CHARS {
            return Err(Error::Invalid(format!("the style profile is a short note (at most {STYLE_MAX_CHARS} characters), because it is sent with every request")));
        }
        let old = self.style()?;
        if old.as_ref().is_some_and(|o| o.text == t) {
            return Ok(old.unwrap());
        }
        let version = old.as_ref().map_or(1, |o| o.version + 1);
        let s = Style { text: t.to_string(), version, updated: now() };
        self.store.with(|c| c.execute("insert into style_profile(id, text, version, updated) values ('style', ?1, ?2, ?3) on conflict(id) do update set text = excluded.text, version = excluded.version, updated = excluded.updated", (&s.text, s.version, s.updated)).map(|_| ()))?;
        self.note(("style", "style"), version, if old.is_some() { "edit" } else { "create" }, by, &json!({"text": s.text}), "")?;
        Ok(s)
    }

    pub fn rollback_style(&self, version: i64, by: &str) -> Result<Style> {
        let snap = self.version_content("style", "style", version)?;
        let text = snap["text"].as_str().unwrap_or("").to_string();
        let cur = self.style()?.ok_or_else(|| Error::NotFound("style profile".into()))?;
        let s = Style { text, version: cur.version + 1, updated: now() };
        self.store.with(|c| c.execute("update style_profile set text = ?1, version = ?2, updated = ?3 where id = 'style'", (&s.text, s.version, s.updated)).map(|_| ()))?;
        self.note(("style", "style"), s.version, "rollback", by, &json!({"text": s.text}), &format!("back to version {version}"))?;
        Ok(s)
    }
}
