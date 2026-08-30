use futures::stream;
use zaino_proto::proto::service::{Address, BlockId, BlockRange};
use zaino_serve::rpc::profile::PrivacyMethodPolicy;

use super::support::{assert_no_application_metadata, TransportFixture};

/// Existing checksum-valid Zcash testnet transparent P2PKH fixture.
const TESTNET_TRANSPARENT_P2PKH_ADDRESS: &str = "tmVqEASZxBNKFTbmASZikGa5fPLkd68iJyx";

#[tokio::test]
async fn successful_streaming_categories_when_served_over_tcp_retain_metadata_privacy() {
    let fixture = TransportFixture::launch(PrivacyMethodPolicy::default()).await;

    let server_stream = TransportFixture::compact_client(fixture.privacy_address)
        .await
        .get_block_range(BlockRange {
            start: Some(BlockId::default()),
            end: Some(BlockId::default()),
            pool_types: Vec::new(),
        })
        .await
        .expect("common server stream must succeed over transport");
    assert_no_application_metadata(server_stream.metadata());

    let client_stream = TransportFixture::compact_client(fixture.legacy_address)
        .await
        .get_taddress_balance_stream(stream::iter([Address {
            address: TESTNET_TRANSPARENT_P2PKH_ADDRESS.to_owned(),
        }]))
        .await
        .expect("legacy client stream must succeed over transport");
    assert_no_application_metadata(client_stream.metadata());

    fixture.close().await;
}
