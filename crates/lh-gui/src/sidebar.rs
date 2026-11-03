//! How the sidebar sorts chats into sections, without drawing anything.
//!
//! From the top: Needs you (a question or a memory waits), Pinned, Active (touched in the last few days or working
//! now), one group per project folder, and Earlier (quiet chats that settled). A snoozed chat is hidden until its
//! time, unless it needs you. New activity lifts a settled chat back to Active on its own, since Active is decided
//! by the time of the last message.

use std::collections::BTreeMap;

pub const DAY: i64 = 86_400;
/// Days without a message before a chat settles into Earlier, when Settings says nothing.
pub const SETTLE_DAYS: u32 = 3;
/// Rows Earlier shows before "Show more".
pub const EARLIER_SHOWN: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Section {
    NeedsYou,
    Pinned,
    Active,
    /// A project folder; the label is its last path part.
    Project { folder: String, label: String },
    Earlier,
}

impl Section {
    pub fn title(&self, rtl: bool) -> String {
        match (self, rtl) {
            (Section::NeedsYou, false) => "Needs you".into(),
            (Section::NeedsYou, true) => "بانتظارك".into(),
            (Section::Pinned, false) => "Pinned".into(),
            (Section::Pinned, true) => "المثبتة".into(),
            (Section::Active, false) => "Active".into(),
            (Section::Active, true) => "النشطة".into(),
            (Section::Project { label, .. }, _) => label.clone(),
            (Section::Earlier, false) => "Earlier".into(),
            (Section::Earlier, true) => "سابقًا".into(),
        }
    }
}

/// What the sidebar needs to know about one chat.
#[derive(Clone, Debug, Default)]
pub struct Facts {
    pub key: u64,
    pub session: Option<String>,
    /// Unix seconds of the last message; 0 when unknown.
    pub last: i64,
    pub project: String,
    /// A permission question or something else waits for the person.
    pub needs_you: bool,
    /// A turn is running.
    pub busy: bool,
}

/// The sections in order, each with its chat keys, newest first. Empty sections are left out.
pub fn group(chats: &[Facts], pinned: &[String], snoozed: &BTreeMap<String, i64>, now: i64, settle_days: u32) -> Vec<(Section, Vec<u64>)> {
    let mut sorted: Vec<&Facts> = chats.iter().collect();
    sorted.sort_by(|a, b| b.last.cmp(&a.last));
    let settle = now - settle_days.max(1) as i64 * DAY;
    let (mut needs, mut pins, mut active, mut earlier) = (vec![], vec![], vec![], vec![]);
    let mut projects: Vec<(String, Vec<u64>)> = vec![];
    for c in sorted {
        let sid = c.session.as_deref().unwrap_or("");
        if c.needs_you {
            needs.push(c.key);
            continue;
        }
        if snoozed.get(sid).is_some_and(|until| *until > now) {
            continue;
        }
        if !sid.is_empty() && pinned.iter().any(|p| p == sid) {
            pins.push(c.key);
        } else if !c.project.is_empty() {
            // "/a/b/" and "/a/b" are one folder
            let folder = c.project.trim_end_matches('/').to_string();
            match projects.iter_mut().find(|(f, _)| *f == folder) {
                Some((_, keys)) => keys.push(c.key),
                None => projects.push((folder, vec![c.key])),
            }
        } else if c.busy || c.last >= settle || c.last == 0 {
            active.push(c.key);
        } else {
            earlier.push(c.key);
        }
    }
    let mut out = vec![(Section::NeedsYou, needs), (Section::Pinned, pins), (Section::Active, active)];
    for (folder, keys) in projects {
        let label = project_label(&folder);
        out.push((Section::Project { folder, label }, keys));
    }
    out.push((Section::Earlier, earlier));
    out.retain(|(_, k)| !k.is_empty());
    out
}

