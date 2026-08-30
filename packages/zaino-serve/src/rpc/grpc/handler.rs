use std::time::Instant;

use tonic::Status;

use crate::rpc::profile::{
    metrics::ObservedMethod, EndpointContext, EndpointObservability, GrpcMethod,
    PrivacyMetricsRecorder, PrivacyOutcome,
};

pub(super) enum HandlerObservation {
    Privacy(PrivacyMetricsRecorder, ObservedMethod, Instant),
    #[cfg(feature = "prometheus")]
    Legacy(&'static str, Instant),
    #[cfg(not(feature = "prometheus"))]
    Legacy,
}

impl HandlerObservation {
    pub(super) fn begin(
        context: &EndpointContext,
        method: GrpcMethod,
        _legacy_name: &'static str,
    ) -> Result<Self, Status> {
        match context.observability() {
            EndpointObservability::Legacy => {
                method.authorize(context)?;
                #[cfg(feature = "prometheus")]
                return Ok(Self::Legacy(_legacy_name, Instant::now()));
                #[cfg(not(feature = "prometheus"))]
                return Ok(Self::Legacy);
            }
            EndpointObservability::Privacy(recorder) => {
                let start = Instant::now();
                if let Err(status) = method.authorize(context) {
                    recorder.record(
                        ObservedMethod::Grpc(method),
                        PrivacyOutcome::Denied,
                        start.elapsed(),
                    );
                    return Err(status);
                }
                Ok(Self::Privacy(
                    recorder.clone(),
                    ObservedMethod::Grpc(method),
                    start,
                ))
            }
        }
    }

    pub(super) fn finish<T>(&self, result: &Result<tonic::Response<T>, Status>) {
        match self {
            Self::Privacy(recorder, method, start) => {
                let outcome = match result {
                    Ok(_) => PrivacyOutcome::Ok,
                    Err(_) => PrivacyOutcome::Error,
                };
                recorder.record(*method, outcome, start.elapsed());
            }
            #[cfg(feature = "prometheus")]
            Self::Legacy(method, start) => record_legacy_grpc_metrics(method, *start, result),
            #[cfg(not(feature = "prometheus"))]
            Self::Legacy => {}
        }
    }
}

#[cfg(feature = "prometheus")]
fn record_legacy_grpc_metrics<T>(
    method: &'static str,
    start: Instant,
    result: &Result<tonic::Response<T>, Status>,
) {
    use crate::metric_names::*;
    metrics::counter!(GRPC_REQUESTS_TOTAL, "method" => method).increment(1);
    metrics::histogram!(GRPC_REQUEST_DURATION_SECONDS, "method" => method)
        .record(start.elapsed().as_secs_f64());
    if let Err(status) = result {
        metrics::counter!(
            GRPC_ERRORS_TOTAL,
            "method" => method,
            "code" => status.code().description(),
        )
        .increment(1);
    }
}

macro_rules! client_method_helper {
    ($self:ident $input:ident $method_name:ident) => {
        tonic::Response::new(
            $self
                .service_subscriber
                .inner_ref()
                .$method_name($input.into_inner())
                .await
                .map_err(Into::into)?,
        )
    };
    (streaming $self:ident $input:ident $method_name:ident) => {
        tonic::Response::new(Box::pin(
            $self
                .service_subscriber
                .inner_ref()
                .$method_name($input.into_inner())
                .await
                .map_err(Into::into)?,
        ))
    };
    (empty $self:ident $input:ident $method_name:ident) => {
        tonic::Response::new(
            $self
                .service_subscriber
                .inner_ref()
                .$method_name()
                .await
                .map_err(Into::into)?,
        )
    };
    (streamingempty $self:ident $input:ident $method_name:ident) => {
        tonic::Response::new(Box::pin(
            $self
                .service_subscriber
                .inner_ref()
                .$method_name()
                .await
                .map_err(Into::into)?,
        ))
    };
}

macro_rules! implement_client_methods {
    ($($comment:literal $method_name:ident($input_type:ty ) -> $return:ty $( as $streaming:ident)? => $grpc_method:ident,)+) => {
        $(
            #[doc = $comment]
            fn $method_name<'life, 'async_trait>(
                &'life self,
                __input: tonic::Request<$input_type>,
            ) -> core::pin::Pin<
                Box<
                    dyn core::future::Future<
                            Output = std::result::Result<tonic::Response<$return>, tonic::Status>,
                        > + core::marker::Send
                        + 'async_trait,
                >,
            >
            where
                'life: 'async_trait,
                Self: 'async_trait,
            {
                if self.endpoint_context().request_logging_mode().emits_method_events() {
                    tracing::info!(method = stringify!($method_name), "[TEST] received call");
                }
                Box::pin(async {
                    let observation = HandlerObservation::begin(
                        self.endpoint_context(),
                        GrpcMethod::$grpc_method,
                        stringify!($method_name),
                    )?;
                    let grpc_result: std::result::Result<tonic::Response<$return>, tonic::Status> = async {
                        Ok(client_method_helper!($($streaming)? self __input $method_name))
                    }.await;
                    observation.finish(&grpc_result);
                    grpc_result
                })
            }
        )+
    };
}

pub(super) use {client_method_helper, implement_client_methods};
