use std::net::SocketAddr;

use tokio::net::TcpStream;
use zaino_serve::{
    rpc::{
        grpc_routes, grpc_routes_with_context,
        profile::{EndpointContext, PrivacyMethodPolicy, PrivacyWindowMetrics},
    },
    server::{config::GrpcServerConfig, grpc::TonicServer},
};
use zaino_state::{IndexerService, ZcashService};
use zaino_status::{StatusType, StatusType::Ready};

use super::listener;
use crate::indexer::{
    lifecycle::EndpointServers,
    test_support::{LifecycleTestService, LifecycleTestServiceConfig},
    Indexer,
};

async fn assembled_indexer(
    with_privacy: bool,
) -> (
    Indexer<LifecycleTestService>,
    SocketAddr,
    Option<SocketAddr>,
) {
    let service_config = LifecycleTestServiceConfig::default();
    let service = IndexerService::<LifecycleTestService>::spawn(service_config)
        .await
        .expect("test service must spawn");
    let (legacy_listener, legacy_address) = listener();
    let routes = grpc_routes(service.inner_ref().get_subscriber());
    let legacy = TonicServer::spawn_from_listener(
        routes,
        GrpcServerConfig {
            listen_address: legacy_address,
            tls: None,
        },
        legacy_listener,
    )
    .await
    .expect("legacy server must spawn");
    let (privacy, privacy_metrics, privacy_address) = if with_privacy {
        let (privacy_listener, privacy_address) = listener();
        let metrics = PrivacyWindowMetrics::new(std::time::Duration::from_secs(60))
            .expect("privacy metrics owner must start");
        let context = EndpointContext::privacy(PrivacyMethodPolicy::default(), metrics.recorder());
        let routes = grpc_routes_with_context(service.inner_ref().get_subscriber(), context);
        let privacy = TonicServer::spawn_from_listener(
            routes,
            GrpcServerConfig {
                listen_address: privacy_address,
                tls: None,
            },
            privacy_listener,
        )
        .await
        .expect("privacy server must spawn");
        (Some(privacy), Some(metrics), Some(privacy_address))
    } else {
        (None, None, None)
    };
    (
        Indexer {
            servers: EndpointServers {
                json_rpc: None,
                grpc_legacy: Some(legacy),
                grpc_privacy: privacy,
                privacy_metrics,
            },
            service: Some(service),
        },
        legacy_address,
        privacy_address,
    )
}

#[tokio::test]
async fn configured_privacy_when_not_ready_prevents_combined_readiness() {
    let (mut indexer, _, _) = assembled_indexer(true).await;
    indexer
        .servers
        .grpc_privacy
        .as_ref()
        .expect("privacy is configured")
        .status
        .store(StatusType::Spawning);

    assert_eq!(indexer.status(), StatusType::Spawning);
    indexer
        .servers
        .grpc_privacy
        .as_ref()
        .expect("privacy is configured")
        .status
        .store(Ready);
    assert_eq!(indexer.status(), Ready);
    indexer.close().await;
}

#[tokio::test]
async fn absent_privacy_when_legacy_ready_is_neutral() {
    let (mut indexer, _, _) = assembled_indexer(false).await;

    assert_eq!(indexer.status(), Ready);
    indexer.close().await;
}

#[tokio::test]
async fn either_configured_grpc_failure_triggers_critical_detection() {
    for privacy_fails in [false, true] {
        let (mut indexer, _, _) = assembled_indexer(true).await;
        let failed = if privacy_fails {
            indexer
                .servers
                .grpc_privacy
                .as_ref()
                .expect("privacy is configured")
        } else {
            indexer
                .servers
                .grpc_legacy
                .as_ref()
                .expect("legacy is configured")
        };
        failed.status.store(StatusType::Offline);

        assert!(indexer.check_for_critical_errors());
        indexer.close().await;
    }
}

#[tokio::test]
async fn graceful_close_waits_for_both_configured_grpc_servers() {
    let (mut indexer, legacy_address, privacy_address) = assembled_indexer(true).await;

    indexer.close().await;

    assert!(indexer.servers.privacy_metrics.is_none());
    assert!(TcpStream::connect(legacy_address).await.is_err());
    assert!(
        TcpStream::connect(privacy_address.expect("privacy is configured"))
            .await
            .is_err()
    );
}
