//! Bounded fixed-window privacy aggregates.

mod method;
mod runtime;
mod window;

pub(crate) use method::ObservedMethod;
pub use runtime::{PrivacyMetricsError, PrivacyMetricsRecorder, PrivacyWindowMetrics};

#[cfg(test)]
pub(crate) use runtime::PrivacyWindowTestHarness;
#[cfg(test)]
pub(crate) use window::STATIC_SERIES_COUNT;

/// A coarse, identity-free result for one privacy endpoint call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivacyOutcome {
    /// The admitted handler completed successfully.
    Ok,
    /// Endpoint policy rejected the call before handler execution.
    Denied,
    /// The admitted handler returned an error.
    Error,
}

impl PrivacyOutcome {
    pub(crate) const ALL: [Self; 3] = [Self::Ok, Self::Denied, Self::Error];

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Ok => 0,
            Self::Denied => 1,
            Self::Error => 2,
        }
    }

    #[cfg(feature = "prometheus")]
    pub(crate) const fn canonical_name(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Denied => "denied",
            Self::Error => "error",
        }
    }
}
