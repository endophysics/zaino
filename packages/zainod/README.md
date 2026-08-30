# zainod

`zainod` is the Zaino indexer daemon — an indexer for the Zcash blockchain,
written in Rust.

It sits between a Zcash full validator (Zebra) and client
applications, serving:

- the [lightclient protocol API](https://github.com/zcash/lightwallet-protocol), the interface today
  served by [lightwalletd](https://github.com/zcash/lightwalletd), and
- a **JSON-RPC API** covering the subset of Zcash RPCs needed by wallets and
  block explorers.

This crate ships the `zainod` binary. The library half of the crate,
`zainodlib`, exposes the `run` entrypoint and configuration types for embedding
the daemon in other Rust programs.

For project background and architecture, see the
[Zaino repository](https://github.com/zingolabs/zaino).

## CLI

```text
zainod generate-config [--output FILE]   # write a default config file
zainod start [--config FILE]             # start the indexer
```

When `--config`/`--output` is omitted, the path defaults to
`$XDG_CONFIG_HOME/zaino/zainod.toml` (falling back to
`$HOME/.config/zaino/zainod.toml`).

Configuration is layered, highest priority first:

1. environment variables (prefix `ZAINO_`),
2. the TOML config file,
3. built-in defaults.

All supported fields, including sensitive values, follow this precedence.
Deployments must protect sensitive values according to the permissions and
configuration of their chosen secret-injection mechanism.

## Optional Privacy gRPC Configuration

`[privacy_grpc_settings]` reserves an optional, separately configured gRPC
endpoint for the privacy profile. Omitting the table keeps legacy-only behavior
unchanged. When present, Zaino starts a second profile-aware listener over the
same indexer service and subscriber source as the legacy endpoint.

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

Both read flags default to `false`; `metrics_window_seconds` defaults to `60`
and must be in `1..=u32::MAX`. The privacy address must differ from both the
legacy gRPC and optional JSON-RPC addresses. TLS certificate and key paths are
validated independently for each configured gRPC endpoint, and a public
plaintext privacy bind is rejected unless built with
`no_tls_use_unencrypted_traffic`.

The legacy endpoint enables every `CompactTxStreamer` method. Privacy enables
common chain reads by default. Its two flags independently enable the
transaction-specific and transparent-address classes. Those enabled methods
remain sensitive and gain no anonymity guarantee. Privacy always denies mempool
methods, transaction submission, and `Ping`. See the exact
[method matrix](../../docs/rpc_api.md).

Startup is transactional: if either configured gRPC bind fails, Zaino closes
any endpoint already started and the shared indexer service before returning the
typed startup error. Readiness, critical-error restart detection, status logs,
and graceful shutdown include every configured endpoint.

The privacy profile creates no session ID, cookie, `Set-Cookie` metadata, or
affinity requirement. These are Zaino application properties. Operators must
independently configure proxies and load balancers not to record client identity
or add cookies and affinity.

Environment variables override TOML values, including flattened server fields:
`ZAINO_PRIVACY_GRPC_SETTINGS__LISTEN_ADDRESS`,
`ZAINO_PRIVACY_GRPC_SETTINGS__ALLOW_TRANSACTION_SPECIFIC_READS`,
`ZAINO_PRIVACY_GRPC_SETTINGS__ALLOW_TRANSPARENT_ADDRESS_READS`, and
`ZAINO_PRIVACY_GRPC_SETTINGS__METRICS_WINDOW_SECONDS`.

Run the repository inspection scenario from the workspace root. It requires
`just`, `cargo-nextest`, built or linked live-test binaries under
`live-tests/test_binaries/bins`, and the normal clientless validator test
environment:

```sh
TEST_BINARIES_DIR="$PWD/live-tests/test_binaries/bins" just inspect-zaino-profiles
```

It starts one validator and one Zaino process with both endpoints, then prints
labeled `PROFILE_*` results for common queries, enforced denials, legacy
submission handling, all 20 capability decisions, profile logging, completed
window dimensions, and cleanup. A successful run ends with the focused test
passing and includes `PROFILE_CLEANUP zaino=closed validator=drop_guarded`.

## Launching

`zainod` needs a running validator to connect to. The examples below assume one
is reachable at the address in your config.

### From crates.io

```sh
cargo install zainod
zainod generate-config            # writes the default config, then edit it
zainod start                      # uses the default config path
# or point at an explicit file:
zainod start --config ./zainod.toml
```

### From source

```sh
git clone https://github.com/zingolabs/zaino.git
cd zaino
cargo run --release -p zainod -- start --config ./zainod.toml
```

### With Podman (rootless)

The daemon is published as a container image with `zainod start` as the default
command. It runs as a non-root user (UID 1000) and refuses to start as root,
which makes it a natural fit for rootless Podman.

Run it directly, mounting a config file and a data volume:

```sh
podman run --rm \
  -p 8137:8137 \
  -p 8237:8237 \
  -v ./zainod.toml:/app/config/zainod.toml:ro,Z \
  -v zaino-data:/app/data \
  zainod:latest
```

`--userns=keep-id` maps the container's UID 1000 to your host user, so files in
the mounted data volume stay owned by you:

```sh
podman run --rm --userns=keep-id \
  -p 8137:8137 \
  -v ./zainod.toml:/app/config/zainod.toml:ro,Z \
  -v zaino-data:/app/data \
  zainod:latest
```

A typical deployment runs `zainod` alongside Zebra with `podman compose`:

```yaml
services:
  zaino:
    image: zainod:latest
    ports:
      - "8137:8137"   # gRPC
      - "8237:8237"   # JSON-RPC (if enabled)
    volumes:
      - ./config:/app/config:ro,Z
      - zaino-data:/app/data
    depends_on:
      - zebra

volumes:
  zaino-data:
```

```sh
podman compose up
```

See [`docs/docker.md`](https://github.com/zingolabs/zaino/blob/dev/docs/docker.md)
for the full container guide.

## License

Apache-2.0.
