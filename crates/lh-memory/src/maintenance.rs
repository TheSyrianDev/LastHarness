//! The maintenance pass: look for duplicates, broken links, stale pages and pages that may
//! contradict each other. It changes nothing. Duplicates become suggestions to archive the older
//! page; the rest are listed for the user to look at. Contradictions need a model to judge, so this
//! module only finds the candidate pairs and reads the model's answer.

use std::collections::HashSet;

use lh_store::now;

use crate::{text, Kind, Layer, Memory, NewProposal, Page, Result, State};

#[derive(Clone, Debug, PartialEq)]
pub enum FindingKind {
    /// Nearly the same words. A suggestion to archive the older page was made.
    Duplicate,
    /// A `[[link]]` that points at no page.
    BrokenLink,
    /// Not touched for a long time, so the fact may no longer hold.
    Stale,
    /// A page nobody links to and that links to nothing, in a layer that otherwise uses links.
    Unlinked,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub kind: FindingKind,
    pub pages: Vec<String>,
    pub detail: String,
}

/// A fact page older than this (in days) is listed as possibly stale.
pub const STALE_DAYS: i64 = 365;
const DUPLICATE_AT: f64 = 0.8;

fn word_set(p: &Page) -> HashSet<String> {
    text::tokens(&format!("{} {}", p.title, p.body)).into_iter().collect()
}

pub fn similarity(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    a.intersection(b).count() as f64 / a.union(b).count() as f64
}

impl Memory {
    pub fn maintenance(&self) -> Result<Vec<Finding>> {
        let pages = self.list(None, State::Live)?;
        let mut out = vec![];
        let sets: Vec<HashSet<String>> = pages.iter().map(word_set).collect();
        let mut proposed: HashSet<String> = HashSet::new();
        for i in 0..pages.len() {
            for j in i + 1..pages.len() {
                if pages[i].layer != pages[j].layer || pages[i].project != pages[j].project {
                    continue;
                }
                let s = similarity(&sets[i], &sets[j]);
                if s >= DUPLICATE_AT {
                    let (older, newer) = if pages[i].updated <= pages[j].updated { (&pages[i], &pages[j]) } else { (&pages[j], &pages[i]) };
                    if !older.pinned && proposed.insert(older.id.clone()) {
                        self.add_proposal(NewProposal {
                            kind: Kind::Archive,
                            layer: older.layer,
                            title: older.title.clone(),
                            body: older.body.clone(),
                            gist: older.gist.clone(),
                            target_page: Some(older.id.clone()),
                            privacy: older.privacy,
                            privacy_reason: older.privacy_reason.clone(),
                            evidence: format!("nearly the same as \"{}\"", newer.title),
                            conv: None,
                            project: None,
                        })?;
                    }
                    out.push(Finding { kind: FindingKind::Duplicate, pages: vec![older.id.clone(), newer.id.clone()], detail: format!("{:.0}% the same words", s * 100.0) });
                }
            }
        }
        let slugs: HashSet<(Layer, &str)> = pages.iter().map(|p| (p.layer, p.slug.as_str())).collect();
        let any_slug = |s: &str| slugs.iter().any(|(_, x)| *x == s);
        let mut linked: HashSet<String> = HashSet::new();
        for p in &pages {
            for l in text::links(&p.body) {
                let target = crate::text::slugify(&l);
                if any_slug(&l) || any_slug(&target) {
                    linked.insert(l.clone());
                    linked.insert(target);
                } else {
                    out.push(Finding { kind: FindingKind::BrokenLink, pages: vec![p.id.clone()], detail: format!("[[{l}]] points at no page") });
                }
            }
        }
        let uses_links = pages.iter().any(|p| !text::links(&p.body).is_empty());
        let cutoff = now() - STALE_DAYS * 86_400;
        for p in &pages {
            if p.layer != Layer::Preferences && !p.pinned && p.updated < cutoff {
                out.push(Finding { kind: FindingKind::Stale, pages: vec![p.id.clone()], detail: format!("not changed for over {STALE_DAYS} days") });
            }
            if uses_links && !p.pinned && text::links(&p.body).is_empty() && !linked.contains(&p.slug) {
                out.push(Finding { kind: FindingKind::Unlinked, pages: vec![p.id.clone()], detail: "no page links to it and it links to none".into() });
            }
        }
        Ok(out)
    }

    /// Pairs of live pages in the same layer that share some words but are not duplicates: the ones
    /// worth asking a model whether they disagree. Best candidates first, at most `limit`.
    pub fn contradiction_candidates(&self, limit: usize) -> Result<Vec<(Page, Page)>> {
        let pages = self.list(None, State::Live)?;
        let sets: Vec<HashSet<String>> = pages.iter().map(word_set).collect();
        let mut pairs: Vec<(f64, usize, usize)> = vec![];
        for i in 0..pages.len() {
            for j in i + 1..pages.len() {
                if pages[i].layer == pages[j].layer && pages[i].project == pages[j].project {
                    let s = similarity(&sets[i], &sets[j]);
                    if (0.2..DUPLICATE_AT).contains(&s) {
                        pairs.push((s, i, j));
                    }
                }
            }
        }
        pairs.sort_by(|a, b| b.0.total_cmp(&a.0));
        Ok(pairs.into_iter().take(limit).map(|(_, i, j)| (pages[i].clone(), pages[j].clone())).collect())
    }
}

pub fn contradiction_prompt(a: &Page, b: &Page) -> String {
    format!(
        "Two notes about the same person. Do they contradict each other (cannot both be true now)? Answer with one word, YES or NO, then a colon and a short reason.\n\nNote 1 ({}): {}\n\nNote 2 ({}): {}",
        a.title, a.body, b.title, b.body
    )
}

/// `Some(reason)` when the model says the notes contradict.
pub fn parse_contradiction(reply: &str) -> Option<String> {
    let first = reply.trim().lines().next()?.trim().trim_matches(|c: char| !c.is_alphanumeric());
    first.to_uppercase().starts_with("YES").then(|| first.split_once(':').map(|(_, r)| r.trim().to_string()).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contradiction_answers() {
        assert_eq!(parse_contradiction("YES: one says Cairo, the other Berlin"), Some("one says Cairo, the other Berlin".into()));
        assert_eq!(parse_contradiction("**Yes**"), Some(String::new()));
        assert_eq!(parse_contradiction("NO: compatible"), None);
        assert_eq!(parse_contradiction("maybe"), None);
        assert_eq!(parse_contradiction(""), None);
    }
}
