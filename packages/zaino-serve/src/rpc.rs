//! gRPC / JsonRPC service implementations.

use std::sync::Arc;

use zaino_proto::proto::privacy_profile::privacy_profile_service_server::PrivacyProfileServiceServer;
use zaino_proto::proto::service::compact_tx_streamer_server::CompactTxStreamerServer;
use zaino_state::{IndexerSubscriber, LightWalletIndexer, ZcashIndexer};

use profile::{CapabilityService, EndpointContext};

pub mod grpc;
pub mod jsonrpc;
pub mod profile;

#[cfg(any(test, feature = "test_dependencies"))]
/// Feature-gated in-memory RPC subscriber used by lifecycle tests.
pub mod test_support;

#[cfg(test)]
mod tests;

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
