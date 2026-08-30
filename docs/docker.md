# Container Usage

This document covers running Zaino using the official container image.

## Overview

The container image runs `zainod` - the Zaino indexer daemon. The image:

- Uses `zainod` as the entrypoint with `start` as the default subcommand
- Runs as non-root user (`container_user`, UID 1000)
- Refuses to start if run as root

For CLI usage details, see the CLI documentation or run `docker run --rm zaino --help`.

## Configuration Options

The container can be configured via:

1. **Environment variables only** - Set all supported fields through `ZAINO_*` variables
2. **Config file + env vars** - Mount a baseline config file and override any supported fields with environment variables
3. **Config file only** - Mount a complete config file without environment overrides

All supported fields use the same precedence: environment variables override
TOML values, which override built-in defaults.

For data persistence, volume mounts are recommended for the database/cache directory.

## Deployment with Docker Compose

The recommended way to run Zaino is with Docker Compose, typically alongside Zebra:

```yaml
services:
  zaino:
    image: zaino:latest
    ports:
      - "8137:8137"   # gRPC
      - "8237:8237"   # JSON-RPC (if enabled)
    volumes:
      - ./config:/app/config:ro
      - zaino-data:/app/data
    environment:
      - ZAINO_VALIDATOR_SETTINGS__VALIDATOR_JSONRPC_LISTEN_ADDRESS=zebra:18232
    depends_on:
      - zebra

  zebra:
    image: zfnd/zebra:latest
    volumes:
      - zebra-data:/home/zebra/.cache/zebra
    # ... zebra configuration

volumes:
  zaino-data:
  zebra-data:
```

If Zebra runs on a different host/network, adjust `VALIDATOR_JSONRPC_LISTEN_ADDRESS` accordingly.

## Initial Setup: Generating Configuration

To generate a config file on your host for customization:

```bash
mkdir -p ./config

docker run --rm -v ./config:/app/config zaino generate-config

# Config is now at ./config/zainod.toml - edit as needed
```

## Container Paths

The container provides simple mount points:

| Purpose | Mount Point |
|---------|-------------|
| Config | `/app/config` |
| Database | `/app/data` |

These are symlinked internally to the XDG paths that Zaino expects.

## Volume Permissions

The container runs as `container_user` (UID 1000, GID 1000) and never
starts as root. Mounted volumes must be writable by this user.

For named volumes (e.g. `zaino-data:/app/data`), the container runtime
handles ownership automatically.

For bind mounts to host directories, ensure the host directory is owned
by UID 1000 before starting the container:

```bash
mkdir -p ./data
chown 1000:1000 ./data
```

### Read-Only Config Mounts

Config files can (and should) be mounted read-only:

```yaml
volumes:
  - ./config:/app/config:ro
```

## Configuration via Environment Variables

Config values can be set via environment variables prefixed with `ZAINO_`, using `__` for nesting:

```yaml
environment:
  - ZAINO_NETWORK=Mainnet
  - ZAINO_VALIDATOR_SETTINGS__VALIDATOR_JSONRPC_LISTEN_ADDRESS=zebra:18232
  - ZAINO_GRPC_SETTINGS__LISTEN_ADDRESS=0.0.0.0:8137
```

### Sensitive Fields

Sensitive fields, including passwords, secrets, tokens, cookies, and private
keys, may be supplied through `ZAINO_*` variables like any other supported
field. For production deployments, prefer runtime or orchestrator secret
facilities and avoid committing sensitive values directly in Compose files.
Environment variables and mounted config files can both expose values; their
security depends on platform permissions and configuration.

## Health Check

The image includes a health check:

```bash
docker inspect --format='{{.State.Health.Status}}' <container>
```
