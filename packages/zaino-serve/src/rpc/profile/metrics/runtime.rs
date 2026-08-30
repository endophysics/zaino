use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use thiserror::Error;
use tokio::{sync::watch, task::JoinHandle};

use super::{
    method::ObservedMethod,
    window::{publish_all, MetricsPublisher, WindowPublisher, WindowState},
    PrivacyOutcome,
};

#[cfg(test)]
use super::window::PublishedWindow;

trait MonotonicClock: Send + Sync + 'static {
    fn now(&self) -> Duration;
}

struct SystemClock(Instant);

impl MonotonicClock for SystemClock {
    fn now(&self) -> Duration {
        self.0.elapsed()
    }
}

struct Inner {
    window: Duration,
    clock: Arc<dyn MonotonicClock>,
    publisher: Arc<dyn WindowPublisher>,
    state: Mutex<WindowState>,
}

impl Inner {
    fn lock_state(&self) -> std::sync::MutexGuard<'_, WindowState> {
        let state = self.state.lock();
        // SAFE-EXPECT: privacy metric critical sections contain no panicking operations.
        state.expect("privacy metrics state mutex poisoned")
    }

    fn tick(&self) {
        let completed = self.lock_state().take_completed(self.clock.now());
        publish_all(&self.publisher, completed);
    }

    fn record(&self, method: ObservedMethod, outcome: PrivacyOutcome, duration: Duration) {
        let completed = {
            let mut state = self.lock_state();
            let completed = state.take_completed(self.clock.now());
            state.record(method, outcome, duration);
            completed
        };
        publish_all(&self.publisher, completed);
    }

    fn close(&self) {
        self.lock_state().close();
    }
}

/// Configuration errors for the privacy aggregate owner.
#[derive(Debug, Error)]
pub enum PrivacyMetricsError {
    /// Fixed windows must have positive duration.
    #[error("privacy metrics window duration must be positive")]
    ZeroWindow,
    /// System time could not be represented relative to the Unix epoch.
    #[error("system time is before the Unix epoch")]
    SystemTimeBeforeUnixEpoch,
}

/// Cloneable handler-facing recorder with a typed, label-free API.
#[derive(Clone)]
pub struct PrivacyMetricsRecorder(Arc<Inner>);

impl std::fmt::Debug for PrivacyMetricsRecorder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PrivacyMetricsRecorder")
            .finish_non_exhaustive()
    }
}

impl PrivacyMetricsRecorder {
    pub(crate) fn window(&self) -> Duration {
        self.0.window
    }

    pub(crate) fn record(
        &self,
        method: ObservedMethod,
        outcome: PrivacyOutcome,
        duration: Duration,
    ) {
        self.0.record(method, outcome, duration);
    }
}

/// Endpoint-lifecycle owner for the privacy window timer and active state.
pub struct PrivacyWindowMetrics {
    inner: Arc<Inner>,
    close_tx: watch::Sender<bool>,
    timer: Option<JoinHandle<()>>,
}

impl PrivacyWindowMetrics {
    /// Starts a fixed-window aggregate owner anchored at construction time.
    pub fn new(window: Duration) -> Result<Self, PrivacyMetricsError> {
        if window.is_zero() {
            return Err(PrivacyMetricsError::ZeroWindow);
        }
        let start_unix_seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| PrivacyMetricsError::SystemTimeBeforeUnixEpoch)?
            .as_secs();
        let inner = Arc::new(Inner {
            window,
            clock: Arc::new(SystemClock(Instant::now())),
            publisher: Arc::new(MetricsPublisher),
            state: Mutex::new(WindowState::new(start_unix_seconds, window)),
        });
        let (close_tx, mut close_rx) = watch::channel(false);
        let timer_inner = Arc::clone(&inner);
        let timer = tokio::spawn(async move {
            let mut interval =
                tokio::time::interval_at(tokio::time::Instant::now() + window, window);
            loop {
                tokio::select! {
                    changed = close_rx.changed() => {
                        let _ = changed;
                        break;
                    }
                    _ = interval.tick() => timer_inner.tick(),
                }
            }
        });
        Ok(Self {
            inner,
            close_tx,
            timer: Some(timer),
        })
    }

    /// Returns the cloneable recorder passed through the privacy endpoint context.
    pub fn recorder(&self) -> PrivacyMetricsRecorder {
        PrivacyMetricsRecorder(Arc::clone(&self.inner))
    }

    /// Stops publication and deliberately discards the active partial window.
    pub async fn close(&mut self) {
        self.inner.close();
        let _ = self.close_tx.send(true);
        if let Some(timer) = self.timer.take() {
            let _ = timer.await;
        }
    }
}

impl Drop for PrivacyWindowMetrics {
    fn drop(&mut self) {
        self.inner.close();
        if let Some(timer) = self.timer.take() {
            timer.abort();
        }
    }
}

#[cfg(test)]
#[derive(Default)]
struct ManualClock(std::sync::atomic::AtomicU64);

#[cfg(test)]
impl MonotonicClock for ManualClock {
    fn now(&self) -> Duration {
        Duration::from_nanos(self.0.load(std::sync::atomic::Ordering::SeqCst))
    }
}

#[cfg(test)]
#[derive(Default)]
struct TestPublisher(Mutex<Vec<PublishedWindow>>);

#[cfg(test)]
impl WindowPublisher for TestPublisher {
    fn publish(&self, window: PublishedWindow) {
        self.0
            .lock()
            .expect("test publisher mutex poisoned")
            .push(window);
    }
}

#[cfg(test)]
pub(crate) struct PrivacyWindowTestHarness {
    owner: PrivacyWindowMetrics,
    clock: Arc<ManualClock>,
    publisher: Arc<TestPublisher>,
}

#[cfg(test)]
impl PrivacyWindowTestHarness {
    pub(crate) fn new(start_unix_seconds: u64, window: Duration) -> Self {
        let clock = Arc::new(ManualClock::default());
        let publisher = Arc::new(TestPublisher::default());
        let inner = Arc::new(Inner {
            window,
            clock: clock.clone(),
            publisher: publisher.clone(),
            state: Mutex::new(WindowState::new(start_unix_seconds, window)),
        });
        let (close_tx, _close_rx) = watch::channel(false);
        Self {
            owner: PrivacyWindowMetrics {
                inner,
                close_tx,
                timer: None,
            },
            clock,
            publisher,
        }
    }

    pub(crate) fn recorder(&self) -> PrivacyMetricsRecorder {
        self.owner.recorder()
    }

    pub(crate) fn advance(&self, duration: Duration) {
        self.clock.0.fetch_add(
            u64::try_from(duration.as_nanos()).expect("test duration fits u64 nanoseconds"),
            std::sync::atomic::Ordering::SeqCst,
        );
    }

    pub(crate) fn tick(&self) {
        self.owner.inner.tick();
    }

    pub(crate) fn close(&self) {
        self.owner.inner.close();
        let _ = self.owner.close_tx.send(true);
    }

    pub(crate) fn published(&self) -> Vec<PublishedWindow> {
        self.publisher
            .0
            .lock()
            .expect("test publisher mutex poisoned")
            .clone()
    }
}
