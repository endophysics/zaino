use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use zaino_serve::rpc::test_support::TestIndexer;
use zaino_state::{
    FinalisedStateMode, IndexerSubscriber, NodeBackedIndexerServiceError, ZcashService,
};
use zaino_status::{Status, StatusType};

/// Shared controls and counters for the feature-gated daemon lifecycle seam.
#[derive(Clone)]
pub struct LifecycleTestServiceConfig {
    subscriber: TestIndexer,
    spawn_count: Arc<AtomicUsize>,
    close_count: Arc<AtomicUsize>,
}

impl Default for LifecycleTestServiceConfig {
    fn default() -> Self {
        Self {
            subscriber: TestIndexer::default(),
            spawn_count: Arc::new(AtomicUsize::new(0)),
            close_count: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl LifecycleTestServiceConfig {
    /// Returns the subscriber control shared with any spawned service.
    pub fn subscriber(&self) -> TestIndexer {
        self.subscriber.clone()
    }

    /// Returns how many services were spawned from this configuration.
    pub fn spawn_count(&self) -> usize {
        self.spawn_count.load(Ordering::SeqCst)
    }

    /// Returns how many spawned services completed explicit shutdown.
    pub fn close_count(&self) -> usize {
        self.close_count.load(Ordering::SeqCst)
    }
}

/// In-memory service used to drive the assembled daemon lifecycle in tests.
pub struct LifecycleTestService {
    subscriber: TestIndexer,
    close_count: Arc<AtomicUsize>,
}

impl Status for LifecycleTestService {
    fn status(&self) -> StatusType {
        self.subscriber.status()
    }
}

impl ZcashService for LifecycleTestService {
    type Subscriber = TestIndexer;
    type Config = LifecycleTestServiceConfig;

    async fn spawn(config: Self::Config) -> Result<Self, NodeBackedIndexerServiceError> {
        config.spawn_count.fetch_add(1, Ordering::SeqCst);
        Ok(Self {
            subscriber: config.subscriber,
            close_count: config.close_count,
        })
    }

    fn get_subscriber(&self) -> IndexerSubscriber<Self::Subscriber> {
        IndexerSubscriber::new(self.subscriber.clone())
    }

    fn finalised_state_mode(&self) -> FinalisedStateMode {
        FinalisedStateMode::EphemeralConfigured
    }

    fn close(&mut self) {
        self.close_count.fetch_add(1, Ordering::SeqCst);
        self.subscriber.set_status(StatusType::Offline);
    }
}
