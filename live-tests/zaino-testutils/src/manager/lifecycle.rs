use tracing::debug;
use zcash_local_net::validator::Validator;

use super::TestManager;
use crate::{ValidatorConnectionMarker, ValidatorExt, ValidatorOracle};

impl<C, Conn> TestManager<C, Conn>
where
    C: ValidatorExt,
    Conn: ValidatorConnectionMarker,
{
    /// Returns the service subscriber, panicking if Zaino was not enabled.
    pub fn subscriber(&self) -> &zaino_state::NodeBackedIndexerServiceSubscriber {
        self.service_subscriber
            .as_ref()
            .expect("TestManager::subscriber called but service_subscriber is None (zaino disabled at launch?)")
    }

    #[cfg(test)]
    pub(crate) fn grpc_socket_to_uri(&self) -> http::Uri {
        http::Uri::builder()
            .scheme("http")
            .authority(
                self.zaino_grpc_listen_address
                    .expect("grpc_listen_address should be set")
                    .to_string(),
            )
            .path_and_query("/")
            .build()
            .unwrap()
    }

    /// A raw JSON-RPC connection to the backing validator.
    pub async fn full_node_jsonrpc_connector(&self) -> ValidatorOracle {
        ValidatorOracle::new(&self.full_node_rpc_listen_address.to_string())
    }

    /// Closes the TestManager.
    pub async fn close(&mut self) {
        if let Some(handle) = self.zaino_handle.take() {
            handle.abort();
            let _ = handle.await;
        }
    }

    /// Explicitly interrupts the owned Zaino supervisor for live cleanup tests.
    pub async fn interrupt_zaino_and_close_for_test(mut self) {
        let handle = self
            .zaino_handle
            .take()
            .expect("interruption test requires an owned Zaino supervisor handle");
        handle.abort();
        let join_error = handle
            .await
            .expect_err("aborted Zaino supervisor must not complete normally");
        assert!(
            join_error.is_cancelled(),
            "aborted Zaino supervisor must report cancellation: {join_error}"
        );
        println!("PROFILE_INTERRUPT_CANCELLATION join_error=cancelled");
        self.close().await;
    }
}

impl<C: Validator, Conn: ValidatorConnectionMarker> Drop for TestManager<C, Conn> {
    fn drop(&mut self) {
        debug!("[TEST] Shutting down test environment");
        if let Some(handle) = &self.zaino_handle {
            debug!("[TEST] Aborting Zaino handle");
            handle.abort();
        }
        debug!("[TEST] Test environment shutdown complete");
    }
}
