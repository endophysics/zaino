use tonic::{Code, Request};
use zaino_proto::proto::{
    privacy_profile::{EndpointProfile, LoggingMode},
    service::{
        compact_tx_streamer_client::CompactTxStreamerClient, AddressList, BlockId, RawTransaction,
        TxFilter,
    },
};
use zaino_testutils::{PrivacyEndpointConfig, Rpc, TestManager, ValidatorKind};
use zcash_local_net::validator::zebrad::Zebrad;

#[path = "privacy_profiles/cleanup.rs"]
mod cleanup;
#[path = "privacy_profiles/observability.rs"]
mod observability;
#[path = "privacy_profiles/probes.rs"]
mod probes;

use cleanup::interrupt_after_ready;
use probes::{assert_identity_free, capability, PRIVACY_POLICY};

const INTERRUPT_AFTER_READY_ENV: &str = "ZAINO_INSPECT_TEST_INTERRUPT_AFTER_READY";
const TXID_SENTINEL: [u8; 32] = [0xA7; 32];
const ADDRESS_SENTINEL: &str = "tmVqEASZxBNKFTbmASZikGa5fPLkd68iJyx";
const PRIVACY_TEST_METADATA_SENTINEL: &str = "privacy-test-metadata-sentinel";

/// multi_thread required: the test manager spawns a validator and dual-endpoint indexer.
#[tokio::test(flavor = "multi_thread")]
async fn inspect_zaino_profiles() {
    let (logs, metrics) = observability::install();
    let mut manager = TestManager::<Zebrad, Rpc>::launch_with_privacy(
        &ValidatorKind::Zebrad,
        PrivacyEndpointConfig {
            allow_transaction_specific_reads: false,
            allow_transparent_address_reads: false,
            metrics_window_seconds: 1,
        },
    )
    .await
    .expect("real validator and dual-profile Zaino must launch");
    metrics.snapshot();

    let legacy_address = manager
        .zaino_grpc_listen_address
        .expect("dual-profile launch must expose the legacy address");
    let privacy_address = manager
        .zaino_privacy_grpc_listen_address
        .expect("dual-profile launch must expose the privacy address");
    assert_ne!(legacy_address, privacy_address);
    let mut legacy = CompactTxStreamerClient::connect(format!("http://{legacy_address}"))
        .await
        .expect("legacy client must connect");
    let mut privacy = CompactTxStreamerClient::connect(format!("http://{privacy_address}"))
        .await
        .expect("privacy client must connect");

    if std::env::var_os(INTERRUPT_AFTER_READY_ENV).is_some() {
        drop(legacy);
        drop(privacy);
        return interrupt_after_ready(manager, legacy_address, privacy_address).await;
    }

    let privacy_log_checkpoint = logs.checkpoint();

    let mut query = Request::new(BlockId {
        height: 1,
        hash: Vec::new(),
    });
    query.metadata_mut().insert(
        "x-privacy-test-session-id",
        PRIVACY_TEST_METADATA_SENTINEL
            .parse()
            .expect("privacy test metadata sentinel must parse"),
    );
    let privacy_block = privacy
        .get_block(query)
        .await
        .expect("privacy compact-block query must succeed");
    assert_identity_free(privacy_block.metadata());

    let transaction = privacy
        .get_transaction(TxFilter {
            hash: TXID_SENTINEL.to_vec(),
            ..Default::default()
        })
        .await
        .expect_err("transaction-specific lookup must be disabled");
    let transparent = privacy
        .get_taddress_balance(AddressList {
            addresses: vec![ADDRESS_SENTINEL.to_owned()],
        })
        .await
        .expect_err("transparent lookup must be disabled");
    assert_eq!(transaction.code(), Code::PermissionDenied);
    assert_eq!(transparent.code(), Code::PermissionDenied);
    println!("PROFILE_SENSITIVE_DENIAL transaction=PermissionDenied transparent=PermissionDenied");

    let privacy_submission = privacy
        .send_transaction(RawTransaction::default())
        .await
        .expect_err("privacy submission must be denied");
    assert_eq!(privacy_submission.code(), Code::PermissionDenied);

    let privacy_capability = capability(privacy_address).await;
    let privacy_request_log = logs.contents_since(privacy_log_checkpoint);
    observability::assert_private_request_window(&privacy_request_log);

    let legacy_log_checkpoint = logs.checkpoint();
    let legacy_block = legacy
        .get_block(BlockId {
            height: 1,
            hash: Vec::new(),
        })
        .await
        .expect("legacy compact-block query must succeed");
    assert_eq!(legacy_block.into_inner(), privacy_block.into_inner());
    println!("PROFILE_COMMON_QUERY legacy=PASS privacy=PASS equal=PASS");

    let legacy_submission = legacy
        .send_transaction(RawTransaction::default())
        .await
        .expect_err("invalid transaction must reach normal legacy validation");
    assert_ne!(legacy_submission.code(), Code::PermissionDenied);
    println!(
        "PROFILE_SUBMISSION privacy=PermissionDenied legacy_handler={:?}",
        legacy_submission.code()
    );
    let legacy_capability = capability(legacy_address).await;
    let legacy_request_log = logs.contents_since(legacy_log_checkpoint);
    assert_eq!(
        legacy_capability.endpoint_profile,
        EndpointProfile::Legacy as i32
    );
    assert_eq!(legacy_capability.methods.len(), 20);
    assert!(legacy_capability
        .methods
        .iter()
        .all(|method| method.enabled));
    assert_eq!(
        privacy_capability.endpoint_profile,
        EndpointProfile::Privacy as i32
    );
    assert_eq!(
        privacy_capability.logging_mode,
        LoggingMode::PrivacyAggregateOnly as i32
    );
    assert_eq!(privacy_capability.metric_window_seconds, 1);
    assert_eq!(privacy_capability.methods.len(), PRIVACY_POLICY.len());
    for (actual, (name, risk, enabled)) in privacy_capability.methods.iter().zip(PRIVACY_POLICY) {
        assert_eq!(actual.name, name);
        assert_eq!(actual.risk_class, risk as i32);
        assert_eq!(actual.enabled, enabled);
    }
    println!("PROFILE_CAPABILITY methods=20 decisions=exact logging=privacy_aggregate_only window_seconds=1");

    let handler_events = legacy_request_log
        .lines()
        .filter(|line| line.contains("[TEST] received call"))
        .collect::<Vec<_>>();
    assert_eq!(
        handler_events
            .iter()
            .filter(|line| line.contains("method=\"get_block\""))
            .count(),
        1
    );
    assert_eq!(
        handler_events
            .iter()
            .filter(|line| line.contains("method=\"send_transaction\""))
            .count(),
        1
    );
    println!("PROFILE_LOGGING privacy_window=empty legacy_method_events=exact");

    observability::assert_completed_privacy_window(metrics).await;
    println!("PROFILE_METRICS completed_window=PASS dimensions=approved_only");

    manager.close().await;
    println!("PROFILE_CLEANUP zaino=closed validator=drop_guarded");
}
