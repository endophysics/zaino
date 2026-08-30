use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
};

use tonic::Request;
use tracing_subscriber::fmt::MakeWriter;
use zaino_proto::proto::service::{
    compact_tx_streamer_server::CompactTxStreamer, BlockId, ChainSpec, RawTransaction,
};
use zaino_state::IndexerSubscriber;

use crate::rpc::{
    profile::{metrics::PrivacyWindowTestHarness, EndpointContext, PrivacyMethodPolicy},
    test_support::TestIndexer,
    GrpcClient,
};

#[derive(Clone, Default)]
struct CapturedWriter(Arc<Mutex<Vec<u8>>>);

struct CapturedGuard(Arc<Mutex<Vec<u8>>>);

impl Write for CapturedGuard {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .expect("capture buffer mutex must remain healthy")
            .extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'writer> MakeWriter<'writer> for CapturedWriter {
    type Writer = CapturedGuard;

    fn make_writer(&'writer self) -> Self::Writer {
        CapturedGuard(Arc::clone(&self.0))
    }
}

#[test]
fn legacy_handler_when_called_emits_existing_method_event() {
    let writer = CapturedWriter::default();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(writer.clone())
        .finish();
    let client = GrpcClient::new(
        IndexerSubscriber::new(TestIndexer::default()),
        EndpointContext::legacy(),
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime must build");

    tracing::subscriber::with_default(subscriber, || {
        let _result = runtime.block_on(CompactTxStreamer::send_transaction(
            &client,
            Request::new(RawTransaction::default()),
        ));
    });

    let captured = String::from_utf8(
        writer
            .0
            .lock()
            .expect("capture buffer mutex must remain healthy")
            .clone(),
    )
    .expect("tracing output must be UTF-8");
    assert!(captured.contains("[TEST] received call"));
    assert!(captured.contains("method=\"send_transaction\""));
}

#[test]
fn privacy_handlers_when_allowed_denied_or_error_emit_no_method_event() {
    let writer = CapturedWriter::default();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(writer.clone())
        .finish();
    let harness = PrivacyWindowTestHarness::new(10_000, std::time::Duration::from_secs(10));
    let client = GrpcClient::new(
        IndexerSubscriber::new(TestIndexer::default()),
        EndpointContext::privacy(PrivacyMethodPolicy::default(), harness.recorder()),
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime must build");

    tracing::subscriber::with_default(subscriber, || {
        let _allowed = runtime.block_on(CompactTxStreamer::get_latest_block(
            &client,
            Request::new(ChainSpec::default()),
        ));
        let _error = runtime.block_on(CompactTxStreamer::get_block(
            &client,
            Request::new(BlockId::default()),
        ));
        let _denied = runtime.block_on(CompactTxStreamer::send_transaction(
            &client,
            Request::new(RawTransaction::default()),
        ));
    });

    let captured = String::from_utf8(
        writer
            .0
            .lock()
            .expect("capture buffer mutex must remain healthy")
            .clone(),
    )
    .expect("tracing output must be UTF-8");
    assert!(!captured.contains("[TEST] received call"));
    harness.close();
}
