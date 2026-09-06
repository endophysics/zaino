//! gRPC / JsonRPC service implementations.

use std::sync::Arc;

use tokio::sync::watch;
use zaino_proto::proto::indexed_tip::indexed_tip_service_server::IndexedTipServiceServer;
use zaino_proto::proto::privacy_profile::privacy_profile_service_server::PrivacyProfileServiceServer;
use zaino_proto::proto::service::compact_tx_streamer_server::CompactTxStreamerServer;
use zaino_state::{IndexedTipIndexer, IndexerSubscriber, LightWalletIndexer, ZcashIndexer};

use profile::{CapabilityService, EndpointContext};

pub mod grpc;
mod indexed_tip;
pub mod jsonrpc;
pub mod profile;

#[cfg(any(test, feature = "test_dependencies"))]
/// Feature-gated in-memory RPC subscriber used by lifecycle tests.
pub mod test_support;

#[cfg(test)]
mod tests;

use indexed_tip::IndexedTipService;

#[derive(Clone)]
/// Zaino gRPC service.
pub struct GrpcClient<Indexer: ZcashIndexer + LightWalletIndexer> {
    /// Chain fetch service subscriber.
    service_subscriber: IndexerSubscriber<Indexer>,
    endpoint_context: Arc<EndpointContext>,
}

impl<Indexer: ZcashIndexer + LightWalletIndexer> GrpcClient<Indexer> {
    fn new(
        service_subscriber: IndexerSubscriber<Indexer>,
        endpoint_context: Arc<EndpointContext>,
    ) -> Self {
        Self {
            service_subscriber,
            endpoint_context,
        }
    }

    fn endpoint_context(&self) -> &EndpointContext {
        &self.endpoint_context
    }
}

#[derive(Clone)]
/// Zaino JSONRPC service.
pub struct JsonRpcClient<Indexer: ZcashIndexer + LightWalletIndexer> {
    /// Chain fetch service subscriber.
    pub service_subscriber: IndexerSubscriber<Indexer>,
}

/// Wraps an [`IndexerSubscriber`] in the generated `CompactTxStreamer`
/// gRPC service and produces type-erased [`tonic::service::Routes`].
///
/// Lives here (next to [`GrpcClient`]) so callers don't need a direct
/// dependency on `zaino-proto` to wire the gRPC dispatcher. The
/// transport-layer entrypoint
/// [`crate::server::grpc::TonicServer::spawn`] accepts the returned
/// [`tonic::service::Routes`] directly.
pub fn grpc_routes<Indexer: ZcashIndexer + LightWalletIndexer>(
    service_subscriber: IndexerSubscriber<Indexer>,
) -> tonic::service::Routes {
    grpc_routes_with_context(service_subscriber, EndpointContext::legacy())
}

/// Builds the legacy/operator routes, including Zaino's indexed-tip extension.
pub fn legacy_grpc_routes<Indexer>(
    service_subscriber: IndexerSubscriber<Indexer>,
    shutdown: watch::Receiver<()>,
) -> tonic::service::Routes
where
    Indexer: ZcashIndexer + LightWalletIndexer + IndexedTipIndexer + Clone,
{
    let indexed_tip_service = IndexedTipService::new(service_subscriber.inner_clone(), shutdown);
    grpc_routes(service_subscriber).add_service(IndexedTipServiceServer::new(indexed_tip_service))
}

/// Builds `CompactTxStreamer` routes with an immutable endpoint policy.
pub fn grpc_routes_with_context<Indexer: ZcashIndexer + LightWalletIndexer>(
    service_subscriber: IndexerSubscriber<Indexer>,
    endpoint_context: Arc<EndpointContext>,
) -> tonic::service::Routes {
    let capability = CapabilityService::new(service_subscriber.clone(), endpoint_context.clone());
    tonic::service::Routes::new(CompactTxStreamerServer::new(GrpcClient::new(
        service_subscriber,
        endpoint_context,
    )))
    .add_service(PrivacyProfileServiceServer::new(capability))
}
