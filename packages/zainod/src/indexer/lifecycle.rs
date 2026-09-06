use std::{net::TcpListener, time::Duration};

use tracing::info;
use zaino_serve::{
    rpc::{
        grpc_routes_with_context, legacy_grpc_routes,
        profile::{EndpointContext, PrivacyMethodPolicy, PrivacyWindowMetrics},
    },
    server::{
        config::{GrpcServerConfig, JsonRpcServerConfig},
        grpc::TonicServer,
        jsonrpc::JsonRpcServer,
    },
};
use zaino_state::{IndexedTipIndexer, IndexerSubscriber, LightWalletIndexer, ZcashIndexer};
use zaino_status::StatusType;

use crate::{config::ZainodConfig, error::IndexerError};

pub(super) struct EndpointListeners {
    pub(super) grpc_legacy: Option<TcpListener>,
    pub(super) grpc_privacy: Option<TcpListener>,
    pub(super) json_rpc: Option<TcpListener>,
}

impl EndpointListeners {
    pub(super) const fn production() -> Self {
        Self {
            grpc_legacy: None,
            grpc_privacy: None,
            json_rpc: None,
        }
    }
}

pub(super) struct EndpointServers {
    pub(super) json_rpc: Option<JsonRpcServer>,
    pub(super) grpc_legacy: Option<TonicServer>,
    pub(super) grpc_privacy: Option<TonicServer>,
    pub(super) privacy_metrics: Option<PrivacyWindowMetrics>,
}

pub(super) struct EndpointPlan {
    json_rpc: Option<JsonRpcServerConfig>,
    grpc_legacy: GrpcServerConfig,
    grpc_privacy: Option<PrivacyEndpointPlan>,
}

struct PrivacyEndpointPlan {
    server: GrpcServerConfig,
    policy: PrivacyMethodPolicy,
    metric_window_seconds: u32,
}

impl EndpointPlan {
    pub(super) fn from_config(config: ZainodConfig) -> Result<Self, IndexerError> {
        let grpc_privacy = config
            .privacy_grpc_settings
            .map(|privacy| -> Result<PrivacyEndpointPlan, IndexerError> {
                let metric_window_seconds =
                    u32::try_from(privacy.metrics_window_seconds).map_err(|_| {
                        IndexerError::ConfigError(
                            "privacy_grpc_settings.metrics_window_seconds must fit in uint32."
                                .to_string(),
                        )
                    })?;
                let policy = PrivacyMethodPolicy::new(
                    privacy.allow_transaction_specific_reads,
                    privacy.allow_transparent_address_reads,
                );
                Ok(PrivacyEndpointPlan {
                    server: privacy.server,
                    policy,
                    metric_window_seconds,
                })
            })
            .transpose()?;

        Ok(Self {
            json_rpc: config.json_server_settings,
            grpc_legacy: config.grpc_settings,
            grpc_privacy,
        })
    }
}

