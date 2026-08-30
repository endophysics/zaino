//! Endpoint profiles and `CompactTxStreamer` method policy.

use std::{sync::Arc, time::Duration};

mod capability;
mod method;
pub(crate) mod metrics;

pub(crate) use capability::CapabilityService;
pub use capability::{CapabilityDocument, CapabilityError};
pub use method::{GrpcMethod, MethodRiskClass};
pub use metrics::{
    PrivacyMetricsError, PrivacyMetricsRecorder, PrivacyOutcome, PrivacyWindowMetrics,
};

/// The policy identity attached to one gRPC endpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndpointProfile {
    /// The fully compatible, allow-all endpoint.
    Legacy,
    /// The reduced-metadata endpoint.
    Privacy,
}

/// Request-observation behavior shared by handlers and capability output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestLoggingMode {
    /// Emit the current method-level event for every request.
    MethodLevelRequest,
    /// Emit no per-request event; report only aggregate windows.
    AggregateOnly,
}

impl RequestLoggingMode {
    /// Returns the stable machine-readable mode name.
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::MethodLevelRequest => "method_level_request",
            Self::AggregateOnly => "privacy_aggregate_only",
        }
    }

    pub(crate) const fn emits_method_events(self) -> bool {
        match self {
            Self::MethodLevelRequest => true,
            Self::AggregateOnly => false,
        }
    }
}

impl EndpointProfile {
    /// Returns the stable machine-readable profile name.
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::Legacy => "legacy",
            Self::Privacy => "privacy",
        }
    }
}

/// Configurable privacy v1 method decisions.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PrivacyMethodPolicy {
    allow_transaction_specific_reads: bool,
    allow_transparent_address_reads: bool,
}

impl PrivacyMethodPolicy {
    /// Creates a privacy v1 policy from its two sensitive-read flags.
    pub const fn new(
        allow_transaction_specific_reads: bool,
        allow_transparent_address_reads: bool,
    ) -> Self {
        Self {
            allow_transaction_specific_reads,
            allow_transparent_address_reads,
        }
    }
}

#[derive(Debug)]
enum EndpointState {
    Legacy,
    Privacy {
        policy: PrivacyMethodPolicy,
        metrics: PrivacyMetricsRecorder,
    },
}

pub(crate) enum EndpointObservability<'context> {
    Legacy,
    Privacy(&'context PrivacyMetricsRecorder),
}

/// Immutable method policy shared by one endpoint's routes and handlers.
#[derive(Debug)]
pub struct EndpointContext {
    state: EndpointState,
}

impl EndpointContext {
    /// Creates an allow-all legacy endpoint context.
    pub fn legacy() -> Arc<Self> {
        Arc::new(Self {
            state: EndpointState::Legacy,
        })
    }

    /// Creates a privacy endpoint context with the supplied sensitive-read policy.
    ///
    /// ```compile_fail
    /// use zaino_serve::rpc::profile::{EndpointContext, PrivacyMethodPolicy};
    ///
    /// let _context = EndpointContext::privacy(PrivacyMethodPolicy::default());
    /// ```
    pub fn privacy(policy: PrivacyMethodPolicy, metrics: PrivacyMetricsRecorder) -> Arc<Self> {
        Arc::new(Self {
            state: EndpointState::Privacy { policy, metrics },
        })
    }

    /// Returns this endpoint's profile identity.
    pub const fn profile(&self) -> EndpointProfile {
        match self.state {
            EndpointState::Legacy => EndpointProfile::Legacy,
            EndpointState::Privacy { .. } => EndpointProfile::Privacy,
        }
    }

    /// Returns the request-observation mode enforced by handlers.
    pub const fn request_logging_mode(&self) -> RequestLoggingMode {
        match self.state {
            EndpointState::Legacy => RequestLoggingMode::MethodLevelRequest,
            EndpointState::Privacy { .. } => RequestLoggingMode::AggregateOnly,
        }
    }

    pub(crate) const fn observability(&self) -> EndpointObservability<'_> {
        match &self.state {
            EndpointState::Legacy => EndpointObservability::Legacy,
            EndpointState::Privacy { metrics, .. } => EndpointObservability::Privacy(metrics),
        }
    }

    pub(crate) fn metric_window(&self) -> Option<Duration> {
        match &self.state {
            EndpointState::Legacy => None,
            EndpointState::Privacy { metrics, .. } => Some(metrics.window()),
        }
    }

    /// Returns whether this endpoint admits a method.
    pub const fn allows(&self, method: GrpcMethod) -> bool {
        match self.state {
            EndpointState::Legacy => true,
            EndpointState::Privacy { policy, .. } => match method.risk_class() {
                MethodRiskClass::CommonChainData => true,
                MethodRiskClass::TransactionSpecificLookup => {
                    policy.allow_transaction_specific_reads
                }
                MethodRiskClass::TransparentAddressLookup => policy.allow_transparent_address_reads,
                MethodRiskClass::MempoolPersonalization
                | MethodRiskClass::TransactionSubmission
                | MethodRiskClass::AdministrationDebug => false,
            },
        }
    }
}

#[cfg(test)]
mod metrics_tests;
#[cfg(test)]
mod tests;
