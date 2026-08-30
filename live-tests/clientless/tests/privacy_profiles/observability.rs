use std::{
    collections::BTreeSet,
    io::{self, Write},
    sync::{Arc, Mutex},
    time::Duration,
};

use metrics_util::debugging::{DebugValue, DebuggingRecorder, Snapshotter};
use tracing_subscriber::fmt::MakeWriter;
use zaino_serve::metric_names::{
    PRIVACY_WINDOW_DURATION_SECONDS, PRIVACY_WINDOW_DURATION_SECONDS_SUM,
    PRIVACY_WINDOW_ERROR_COUNT, PRIVACY_WINDOW_REQUEST_COUNT, PRIVACY_WINDOW_START_SECONDS,
};

#[derive(Clone, Default)]
pub(super) struct CapturedWriter(Arc<Mutex<Vec<u8>>>);

pub(super) struct CapturedGuard(Arc<Mutex<Vec<u8>>>);

impl CapturedWriter {
    pub(super) fn checkpoint(&self) -> usize {
        self.0
            .lock()
            .expect("capture buffer mutex must remain healthy")
            .len()
    }

    pub(super) fn contents_since(&self, checkpoint: usize) -> String {
        let captured = self
            .0
            .lock()
            .expect("capture buffer mutex must remain healthy");
        String::from_utf8(
            captured
                .get(checkpoint..)
                .expect("capture checkpoint must belong to this writer")
                .to_vec(),
        )
        .expect("tracing output must be UTF-8")
    }
}

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

pub(super) fn install() -> (CapturedWriter, Snapshotter) {
    let logs = CapturedWriter::default();
    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt()
            .without_time()
            .with_ansi(false)
            .with_writer(logs.clone())
            .finish(),
    )
    .expect("inspection test must own the tracing subscriber");
    let recorder = DebuggingRecorder::new();
    let metrics = recorder.snapshotter();
    metrics::set_global_recorder(recorder).expect("inspection test must own the metrics recorder");
    (logs, metrics)
}

pub(super) async fn assert_completed_privacy_window(metrics: Snapshotter) {
    let snapshot = tokio::time::timeout(Duration::from_secs(3), async {
        let mut interval = tokio::time::interval(Duration::from_millis(25));
        loop {
            interval.tick().await;
            let snapshot = metrics.snapshot().into_vec();
            if snapshot.iter().any(|(key, _, _, value)| {
                key.key().name() == PRIVACY_WINDOW_REQUEST_COUNT
                    && labels_match(key.key(), "GetBlock", "common_chain_data", "ok")
                    && matches!(value, DebugValue::Gauge(value) if value.into_inner() >= 1.0)
            }) {
                break snapshot;
            }
        }
    })
    .await
    .expect("privacy metrics must publish a completed window");

    let mut aggregate_series = 0;
    let mut window_series = 0;
    for (key, _, _, _) in &snapshot {
        let name = key.key().name();
        if [
            PRIVACY_WINDOW_REQUEST_COUNT,
            PRIVACY_WINDOW_ERROR_COUNT,
            PRIVACY_WINDOW_DURATION_SECONDS_SUM,
        ]
        .contains(&name)
        {
            aggregate_series += 1;
            let labels = key
                .key()
                .labels()
                .map(|label| label.key())
                .collect::<BTreeSet<_>>();
            assert_eq!(
                labels,
                BTreeSet::from(["endpoint_profile", "method", "outcome", "risk_class"])
            );
        } else if [
            PRIVACY_WINDOW_START_SECONDS,
            PRIVACY_WINDOW_DURATION_SECONDS,
        ]
        .contains(&name)
        {
            window_series += 1;
            let labels = key
                .key()
                .labels()
                .map(|label| label.key())
                .collect::<BTreeSet<_>>();
            assert_eq!(labels, BTreeSet::from(["endpoint_profile"]));
        }
    }
    assert_eq!(aggregate_series, 63 * 3);
    assert_eq!(window_series, 2);
}

pub(super) fn assert_private_request_window(captured: &str) {
    let request_events = captured
        .lines()
        .filter(|line| line.contains("zaino_serve::rpc::grpc"))
        .collect::<Vec<_>>();
    assert_eq!(
        request_events,
        Vec::<&str>::new(),
        "privacy request window must have an empty allowlist of zaino-serve request events"
    );
}

fn labels_match(key: &metrics::Key, method: &str, risk_class: &str, outcome: &str) -> bool {
    key.labels()
        .any(|label| label.key() == "method" && label.value() == method)
        && key
            .labels()
            .any(|label| label.key() == "risk_class" && label.value() == risk_class)
        && key
            .labels()
            .any(|label| label.key() == "outcome" && label.value() == outcome)
}
