# Deploying HollowChat's server

The server (`crates/server`) is the only piece that needs a public deployment - the desktop client and browser extension are distributed straight to users. This covers getting the server running behind TLS with a real Postgres.

## Option A: Docker Compose (recommended)

```sh
cp .env.example .env
# fill in POSTGRES_PASSWORD, PASSWORD_PEPPER, APP_BASE_URL at minimum
docker compose up -d
```

This builds the server image from [`crates/server/Dockerfile`](../crates/server/Dockerfile) and starts it alongside Postgres. Migrations run automatically on boot. The server listens on `127.0.0.1:8080` only - it's meant to sit behind a reverse proxy (see below), not face the internet directly.

## Option B: systemd (bare metal, no containers)

```sh
cargo build --release -p hollowchat-server
sudo useradd --system --create-home --home-dir /opt/hollowchat hollowchat
sudo cp target/release/hollowchat-server /opt/hollowchat/
sudo cp .env /opt/hollowchat/.env   # filled in as above, BIND_ADDR=127.0.0.1:8080
sudo mkdir -p /opt/hollowchat/data
sudo chown -R hollowchat:hollowchat /opt/hollowchat
sudo cp deploy/hollowchat-server.service /etc/systemd/system/
sudo systemctl enable --now hollowchat-server
```

Postgres itself isn't covered here - run it however you already manage databases on this host (a Postgres 16+ instance is all that's required; the schema is applied automatically via embedded migrations).

## Reverse proxy + TLS

The server speaks plain HTTP and needs a proxy in front of it for TLS and to forward WebSocket upgrades (used by `/gateway`, `/devicelink`, and calls). Caddy example - it handles TLS certificates automatically:

```caddyfile
your-domain.example {
    reverse_proxy 127.0.0.1:8080
}
```

Caddy proxies WebSocket upgrades by default, no extra config needed. For nginx, the upgrade headers must be forwarded explicitly:

```nginx
server {
    listen 443 ssl http2;
    server_name your-domain.example;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
    }
}
```

Set `APP_BASE_URL=https://your-domain.example` to match whatever's in front of the proxy - it's used for invite links and billing callbacks.

## Before going live

- [ ] `PASSWORD_PEPPER` is a real random value, generated once, backed up somewhere durable - losing it locks out every account, reusing the dev placeholder makes every password hash on this deployment guessable the moment the pepper leaks anywhere.
- [ ] `CORS_ALLOWED_ORIGINS` is set to exactly what needs cross-origin access (a web client, the browser extension's origin) - left empty is safe but blocks everything.
- [ ] A TURN server is configured (`ICE_TURN_URL`/`_USERNAME`/`_CREDENTIAL`) if you want calls to work reliably for users behind NAT - STUN alone frequently isn't enough.
- [ ] Postgres backups are actually running and actually tested (a backup nobody has restored from is a hypothesis, not a backup).
- [ ] The attachments volume/directory is backed up too - it's not in the database.
