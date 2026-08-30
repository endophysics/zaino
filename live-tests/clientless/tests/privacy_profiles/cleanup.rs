use std::{net::SocketAddr, time::Duration};

use tokio::net::TcpStream;
use zaino_testutils::{Rpc, TestManager};
use zcash_local_net::validator::zebrad::Zebrad;

pub(super) async fn interrupt_after_ready(
    manager: TestManager<Zebrad, Rpc>,
    legacy_address: SocketAddr,
    privacy_address: SocketAddr,
) {
    let validator_address = manager.full_node_rpc_listen_address;
    assert!(
        manager
            .zaino_handle
            .as_ref()
            .is_some_and(|handle| !handle.is_finished()),
        "Zaino supervisor must be live before deterministic interruption"
    );
    assert_accepts(validator_address).await;
    assert_accepts(legacy_address).await;
    assert_accepts(privacy_address).await;
    println!(
        "PROFILE_INTERRUPT_READY validator=live zaino_supervisor=live legacy=live privacy=live"
    );

    manager.interrupt_zaino_and_close_for_test().await;
    assert_refuses(legacy_address).await;
    assert_refuses(privacy_address).await;
    println!(
        "PROFILE_INTERRUPT_ZAINO cancellation=observed cleanup=closed legacy=refused privacy=refused"
    );
    assert_refuses(validator_address).await;
    println!("PROFILE_INTERRUPT_RESOURCES validator=gone zaino=gone");
    panic!("PROFILE_INTERRUPT_EXPECTED_FAILURE");
}

async fn assert_accepts(address: SocketAddr) {
    let connection = tokio::time::timeout(Duration::from_secs(2), TcpStream::connect(address))
        .await
        .expect("live-resource connection probe must complete")
        .expect("resource must accept connections before interruption");
    drop(connection);
}

async fn assert_refuses(address: SocketAddr) {
    let result = tokio::time::timeout(Duration::from_secs(2), TcpStream::connect(address))
        .await
        .expect("closed-resource connection probe must complete");
    assert!(
        result.is_err(),
        "resource at {address} still accepted connections after cleanup"
    );
}
