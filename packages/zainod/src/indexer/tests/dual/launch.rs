use std::time::Duration;

use tokio::net::TcpStream;
use zaino_proto::proto::privacy_profile::EndpointProfile;
use zaino_serve::server::config::JsonRpcServerConfig;
use zaino_status::StatusType;

use super::{capability, common_rpc, listener, privacy_settings};
use crate::{
    config::ZainodConfig,
    error::IndexerError,
    indexer::{
        test_support::{LifecycleTestService, LifecycleTestServiceConfig},
        Indexer,
    },
};

#[tokio::test]
async fn configured_privacy_when_launched_serves_both_profiles_from_one_service() {
    let (legacy_listener, legacy_address) = listener();
    let (privacy_listener, privacy_address) = listener();
    let service_config = LifecycleTestServiceConfig::default();
    let control = service_config.subscriber();
    let mut indexer_config = ZainodConfig::default();
    indexer_config.grpc_settings.listen_address = legacy_address;
    indexer_config.privacy_grpc_settings = Some(privacy_settings(privacy_address));

    let (handle, _) = Indexer::<LifecycleTestService>::launch_inner_with_listeners(
        service_config.clone(),
        indexer_config,
        legacy_listener,
        None,
        Some(privacy_listener),
    )
    .await
    .expect("dual endpoint indexer must launch");

    let legacy_capability = capability(legacy_address).await;
    let privacy_capability = capability(privacy_address).await;
    common_rpc(legacy_address).await;
    common_rpc(privacy_address).await;

    assert_eq!(service_config.spawn_count(), 1);
    assert_eq!(
        legacy_capability.endpoint_profile,
        EndpointProfile::Legacy as i32
    );
    assert_eq!(
        privacy_capability.endpoint_profile,
        EndpointProfile::Privacy as i32
    );
    assert_eq!(privacy_capability.metric_window_seconds, 73);
    assert!(
        privacy_capability
            .methods
            .iter()
            .find(|method| method.name == "GetTransaction")
            .expect("transaction policy must be advertised")
            .enabled
    );
    assert!(
        !privacy_capability
            .methods
            .iter()
            .find(|method| method.name == "GetTaddressTxids")
            .expect("transparent policy must be advertised")
            .enabled
    );

    control.set_status(StatusType::Closing);
    tokio::time::timeout(Duration::from_secs(2), handle)
        .await
        .expect("dual shutdown must not hang")
        .expect("supervisor task must join")
        .expect("dual shutdown must succeed");
    assert_eq!(service_config.close_count(), 1);
    assert!(TcpStream::connect(legacy_address).await.is_err());
    assert!(TcpStream::connect(privacy_address).await.is_err());
}

#[tokio::test]
async fn oversized_privacy_metrics_window_when_launched_returns_before_service_spawn() {
    let service_config = LifecycleTestServiceConfig::default();
    let mut indexer_config = ZainodConfig::default();
    let mut privacy = privacy_settings("127.0.0.1:0".parse().expect("address must parse"));
    privacy.metrics_window_seconds = u64::from(u32::MAX) + 1;
    indexer_config.privacy_grpc_settings = Some(privacy);

    let result =
        Indexer::<LifecycleTestService>::launch_inner(service_config.clone(), indexer_config).await;

    assert!(matches!(
        result,
        Err(IndexerError::ConfigError(message))
            if message == "privacy_grpc_settings.metrics_window_seconds must fit in uint32."
    ));
    assert_eq!(service_config.spawn_count(), 0);
    assert_eq!(service_config.close_count(), 0);
}

#[tokio::test]
async fn privacy_listener_without_config_when_launched_is_rejected_before_service_spawn() {
    let (legacy_listener, legacy_address) = listener();
    let (privacy_listener, privacy_address) = listener();
    let service_config = LifecycleTestServiceConfig::default();
    let mut indexer_config = ZainodConfig::default();
    indexer_config.grpc_settings.listen_address = legacy_address;

    let result = Indexer::<LifecycleTestService>::launch_inner_with_listeners(
        service_config.clone(),
        indexer_config,
        legacy_listener,
        None,
        Some(privacy_listener),
    )
    .await;

    assert!(matches!(
        result,
        Err(IndexerError::ConfigError(message))
            if message == "privacy_grpc_listener requires privacy_grpc_settings in launch_inner_with_listeners."
    ));
    assert_eq!(service_config.spawn_count(), 0);
    assert_eq!(service_config.close_count(), 0);
    assert!(TcpStream::connect(legacy_address).await.is_err());
    assert!(TcpStream::connect(privacy_address).await.is_err());
}

#[tokio::test]
async fn privacy_config_without_listener_when_test_launched_is_rejected_before_service_spawn() {
    let (legacy_listener, legacy_address) = listener();
    let (privacy_listener, privacy_address) = listener();
    drop(privacy_listener);
    let service_config = LifecycleTestServiceConfig::default();
    let mut indexer_config = ZainodConfig::default();
    indexer_config.grpc_settings.listen_address = legacy_address;
    indexer_config.privacy_grpc_settings = Some(privacy_settings(privacy_address));

    let result = Indexer::<LifecycleTestService>::launch_inner_with_listeners(
        service_config.clone(),
        indexer_config,
        legacy_listener,
        None,
        None,
    )
    .await;

    assert!(matches!(
        result,
        Err(IndexerError::ConfigError(message))
            if message == "privacy_grpc_settings requires privacy_grpc_listener in launch_inner_with_listeners."
    ));
    assert_eq!(service_config.spawn_count(), 0);
    assert_eq!(service_config.close_count(), 0);
    assert!(TcpStream::connect(legacy_address).await.is_err());
    assert!(TcpStream::connect(privacy_address).await.is_err());
}

#[tokio::test]
async fn privacy_bind_failure_when_legacy_and_json_started_rolls_back_everything() {
    let (legacy_reservation, legacy_address) = listener();
    let (json_reservation, json_address) = listener();
    let (privacy_occupier, privacy_address) = listener();
    drop(legacy_reservation);
    drop(json_reservation);
    let service_config = LifecycleTestServiceConfig::default();
    let mut indexer_config = ZainodConfig::default();
    indexer_config.grpc_settings.listen_address = legacy_address;
    indexer_config.json_server_settings = Some(JsonRpcServerConfig {
        json_rpc_listen_address: json_address,
        cookie_dir: None,
    });
    indexer_config.privacy_grpc_settings = Some(privacy_settings(privacy_address));

    let result =
        Indexer::<LifecycleTestService>::launch_inner(service_config.clone(), indexer_config).await;

    assert!(matches!(result, Err(IndexerError::ServerError(_))));
    assert_eq!(service_config.spawn_count(), 1);
    assert_eq!(service_config.close_count(), 1);
    assert!(TcpStream::connect(legacy_address).await.is_err());
    assert!(TcpStream::connect(json_address).await.is_err());
    drop(privacy_occupier);
}

#[tokio::test]
async fn legacy_bind_failure_returns_typed_error_and_closes_service() {
    let (occupier, occupied_address) = listener();
    let service_config = LifecycleTestServiceConfig::default();
    let mut indexer_config = ZainodConfig::default();
    indexer_config.grpc_settings.listen_address = occupied_address;

    let result =
        Indexer::<LifecycleTestService>::launch_inner(service_config.clone(), indexer_config).await;

    assert!(matches!(result, Err(IndexerError::ServerError(_))));
    assert_eq!(service_config.spawn_count(), 1);
    assert_eq!(service_config.close_count(), 1);
    drop(occupier);
}
