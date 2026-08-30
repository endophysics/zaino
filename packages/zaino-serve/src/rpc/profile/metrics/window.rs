use std::{sync::Arc, time::Duration};

use super::{method::ObservedMethod, PrivacyOutcome};

pub(crate) const STATIC_SERIES_COUNT: usize = ObservedMethod::ALL.len() * PrivacyOutcome::ALL.len();

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Aggregate {
    request_count: u64,
    error_count: u64,
    duration_seconds_sum: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PublishedWindow {
    start_unix_seconds: u64,
    duration: Duration,
    aggregates: [Aggregate; STATIC_SERIES_COUNT],
}

impl PublishedWindow {
    #[cfg(test)]
    pub(crate) const fn start_unix_seconds(&self) -> u64 {
        self.start_unix_seconds
    }

    #[cfg(test)]
    pub(crate) const fn duration(&self) -> Duration {
        self.duration
    }

    #[cfg(test)]
    pub(crate) const fn series_count(&self) -> usize {
        self.aggregates.len()
    }

    #[cfg(test)]
    pub(crate) fn all_zero(&self) -> bool {
        self.aggregates
            .iter()
            .all(|aggregate| *aggregate == Aggregate::default())
    }

    #[cfg(test)]
    pub(crate) fn aggregate(
        &self,
        method: ObservedMethod,
        outcome: PrivacyOutcome,
    ) -> (u64, u64, f64) {
        let aggregate = self.aggregates[series_index(method, outcome)];
        (
            aggregate.request_count,
            aggregate.error_count,
            aggregate.duration_seconds_sum,
        )
    }

    #[cfg(test)]
    pub(crate) fn request_count(&self, method: ObservedMethod, outcome: PrivacyOutcome) -> u64 {
        self.aggregates[series_index(method, outcome)].request_count
    }

    #[cfg(test)]
    pub(crate) fn total_request_count(&self) -> u64 {
        self.aggregates
            .iter()
            .map(|aggregate| aggregate.request_count)
            .sum()
    }
}

pub(crate) trait WindowPublisher: Send + Sync + 'static {
    fn publish(&self, window: PublishedWindow);
}

pub(crate) struct WindowState {
    duration: Duration,
    start_offset: Duration,
    start_unix_seconds: u64,
    aggregates: [Aggregate; STATIC_SERIES_COUNT],
    closed: bool,
}

impl WindowState {
    pub(crate) fn new(start_unix_seconds: u64, duration: Duration) -> Self {
        Self {
            duration,
            start_offset: Duration::ZERO,
            start_unix_seconds,
            aggregates: [Aggregate::default(); STATIC_SERIES_COUNT],
            closed: false,
        }
    }

    pub(crate) fn take_completed(&mut self, now: Duration) -> Vec<PublishedWindow> {
        let mut completed = Vec::new();
        while !self.closed && now >= self.start_offset + self.duration {
            completed.push(PublishedWindow {
                start_unix_seconds: self.start_unix_seconds,
                duration: self.duration,
                aggregates: std::mem::replace(
                    &mut self.aggregates,
                    [Aggregate::default(); STATIC_SERIES_COUNT],
                ),
            });
            self.start_offset += self.duration;
            self.start_unix_seconds += self.duration.as_secs();
        }
        completed
    }

    pub(crate) fn record(
        &mut self,
        method: ObservedMethod,
        outcome: PrivacyOutcome,
        duration: Duration,
    ) {
        if self.closed {
            return;
        }
        let aggregate = &mut self.aggregates[series_index(method, outcome)];
        aggregate.request_count += 1;
        aggregate.error_count += u64::from(outcome == PrivacyOutcome::Error);
        aggregate.duration_seconds_sum += duration.as_secs_f64();
    }

    pub(crate) fn close(&mut self) {
        self.closed = true;
        self.aggregates = [Aggregate::default(); STATIC_SERIES_COUNT];
    }
}

pub(crate) fn publish_all(publisher: &Arc<dyn WindowPublisher>, windows: Vec<PublishedWindow>) {
    for window in windows {
        publisher.publish(window);
    }
}

const fn series_index(method: ObservedMethod, outcome: PrivacyOutcome) -> usize {
    method.index() * PrivacyOutcome::ALL.len() + outcome.index()
}

pub(crate) struct MetricsPublisher;

impl WindowPublisher for MetricsPublisher {
    fn publish(&self, window: PublishedWindow) {
        #[cfg(feature = "prometheus")]
        publish_prometheus(&window);
        #[cfg(not(feature = "prometheus"))]
        let _ = window;
    }
}

#[cfg(feature = "prometheus")]
fn publish_prometheus(window: &PublishedWindow) {
    use crate::metric_names::*;

    metrics::gauge!(PRIVACY_WINDOW_START_SECONDS, "endpoint_profile" => "privacy")
        .set(Duration::from_secs(window.start_unix_seconds).as_secs_f64());
    metrics::gauge!(PRIVACY_WINDOW_DURATION_SECONDS, "endpoint_profile" => "privacy")
        .set(window.duration.as_secs_f64());
    for method in ObservedMethod::ALL {
        let labels = method.static_labels();
        for outcome in PrivacyOutcome::ALL {
            let aggregate = window.aggregates[series_index(method, outcome)];
            let labels = [
                ("endpoint_profile", labels.endpoint_profile),
                ("method", labels.method),
                ("risk_class", labels.risk_class),
                ("outcome", outcome.canonical_name()),
            ];
            metrics::gauge!(PRIVACY_WINDOW_REQUEST_COUNT, &labels)
                .set(Duration::from_secs(aggregate.request_count).as_secs_f64());
            metrics::gauge!(PRIVACY_WINDOW_ERROR_COUNT, &labels)
                .set(Duration::from_secs(aggregate.error_count).as_secs_f64());
            metrics::gauge!(PRIVACY_WINDOW_DURATION_SECONDS_SUM, &labels)
                .set(aggregate.duration_seconds_sum);
        }
    }
}
