# Vigo Sync Server

Self-hosted, zero-knowledge sync server for the Vigo browser.

## Architecture

- **Language**: Go (for simple deployment, single binary)
- **Database**: SQLite (single user), PostgreSQL (multi-user future)
- **Protocol**: REST API over HTTPS with Ed25519 request signing
- **Deployment**: Docker container, `docker-compose` for easy setup

## Security

- The server stores only **opaque encrypted blobs** — it cannot read any user data.
- All encryption/decryption happens client-side in the browser.
- Device authentication uses Ed25519 signatures.
- TLS 1.3 minimum (handled by reverse proxy or built-in).

## Quick Start

```bash
docker-compose up -d
```

Server will be available at `http://localhost:8443`.

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/health` | Health check |
| POST | `/api/v1/devices` | Register a device |
| DELETE | `/api/v1/devices/:id` | Deregister a device |
| PUT | `/api/v1/collections/:type/records` | Push a record |
| PUT | `/api/v1/collections/:type/records/batch` | Push records batch |
| GET | `/api/v1/collections/:type/records?since=` | Pull records |
| DELETE | `/api/v1/collections/:type/records/:id` | Delete a record |
| POST | `/api/v1/keys/wrap` | Push wrapped K_root |
| GET | `/api/v1/keys/wrap/:device_id` | Fetch wrapped K_root |

## Configuration

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `VIGO_SYNC_PORT` | `8443` | Listen port |
| `VIGO_SYNC_DB_PATH` | `./data/vigo_sync.db` | SQLite database path |
| `VIGO_SYNC_TLS_CERT` | (optional) | TLS certificate file |
| `VIGO_SYNC_TLS_KEY` | (optional) | TLS key file |
| `VIGO_SYNC_LOG_LEVEL` | `info` | Log level |

## License

Copyright (c) 2025 Vigo Browser. All rights reserved.
Proprietary and confidential.
