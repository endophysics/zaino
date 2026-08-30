use std::net::{Ipv4Addr, SocketAddr, TcpListener};

use zaino_proto::proto::{
    privacy_profile::{
        privacy_profile_service_client::PrivacyProfileServiceClient, GetPrivacyProfileRequest,
    },
    service::{compact_tx_streamer_client::CompactTxStreamerClient, ChainSpec},
};
use zaino_serve::server::config::GrpcServerConfig;

use crate::config::PrivacyGrpcSettings;

mod launch;
mod status;

fn listener() -> (TcpListener, SocketAddr) {
    let listener =
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("loopback listener must bind");
    let address = listener
        .local_addr()
        .expect("bound listener has an address");
    (listener, address)
}

fn privacy_settings(address: SocketAddr) -> PrivacyGrpcSettings {
    PrivacyGrpcSettings {
        server: GrpcServerConfig {
            listen_address: address,
            tls: None,
        },
        allow_transaction_specific_reads: true,
        allow_transparent_address_reads: false,
        metrics_window_seconds: 73,
    }
}

async fn capability(
    address: SocketAddr,
) -> zaino_proto::proto::privacy_profile::GetPrivacyProfileResponse {
    PrivacyProfileServiceClient::connect(format!("http://{address}"))
        .await
        .expect("capability client must connect")
        .get_privacy_profile(GetPrivacyProfileRequest {})
        .await
        .expect("capability RPC must succeed")
        .into_inner()
}

async fn common_rpc(address: SocketAddr) {
    CompactTxStreamerClient::connect(format!("http://{address}"))
        .await
        .expect("compact client must connect")
        .get_latest_block(ChainSpec::default())
        .await
        .expect("common RPC must succeed");
}
