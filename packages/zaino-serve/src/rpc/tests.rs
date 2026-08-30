use std::{
    io,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::Poll,
};

use futures::stream::poll_fn;
use tonic::{
    codec::{DecodeBuf, Decoder},
    service::AxumBody,
    Code, Request, Status, Streaming,
};
use tower::ServiceExt;
use zaino_proto::proto::service::{
    compact_tx_streamer_server::CompactTxStreamer, Address, RawTransaction,
};
use zaino_state::IndexerSubscriber;

use super::{
    grpc_routes, grpc_routes_with_context,
    profile::{
        metrics::PrivacyWindowTestHarness, EndpointContext, GrpcMethod, PrivacyMethodPolicy,
    },
    test_support::TestIndexer,
    GrpcClient,
};

#[test]
fn legacy_route_construction_when_given_a_subscriber_does_not_access_it() {
    let probe = TestIndexer::default();
    let subscriber = IndexerSubscriber::new(probe.clone());

    let routes = grpc_routes(subscriber);

    assert_eq!(probe.accesses.load(Ordering::SeqCst), 0);
    drop(routes);
}

#[test]
fn legacy_context_when_checked_retains_all_twenty_methods() {
    let context = EndpointContext::legacy();

    let enabled = GrpcMethod::ALL
        .into_iter()
        .filter(|method| context.allows(*method))
        .count();

    assert_eq!(enabled, 20);
}

#[tokio::test]
async fn legacy_submission_handler_when_called_reaches_the_subscriber() {
    let probe = TestIndexer::default();
    let client = GrpcClient::new(
        IndexerSubscriber::new(probe.clone()),
        EndpointContext::legacy(),
    );

    let result =
        CompactTxStreamer::send_transaction(&client, Request::new(RawTransaction::default())).await;

    let status = result.expect_err("the probe subscriber always returns unavailable");
    assert_eq!(status.code(), Code::Unavailable);
    assert_eq!(probe.accesses.load(Ordering::SeqCst), 1);
}

#[test]
fn profile_route_construction_when_given_privacy_context_does_not_access_subscriber() {
    let probe = TestIndexer::default();
    let subscriber = IndexerSubscriber::new(probe.clone());
    let harness = PrivacyWindowTestHarness::new(12_700, std::time::Duration::from_secs(10));
    let context = EndpointContext::privacy(PrivacyMethodPolicy::default(), harness.recorder());

    let routes = grpc_routes_with_context(subscriber, context);

    assert_eq!(probe.accesses.load(Ordering::SeqCst), 0);
    drop(routes);
    harness.close();
}

async fn capability_route_is_registered(routes: tonic::service::Routes) {
    let request = tonic::codegen::http::Request::builder()
        .uri("/zaino.privacy.v1.PrivacyProfileService/GetPrivacyProfile")
        .header("content-type", "application/grpc")
        .body(AxumBody::from(tonic::codegen::Bytes::from_static(&[
            0, 0, 0, 0, 0,
        ])))
        .expect("the static capability request must build");

    let response = routes
        .oneshot(request)
        .await
        .expect("tonic routes are infallible");

    assert_ne!(
        response.status(),
        tonic::codegen::http::StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn legacy_routes_when_called_expose_capability_service() {
    let subscriber = IndexerSubscriber::new(TestIndexer::default());

    capability_route_is_registered(grpc_routes(subscriber)).await;
}

#[tokio::test]
async fn privacy_routes_when_called_expose_capability_service() {
    let subscriber = IndexerSubscriber::new(TestIndexer::default());
    let harness = PrivacyWindowTestHarness::new(12_800, std::time::Duration::from_secs(10));
    let context = EndpointContext::privacy(PrivacyMethodPolicy::default(), harness.recorder());

    capability_route_is_registered(grpc_routes_with_context(subscriber, context)).await;
    harness.close();
}

#[tokio::test]
async fn privacy_submission_handler_when_denied_does_not_access_subscriber() {
    let probe = TestIndexer::default();
    let harness = PrivacyWindowTestHarness::new(12_900, std::time::Duration::from_secs(10));
    let client = GrpcClient::new(
        IndexerSubscriber::new(probe.clone()),
        EndpointContext::privacy(PrivacyMethodPolicy::default(), harness.recorder()),
    );

    let result =
        CompactTxStreamer::send_transaction(&client, Request::new(RawTransaction::default())).await;

    let status = result.expect_err("privacy must deny transaction submission");
    assert_eq!(status.code(), Code::PermissionDenied);
    assert_eq!(probe.accesses.load(Ordering::SeqCst), 0);
    harness.close();
}

struct AddressDecoder;

impl Decoder for AddressDecoder {
    type Item = Address;
    type Error = Status;

    fn decode(&mut self, _source: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        Ok(None)
    }
}

#[tokio::test]
async fn privacy_balance_stream_when_denied_is_not_consumed_or_forwarded() {
    let probe = TestIndexer::default();
    let harness = PrivacyWindowTestHarness::new(13_000, std::time::Duration::from_secs(10));
    let client = GrpcClient::new(
        IndexerSubscriber::new(probe.clone()),
        EndpointContext::privacy(PrivacyMethodPolicy::default(), harness.recorder()),
    );
    let polls = Arc::new(AtomicUsize::new(0));
    let tracked_polls = polls.clone();
    let body_stream = poll_fn(move |_| {
        tracked_polls.fetch_add(1, Ordering::SeqCst);
        Poll::<Option<Result<tonic::codegen::Bytes, io::Error>>>::Pending
    });
    let request_stream = Streaming::new_request(
        AddressDecoder,
        AxumBody::from_stream(body_stream),
        None,
        None,
    );

    let result =
        CompactTxStreamer::get_taddress_balance_stream(&client, Request::new(request_stream)).await;
    tokio::task::yield_now().await;

    let status = result.expect_err("privacy must deny transparent balance streams by default");
    assert_eq!(status.code(), Code::PermissionDenied);
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    assert_eq!(probe.accesses.load(Ordering::SeqCst), 0);
    harness.advance(std::time::Duration::from_secs(10));
    harness.tick();
    let published = harness.published();
    assert_eq!(published.len(), 1);
    assert_eq!(published[0].total_request_count(), 1);
    assert!(!client
        .endpoint_context()
        .allows(GrpcMethod::GetTaddressBalanceStream));
    harness.close();
}
