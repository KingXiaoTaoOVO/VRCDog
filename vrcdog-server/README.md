# VRCDog Standalone Server

This directory is a self-contained server project that implements the same HTTP
API used by the embedded VRCDog server. It can run on Windows or Linux, or be
deployed with Docker.

## Run with Rust

```bash
cp .env.example .env
cargo run --release
```

The default endpoint is `http://0.0.0.0:11451`. In VRCDog, enter server mode,
verify the administrator password, choose **Remote service**, and enter the
public URL of this server.

## Run with Docker

```bash
docker compose up -d --build
```

Persisted users, bans, freezes, and roles are stored in
`data/server-state.json`. Keep the `data` directory on a persistent volume.

## Reverse proxy

The server speaks plain HTTP. For an internet-facing deployment, place it
behind Caddy, Nginx, or another TLS reverse proxy and connect with an HTTPS URL.
The client requires the base URL only, without `/ping`.

## Environment variables

| Variable | Default | Purpose |
| --- | --- | --- |
| `VRCDOG_HOST` | `0.0.0.0` | Listening interface |
| `VRCDOG_PORT` | `11451` | Listening port |
| `VRCDOG_DATA_FILE` | `./data/server-state.json` | JSON persistence path |
| `VRCDOG_SERVER_PASSWORD_BCRYPT` | Hash for `root` | BCrypt hash used by the remote administration API |
| `RUST_LOG` | `vrcdog_server=info,tower_http=info` | Log filtering |

## Health check

```bash
curl http://127.0.0.1:11451/ping
```
