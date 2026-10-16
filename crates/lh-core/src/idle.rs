//! Idle mode: when nothing has happened for a while, the core lets go of what it can and stops
//! using the CPU. While idle there is no timer and no polling; the next request wakes it.
//!
//! One task runs the whole thing. Awake, it sleeps until the idle time would run out (a single
//! timer). If a request arrives first, the timer is reset. Once idle it waits on a signal with no
//! timer at all.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::time::Instant;

use tokio::sync::Notify;

type Hook = Box<dyn Fn(bool) + Send + Sync>;

pub struct Idle {
    /// Milliseconds of quiet before going idle; 0 means never. Can change while running.
    after: AtomicU64,
    start: Instant,
    /// Milliseconds since `start` of the last activity.
    last: AtomicUsize,
    idle: AtomicBool,
    active: AtomicUsize,
    wake: Notify,
    hooks: Mutex<Vec<Hook>>,
}

/// Held while work is in progress (a streaming turn, a waiting question). Idle never starts under it.
pub struct Busy(Arc<Idle>);
impl Drop for Busy {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::SeqCst);
        self.0.touch();
        self.0.wake.notify_one();
    }
}

impl Idle {
    /// `after` of zero means the core never goes idle.
    pub fn new(after: Duration) -> Arc<Self> {
        Arc::new(Self {
            after: AtomicU64::new(after.as_millis() as u64),
            start: Instant::now(),
            last: AtomicUsize::new(0),
            idle: AtomicBool::new(false),
            active: AtomicUsize::new(0),
            wake: Notify::new(),
            hooks: Mutex::new(vec![]),
        })
    }

    pub fn after(&self) -> Duration {
        Duration::from_millis(self.after.load(Ordering::SeqCst))
    }

    /// Change the quiet time while running. Zero means never. The manager picks the new value up at once.
    pub fn set_after(&self, after: Duration) {
        self.after.store(after.as_millis() as u64, Ordering::SeqCst);
        self.wake.notify_one();
    }

    /// `f(true)` runs when the core goes idle, `f(false)` when it wakes.
    pub fn on_change(&self, f: impl Fn(bool) + Send + Sync + 'static) {
        self.hooks.lock().unwrap().push(Box::new(f));
    }

    pub fn is_idle(&self) -> bool {
        self.idle.load(Ordering::SeqCst)
    }

    fn run_hooks(&self, idle: bool) {
        for h in self.hooks.lock().unwrap().iter() {
            h(idle);
        }
    }

    /// Something happened. Wakes the core if it was idle.
    pub fn touch(&self) {
        self.last.store(self.start.elapsed().as_millis() as usize, Ordering::SeqCst);
        if self.idle.swap(false, Ordering::SeqCst) {
            self.run_hooks(false);
            self.wake.notify_one();
        }
    }

    pub fn busy(self: &Arc<Self>) -> Busy {
        self.active.fetch_add(1, Ordering::SeqCst);
        self.touch();
        Busy(self.clone())
    }

    /// The idle manager. Spawn it once.
    pub async fn run(self: Arc<Self>) {
        loop {
            let after = self.after();
            if after.is_zero() {
                self.wake.notified().await; // never idle for now; a change of the setting wakes us
                continue;
            }
            if self.active.load(Ordering::SeqCst) > 0 {
                self.wake.notified().await; // a `Busy` ended
                continue;
            }
            let since = Duration::from_millis((self.start.elapsed().as_millis() as usize).saturating_sub(self.last.load(Ordering::SeqCst)) as u64);
            if since < after {
                tokio::select! {
                    _ = tokio::time::sleep(after - since) => {}
                    _ = self.wake.notified() => {}
                }
                continue;
            }
            self.idle.store(true, Ordering::SeqCst);
            self.run_hooks(true);
            while self.idle.load(Ordering::SeqCst) {
                self.wake.notified().await; // no timer while idle
            }
        }
    }
}

/// Give freed heap memory back to the OS (glibc only; other allocators already do this).
pub fn trim_memory() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        extern "C" {
            fn malloc_trim(pad: usize) -> i32;
        }
        // SAFETY: malloc_trim takes no pointers and is safe to call from any thread.
        unsafe {
            malloc_trim(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    fn counting(idle: &Arc<Idle>) -> Arc<(AtomicUsize, AtomicUsize)> {
        let c = Arc::new((AtomicUsize::new(0), AtomicUsize::new(0)));
        let c2 = c.clone();
        idle.on_change(move |i| {
            if i { &c2.0 } else { &c2.1 }.fetch_add(1, Ordering::SeqCst);
        });
        c
    }
    async fn settle() {
        for _ in 0..5 {
            tokio::task::yield_now().await;
        }
    }

    #[tokio::test(start_paused = true)]
    async fn goes_idle_after_the_quiet_time_and_wakes_on_a_touch() {
        let idle = Idle::new(Duration::from_secs(60));
        let c = counting(&idle);
        tokio::spawn(idle.clone().run());
        settle().await;
        tokio::time::advance(Duration::from_secs(59)).await;
        settle().await;
        assert!(!idle.is_idle());
        tokio::time::advance(Duration::from_secs(2)).await;
        settle().await;
        assert!(idle.is_idle());
        assert_eq!(c.0.load(Ordering::SeqCst), 1);
        // a long stretch of idle time does nothing more
        tokio::time::advance(Duration::from_secs(36_000)).await;
        settle().await;
        assert_eq!((c.0.load(Ordering::SeqCst), c.1.load(Ordering::SeqCst)), (1, 0));
        idle.touch();
        settle().await;
        assert!(!idle.is_idle());
        assert_eq!(c.1.load(Ordering::SeqCst), 1);
        // and the cycle starts again
        tokio::time::advance(Duration::from_secs(61)).await;
        settle().await;
        assert_eq!(c.0.load(Ordering::SeqCst), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn activity_resets_the_timer_and_work_in_progress_blocks_idle() {
        let idle = Idle::new(Duration::from_secs(60));
        let c = counting(&idle);
        tokio::spawn(idle.clone().run());
        settle().await;
        tokio::time::advance(Duration::from_secs(50)).await;
        idle.touch();
        tokio::time::advance(Duration::from_secs(50)).await;
        settle().await;
        assert!(!idle.is_idle(), "the touch reset the clock");
        let work = idle.busy();
        tokio::time::advance(Duration::from_secs(600)).await;
        settle().await;
        assert!(!idle.is_idle(), "a running turn keeps the core awake");
        drop(work);
        settle().await;
        tokio::time::advance(Duration::from_secs(61)).await;
        settle().await;
        assert!(idle.is_idle());
        assert_eq!(c.0.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn zero_means_never() {
        let idle = Idle::new(Duration::ZERO);
        tokio::spawn(idle.clone().run());
        tokio::time::advance(Duration::from_secs(100_000)).await;
        settle().await;
        assert!(!idle.is_idle());
    }

    #[tokio::test(start_paused = true)]
    async fn the_quiet_time_can_change_while_running() {
        let idle = Idle::new(Duration::ZERO);
        tokio::spawn(idle.clone().run());
        settle().await;
        idle.set_after(Duration::from_secs(60)); // from never to a minute
        settle().await;
        tokio::time::advance(Duration::from_secs(61)).await;
        settle().await;
        assert!(idle.is_idle());
        idle.touch();
        idle.set_after(Duration::ZERO); // and back to never
        settle().await;
        tokio::time::advance(Duration::from_secs(1000)).await;
        settle().await;
        assert!(!idle.is_idle());
    }
}
