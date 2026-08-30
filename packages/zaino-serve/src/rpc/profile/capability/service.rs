use std::sync::Arc;

use tonic::{Request, Response, Status};
use zaino_proto::proto::privacy_profile::{
    privacy_profile_service_server::PrivacyProfileService, GetPrivacyProfileRequest,
    GetPrivacyProfileResponse,
};
use zaino_state::{IndexerSubscriber, LightWalletIndexer, ZcashIndexer};

use super::{CapabilityDocument, EndpointContext};
use crate::rpc::profile::{metrics::ObservedMethod, EndpointObservability, PrivacyOutcome};

#[derive(Clone)]
pub(crate) struct CapabilityService<Indexer: ZcashIndexer + LightWalletIndexer> {
    subscriber: IndexerSubscriber<Indexer>,
    context: Arc<EndpointContext>,
}

impl<Indexer: ZcashIndexer + LightWalletIndexer> CapabilityService<Indexer> {
    pub(crate) const fn new(
        subscriber: IndexerSubscriber<Indexer>,
        context: Arc<EndpointContext>,
    ) -> Self {
        Self {
            subscriber,
            context,
        }
    }
}

#[tonic::async_trait]
impl<Indexer> PrivacyProfileService for CapabilityService<Indexer>
where
    Indexer: ZcashIndexer + LightWalletIndexer,
    Indexer::Error: Into<Status>,
{
    async fn get_privacy_profile(
        &self,
        _request: Request<GetPrivacyProfileRequest>,
    ) -> Result<Response<GetPrivacyProfileResponse>, Status> {
        let start = std::time::Instant::now();
        let result = async {
            let lightd_info = self
                .subscriber
                .inner_ref()
                .get_lightd_info()
                .await
                .map_err(Into::into)?;
            let document = CapabilityDocument::new(
                self.context.clone(),
                lightd_info,
                time::OffsetDateTime::now_utc(),
            )
            .map_err(|error| Status::failed_precondition(error.to_string()))?;
            let response = document
                .to_wire()
                .map_err(|error| Status::internal(error.to_string()))?;
            Ok(Response::new(response))
        }
        .await;
        match self.context.observability() {
            EndpointObservability::Legacy => {}
            EndpointObservability::Privacy(recorder) => {
                let outcome = match &result {
                    Ok(_) => PrivacyOutcome::Ok,
                    Err(_) => PrivacyOutcome::Error,
                };
                recorder.record(
                    ObservedMethod::PrivacyProfileService,
                    outcome,
                    start.elapsed(),
                );
            }
        }
        result
    }
}
