//! Watching and steering workers: a list of who is doing what, and pause, resume and stop for each.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::{watch, Notify};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Queued,
    Running,
    Paused,
    Done,
    Failed,
    Stopped,
    /// Set aside for the user because nobody was there to decide.
    Parked,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Queued => "queued",
            Status::Running => "running",
            Status::Paused => "paused",
            Status::Done => "done",
            Status::Failed => "failed",
            Status::Stopped => "stopped",
            Status::Parked => "parked",
        }
    }
}

/// A handle a worker checks between model calls. Cloning it shares the same switches.
#[derive(Clone)]
pub struct StepControl {
    stop: Arc<AtomicBool>,
    stop_note: Arc<Notify>,
    pause: Arc<watch::Sender<bool>>,
}

impl Default for StepControl {
    fn default() -> Self {
        Self { stop: Arc::new(AtomicBool::new(false)), stop_note: Arc::new(Notify::new()), pause: Arc::new(watch::channel(false).0) }
    }
}

impl StepControl {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        self.stop_note.notify_waiters();
        let _ = self.pause.send(false); // wake a paused worker so it can see the stop
    }
    pub fn pause(&self) {
        let _ = self.pause.send(true);
    }
    pub fn resume(&self) {
        let _ = self.pause.send(false);
    }
    pub fn is_stopped(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }
    pub fn is_paused(&self) -> bool {
        *self.pause.borrow()
    }

    /// Call between model calls. Waits while paused. Returns `false` if the worker must stop.
    pub async fn checkpoint(&self) -> bool {
        let mut rx = self.pause.subscribe();
        while *rx.borrow() && !self.is_stopped() {
            if rx.changed().await.is_err() {
                break;
            }
        }
        !self.is_stopped()
    }

    /// Completes when stop is pressed.
    pub async fn stopped(&self) {
        loop {
            let n = self.stop_note.notified();
            if self.is_stopped() {
                return;
            }
            n.await;
        }
    }
}

#[derive(Clone, Debug)]
pub struct WorkerInfo {
    pub id: String,
    pub dept: String,
    pub role: String,
    pub model: String,
    pub step: String,
    pub status: Status,
    pub attempt: u32,
}

/// Who is working on what right now. The terminal shows this list.
#[derive(Default)]
pub struct Registry {
    next: AtomicU64,
    workers: Mutex<BTreeMap<u64, (WorkerInfo, StepControl)>>,
}

impl Registry {
    pub fn add(&self, mut info: WorkerInfo) -> (String, StepControl) {
        let n = self.next.fetch_add(1, Ordering::SeqCst) + 1;
        info.id = format!("w{n}");
        let ctl = StepControl::default();
        self.workers.lock().unwrap().insert(n, (info.clone(), ctl.clone()));
        (info.id, ctl)
    }

    fn with(&self, id: &str, f: impl FnOnce(&mut WorkerInfo, &StepControl)) -> bool {
        let Some(n) = id.strip_prefix('w').and_then(|n| n.parse::<u64>().ok()) else { return false };
        match self.workers.lock().unwrap().get_mut(&n) {
            Some((i, c)) => {
                f(i, c);
                true
            }
            None => false,
        }
    }

    pub fn set(&self, id: &str, status: Status, model: Option<&str>, attempt: Option<u32>) {
        self.with(id, |i, _| {
            // a paused worker stays paused until resumed, whatever else changes
            if i.status != Status::Paused || !matches!(status, Status::Running) {
                i.status = status;
            }
            if let Some(m) = model {
                i.model = m.to_string();
            }
            if let Some(a) = attempt {
                i.attempt = a;
            }
        });
    }

    pub fn list(&self) -> Vec<WorkerInfo> {
        self.workers.lock().unwrap().values().map(|(i, _)| i.clone()).collect()
    }

    pub fn pause(&self, id: &str) -> bool {
        self.with(id, |i, c| {
            if i.status == Status::Running || i.status == Status::Queued {
                c.pause();
                i.status = Status::Paused;
            }
        })
    }

    pub fn resume(&self, id: &str) -> bool {
        self.with(id, |i, c| {
            if i.status == Status::Paused {
                c.resume();
                i.status = Status::Running;
            }
        })
    }

    pub fn stop(&self, id: &str) -> bool {
        self.with(id, |i, c| {
            if matches!(i.status, Status::Queued | Status::Running | Status::Paused) {
                c.stop();
                i.status = Status::Stopped;
            }
        })
    }

    /// Stop everything that is not finished.
    pub fn stop_all(&self) {
        let ids: Vec<String> = self.list().into_iter().map(|w| w.id).collect();
        for id in ids {
            self.stop(&id);
        }
    }

    /// Forget finished workers, so the list shows only what matters.
    pub fn clear_finished(&self) {
        self.workers.lock().unwrap().retain(|_, (i, _)| matches!(i.status, Status::Queued | Status::Running | Status::Paused));
    }
}
