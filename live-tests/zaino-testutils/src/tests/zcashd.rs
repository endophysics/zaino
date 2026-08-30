use zcash_local_net::validator::{zcashd::Zcashd, Validator as _};

use super::{build_client, launch_minimal};
use crate::{Rpc, ValidatorKind, ZCASHD_CHAIN_CACHE_DIR};

#[tokio::test(flavor = "multi_thread")]
#[allow(deprecated)]
async fn basic() {
    let mut manager = launch_minimal::<Zcashd, Rpc>(&ValidatorKind::Zcashd, None, false).await;
    assert_eq!(2, manager.local_net.get_chain_height().await);
    manager.close().await;
}

#[tokio::test(flavor = "multi_thread")]
#[allow(deprecated)]
async fn generate_blocks() {
    let mut manager = launch_minimal::<Zcashd, Rpc>(&ValidatorKind::Zcashd, None, false).await;
    assert_eq!(2, manager.local_net.get_chain_height().await);
    manager
        .local_net
        .generate_blocks(1)
        .await
        .expect("validator must generate a block");
    assert_eq!(3, manager.local_net.get_chain_height().await);
    manager.close().await;
}

#[ignore = "chain cache needs development"]
#[tokio::test(flavor = "multi_thread")]
#[allow(deprecated)]
async fn with_chain() {
    let mut manager = launch_minimal::<Zcashd, Rpc>(
        &ValidatorKind::Zcashd,
        ZCASHD_CHAIN_CACHE_DIR.clone(),
        false,
    )
    .await;
    assert_eq!(10, manager.local_net.get_chain_height().await);
    manager.close().await;
}

#[tokio::test(flavor = "multi_thread")]
#[allow(deprecated)]
async fn zaino() {
    let mut manager = launch_minimal::<Zcashd, Rpc>(&ValidatorKind::Zcashd, None, true).await;
    let _grpc_client = build_client(manager.grpc_socket_to_uri())
        .await
        .expect("gRPC client must connect");
    manager.close().await;
}
