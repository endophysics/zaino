use std::net::{Ipv4Addr, TcpListener};

use tokio::net::TcpStream;
use zaino_status::{Status, StatusType};

use super::{
    test_support::{LifecycleTestService, LifecycleTestServiceConfig},
    Indexer,
};
use crate::config::ZainodConfig;

mod dual;

#[tokio::test]
async fn default_config_when_launched_uses_legacy_prebound_listener_and_closes_cleanly() {
    let listener =
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("legacy loopback listener must bind");
    let address = listener
        .local_addr()
        .expect("bound listener has an address");
    let service_config = LifecycleTestServiceConfig::default();
    let control = service_config.subscriber();
    let mut indexer_config = ZainodConfig::default();
    indexer_config.grpc_settings.listen_address = address;

    let (handle, subscriber) = Indexer::<LifecycleTestService>::launch_inner_with_listeners(
        service_config.clone(),
        indexer_config,
        listener,
        None,
        None,
    )
    .await
    .expect("legacy-only indexer must launch");

    assert_eq!(service_config.spawn_count(), 1);
    assert_eq!(subscriber.status(), StatusType::Ready);
    TcpStream::connect(address)
        .await
        .expect("pre-bound legacy endpoint must accept connections");

    control.set_status(StatusType::Closing);
    handle
        .await
        .expect("supervisor task must join")
        .expect("graceful close must succeed");

    assert_eq!(service_config.close_count(), 1);
    assert!(TcpStream::connect(address).await.is_err());
}

#[tokio::test]
async fn legacy_prebound_listener_when_config_address_differs_still_serves_bound_socket() {
    let listener =
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("legacy loopback listener must bind");
    let address = listener
        .local_addr()
        .expect("bound listener has an address");
    let service_config = LifecycleTestServiceConfig::default();
    let control = service_config.subscriber();

    let (handle, _) = Indexer::<LifecycleTestService>::launch_inner_with_listeners(
        service_config,
        ZainodConfig::default(),
        listener,
        None,
        None,
    )
    .await
    .expect("pre-bound listener must override the transport bind path");

    TcpStream::connect(address)
        .await
        .expect("the handed-off listener must accept connections");
    control.set_status(StatusType::Closing);
    handle
        .await
        .expect("supervisor task must join")
        .expect("graceful close must succeed");
}
