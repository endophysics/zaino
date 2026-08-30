use futures::StreamExt as _;
use zaino_proto::proto::{
    compact_formats::CompactBlock,
    service::{BlockId, BlockRange},
    utils::{pool_types_into_i32_vec, PoolTypeFilter},
};
use zaino_state::LightWalletIndexer;

/// All four value pools as the `i32`s a `get_block_range` request carries.
pub fn all_pools_i32() -> Vec<i32> {
    pool_types_into_i32_vec(&PoolTypeFilter::includes_all().to_pool_types_vector())
}

/// The shielded pools as request `i32`s.
pub fn shielded_pools_i32() -> Vec<i32> {
    pool_types_into_i32_vec(&PoolTypeFilter::default().to_pool_types_vector())
}

/// Collect a `get_block_range` query over heights `[start, end]`.
#[allow(deprecated)]
pub async fn collect_block_range<S: LightWalletIndexer>(
    subscriber: &S,
    start: u64,
    end: u64,
    pool_types: Vec<i32>,
) -> Vec<CompactBlock> {
    subscriber
        .get_block_range(BlockRange {
            start: Some(BlockId {
                height: start,
                hash: vec![],
            }),
            end: Some(BlockId {
                height: end,
                hash: vec![],
            }),
            pool_types,
        })
        .await
        .expect("get_block_range")
        .map(|block| block.expect("compact block in range"))
        .collect()
        .await
}

/// Drain a block-range query and report whether its stream errored.
#[allow(deprecated)]
pub async fn drain_block_range<S: LightWalletIndexer>(
    subscriber: &S,
    start: u64,
    end: u64,
    pool_types: Vec<i32>,
) -> (Vec<CompactBlock>, bool) {
    let mut stream = subscriber
        .get_block_range(BlockRange {
            start: Some(BlockId {
                height: start,
                hash: vec![],
            }),
            end: Some(BlockId {
                height: end,
                hash: vec![],
            }),
            pool_types,
        })
        .await
        .expect("get_block_range");
    let mut blocks = Vec::new();
    let mut errored = false;
    while let Some(item) = stream.next().await {
        match item {
            Ok(block) => blocks.push(block),
            Err(_) => {
                errored = true;
                break;
            }
        }
    }
    (blocks, errored)
}
