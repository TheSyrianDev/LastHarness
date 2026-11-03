//! Scheduled tasks: a prompt the core runs by itself at set times, in a folder, with a model. The core's
//! scheduler asks for the ones that are due, runs each as an unattended chat, and records how it went.

use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};

use crate::{Result, Store};

/// When a task runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum When {
    /// Every so many seconds, counted from the last start.
    Every(i64),
    /// Each day at this local time (hour, minute).
    Daily(u32, u32),
}

impl When {
    /// Read "every 30m", "every 2h", "every 1d" or "daily 08:00". Says in words what is wrong otherwise.
    pub fn parse(s: &str) -> std::result::Result<When, String> {
        let s = s.trim().to_lowercase();
        let bad = || format!("\"{s}\" is not a time I understand. Write for example \"every 30m\", \"every 2h\", \"every 1d\" or \"daily 08:00\".");
        if let Some(rest) = s.strip_prefix("every ") {
            let rest = rest.trim();
            let (num, unit) = rest.split_at(rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len()));
            let n: i64 = num.parse().map_err(|_| bad())?;
            let secs = match unit.trim() {
                "m" | "min" | "minute" | "minutes" => n * 60,
                "h" | "hour" | "hours" => n * 3600,
                "d" | "day" | "days" => n * 86400,
                _ => return Err(bad()),
            };
            if secs < 300 {
                return Err("The shortest gap is 5 minutes.".into());
            }
            return Ok(When::Every(secs));
        }
        if let Some(rest) = s.strip_prefix("daily ") {
            let (h, m) = rest.trim().split_once(':').ok_or_else(bad)?;
            let (h, m): (u32, u32) = (h.parse().map_err(|_| bad())?, m.parse().map_err(|_| bad())?);
            if h > 23 || m > 59 {
                return Err(bad());
            }
            return Ok(When::Daily(h, m));
        }
        Err(bad())
    }

    pub fn text(&self) -> String {
        match self {
            When::Every(s) if s % 86400 == 0 => format!("every {}d", s / 86400),
            When::Every(s) if s % 3600 == 0 => format!("every {}h", s / 3600),
            When::Every(s) => format!("every {}m", s / 60),
            When::Daily(h, m) => format!("daily {h:02}:{m:02}"),
        }
    }

    /// The next start after `now` (unix seconds). `local_offset` is the machine's offset from UTC in seconds.
    pub fn next_after(&self, now: i64, local_offset: i64) -> i64 {
        match self {
            When::Every(s) => now + s,
            When::Daily(h, m) => {
                let local = now + local_offset;
                let day_start = local - local.rem_euclid(86400);
                let mut t = day_start + (*h as i64) * 3600 + (*m as i64) * 60;
                if t <= local {
                    t += 86400;
                }
                t - local_offset
            }
        }
    }
}

/// The machine's offset from UTC in seconds now, from `date +%z`. 0 when it cannot be read.
pub fn local_offset() -> i64 {
    let out = std::process::Command::new("date").arg("+%z").output().ok();
    let s = out.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    parse_offset(&s)
}

