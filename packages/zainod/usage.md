# `zainod` - usage

`zainod` owns daemon configuration and startup. Its configuration boundary
parses TOML and environment values into `ZainodConfig`, validates them before
startup, and preserves a legacy-only daemon when the optional privacy section
is absent.

## Optional privacy gRPC configuration

`ZainodConfig::privacy_grpc_settings` is an `Option<PrivacyGrpcSettings>`. Use
`None` in Rust struct literals that do not configure the privacy profile. An
omitted `[privacy_grpc_settings]` table stays `None` and is omitted again when
the legacy configuration is serialized.

```toml
[grpc_settings]
listen_address = "127.0.0.1:8137"

[grpc_settings.tls]
cert_path = "/path/to/legacy-cert.pem"
key_path = "/path/to/legacy-key.pem"

[privacy_grpc_settings]
listen_address = "127.0.0.1:9137"
allow_transaction_specific_reads = false
allow_transparent_address_reads = false
metrics_window_seconds = 60

[privacy_grpc_settings.tls]
cert_path = "/path/to/privacy-cert.pem"
key_path = "/path/to/privacy-key.pem"

[json_server_settings]
json_rpc_listen_address = "127.0.0.1:8237"
```

`PrivacyGrpcSettings` flattens `GrpcServerConfig`, so `listen_address` and the
optional `tls` table are siblings of the privacy policy settings. Both read
flags default to `false`; `metrics_window_seconds` remains a TOML `u64`, defaults
to 60, and must be in `1..=u32::MAX` because the capability wire descriptor is
`uint32`. Invalid values are rejected before the indexer service starts. When
this table is present, startup creates a second tonic server. Its method policy
reflects the two read flags, and its capability response reports the configured
metrics window.

Both tonic servers use clones of one subscriber from one indexer service. The
legacy endpoint continues to use `grpc_routes`; the privacy endpoint uses the
profile-aware route builder. The additive privacy capability service is
available on both, while `CompactTxStreamer` remains allow-all on legacy and is
policy-restricted on privacy.

The privacy endpoint denies sensitive classes unless their specific flag is
enabled. An enabled transaction-specific or transparent-address read is still
sensitive and isn't made anonymous. Mempool methods, transaction submission, and
debug access are denied for every privacy v1 flag combination. The exact policy
is in the [RPC method matrix](../../docs/rpc_api.md).

## Validation and layering

Privacy TLS paths and public-bind restrictions use the same validation as the
legacy gRPC endpoint. A public plaintext bind is rejected unless the daemon is
built with `no_tls_use_unencrypted_traffic`. Privacy, legacy gRPC, and JSON-RPC
addresses must all differ where configured.

Environment values override TOML values. The flattened server address uses:

```text
ZAINO_PRIVACY_GRPC_SETTINGS__LISTEN_ADDRESS=127.0.0.1:9137
ZAINO_PRIVACY_GRPC_SETTINGS__ALLOW_TRANSACTION_SPECIFIC_READS=true
ZAINO_PRIVACY_GRPC_SETTINGS__ALLOW_TRANSPARENT_ADDRESS_READS=true
ZAINO_PRIVACY_GRPC_SETTINGS__METRICS_WINDOW_SECONDS=120
```

## Lifecycle

Omitting the privacy table starts exactly one legacy gRPC endpoint. Configuring
it makes readiness depend on both gRPC servers. A startup failure rolls back the
JSON-RPC server, any gRPC server already started, and the shared indexer service.
A runtime failure in either configured gRPC server triggers the daemon restart
path. Graceful shutdown waits for both; an absent privacy endpoint is neutral in
status and is reported as `grpc_privacy=disabled` in status logs. The privacy
metrics timer starts before privacy route/server construction. Startup rollback
closes it, and graceful shutdown first drains both tonic servers, then stops the
timer and discards the active partial window.

The feature-gated `launch_inner_with_listeners` test seam requires its optional
privacy listener to exactly match `privacy_grpc_settings`: both must be present
for privacy tests or both absent for legacy-only tests. Either mismatch returns a
configuration error before the shared indexer service starts and drops every
supplied listener. Production `launch_inner` does not require a pre-bound
privacy listener and binds the configured address normally.

## Operator inspection and boundaries

With `just`, `cargo-nextest`, the live-test binaries under
`live-tests/test_binaries/bins`, and the normal clientless validator test
environment available, run this from the repository root:

```sh
TEST_BINARIES_DIR="$PWD/live-tests/test_binaries/bins" just inspect-zaino-profiles
```

The focused scenario starts a validator and one Zaino process, exercises both
gRPC profiles, prints deterministic `PROFILE_*` evidence, and checks cleanup.
The output includes all 20 effective method decisions and the approved completed
window dimensions.

Zaino doesn't create session IDs, cookies, or affinity for the privacy endpoint.
Its application logs omit per-request and client identity data there. External
proxies and load balancers aren't controlled by these guarantees. Operators must
configure them independently to avoid identity logs, cookies, and affinity.

This server profile doesn't implement range bucketing, canonical reads, mempool
epochs, transparent local scanning, Tor, or OHTTP. Capability discovery reports
the Zaino endpoint, not Vizor WP11 wallet behavior.

## Related

- [`README.md`](./README.md) - operator CLI and deployment guidance.
- [`zaino-serve`](../zaino-serve/) - server configuration shared by the
  flattened `GrpcServerConfig`.
