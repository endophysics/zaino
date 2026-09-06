use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration as StdDuration,
};

use futures::Stream;
use tonic::Code;
use zaino_proto::proto::{
    indexed_tip::SubscribeIndexedTipsRequest,
    privacy_profile::{EndpointProfile, GetPrivacyProfileRequest},
    service::{
        Address, AddressList, ChainSpec, Duration, Empty, GetMempoolTxRequest, RawTransaction,
        TxFilter,
    },
};
use zaino_serve::rpc::profile::PrivacyMethodPolicy;

use super::support::{assert_no_application_metadata, TransportFixture};

#[tokio::test]
async fn both_profiles_when_served_over_tcp_retain_capability_common_rpc_and_metadata_privacy() {
    let fixture = TransportFixture::launch(PrivacyMethodPolicy::default()).await;

    for (address, expected_profile) in [
        (fixture.legacy_address, EndpointProfile::Legacy),
        (fixture.privacy_address, EndpointProfile::Privacy),
    ] {
        let mut capability = TransportFixture::capability_client(address).await;
        for _ in 0..2 {
            let response = capability
                .get_privacy_profile(GetPrivacyProfileRequest {})
                .await
                .expect("capability must be reachable over transport");
            assert_no_application_metadata(response.metadata());
            let body = response.into_inner();
            assert_eq!(body.endpoint_profile, expected_profile as i32);
            assert_eq!(body.methods.len(), 20);
        }
        let response = TransportFixture::compact_client(address)
            .await
            .get_latest_block(ChainSpec::default())
            .await
            .expect("common RPC must succeed over transport");
        assert_no_application_metadata(response.metadata());
    }

    fixture.close().await;
}

#[tokio::test]
async fn indexed_tip_extension_when_served_is_legacy_only() {
    let fixture = TransportFixture::launch(PrivacyMethodPolicy::default()).await;

    let mut legacy = TransportFixture::indexed_tip_client(fixture.legacy_address).await;
    let mut stream = legacy
        .subscribe_indexed_tips(SubscribeIndexedTipsRequest {})
        .await
        .expect("legacy listener must expose indexed tips")
        .into_inner();
    let initial = stream
        .message()
        .await
        .expect("initial tip must decode")
        .expect("initial tip must be present");
    let privacy = TransportFixture::indexed_tip_client(fixture.privacy_address)
        .await
        .subscribe_indexed_tips(SubscribeIndexedTipsRequest {})
        .await
        .expect_err("privacy listener must not expose indexed tips");

    assert_eq!((initial.height, initial.hash), (0, vec![0; 32]));
    assert_eq!(privacy.code(), Code::Unimplemented);
    fixture.close().await;
}

#[tokio::test]
async fn privacy_policy_when_exercised_over_tcp_is_fail_closed_and_flags_are_independent() {
    for (transaction_reads, transparent_reads) in [(false, false), (true, false), (false, true)] {
        let fixture = TransportFixture::launch(PrivacyMethodPolicy::new(
            transaction_reads,
            transparent_reads,
        ))
        .await;
        let mut client = TransportFixture::compact_client(fixture.privacy_address).await;

        let before = fixture.accesses();
        let transaction = client.get_transaction(TxFilter::default()).await;
        assert_no_application_metadata(
            transaction
                .as_ref()
                .expect_err("probe never returns a transaction")
                .metadata(),
        );
        assert_eq!(
            transaction
                .expect_err("probe never returns a transaction")
                .code(),
            if transaction_reads {
                Code::Unavailable
            } else {
                Code::PermissionDenied
            }
        );
        assert_eq!(fixture.accesses() - before, usize::from(transaction_reads));

        let before = fixture.accesses();
        let transparent = client.get_taddress_balance(AddressList::default()).await;
        assert_no_application_metadata(
            transparent
                .as_ref()
                .expect_err("probe never returns a balance")
                .metadata(),
        );
        assert_eq!(
            transparent
                .expect_err("probe never returns a balance")
                .code(),
            if transparent_reads {
                Code::Unavailable
            } else {
                Code::PermissionDenied
            }
        );
        assert_eq!(fixture.accesses() - before, usize::from(transparent_reads));
        fixture.close().await;
    }
}

#[tokio::test]
async fn immutable_denials_and_legacy_submission_when_exercised_over_tcp_retain_compatibility() {
    let fixture = TransportFixture::launch(PrivacyMethodPolicy::new(true, true)).await;
    let mut privacy = TransportFixture::compact_client(fixture.privacy_address).await;
    let before = fixture.accesses();

    let statuses = [
        privacy
            .send_transaction(RawTransaction::default())
            .await
            .expect_err("privacy must deny submission"),
        privacy
            .get_mempool_tx(GetMempoolTxRequest::default())
            .await
            .expect_err("privacy must deny mempool personalization"),
        privacy
            .get_mempool_stream(Empty {})
            .await
            .expect_err("privacy must deny mempool streaming"),
        privacy
            .ping(Duration::default())
            .await
            .expect_err("privacy must deny debug methods"),
    ];
    assert!(statuses
        .iter()
        .all(|status| status.code() == Code::PermissionDenied));
    for status in &statuses {
        assert_no_application_metadata(status.metadata());
    }
    assert_eq!(fixture.accesses(), before);

    let legacy_status = TransportFixture::compact_client(fixture.legacy_address)
        .await
        .send_transaction(RawTransaction::default())
        .await
        .expect_err("probe reports normal handler failure");
    assert_eq!(legacy_status.code(), Code::Unavailable);
    assert_no_application_metadata(legacy_status.metadata());
    assert_eq!(fixture.accesses(), before + 1);
    fixture.close().await;
}

struct PendingAddresses;

impl Stream for PendingAddresses {
    type Item = Address;

    fn poll_next(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Pending
    }
}

#[tokio::test]
async fn denied_client_stream_when_sent_over_tcp_is_not_consumed() {
    let fixture = TransportFixture::launch(PrivacyMethodPolicy::default()).await;

    let result = tokio::time::timeout(
        StdDuration::from_secs(2),
        TransportFixture::compact_client(fixture.privacy_address)
            .await
            .get_taddress_balance_stream(PendingAddresses),
    )
    .await
    .expect("privacy denial must not wait for client stream data")
    .expect_err("privacy must deny transparent client streams");

    assert_eq!(result.code(), Code::PermissionDenied);
    assert_no_application_metadata(result.metadata());
    assert_eq!(fixture.accesses(), 0);
    fixture.close().await;
}