/// The last part of a folder path: "/home/me/thesis/" gives "thesis".
pub fn project_label(folder: &str) -> String {
    folder.trim_end_matches('/').rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or(folder).to_string()
}

/// How long ago, short: "now", "5m", "3h", "2d", "6w".
pub fn age(last: i64, now: i64) -> String {
    if last <= 0 {
        return String::new();
    }
    let s = (now - last).max(0);
    match s {
        0..=59 => "now".into(),
        60..=3599 => format!("{}m", s / 60),
        3600..=86_399 => format!("{}h", s / 3600),
        86_400..=1_209_599 => format!("{}d", s / DAY),
        _ => format!("{}w", s / (7 * DAY)),
    }
}

/// The snooze choices of the chat menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Snooze {
    Hour,
    Tomorrow,
    Monday,
}

impl Snooze {
    pub const ALL: [Snooze; 3] = [Snooze::Hour, Snooze::Tomorrow, Snooze::Monday];

    pub fn label(self, rtl: bool) -> &'static str {
        match (self, rtl) {
            (Snooze::Hour, false) => "Snooze 1 hour",
            (Snooze::Hour, true) => "أجّل ساعة",
            (Snooze::Tomorrow, false) => "Snooze until tomorrow 9:00",
            (Snooze::Tomorrow, true) => "أجّل حتى الغد 9:00",
            (Snooze::Monday, false) => "Snooze until Monday 9:00",
            (Snooze::Monday, true) => "أجّل حتى الاثنين 9:00",
        }
    }

    /// When the chat comes back, in Unix seconds. `offset` is the local time zone's offset from UTC in seconds.
    pub fn until(self, now: i64, offset: i64) -> i64 {
        let local = now + offset;
        let midnight = local - local.rem_euclid(DAY);
        let nine = |days: i64| midnight + days * DAY + 9 * 3600 - offset;
        match self {
            Snooze::Hour => now + 3600,
            Snooze::Tomorrow => nine(1),
            Snooze::Monday => {
                // 1970-01-01 was a Thursday: day 0 is Thursday, so Monday is (days + 3) % 7 == 0
                let weekday = (local.div_euclid(DAY) + 3).rem_euclid(7);
                let ahead = 7 - weekday;
                nine(ahead)
            }
        }
    }
}

/// The local offset from UTC in seconds, read once from `date +%z`; 0 when that fails.
pub fn local_offset() -> i64 {
    let out = std::process::Command::new("date").arg("+%z").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    parse_offset(&out)
}

