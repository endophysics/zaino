use std::time::Duration;

use super::metrics::{
    ObservedMethod, PrivacyOutcome, PrivacyWindowTestHarness, STATIC_SERIES_COUNT,
};
use super::GrpcMethod;

const WINDOW: Duration = Duration::from_secs(10);

/// RFC 5737 TEST-NET-3 address used as a documentation-only fixture.
const RFC_5737_TEST_NET_3_ADDRESS: &str = "203.0.113.4";

fn aggregate(
    harness: &PrivacyWindowTestHarness,
    window: usize,
    method: ObservedMethod,
    outcome: PrivacyOutcome,
) -> (u64, u64, f64) {
    harness.published()[window].aggregate(method, outcome)
}

#[test]
fn exact_boundary_when_recorded_rolls_into_next_window() {
    let harness = PrivacyWindowTestHarness::new(1_000, WINDOW);
    let recorder = harness.recorder();
    recorder.record(
        ObservedMethod::Grpc(GrpcMethod::GetLatestBlock),
        PrivacyOutcome::Ok,
        Duration::from_secs(2),
    );
    harness.advance(WINDOW);
    recorder.record(
        ObservedMethod::Grpc(GrpcMethod::GetLatestBlock),
        PrivacyOutcome::Error,
        Duration::from_secs(3),
    );

    assert_eq!(harness.published().len(), 1);
    assert_eq!(
        aggregate(
            &harness,
            0,
            ObservedMethod::Grpc(GrpcMethod::GetLatestBlock),
            PrivacyOutcome::Ok
        ),
        (1, 0, 2.0)
    );
    assert_eq!(
        aggregate(
            &harness,
            0,
            ObservedMethod::Grpc(GrpcMethod::GetLatestBlock),
            PrivacyOutcome::Error
        ),
        (0, 0, 0.0)
    );
    harness.advance(WINDOW);
    harness.tick();
    assert_eq!(
        aggregate(
            &harness,
            1,
            ObservedMethod::Grpc(GrpcMethod::GetLatestBlock),
            PrivacyOutcome::Error
        ),
        (1, 1, 3.0)
    );
}

#[test]
fn consecutive_ticks_when_windows_complete_publish_each_interval() {
    let harness = PrivacyWindowTestHarness::new(2_000, WINDOW);

    harness.advance(WINDOW);
    harness.tick();
    harness.advance(WINDOW);
    harness.tick();

    let published = harness.published();
    assert_eq!(published.len(), 2);
    assert_eq!(published[0].start_unix_seconds(), 2_000);
    assert_eq!(published[1].start_unix_seconds(), 2_010);
}

#[test]
fn idle_window_when_completed_publishes_every_static_zero_tuple() {
    let harness = PrivacyWindowTestHarness::new(3_000, WINDOW);

    harness.advance(WINDOW);
    harness.tick();

    let published = harness.published();
    assert_eq!(published[0].series_count(), STATIC_SERIES_COUNT);
    assert!(published[0].all_zero());
}

#[test]
fn delayed_tick_when_multiple_windows_elapsed_publishes_every_missed_interval() {
    let harness = PrivacyWindowTestHarness::new(4_000, WINDOW);

    harness.advance(WINDOW * 3);
    harness.tick();

    let starts: Vec<_> = harness
        .published()
        .iter()
        .map(|window| window.start_unix_seconds())
        .collect();
    assert_eq!(starts, [4_000, 4_010, 4_020]);
}

#[test]
fn coarse_outcomes_when_recorded_have_exact_request_and_error_counts() {
    let harness = PrivacyWindowTestHarness::new(5_000, WINDOW);
    let recorder = harness.recorder();
    let method = ObservedMethod::Grpc(GrpcMethod::SendTransaction);

    recorder.record(method, PrivacyOutcome::Ok, Duration::from_secs(1));
    recorder.record(method, PrivacyOutcome::Denied, Duration::from_secs(2));
    recorder.record(method, PrivacyOutcome::Error, Duration::from_secs(3));
    harness.advance(WINDOW);
    harness.tick();

    assert_eq!(
        aggregate(&harness, 0, method, PrivacyOutcome::Ok),
        (1, 0, 1.0)
    );
    assert_eq!(
        aggregate(&harness, 0, method, PrivacyOutcome::Denied),
        (1, 0, 2.0)
    );
    assert_eq!(
        aggregate(&harness, 0, method, PrivacyOutcome::Error),
        (1, 1, 3.0)
    );
}

#[test]
fn close_when_window_is_partial_discards_it_and_ignores_future_records() {
    let harness = PrivacyWindowTestHarness::new(6_000, WINDOW);
    let recorder = harness.recorder();
    let method = ObservedMethod::Grpc(GrpcMethod::GetBlock);
    recorder.record(method, PrivacyOutcome::Ok, Duration::from_secs(1));

    harness.close();
    recorder.record(method, PrivacyOutcome::Error, Duration::from_secs(1));
    harness.advance(WINDOW);
    harness.tick();

    assert!(harness.published().is_empty());
}

#[test]
fn boundary_record_and_close_when_racing_never_publish_partial_or_duplicate() {
    for iteration in 0..64 {
        let harness = std::sync::Arc::new(PrivacyWindowTestHarness::new(7_000, WINDOW));
        let recorder = harness.recorder();
        let method = ObservedMethod::Grpc(GrpcMethod::GetBlock);
        harness.advance(WINDOW);
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
        let close_harness = std::sync::Arc::clone(&harness);
        let close_barrier = std::sync::Arc::clone(&barrier);
        let close_thread = std::thread::spawn(move || {
            close_barrier.wait();
            close_harness.close();
        });
        let record_barrier = std::sync::Arc::clone(&barrier);
        let record_thread = std::thread::spawn(move || {
            record_barrier.wait();
            recorder.record(method, PrivacyOutcome::Ok, Duration::ZERO);
        });

        barrier.wait();
        close_thread.join().expect("close thread must complete");
        record_thread.join().expect("record thread must complete");

        let published = harness.published();
        assert!(published.len() <= 1, "iteration {iteration}");
        assert!(published.iter().all(|window| window.duration() == WINDOW));
        if let Some(window) = published.first() {
            assert!(window.total_request_count() <= 1);
        }
    }
}

#[test]
fn static_registry_when_enumerated_has_twenty_grpc_methods_and_capability() {
    assert_eq!(ObservedMethod::ALL.len(), 21);
    assert_eq!(PrivacyOutcome::ALL.len(), 3);
    assert_eq!(STATIC_SERIES_COUNT, 63);
    assert!(ObservedMethod::ALL.contains(&ObservedMethod::PrivacyProfileService));
}

#[test]
fn static_labels_when_enumerated_are_canonical_and_identity_free() {
    for method in ObservedMethod::ALL {
        let labels = method.static_labels();
        assert_eq!(labels.endpoint_profile, "privacy");
        assert!(!labels.method.is_empty());
        assert!(!labels.risk_class.is_empty());
        for forbidden in [
            RFC_5737_TEST_NET_3_ADDRESS,
            "tmTestWalletAddress",
            "request-identity-42",
            "stream-identity-42",
            "wallet-user-agent",
            "secret-cookie-value",
        ] {
            assert!(!labels.method.contains(forbidden));
            assert!(!labels.risk_class.contains(forbidden));
        }
    }
}
