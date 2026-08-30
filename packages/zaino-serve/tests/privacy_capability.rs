use std::{sync::Arc, time::Duration};

use time::{macros::datetime, OffsetDateTime};
use zaino_proto::proto::{privacy_profile::GetPrivacyProfileResponse, service::LightdInfo};
use zaino_serve::rpc::profile::{
    CapabilityDocument, EndpointContext, PrivacyMethodPolicy, PrivacyWindowMetrics,
};

#[path = "privacy_capability/response.rs"]
mod response;
#[path = "privacy_capability/schema.rs"]
mod schema;

const FIXED_TIME: OffsetDateTime = datetime!(2026-08-28 12:34:56 UTC);

struct PrivacyFixture {
    context: Arc<EndpointContext>,
    metrics: PrivacyWindowMetrics,
    runtime: tokio::runtime::Runtime,
}

impl PrivacyFixture {
    fn new(policy: PrivacyMethodPolicy) -> Self {
        Self::new_with_window(policy, Duration::from_secs(60))
    }

    fn new_with_window(policy: PrivacyMethodPolicy, window: Duration) -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("privacy fixture runtime must build");
        let metrics = {
            let _runtime_guard = runtime.enter();
            PrivacyWindowMetrics::new(window).expect("privacy fixture metrics must build")
        };
        let context = EndpointContext::privacy(policy, metrics.recorder());
        Self {
            context,
            metrics,
            runtime,
        }
    }

    fn context(&self) -> Arc<EndpointContext> {
        Arc::clone(&self.context)
    }
}

impl Drop for PrivacyFixture {
    fn drop(&mut self) {
        self.runtime.block_on(self.metrics.close());
    }
}

fn lightd_info() -> LightdInfo {
    LightdInfo {
        version: "0.6.0".to_owned(),
        vendor: "Zaino".to_owned(),
        chain_name: "main".to_owned(),
        git_commit: "abcdef0123456789".to_owned(),
        zcashd_build: "Zebra 3.0.0".to_owned(),
        zcashd_subversion: "/Zebra:3.0.0/".to_owned(),
        ..LightdInfo::default()
    }
}

fn document(context: Arc<EndpointContext>) -> CapabilityDocument {
    CapabilityDocument::new(context, lightd_info(), FIXED_TIME)
        .expect("the fixed capability fixture must be valid")
}

fn wire(context: Arc<EndpointContext>) -> GetPrivacyProfileResponse {
    document(context)
        .to_wire()
        .expect("the fixed capability fixture must render")
}

fn privacy_wire(policy: PrivacyMethodPolicy) -> GetPrivacyProfileResponse {
    let fixture = PrivacyFixture::new(policy);
    wire(fixture.context())
}