pub fn parse_offset(s: &str) -> i64 {
    let (sign, rest) = match s.as_bytes().first() {
        Some(b'+') => (1, &s[1..]),
        Some(b'-') => (-1, &s[1..]),
        _ => return 0,
    };
    if rest.len() != 4 {
        return 0;
    }
    let h: i64 = rest[..2].parse().unwrap_or(0);
    let m: i64 = rest[2..].parse().unwrap_or(0);
    sign * (h * 3600 + m * 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000;

    fn f(key: u64, sid: &str, days_ago: i64) -> Facts {
        Facts { key, session: Some(sid.into()), last: NOW - days_ago * DAY, ..Default::default() }
    }

    fn keys(g: &[(Section, Vec<u64>)], s: &Section) -> Vec<u64> {
        g.iter().find(|(x, _)| x == s).map(|(_, k)| k.clone()).unwrap_or_default()
    }

    #[test]
    fn quiet_chats_settle_after_the_set_days() {
        let chats = [f(1, "a", 0), f(2, "b", 2), f(3, "c", 4)];
        let g = group(&chats, &[], &BTreeMap::new(), NOW, 3);
        assert_eq!(keys(&g, &Section::Active), vec![1, 2]);
        assert_eq!(keys(&g, &Section::Earlier), vec![3]);
        let g = group(&chats, &[], &BTreeMap::new(), NOW, 5);
        assert!(keys(&g, &Section::Earlier).is_empty(), "a longer setting keeps it active");
    }

    #[test]
    fn a_running_chat_stays_active_however_old() {
        let mut c = f(1, "a", 30);
        c.busy = true;
        let g = group(&[c], &[], &BTreeMap::new(), NOW, 3);
        assert_eq!(keys(&g, &Section::Active), vec![1]);
    }

    #[test]
    fn needs_you_beats_pin_and_snooze() {
        let mut c = f(1, "a", 10);
        c.needs_you = true;
        let snoozed = BTreeMap::from([("a".to_string(), NOW + 100)]);
        let g = group(&[c], &["a".into()], &snoozed, NOW, 3);
        assert_eq!(g, vec![(Section::NeedsYou, vec![1])]);
    }

    #[test]
    fn pinned_and_snoozed() {
        let chats = [f(1, "a", 9), f(2, "b", 0), f(3, "c", 0)];
        let snoozed = BTreeMap::from([("b".to_string(), NOW + 60), ("c".to_string(), NOW - 1)]);
        let g = group(&chats, &["a".into()], &snoozed, NOW, 3);
        assert_eq!(keys(&g, &Section::Pinned), vec![1]);
        assert_eq!(keys(&g, &Section::Active), vec![3], "b is hidden; c's snooze is over");
    }

    #[test]
    fn projects_group_by_folder_newest_first() {
        let mut a = f(1, "a", 1);
        a.project = "/home/me/thesis/".into();
        let mut b = f(2, "b", 0);
        b.project = "/home/me/thesis".into();
        let mut c = f(3, "c", 0);
        c.project = "/srv/backup".into();
        let g = group(&[a, b, c], &[], &BTreeMap::new(), NOW, 3);
        let labels: Vec<String> = g.iter().map(|(s, _)| s.title(false)).collect();
        assert_eq!(labels, vec!["thesis", "backup"], "a trailing slash is the same folder");
        assert_eq!(keys(&g, &g[0].0), vec![2, 1]);
    }

    #[test]
    fn empty_sections_are_left_out_and_unknown_time_is_active() {
        let c = Facts { key: 4, ..Default::default() };
        assert_eq!(group(&[c], &[], &BTreeMap::new(), NOW, 3), vec![(Section::Active, vec![4])]);
        assert!(group(&[], &[], &BTreeMap::new(), NOW, 3).is_empty());
    }

    #[test]
    fn ages() {
        assert_eq!(age(NOW - 5, NOW), "now");
        assert_eq!(age(NOW - 300, NOW), "5m");
        assert_eq!(age(NOW - 7200, NOW), "2h");
        assert_eq!(age(NOW - 3 * DAY, NOW), "3d");
        assert_eq!(age(NOW - 21 * DAY, NOW), "3w");
        assert_eq!(age(0, NOW), "");
    }

    #[test]
    fn snooze_times() {
        // 2027-01-14 is a Thursday; 15:30 UTC
        let thu = 1_799_940_600;
        assert_eq!(Snooze::Hour.until(thu, 0), thu + 3600);
        let tomorrow = Snooze::Tomorrow.until(thu, 0);
        assert_eq!(tomorrow.rem_euclid(DAY), 9 * 3600);
        assert!(tomorrow > thu && tomorrow - thu < DAY);
        let mon = Snooze::Monday.until(thu, 0);
        assert_eq!((mon.div_euclid(DAY) + 3).rem_euclid(7), 0, "lands on a Monday");
        assert_eq!(mon - thu, 3 * DAY + 17 * 3600 + 30 * 60);
        // in UTC+3, 9:00 local is 6:00 UTC
        assert_eq!(Snooze::Tomorrow.until(thu, 3 * 3600).rem_euclid(DAY), 6 * 3600);
    }

    #[test]
    fn offsets() {
        assert_eq!(parse_offset("+0300"), 10_800);
        assert_eq!(parse_offset("-0530"), -19_800);
        assert_eq!(parse_offset("junk"), 0);
    }
}
