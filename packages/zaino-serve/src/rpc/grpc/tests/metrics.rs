use tonic::Request;
use zaino_proto::proto::service::{
    compact_tx_streamer_server::CompactTxStreamer, BlockId, ChainSpec, RawTransaction,
};
use zaino_state::IndexerSubscriber;

use crate::rpc::{
    profile::{
        metrics::{ObservedMethod, PrivacyWindowTestHarness},
        EndpointContext, GrpcMethod, PrivacyMethodPolicy, PrivacyOutcome,
    },
    test_support::TestIndexer,
    GrpcClient,
};

#[test]
fn privacy_handlers_when_allowed_denied_or_error_record_each_outcome_once() {
    let harness = PrivacyWindowTestHarness::new(11_000, std::time::Duration::from_secs(10));
    let client = GrpcClient::new(
        IndexerSubscriber::new(TestIndexer::default()),
        EndpointContext::privacy(PrivacyMethodPolicy::default(), harness.recorder()),
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime must build");

    let allowed = runtime.block_on(CompactTxStreamer::get_latest_block(
        &client,
        Request::new(ChainSpec::default()),
    ));
    let error = runtime.block_on(CompactTxStreamer::get_block(
        &client,
        Request::new(BlockId::default()),
    ));
    let denied = runtime.block_on(CompactTxStreamer::send_transaction(
        &client,
        Request::new(RawTransaction::default()),
    ));
    harness.advance(std::time::Duration::from_secs(10));
    harness.tick();

    assert!(allowed.is_ok());
    assert_eq!(
        error.expect_err("test indexer returns unavailable").code(),
        tonic::Code::Unavailable
    );
    assert_eq!(
        denied.expect_err("privacy denies submission").code(),
        tonic::Code::PermissionDenied
    );
    let published = harness.published();
    assert_eq!(published.len(), 1);
    assert_eq!(
        published[0].request_count(
            ObservedMethod::Grpc(GrpcMethod::GetLatestBlock),
            PrivacyOutcome::Ok
        ),
        1
    );
    assert_eq!(
        published[0].request_count(
            ObservedMethod::Grpc(GrpcMethod::GetBlock),
            PrivacyOutcome::Error
        ),
        1
    );
    assert_eq!(
        published[0].request_count(
            ObservedMethod::Grpc(GrpcMethod::SendTransaction),
            PrivacyOutcome::Denied
        ),
        1
    );
    assert_eq!(published[0].total_request_count(), 3);
    harness.close();
}

#[test]
fn authorized_handler_returning_permission_denied_is_recorded_as_error() {
    let harness = PrivacyWindowTestHarness::new(11_100, std::time::Duration::from_secs(10));
    let probe = TestIndexer::default();
    probe.set_failure(tonic::Status::permission_denied("subscriber decision"));
    let client = GrpcClient::new(
        IndexerSubscriber::new(probe),
        EndpointContext::privacy(PrivacyMethodPolicy::default(), harness.recorder()),
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime must build");

    let result = runtime.block_on(CompactTxStreamer::get_block(
        &client,
        Request::new(BlockId::default()),
    ));
    harness.advance(std::time::Duration::from_secs(10));
    harness.tick();

    assert_eq!(
        result
            .expect_err("subscriber denies admitted request")
            .code(),
        tonic::Code::PermissionDenied
    );
    let published = harness.published();
    assert_eq!(
        published[0].request_count(
            ObservedMethod::Grpc(GrpcMethod::GetBlock),
            PrivacyOutcome::Error
        ),
        1
    );
    assert_eq!(
        published[0].request_count(
            ObservedMethod::Grpc(GrpcMethod::GetBlock),
            PrivacyOutcome::Denied
        ),
        0
    );
    harness.close();
}

#[cfg(feature = "prometheus")]
#[test]
fn legacy_handler_when_called_retains_cumulative_counter_and_histogram_shape() {
    use metrics_util::debugging::{DebugValue, DebuggingRecorder};

    let recorder = DebuggingRecorder::new();
    let snapshotter = recorder.snapshotter();
    let client = GrpcClient::new(
        IndexerSubscriber::new(TestIndexer::default()),
        EndpointContext::legacy(),
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime must build");

    metrics::with_local_recorder(&recorder, || {
        let _result = runtime.block_on(CompactTxStreamer::send_transaction(
            &client,
            Request::new(RawTransaction::default()),
        ));
    });

    let snapshot = snapshotter.snapshot().into_vec();
    assert!(snapshot.iter().any(|(key, _, _, value)| {
        key.key().name() == crate::metric_names::GRPC_REQUESTS_TOTAL
            && key
                .key()
                .labels()
                .any(|label| label.key() == "method" && label.value() == "send_transaction")
            && matches!(value, DebugValue::Counter(1))
    }));
    assert!(snapshot.iter().any(|(key, _, _, value)| {
        key.key().name() == crate::metric_names::GRPC_REQUEST_DURATION_SECONDS
            && key
                .key()
                .labels()
                .any(|label| label.key() == "method" && label.value() == "send_transaction")
            && matches!(value, DebugValue::Histogram(values) if values.len() == 1)
    }));
    assert!(snapshot.iter().any(|(key, _, _, value)| {
        key.key().name() == crate::metric_names::GRPC_ERRORS_TOTAL
            && key
                .key()
                .labels()
                .any(|label| label.key() == "method" && label.value() == "send_transaction")
            && key.key().labels().any(|label| {
                label.key() == "code" && label.value() == "The service is currently unavailable"
            })
            && matches!(value, DebugValue::Counter(1))
    }));
}
