use tracing::{debug, info};
use zaino_status::{Liveness as _, Readiness as _, Status};

use super::TestManager;
use crate::{poll_until_ready, PollableTip, ValidatorConnectionMarker, ValidatorExt};

impl<C, Conn> TestManager<C, Conn>
where
    C: ValidatorExt,
    Conn: ValidatorConnectionMarker,
{
    pub(super) async fn activate_nu5_nu6(&self, enable_zaino: bool) {
        match (enable_zaino, self.service_subscriber.as_ref()) {
            (true, Some(subscriber)) => {
                debug!("[TEST] Waiting for Zaino to be ready");
                poll_until_ready(
                    subscriber,
                    std::time::Duration::from_millis(100),
                    std::time::Duration::from_secs(30),
                )
                .await;
                self.generate_blocks_and_wait_for_tip(1, subscriber).await;
            }
            _ => {
                self.local_net.generate_blocks(1).await.unwrap();
            }
        }
        debug!("[TEST] Test environment ready");
    }

    /// Generate `n` blocks and wait for `pollable` to observe each new tip.
    pub async fn generate_blocks_and_wait_for_tip<P: PollableTip>(&self, n: u32, pollable: &P) {
        fn assert_live<S: Status>(pollable: &S, waiting_for: Option<u64>) {
            if !pollable.is_live() {
                let status = pollable.status();
                match waiting_for {
                    Some(height) => panic!(
                        "Pollable is not live while waiting for block {height} (status: {status:?})."
                    ),
                    None => panic!(
                        "Pollable is not live (status: {status:?}). The backing validator may have crashed or become unreachable."
                    ),
                }
            }
        }

        let chain_height = u64::from(self.local_net.get_chain_height().await);
        let target_height = chain_height + u64::from(n);
        let mut next_block_height = chain_height + 1;
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(50));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await;

        while pollable.tip_height().await < target_height {
            assert_live(pollable, None);
            if n == 0 {
                interval.tick().await;
            } else {
                self.local_net.generate_blocks(1).await.unwrap();
                while pollable.tip_height().await != next_block_height {
                    assert_live(pollable, Some(next_block_height));
                    interval.tick().await;
                }
                next_block_height += 1;
            }
        }

        if !pollable.is_ready() {
            let start = std::time::Instant::now();
            poll_until_ready(
                pollable,
                std::time::Duration::from_millis(50),
                std::time::Duration::from_secs(30),
            )
            .await;
            let elapsed = start.elapsed();
            if elapsed.as_millis() > 0 {
                info!(
                    "Readiness wait after height poll took {:?} (height polling alone was insufficient)",
                    elapsed
                );
            }
        }
    }

    /// Generate blocks and wait for two chain-index subscribers to observe the tip.
    pub async fn generate_blocks_and_wait_for_tips<A: PollableTip, B: PollableTip>(
        &self,
        n: u32,
        mined_against: &A,
        then_synced: &B,
    ) {
        self.generate_blocks_and_wait_for_tip(n, mined_against)
            .await;
        self.generate_blocks_and_wait_for_tip(0, then_synced).await;
    }

    /// Generate blocks in one validator call, then wait for both pollables.
    pub async fn generate_blocks_bulk_and_wait_for_tips<A: PollableTip, B: PollableTip>(
        &self,
        n: u32,
        mined_against: &A,
        then_synced: &B,
    ) {
        self.local_net
            .generate_blocks(n)
            .await
            .expect("validator failed to generate blocks");
        self.generate_blocks_and_wait_for_tips(0, mined_against, then_synced)
            .await;
    }

    /// Mine blocks one at a time and run `check` after both pollables synchronize.
    pub async fn generate_blocks_and_check_each<A: PollableTip, B: PollableTip>(
        &self,
        n: u32,
        mined_against: &A,
        then_synced: &B,
        mut check: impl AsyncFnMut(u32),
    ) {
        for index in 0..n {
            self.generate_blocks_and_wait_for_tips(1, mined_against, then_synced)
                .await;
            check(index).await;
        }
    }
}
