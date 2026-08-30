# Logging Configuration

Zaino provides flexible logging with three output formats and configurable verbosity levels.

## Environment Variables

| Variable | Values | Default | Description |
|----------|--------|---------|-------------|
| `RUST_LOG` | Filter string | `zaino=info,zainod=info` | Log level filter |
| `ZAINOLOG_FORMAT` | `stream`, `tree`, `json` | `stream` | Output format |
| `ZAINOLOG_COLOR` | `true`, `false`, `auto` | `true` | ANSI color output |

## Log Formats

### Stream (default)
Flat chronological output with timestamps. Best for general use and piping to files.
```
14:32:01.234  INFO zaino_state::indexer: Starting indexer
14:32:01.456  INFO zaino_state::indexer: Connected to validator
```

### Tree
Hierarchical span-based output showing call structure. Best for debugging complex flows.
```
indexer
├─ INFO Starting indexer
└─ validator_connection
   └─ INFO Connected to validator
```

### JSON
Machine-parseable output. Best for log aggregation systems (ELK, Loki, etc).
```json
{"timestamp":"2024-01-15T14:32:01.234Z","level":"INFO","target":"zaino_state::indexer","message":"Starting indexer"}
```

## Endpoint profile observability

The legacy gRPC endpoint advertises `method_level_request`. It retains
method-level request events and the cumulative `zaino.grpc.requests_total`,
`zaino.grpc.errors_total`, and `zaino.grpc.request_duration_seconds` metrics.
The privacy gRPC endpoint advertises `privacy_aggregate_only`. It emits no
per-request application log, counter, duration sample, or histogram. Every
configured privacy endpoint runs the fixed-window aggregator and its rollover
and shutdown lifecycle, regardless of build features. Prometheus publication is
separate: these gauges are published only when `zainod` is built with its
optional `prometheus` feature:

- `zaino.privacy.window.request_count`
- `zaino.privacy.window.error_count`
- `zaino.privacy.window.duration_seconds_sum`
- `zaino.privacy.window.start_seconds`
- `zaino.privacy.window.duration_seconds`

The `prometheus` feature is off in default builds, so default builds don't
publish or expose these gauges. To export them, use the existing documented
`zainod` build mechanism with `--features prometheus` and set the optional
top-level `metrics_endpoint` address in the daemon configuration. The build and
configuration pattern is shown in the
[`zaino-bench` usage guide](../packages/zaino-bench/usage.md#the-node-under-test).
Without the feature, `metrics_endpoint` is ignored and no metrics listener
binds.

Privacy aggregate tuple labels are exactly `endpoint_profile="privacy"`, the
canonical method, its static risk class, and outcome `ok`, `denied`, or `error`.
The first three gauges use exactly those tuple labels. Window start and duration
use only `endpoint_profile="privacy"`; their start and duration are numeric gauge
values, never labels. The static registry has 21 methods (the 20
`CompactTxStreamer` methods plus `PrivacyProfileService/GetPrivacyProfile`) and
three outcomes, for 63 bounded tuples. Every completed or idle window produces
the full internal tuple set, including zeros. A `prometheus`-feature build
publishes that set as gauges; a default build discards the completed set instead
of exporting it.

`request_count` counts calls in each tuple. `error_count` increments only for
the `error` outcome, not for policy denials. `duration_seconds_sum` measures from
before authorization through response creation. Server streams measure setup
and admission, not later stream consumption. Capability calls include metadata
lookup and document rendering.

Windows are non-overlapping and close on a timer even when no request follows.
The default is 60 seconds, and `metrics_window_seconds` selects another positive
duration. With Prometheus export enabled, each completed window replaces the
previous exported gauge values, so the endpoint exposes only the last completed
window. Shutdown deliberately discards the active partial window rather than
publishing or relabeling it as complete. These completion and discard semantics
also govern the aggregator in default builds, where publication is a no-op.

Privacy logs, metric keys/labels, and capability snapshots forbid source or peer
IP/socket/address; request metadata, headers, bodies, values, and stream items;
wallet/Zcash addresses; transaction data and txids; user agents; request,
stream, and persistent client identifiers; cookies and `Set-Cookie`; and
affinity or sticky-session data. This application-level contract doesn't control
proxy, load-balancer, operating-system, or hosting-provider logs. Operators must
independently prevent those layers from adding identity logs, cookies, or
affinity.

## Usage Examples

### Local Development

```bash
# Default logging (stream format, zaino crates only at INFO level)
zainod start

# Tree format for debugging span hierarchies
ZAINOLOG_FORMAT=tree zainod start

# Debug level for zaino crates
RUST_LOG=zaino=debug,zainod=debug zainod start

# Include zebra logs
RUST_LOG=info zainod start

# Fine-grained control
RUST_LOG="zaino_state=debug,zaino_serve=info,zebra_state=warn" zainod start

# Disable colors (for file output)
ZAINOLOG_COLOR=false zainod start 2>&1 | tee zainod.log
```

### Tests

The test environment passes logging variables through:

```bash
# Default (stream format)
cargo nextest run

# Tree format in tests
ZAINOLOG_FORMAT=tree cargo nextest run

# Debug logging in tests
RUST_LOG=debug ZAINOLOG_FORMAT=tree cargo nextest run

# JSON output for parsing test logs
ZAINOLOG_FORMAT=json cargo nextest run 2>&1 | jq .
```

### Production

```bash
# JSON for log aggregation
ZAINOLOG_FORMAT=json ZAINOLOG_COLOR=false zainod start

# Structured logging to file
ZAINOLOG_FORMAT=json ZAINOLOG_COLOR=false zainod start 2>> /var/log/zainod.json

# Minimal logging
RUST_LOG=warn zainod start
```
