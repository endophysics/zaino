//! Lightwallet service RPC implementations.

use futures::StreamExt;

use super::handler::{client_method_helper, implement_client_methods, HandlerObservation};
use crate::rpc::{profile::GrpcMethod, GrpcClient};
use zaino_proto::proto::{
    compact_formats::CompactBlock,
    service::{
        compact_tx_streamer_server::CompactTxStreamer, Address, AddressList, Balance, BlockId,
        BlockRange, ChainSpec, Duration, Empty, GetAddressUtxosArg, GetAddressUtxosReplyList,
        GetMempoolTxRequest, GetSubtreeRootsArg, LightdInfo, PingResponse, RawTransaction,
        SendResponse, TransparentAddressBlockFilter, TreeState, TxFilter,
    },
};
use zaino_state::{
    AddressStream, CompactBlockStream, CompactTransactionStream, LightWalletIndexer,
    RawTransactionStream, SubtreeRootReplyStream, UtxoReplyStream, ZcashIndexer,
};

impl<Indexer: ZcashIndexer + LightWalletIndexer> CompactTxStreamer for GrpcClient<Indexer>
where
    Indexer::Error: Into<tonic::Status>,
{
    implement_client_methods!(
        "Return the height of the tip of the best chain."
        get_latest_block(ChainSpec) -> BlockId as empty => GetLatestBlock,
        "Return the compact block corresponding to the given block identifier."
        get_block(BlockId) -> CompactBlock => GetBlock,
        "Same as GetBlock except actions contain only nullifiers."
        get_block_nullifiers(BlockId) -> CompactBlock => GetBlockNullifiers,
        "Return a list of consecutive compact blocks."
        get_block_range(BlockRange) -> Self::GetBlockRangeStream as streaming => GetBlockRange,
        "Same as GetBlockRange except actions contain only nullifiers."
        get_block_range_nullifiers(BlockRange) -> Self::GetBlockRangeStream as streaming => GetBlockRangeNullifiers,
        "Return the requested full (not compact) transaction (as from the legacy full node)."
        get_transaction(TxFilter) -> RawTransaction => GetTransaction,
        "submit the given transaction to the zcash network."
        send_transaction(RawTransaction) -> SendResponse => SendTransaction,
        "Return the transactions corresponding to the given t-address within the given block range"
        get_taddress_transactions(TransparentAddressBlockFilter) -> Self::GetTaddressTransactionsStream as streaming => GetTaddressTransactions,
        "This name is misleading, returns the full transactions that have either inputs or outputs connected to the given transparent address."
        get_taddress_txids(TransparentAddressBlockFilter) -> Self::GetTaddressTxidsStream as streaming => GetTaddressTxids,
        "Returns the total balance for a list of taddrs"
        get_taddress_balance(AddressList) -> Balance => GetTaddressBalance,
        "Returns a stream of compact transactions currently in the mempool."
        get_mempool_tx(GetMempoolTxRequest) -> Self::GetMempoolTxStream as streaming => GetMempoolTx,
        "Returns the note commitment tree state for a block."
        get_tree_state(BlockId) -> TreeState => GetTreeState,
        "Returns roots of Sapling, Orchard, and Ironwood note commitment subtrees."
        get_subtree_roots(GetSubtreeRootsArg) -> Self::GetSubtreeRootsStream as streaming => GetSubtreeRoots,
        "Returns unspent outputs for a list of addresses."
        get_address_utxos(GetAddressUtxosArg) -> GetAddressUtxosReplyList => GetAddressUtxos,
        "Streams unspent outputs for a list of addresses."
        get_address_utxos_stream(GetAddressUtxosArg) -> Self::GetAddressUtxosStreamStream as streaming => GetAddressUtxosStream,
        "Return information about this lightwalletd instance and the blockchain"
        get_lightd_info(Empty) -> LightdInfo as empty => GetLightdInfo,
        "Returns the note commitment tree state for the chain tip."
        get_latest_tree_state(Empty) -> TreeState as empty => GetLatestTreeState,
        "Streams current mempool transactions until a new block is mined."
        get_mempool_stream(Empty) -> Self::GetMempoolStreamStream as streamingempty => GetMempoolStream,
        "Testing-only insecure ping RPC."
        ping(Duration) -> PingResponse => Ping,
    );

    type GetBlockRangeStream = std::pin::Pin<Box<CompactBlockStream>>;
    type GetBlockRangeNullifiersStream = std::pin::Pin<Box<CompactBlockStream>>;
    type GetTaddressTransactionsStream = std::pin::Pin<Box<RawTransactionStream>>;
    type GetTaddressTxidsStream = std::pin::Pin<Box<RawTransactionStream>>;

    fn get_taddress_balance_stream<'life0, 'async_trait>(
        &'life0 self,
        request: tonic::Request<tonic::Streaming<Address>>,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<tonic::Response<Balance>, tonic::Status>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        if self
            .endpoint_context()
            .request_logging_mode()
            .emits_method_events()
        {
            tracing::info!(
                method = "get_taddress_balance_stream",
                "[TEST] received call"
            );
        }
        Box::pin(async {
            let observation = HandlerObservation::begin(
                self.endpoint_context(),
                GrpcMethod::GetTaddressBalanceStream,
                "get_taddress_balance_stream",
            )?;
            let (channel_tx, channel_rx) =
                tokio::sync::mpsc::channel::<Result<Address, tonic::Status>>(32);
            let mut request_stream = request.into_inner();
            tokio::spawn(async move {
                while let Some(address_result) = request_stream.next().await {
                    if channel_tx.send(address_result).await.is_err() {
                        break;
                    }
                }
            });
            let address_stream = AddressStream::new(channel_rx);
            let grpc_result: Result<tonic::Response<Balance>, tonic::Status> = async {
                Ok(tonic::Response::new(
                    self.service_subscriber
                        .inner_ref()
                        .get_taddress_balance_stream(address_stream)
                        .await
                        .map_err(Into::into)?,
                ))
            }
            .await;
            observation.finish(&grpc_result);
            grpc_result
        })
    }

    type GetMempoolTxStream = std::pin::Pin<Box<CompactTransactionStream>>;
    type GetMempoolStreamStream = std::pin::Pin<Box<RawTransactionStream>>;
    type GetSubtreeRootsStream = std::pin::Pin<Box<SubtreeRootReplyStream>>;
    type GetAddressUtxosStreamStream = std::pin::Pin<Box<UtxoReplyStream>>;
}
