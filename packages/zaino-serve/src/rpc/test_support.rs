use std::{future::ready, future::Future, sync::Arc};

use tonic::Status as TonicStatus;
use zaino_address::{ValidatedAddress, ZValidatedAddress};
use zaino_primitives::types::{
    rpc::{
        AddressDeltas, AddressDeltasRequest, BlockDeltas, BlockHeaderVerbose, BlockSubsidy,
        ChainTip, MiningInfo, NodeInfo, PeerInfo, SpentInfo, SpentOutpoint, SubtreeRoots, TxOut,
    },
    AddressBalance, BlockchainInfo, ShieldedPool, TransactionId, Treestate, TxOutSetInfo, Utxo,
};
use zaino_proto::proto::{
    compact_formats::CompactBlock,
    service::{
        AddressList, Balance, BlockId, BlockRange, Duration, GetAddressUtxosArg,
        GetAddressUtxosReplyList, GetMempoolTxRequest, LightdInfo, PingResponse, RawTransaction,
        SendResponse, TransparentAddressBlockFilter, TreeState, TxFilter,
    },
};
use zaino_state::{
    AddressStream, CompactBlockStream, CompactTransactionStream, IndexedTipIndexer,
    LightWalletIndexer, MempoolInfo, NodeBackedIndexerServiceError, RawTransactionStream,
    UtxoReplyStream, ZcashIndexer,
};
use zaino_status::{NamedAtomicStatus, Status, StatusType};
use zebra_chain::{block::Height, parameters::Network, subtree::NoteCommitmentSubtreeIndex};
use zebra_rpc::methods::{
    GetAddressBalanceRequest, GetAddressTxIdsRequest, GetBlock, GetBlockHash, GetRawTransaction,
};

/// Controllable in-memory subscriber for cross-crate lifecycle tests.
#[derive(Clone)]
pub struct TestIndexer {
    /// Number of RPC methods that reached this subscriber.
    pub accesses: Arc<std::sync::atomic::AtomicUsize>,
    status: NamedAtomicStatus,
    failure: Arc<std::sync::Mutex<TonicStatus>>,
    lightd_info_fails: Arc<std::sync::atomic::AtomicBool>,
    unsupported_lightd_info: Arc<std::sync::atomic::AtomicBool>,
}

