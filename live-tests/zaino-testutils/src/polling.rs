use tracing::{debug, instrument};
use zaino_state::{
    BlockchainSource, ChainIndex, LightWalletIndexer, NodeBackedChainIndexSubscriber,
    NodeBackedIndexerServiceSubscriber, WithChainHeadSource,
};
use zaino_status::{Readiness, Status};

/// Polls until the given component reports ready.
#[instrument(name = "poll_until_ready", skip(component), fields(timeout_ms = timeout.as_millis() as u64))]
pub async fn poll_until_ready(
    component: &impl Readiness,
    poll_interval: std::time::Duration,
    timeout: std::time::Duration,
) -> bool {
    debug!("[POLL] Waiting for component to be ready");
    let result = tokio::time::timeout(timeout, async {
        let mut interval = tokio::time::interval(poll_interval);
        loop {
            interval.tick().await;
            if component.is_ready() {
                return;
            }
        }
    })
    .await
    .is_ok();
    if result {
        debug!("[POLL] Component is ready");
    } else {
        debug!("[POLL] Timeout waiting for component");
    }
    result
}

/// Source of current tip height for block-and-wait helpers.
pub trait PollableTip: Status + Sync {
    /// Current observable tip height, in absolute block-height units.
    fn tip_height(&self) -> impl std::future::Future<Output = u64>;
}

impl<Source: BlockchainSource + WithChainHeadSource> PollableTip
    for NodeBackedIndexerServiceSubscriber<Source>
{
    async fn tip_height(&self) -> u64 {
        self.get_latest_block()
            .await
            .expect("PollableTip: NodeBackedIndexerServiceSubscriber::get_latest_block failed")
            .height
    }
}

impl<Source: BlockchainSource + WithChainHeadSource> PollableTip
    for NodeBackedChainIndexSubscriber<Source>
{
    async fn tip_height(&self) -> u64 {
        let snapshot = self.snapshot_nonfinalized_state();
        u64::from(u32::from(
            self.best_chaintip(&snapshot)
                .await
                .expect("PollableTip: chain-index best_chaintip failed")
                .height,
        ))
    }
}
