//! What memory goes into a request. One rule decides what may leave: `visible`. Every path that
//! builds text for a model goes through it, before ranking, so a page that may not be sent cannot be
//! picked, quoted, counted into the summary or leaked by a tie.

use std::collections::{HashMap, HashSet};

use crate::{Layer, Memory, Page, Privacy, Result, State, Trust};

#[derive(Clone, Debug)]
pub struct RecallConfig {
    /// Most notes added to one request.
    pub max_items: usize,
    /// Most characters of each note.
    pub excerpt_chars: usize,
    /// Most characters of all notes together. Bounds what one request can carry.
    pub budget_chars: usize,
    /// Size cap of the always-on "about you" block.
    pub about_chars: usize,
    pub semantic_weight: f64,
    pub keyword_weight: f64,
}

impl Default for RecallConfig {
    fn default() -> Self {
        Self { max_items: 6, excerpt_chars: 600, budget_chars: 3000, about_chars: 1200, semantic_weight: 1.0, keyword_weight: 0.35 }
    }
}

/// Where the memory is going.
#[derive(Clone, Debug)]
pub struct Destination {
    pub trust: Trust,
    /// The project folder of the session, so project pages of other projects stay out.
    pub project: Option<String>,
}

/// May this page be put into a request to `dest`?
pub fn visible(p: &Page, dest: &Destination) -> bool {
    p.archived_at.is_none() && p.privacy.allowed_at(dest.trust) && applies(p, dest)
}

fn applies(p: &Page, dest: &Destination) -> bool {
    p.project.is_none() || p.project == dest.project
}

/// Which version of which page went into a request, for the "what was sent" record.
#[derive(Clone, Debug, PartialEq)]
pub struct PageRef {
    pub id: String,
    pub version: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub page: PageRef,
    pub layer: Layer,
    pub title: String,
    pub text: String,
    /// When the page last changed (seconds since 1970), so the model can tell which of two notes is newer.
    pub updated: i64,
}

#[derive(Clone, Debug, Default)]
pub struct Recalled {
    pub notes: Vec<Note>,
    /// Matching pages kept back because the destination is not trusted with them. Only a count.
    pub withheld: usize,
}

impl Recalled {
    pub fn refs(&self) -> Vec<PageRef> {
        self.notes.iter().map(|n| n.page.clone()).collect()
    }

    /// The text added to the user's message. Empty when there is nothing to add.
    pub fn render(&self) -> String {
        if self.notes.is_empty() {
            return String::new();
        }
        let mut s = String::from("[Notes from the user's memory that may help with this message. Use them only if they are relevant. If two disagree, the newer one holds.]\n");
        for n in &self.notes {
            s.push_str(&format!("- ({}, {}) {}: {}\n", n.layer.as_str(), day(n.updated), n.title, n.text.replace('\n', " ")));
        }
        s
    }
}

/// A date as YYYY-MM-DD, from seconds since 1970 (UTC).
fn day(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    // civil-from-days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn excerpt(text: &str, max: usize) -> String {
    let one = text.trim();
    if one.chars().count() <= max {
        return one.to_string();
    }
    let cut: String = one.chars().take(max).collect();
    format!("{}…", cut.trim_end())
}

fn first_sentence(body: &str, max: usize) -> String {
    let flat = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let end = flat.find(['.', '!', '?', '؟']).map_or(flat.len(), |i| i + 1);
    excerpt(&flat[..end], max)
}

impl Memory {
    /// The short always-included block. Built from each page's one-line gist, pinned pages first,
    /// then preferences, facts and projects, newest first, up to the size cap. Never written by a model.
    pub fn about_you(&self, dest: &Destination, cfg: &RecallConfig) -> Result<(String, Vec<PageRef>)> {
        let mut pages: Vec<Page> = self.list(None, State::Live)?.into_iter().filter(|p| visible(p, dest)).collect();
        let rank = |l: Layer| match l {
            Layer::Preferences => 0,
            Layer::Facts => 1,
            Layer::Projects => 2,
        };
        pages.sort_by_key(|p| (!p.pinned, rank(p.layer), std::cmp::Reverse(p.updated)));
        let head = "What you know about the user (from their memory):\n";
        let (mut text, mut refs) = (String::new(), vec![]);
        for p in pages {
            let line = if p.gist.is_empty() { first_sentence(&p.body, 140) } else { p.gist.clone() };
            let row = format!("- {line}\n");
            if head.chars().count() + text.chars().count() + row.chars().count() > cfg.about_chars {
                continue; // a shorter line further down may still fit
            }
            text.push_str(&row);
            refs.push(PageRef { id: p.id, version: p.version });
        }
        if text.is_empty() {
            return Ok((String::new(), refs));
        }
        Ok((format!("{head}{text}"), refs))
    }

    /// Notes relevant to `query`. `semantic` holds (page id, similarity) from the embedding search,
    /// best first; it may be empty, and then only the keyword search is used. `already` lists page
    /// versions the conversation already carries, which are not added twice.
    pub fn recall(&self, query: &str, dest: &Destination, cfg: &RecallConfig, semantic: &[(String, f64)], already: &HashSet<(String, i64)>) -> Result<Recalled> {
        let mut score: HashMap<String, f64> = HashMap::new();
        let mut pages: HashMap<String, Page> = HashMap::new();
        for (rank, hit) in self.search(query, State::Live, 30)?.into_iter().enumerate() {
            *score.entry(hit.page.id.clone()).or_default() += cfg.keyword_weight / (60.0 + rank as f64);
            pages.insert(hit.page.id.clone(), hit.page);
        }
        for (rank, (id, _)) in semantic.iter().enumerate() {
            *score.entry(id.clone()).or_default() += cfg.semantic_weight / (60.0 + rank as f64);
            if !pages.contains_key(id) {
                if let Ok(p) = self.get(id) {
                    pages.insert(id.clone(), p);
                }
            }
        }
        let mut ranked: Vec<(&String, &f64)> = score.iter().collect();
        ranked.sort_by(|a, b| b.1.total_cmp(a.1).then(a.0.cmp(b.0)));
        let (mut out, mut used, mut withheld) = (Recalled::default(), 0usize, 0usize);
        for (id, _) in ranked {
            let Some(p) = pages.get(id) else { continue };
            if p.archived_at.is_some() || !applies(p, dest) {
                continue;
            }
            if p.privacy == Privacy::Sensitive && !p.privacy.allowed_at(dest.trust) {
                withheld += 1;
                continue;
            }
            if already.contains(&(p.id.clone(), p.version)) || out.notes.len() >= cfg.max_items {
                continue;
            }
            let text = excerpt(&p.body, cfg.excerpt_chars);
            if used + text.chars().count() > cfg.budget_chars {
                continue;
            }
            used += text.chars().count();
            out.notes.push(Note { page: PageRef { id: p.id.clone(), version: p.version }, layer: p.layer, title: p.title.clone(), text, updated: p.updated });
        }
        out.withheld = withheld;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn days_are_written_as_dates() {
        assert_eq!(super::day(0), "1970-01-01");
        assert_eq!(super::day(951_782_400), "2000-02-29");
        assert_eq!(super::day(1_792_022_400), "2026-10-15");
    }
}