impl EndpointServers {
    pub(super) async fn spawn<Indexer>(
        subscriber: IndexerSubscriber<Indexer>,
        plan: EndpointPlan,
        listeners: EndpointListeners,
    ) -> Result<Self, IndexerError>
    where
        Indexer: ZcashIndexer + LightWalletIndexer + IndexedTipIndexer + Clone,
    {
        let mut servers = Self {
            json_rpc: None,
            grpc_legacy: None,
            grpc_privacy: None,
            privacy_metrics: None,
        };

        if let Some(json_config) = plan.json_rpc {
            let result = match listeners.json_rpc {
                #[cfg(feature = "test_dependencies")]
                Some(listener) => {
                    JsonRpcServer::spawn_from_listener(subscriber.clone(), json_config, listener)
                        .await
                }
                _ => JsonRpcServer::spawn(subscriber.clone(), json_config).await,
            };
            match result {
                Ok(server) => servers.json_rpc = Some(server),
                Err(error) => return Err(error.into()),
            }
        }

        let legacy_result = match listeners.grpc_legacy {
            #[cfg(feature = "test_dependencies")]
            Some(listener) => {
                TonicServer::spawn_named_from_listener_with_routes(
                    |shutdown| legacy_grpc_routes(subscriber.clone(), shutdown),
                    plan.grpc_legacy,
                    listener,
                    "grpc_legacy",
                )
                .await
            }
            _ => {
                TonicServer::spawn_named_with_routes(
                    |shutdown| legacy_grpc_routes(subscriber.clone(), shutdown),
                    plan.grpc_legacy,
                    "grpc_legacy",
                )
                .await
            }
        };
        match legacy_result {
            Ok(server) => servers.grpc_legacy = Some(server),
            Err(error) => {
                servers.close().await;
                return Err(error.into());
            }
        }

        if let Some(privacy) = plan.grpc_privacy {
            let metrics = match PrivacyWindowMetrics::new(Duration::from_secs(u64::from(
                privacy.metric_window_seconds,
            ))) {
                Ok(metrics) => metrics,
                Err(error) => {
                    servers.close().await;
                    return Err(IndexerError::ConfigError(error.to_string()));
                }
            };
            let context = EndpointContext::privacy(privacy.policy, metrics.recorder());
            servers.privacy_metrics = Some(metrics);
            let privacy_routes = grpc_routes_with_context(subscriber, context);
            let privacy_result = match listeners.grpc_privacy {
                #[cfg(feature = "test_dependencies")]
                Some(listener) => {
                    TonicServer::spawn_named_from_listener(
                        privacy_routes,
                        privacy.server,
                        listener,
                        "grpc_privacy",
                    )
                    .await
                }
                _ => TonicServer::spawn_named(privacy_routes, privacy.server, "grpc_privacy").await,
            };
            match privacy_result {
                Ok(server) => servers.grpc_privacy = Some(server),
                Err(error) => {
                    servers.close().await;
                    return Err(error.into());
                }
            }
        }

        Ok(servers)
    }

    pub(super) fn status(&self) -> StatusType {
        let legacy = self
            .grpc_legacy
            .as_ref()
            .map_or(StatusType::Offline, TonicServer::status);
        let grpc = self
            .grpc_privacy
            .as_ref()
            .map_or(legacy, |privacy| legacy.combine(privacy.status()));
        self.json_rpc
            .as_ref()
            .map_or(grpc, |json| grpc.combine(json.status()))
    }

    pub(super) fn log_status(&self, chain_state: StatusType, finalised_state_mode: &str) {
        let json_rpc = self
            .json_rpc
            .as_ref()
            .map_or("disabled".to_string(), |server| server.status().to_string());
        let grpc_legacy = self
            .grpc_legacy
            .as_ref()
            .map_or(StatusType::Offline, TonicServer::status);
        let grpc_privacy = self
            .grpc_privacy
            .as_ref()
            .map_or("disabled".to_string(), |server| server.status().to_string());
        info!(
            chain_state = %chain_state,
            fs_mode = %finalised_state_mode,
            json_rpc = %json_rpc,
            grpc_legacy = %grpc_legacy,
            grpc_privacy = %grpc_privacy,
            "Zaino status check"
        );
    }

    pub(super) async fn close(&mut self) {
        if let Some(mut json_rpc) = self.json_rpc.take() {
            json_rpc.close().await;
            json_rpc.status.store(StatusType::Offline);
        }

        let legacy_close = async {
            if let Some(mut legacy) = self.grpc_legacy.take() {
                legacy.close().await;
                legacy.status.store(StatusType::Offline);
            }
        };
        let privacy_close = async {
            if let Some(mut privacy) = self.grpc_privacy.take() {
                privacy.close().await;
                privacy.status.store(StatusType::Offline);
            }
        };
        tokio::join!(legacy_close, privacy_close);
        if let Some(mut metrics) = self.privacy_metrics.take() {
            metrics.close().await;
        }
    }
}
