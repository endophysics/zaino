use std::{
    net::{Ipv4Addr, SocketAddr, TcpListener},
    sync::atomic::Ordering,
    time::Duration,
};

use tonic::metadata::{KeyAndValueRef, MetadataMap};
use zaino_proto::proto::{
    privacy_profile::privacy_profile_service_client::PrivacyProfileServiceClient,
    service::compact_tx_streamer_client::CompactTxStreamerClient,
};
use zaino_serve::{
    rpc::{
        grpc_routes, grpc_routes_with_context,
        profile::{EndpointContext, PrivacyMethodPolicy, PrivacyWindowMetrics},
        test_support::TestIndexer,
    },
    server::{config::GrpcServerConfig, grpc::TonicServer},
};
use zaino_state::IndexerSubscriber;

pub(super) struct TransportFixture {
    pub(super) legacy_address: SocketAddr,
    pub(super) privacy_address: SocketAddr,
    pub(super) probe: TestIndexer,
    legacy: TonicServer,
    privacy: TonicServer,
    privacy_metrics: PrivacyWindowMetrics,
}

impl TransportFixture {
    pub(super) async fn launch(policy: PrivacyMethodPolicy) -> Self {
        let probe = TestIndexer::default();
        let subscriber = IndexerSubscriber::new(probe.clone());
        let (legacy_listener, legacy_address) = listener();
        let (privacy_listener, privacy_address) = listener();
        let privacy_metrics = PrivacyWindowMetrics::new(Duration::from_secs(60))
            .expect("privacy metric owner must start");
        let privacy_context = EndpointContext::privacy(policy, privacy_metrics.recorder());
        let legacy = TonicServer::spawn_from_listener(
            grpc_routes(subscriber.clone()),
            config(legacy_address),
            legacy_listener,
        )
        .await
        .expect("legacy transport must start");
        let privacy = TonicServer::spawn_from_listener(
            grpc_routes_with_context(subscriber, privacy_context),
            config(privacy_address),
            privacy_listener,
        )
        .await
        .expect("privacy transport must start");
        Self {
            legacy_address,
            privacy_address,
            probe,
            legacy,
            privacy,
            privacy_metrics,
        }
    }

    pub(super) async fn compact_client(
        address: SocketAddr,
    ) -> CompactTxStreamerClient<tonic::transport::Channel> {
        CompactTxStreamerClient::connect(format!("http://{address}"))
            .await
            .expect("compact client must connect")
    }

    pub(super) async fn capability_client(
        address: SocketAddr,
    ) -> PrivacyProfileServiceClient<tonic::transport::Channel> {
        PrivacyProfileServiceClient::connect(format!("http://{address}"))
            .await
            .expect("capability client must connect")
    }

    pub(super) fn accesses(&self) -> usize {
        self.probe.accesses.load(Ordering::SeqCst)
    }

    pub(super) async fn close(mut self) {
        self.legacy.close().await;
        self.privacy.close().await;
        self.privacy_metrics.close().await;
    }
}

pub(super) fn assert_no_application_metadata(metadata: &MetadataMap) {
    for entry in metadata.iter() {
        match entry {
            KeyAndValueRef::Ascii(key, value) => {
                let value = value
                    .to_str()
                    .expect("allowed transport metadata must be ASCII");
                match key.as_str() {
                    "content-type" => assert_eq!(value, "application/grpc"),
                    "grpc-status" => assert_eq!(value, "0"),
                    "date" => assert_http_date(value),
                    name => panic!(
                        "application response metadata must be empty, including values: {name}={value}"
                    ),
                }
            }
            KeyAndValueRef::Binary(key, value) => panic!(
                "application response metadata must be empty, including values: {}={value:?}",
                key.as_str()
            ),
        }
    }
}

fn assert_http_date(value: &str) {
    assert_eq!(value.len(), 29, "transport date must use IMF-fixdate");
    assert!(value.ends_with(" GMT"), "transport date must use GMT");
    assert!(
        value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b", :".contains(&byte)),
        "transport date contained a non-date value: {value}"
    );
}

#[test]
#[should_panic(expected = "x-test-private-field=test-metadata-value-sentinel")]
fn application_response_metadata_when_present_is_rejected_with_test_value() {
    let mut metadata = MetadataMap::new();
    metadata.insert(
        "x-test-private-field",
        "test-metadata-value-sentinel"
            .parse()
            .expect("test sentinel metadata value must parse"),
    );

    assert_no_application_metadata(&metadata);
}

fn listener() -> (TcpListener, SocketAddr) {
    let listener =
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("pre-bound loopback listener must bind");
    let address = listener
        .local_addr()
        .expect("pre-bound listener must have an address");
    (listener, address)
}

fn config(listen_address: SocketAddr) -> GrpcServerConfig {
    GrpcServerConfig {
        listen_address,
        tls: None,
    }
}