fn parse_offset(s: &str) -> i64 {
    if s.len() != 5 {
        return 0;
    }
    let sign = if s.starts_with('-') { -1 } else { 1 };
    let (h, m): (i64, i64) = (s[1..3].parse().unwrap_or(0), s[3..5].parse().unwrap_or(0));
    sign * (h * 3600 + m * 60)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Schedule {
    pub id: i64,
    pub name: String,
    pub prompt: String,
    pub folder: String,
    pub model: String,
    pub when: When,
    pub enabled: bool,
    pub next_run: i64,
    pub last_run: Option<i64>,
    /// "ok", "failed: …" or "refused: …" from the last run.
    pub last_result: String,
}

impl Schedule {
    pub fn json(&self) -> Value {
        json!({"id": self.id, "name": self.name, "prompt": self.prompt, "folder": self.folder, "model": self.model, "when": self.when.text(),
               "enabled": self.enabled, "next_run": self.next_run, "last_run": self.last_run, "last_result": self.last_result})
    }
}

fn row(r: &rusqlite::Row) -> rusqlite::Result<Schedule> {
    let when: String = r.get(5)?;
    Ok(Schedule {
        id: r.get(0)?,
        name: r.get(1)?,
        prompt: r.get(2)?,
        folder: r.get(3)?,
        model: r.get(4)?,
        when: When::parse(&when).unwrap_or(When::Every(86400)),
        enabled: r.get::<_, i64>(6)? != 0,
        next_run: r.get(7)?,
        last_run: r.get(8)?,
        last_result: r.get(9)?,
    })
}

const COLS: &str = "id, name, prompt, folder, model, run_when, enabled, next_run, last_run, last_result";

impl Store {
    #[allow(clippy::too_many_arguments)] // the task's own fields, plus the clock
    pub fn add_schedule(&self, name: &str, prompt: &str, folder: &str, model: &str, when: &When, now: i64, offset: i64) -> Result<i64> {
        let next = when.next_after(now, offset);
        self.with(|c| {
            c.execute("insert into schedules(name, prompt, folder, model, run_when, next_run, created) values (?1, ?2, ?3, ?4, ?5, ?6, ?7)", params![name, prompt, folder, model, when.text(), next, now])?;
            Ok(c.last_insert_rowid())
        })
    }

    pub fn schedules(&self) -> Result<Vec<Schedule>> {
        self.with(|c| c.prepare(&format!("select {COLS} from schedules order by id"))?.query_map([], row)?.collect())
    }

    pub fn schedule(&self, id: i64) -> Result<Option<Schedule>> {
        self.with(|c| c.query_row(&format!("select {COLS} from schedules where id = ?1"), [id], row).optional())
    }

    pub fn remove_schedule(&self, id: i64) -> Result<bool> {
        self.with(|c| Ok(c.execute("delete from schedules where id = ?1", [id])? > 0))
    }

    /// Turn a task on or off. Turning it on counts the next start from now, so missed runs are not made up.
    pub fn enable_schedule(&self, id: i64, on: bool, now: i64, offset: i64) -> Result<bool> {
        let Some(s) = self.schedule(id)? else { return Ok(false) };
        let next = if on { s.when.next_after(now, offset) } else { s.next_run };
        self.with(|c| Ok(c.execute("update schedules set enabled = ?2, next_run = ?3 where id = ?1", params![id, on as i64, next])? > 0))
    }

    /// Tasks that are on and due, oldest first. Each is moved to its next start at once, so a slow run or a
    /// restart never starts the same task twice, and runs missed while the core was off are not made up.
    pub fn take_due(&self, now: i64, offset: i64) -> Result<Vec<Schedule>> {
        let due: Vec<Schedule> = self.with(|c| c.prepare(&format!("select {COLS} from schedules where enabled = 1 and next_run <= ?1 order by next_run"))?.query_map([now], row)?.collect())?;
        for s in &due {
            let next = s.when.next_after(now, offset);
            self.with(|c| c.execute("update schedules set next_run = ?2, last_run = ?3 where id = ?1", params![s.id, next, now]))?;
        }
        Ok(due)
    }

    pub fn finish_schedule(&self, id: i64, result: &str) -> Result<()> {
        self.with(|c| c.execute("update schedules set last_result = ?2 where id = ?1", params![id, result])).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_are_read_in_plain_words() {
        assert_eq!(When::parse("every 30m"), Ok(When::Every(1800)));
        assert_eq!(When::parse("Every 2 hours"), Ok(When::Every(7200)));
        assert_eq!(When::parse("daily 08:05"), Ok(When::Daily(8, 5)));
        assert!(When::parse("every 1m").unwrap_err().contains("5 minutes"));
        assert!(When::parse("daily 25:00").is_err() && When::parse("sometimes").is_err());
        for t in ["every 30m", "every 2h", "every 1d", "daily 08:05"] {
            assert_eq!(When::parse(t).unwrap().text(), t);
        }
    }

    #[test]
    fn the_next_daily_start_is_in_local_time() {
        let day = 1_800_000_000 - 1_800_000_000 % 86400; // a UTC midnight
        // 07:00 UTC at +02:00 is 09:00 local, so 08:00 local is tomorrow, at 06:00 UTC
        assert_eq!(When::Daily(8, 0).next_after(day + 7 * 3600, 7200), day + 86400 + 6 * 3600);
        assert_eq!(When::Daily(10, 0).next_after(day + 7 * 3600, 7200), day + 8 * 3600);
        assert_eq!((parse_offset("+0200"), parse_offset("-0530"), parse_offset("x")), (7200, -19800, 0));
    }

    #[test]
    fn a_due_task_is_taken_once() {
        let s = Store::open_memory().unwrap();
        let id = s.add_schedule("news", "sum up", "/tmp", "m", &When::Every(3600), 1000, 0).unwrap();
        assert!(s.take_due(1000, 0).unwrap().is_empty());
        assert_eq!(s.take_due(4600, 0).unwrap().len(), 1);
        assert!(s.take_due(4601, 0).unwrap().is_empty(), "already moved on");
        s.finish_schedule(id, "ok").unwrap();
        let got = s.schedule(id).unwrap().unwrap();
        assert_eq!((got.next_run, got.last_run, got.last_result.as_str()), (8200, Some(4600), "ok"));
        s.enable_schedule(id, false, 9000, 0).unwrap();
        assert!(s.take_due(99_999, 0).unwrap().is_empty(), "off tasks do not run");
        assert!(s.remove_schedule(id).unwrap() && s.schedules().unwrap().is_empty());
    }
}