impl Default for TestIndexer {
    fn default() -> Self {
        Self {
            accesses: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            status: NamedAtomicStatus::new("test_indexer", StatusType::Ready),
            failure: Arc::new(std::sync::Mutex::new(TonicStatus::unavailable(
                "test indexer",
            ))),
            lightd_info_fails: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            unsupported_lightd_info: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

impl TestIndexer {
    /// Changes the shared status observed by the daemon supervisor.
    pub fn set_status(&self, status: StatusType) {
        self.status.store(status);
    }

    /// Changes the status returned by fallible subscriber methods.
    pub fn set_failure(&self, failure: TonicStatus) {
        *self
            .failure
            .lock()
            .expect("test failure status mutex must remain healthy") = failure;
    }

    /// Makes capability metadata reads return the configured failure status.
    pub fn fail_lightd_info(&self) {
        self.lightd_info_fails
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Makes capability metadata reads return unsupported validator metadata.
    pub fn unsupported_lightd_info(&self) {
        self.unsupported_lightd_info
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    fn unavailable<T>(&self) -> std::future::Ready<Result<T, NodeBackedIndexerServiceError>> {
        self.accesses
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let failure = self
            .failure
            .lock()
            .expect("test failure status mutex must remain healthy")
            .clone();
        ready(Err(NodeBackedIndexerServiceError::TonicStatusError(
            failure,
        )))
    }

    fn available<T>(
        &self,
        value: T,
    ) -> std::future::Ready<Result<T, NodeBackedIndexerServiceError>> {
        self.accesses
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        ready(Ok(value))
    }
}

impl Status for TestIndexer {
    fn status(&self) -> StatusType {
        self.status.load()
    }
}

impl IndexedTipIndexer for TestIndexer {
    fn subscribe_indexed_tips(&self) -> zaino_state::IndexedTipStream {
        let tip = zaino_primitives::types::BlockRef {
            hash: zaino_primitives::types::BlockHash::from([0; 32]),
            height: zaino_primitives::types::Height::try_from(0)
                .expect("zero is a valid test height"),
        };
        Box::pin(futures::stream::once(async move { tip }))
    }
}

macro_rules! unavailable_methods {
    ($(fn $name:ident($($argument:ident: $argument_type:ty),*) -> $output:ty;)+) => {
        $(fn $name(
            &self,
            $($argument: $argument_type),*
        ) -> impl Future<Output = Result<$output, Self::Error>> + Send {
            $(let _ = $argument;)*
            self.unavailable()
        })+
    };
}

#[allow(deprecated)]
impl ZcashIndexer for TestIndexer {
    type Error = NodeBackedIndexerServiceError;

    unavailable_methods! {
        fn get_info() -> NodeInfo;
        fn get_address_deltas(params: AddressDeltasRequest) -> AddressDeltas;
        fn get_blockchain_info() -> BlockchainInfo;
        fn get_difficulty() -> f64;
        fn get_block_subsidy(height: u32) -> BlockSubsidy;
        fn get_mempool_info() -> MempoolInfo;
        fn get_peer_info() -> Vec<PeerInfo>;
        fn z_get_address_balance(addresses: GetAddressBalanceRequest) -> AddressBalance;
        fn send_raw_transaction(transaction: String) -> TransactionId;
        fn get_block_header(hash: String) -> BlockHeaderVerbose;
        fn get_raw_block_header(hash: String) -> Vec<u8>;
        fn z_get_block(hash_or_height: String, verbosity: Option<u8>) -> GetBlock;
        fn get_block_deltas(hash: String) -> BlockDeltas;
        fn get_block_count() -> Height;
        fn get_chain_tips() -> Vec<ChainTip>;
        fn validate_address(address: String) -> ValidatedAddress;
        fn z_validate_address(address: String) -> ZValidatedAddress;
        fn get_best_blockhash() -> GetBlockHash;
        fn get_raw_mempool() -> Vec<String>;
        fn z_get_treestate(hash_or_height: String) -> Treestate;
        fn z_get_subtrees_by_index(pool: ShieldedPool, start: NoteCommitmentSubtreeIndex, limit: Option<NoteCommitmentSubtreeIndex>) -> SubtreeRoots;
        fn get_raw_transaction(txid: String, verbose: Option<u8>) -> GetRawTransaction;
        fn get_tx_out(txid: String, index: u32, include_mempool: Option<bool>) -> Option<TxOut>;
        fn get_spent_info(outpoint: SpentOutpoint) -> SpentInfo;
        fn get_address_tx_ids(request: GetAddressTxIdsRequest) -> Vec<String>;
        fn z_get_address_utxos(addresses: GetAddressBalanceRequest) -> Vec<Utxo>;
        fn get_mining_info() -> MiningInfo;
        fn get_tx_out_set_info() -> Option<TxOutSetInfo>;
        fn get_network_sol_ps(blocks: Option<i32>, height: Option<i32>) -> u64;
        fn chain_height() -> Height;
    }

    fn network(&self) -> Network {
        Network::new_default_testnet()
    }
}

impl LightWalletIndexer for TestIndexer {
    unavailable_methods! {
        fn get_block(request: BlockId) -> CompactBlock;
        fn get_block_nullifiers(request: BlockId) -> CompactBlock;
        fn get_block_range_nullifiers(request: BlockRange) -> CompactBlockStream;
        fn get_transaction(request: TxFilter) -> RawTransaction;
        fn send_transaction(request: RawTransaction) -> SendResponse;
        fn get_taddress_transactions(request: TransparentAddressBlockFilter) -> RawTransactionStream;
        fn get_taddress_txids(request: TransparentAddressBlockFilter) -> RawTransactionStream;
        fn get_taddress_balance(request: AddressList) -> Balance;
        fn get_mempool_tx(request: GetMempoolTxRequest) -> CompactTransactionStream;
        fn get_mempool_stream() -> RawTransactionStream;
        fn get_tree_state(request: BlockId) -> TreeState;
        fn get_latest_tree_state() -> TreeState;
        fn get_address_utxos(request: GetAddressUtxosArg) -> GetAddressUtxosReplyList;
        fn get_address_utxos_stream(request: GetAddressUtxosArg) -> UtxoReplyStream;
        fn ping(request: Duration) -> PingResponse;
    }

    fn get_block_range(
        &self,
        request: BlockRange,
    ) -> impl Future<Output = Result<CompactBlockStream, Self::Error>> + Send {
        let _ = request;
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        drop(sender);
        self.available(CompactBlockStream::new(receiver))
    }

    fn get_taddress_balance_stream(
        &self,
        request: AddressStream,
    ) -> impl Future<Output = Result<Balance, Self::Error>> + Send {
        drop(request);
        self.available(Balance::default())
    }

    fn get_latest_block(&self) -> impl Future<Output = Result<BlockId, Self::Error>> + Send {
        self.available(BlockId::default())
    }

    fn get_lightd_info(&self) -> impl Future<Output = Result<LightdInfo, Self::Error>> + Send {
        if self
            .lightd_info_fails
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            return self.unavailable();
        }
        if self
            .unsupported_lightd_info
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            return self.available(LightdInfo::default());
        }
        self.available(LightdInfo {
            version: "test".to_string(),
            vendor: "zaino-test".to_string(),
            chain_name: "regtest".to_string(),
            zcashd_build: "zebra-v0.0.0-test".to_string(),
            zcashd_subversion: "/Zebra:0.0.0/".to_string(),
            ..LightdInfo::default()
        })
    }

    fn timeout_channel_size(&self) -> (u32, u32) {
        (1, 1)
    }
}
