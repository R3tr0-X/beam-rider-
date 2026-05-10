Both corrections are valid. Let me address them cleanly.

---

## Correction 1: Circle xReserve / USDCx — cut entirely

I picked that up from the Stacks ecosystem research and pattern-matched it into the yield venues. It has no relevance to Hermes, and it doesn't appear in your project context anywhere. The correct Stacks yield venues for `yield-vault.clar` are:

- **Zest Protocol** — sBTC/STX lending, ~3–5% APY
- **Bitflow** — sBTC/STX LP trading fees, variable 5–15%
- **StackingDAO** — liquid stacking → stSTXbtc, sBTC paid daily

USDCx/Circle xReserve is gone from the architecture. Anywhere it appeared in yield venue comparisons, substitute Bitflow sBTC/STX LP instead.

---

## Correction 2: Zero-resource deployment — full section, properly integrated

The previous brief covered app architecture and contracts but dropped the infra. Here it is, wired into BeamRider specifically.

---

### Filesystem layout on your bare metal / VPS

```
/srv/beamrider/
├── docker-compose.yml
├── backend/
│   ├── Dockerfile
│   └── (source copied here on deploy)
├── data/
│   └── beamrider.db          ← SQLite bind mount, stays across redeploys
└── cloudflared/
    └── (no config needed for quick tunnel)
```

---

### `docker-compose.yml`

```yaml
services:
  backend:
    build: ./backend
    container_name: beamrider-api
    restart: unless-stopped

    ports:
      - "3000:3000"

    environment:
      DATABASE_URL: sqlite:///data/beamrider.db
      RUST_LOG: info
      # add all other env vars here or via .env file

    volumes:
      - ./data:/data

    security_opt:
      - no-new-privileges:true

    read_only: true

    tmpfs:
      - /tmp

    deploy:
      resources:
        limits:
          memory: 1G
```

Two things that matter here beyond the generic template: `read_only: true` means the only writable surface is `/data` (your SQLite file). The `tmpfs: /tmp` mount covers any temp file the Axum server or rig-core agent writes mid-request. Without it, writes to `/tmp` fail on a read-only root and your agent crashes silently.

---

### `backend/Dockerfile`

```dockerfile
FROM rust:1.88 AS builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Non-root user — required for read_only: true to work cleanly
RUN useradd -m appuser
USER appuser

COPY --from=builder /app/target/release/beamrider /usr/local/bin/beamrider

EXPOSE 3000

CMD ["beamrider"]
```

Multi-stage keeps the final image under 100MB (no Rust toolchain). The `useradd` + `USER appuser` line is load-bearing with `read_only: true` — running as root with a read-only FS generates noisy permission errors in some Linux distros even for paths you don't actually write to.

---

### Axum bind address

```rust
// src/main.rs
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
```

Not `127.0.0.1`. Docker's bridge network routes external traffic to the container IP, not loopback. Binding to loopback inside a container means the port is unreachable from outside, even with the ports mapping in Compose.

---

### TLS / public URL: Cloudflare Tunnel

No domain purchase. No cert management. Run once:

```bash
cloudflared tunnel --url http://localhost:3000
```

You get a stable-for-the-session URL:

```
https://something-random.trycloudflare.com
```

For a persistent URL across restarts (needed for the Vercel `NEXT_PUBLIC_API_URL` env var and for the x402 facilitator callback), add `cloudflared` as a second service in Compose:

```yaml
  cloudflared:
    image: cloudflare/cloudflared:latest
    restart: unless-stopped
    command: tunnel --url http://backend:3000
    depends_on:
      - backend
```

This keeps the tunnel alive as long as Compose is running. The URL still changes on full container restart, so note it after first deploy and update Vercel's env var once. For a hackathon demo this is fine.

---

### Vercel frontend environment

```env
# .env.production (set in Vercel dashboard under Project → Settings → Environment Variables)
NEXT_PUBLIC_API_URL=https://something-random.trycloudflare.com
```

Update this once after your first server deploy. Vercel redeploys on the next push and picks up the new value.

---

### SQLite WAL mode — set once at startup

In `src/db.rs`, after creating the pool:

```rust
sqlx::query("PRAGMA journal_mode=WAL;")
    .execute(&pool).await?;
sqlx::query("PRAGMA synchronous=NORMAL;")
    .execute(&pool).await?;
```

WAL mode lets readers and the single writer run concurrently. Without it, every read during a write blocks. With the agent running background workers (earnings watcher, rebalance scheduler, Stacks relay) alongside HTTP handlers, you will hit this without WAL.

---

### CI/CD: GitHub Actions → bare metal

```yaml
# .github/workflows/deploy-backend.yml
name: Deploy Backend

on:
  push:
    branches:
      - main

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Deploy to server
        uses: appleboy/ssh-action@v1.0.3
        with:
          host:     ${{ secrets.SERVER_IP }}
          username: ${{ secrets.SERVER_USER }}
          key:      ${{ secrets.SERVER_SSH_KEY }}
          script: |
            cd /srv/beamrider
            git pull
            docker compose up -d --build
```

Vercel handles the frontend automatically on every push — no workflow needed there.

---

### Fly.io alternative (if no bare metal)

If you don't have a bare metal server, Fly.io's free tier (3 shared-cpu-1x VMs, 3GB volumes) replaces the VPS + Cloudflare Tunnel combination entirely. Fly gives you a stable `https://beamrider.fly.dev` URL, persistent volume for SQLite, and lets you deploy a Rust binary directly without Docker fuss.

```toml
# fly.toml
app = "beamrider"
primary_region = "iad"

[build]
  dockerfile = "backend/Dockerfile"

[mounts]
  source = "beamrider_data"
  destination = "/data"

[[services]]
  internal_port = 3000
  protocol = "tcp"
  [[services.ports]]
    port = 443
    handlers = ["tls", "http"]
```

Fly sleeps idle machines after ~15 min on the free tier. Mitigate with a free uptimerobot.com pinger hitting `/healthz` every 5 minutes.

---

### Full capability table (corrected)

| Capability | Tool | Cost |
|---|---|---|
| HTTPS / TLS | Cloudflare Tunnel or Fly.io | $0 |
| Public URL | `trycloudflare.com` or `fly.dev` | $0 |
| CDN (frontend) | Vercel Hobby | $0 |
| Container runtime | Docker Compose on bare metal | $0 |
| SQLite persistence | Bind mount (`/data`) or Fly volume | $0 |
| CI/CD backend | GitHub Actions + SSH | $0 |
| CI/CD frontend | Vercel auto-deploy | $0 |
| Stacks RPC | Hiro public API (`api.hiro.so`) | $0 |
| Celo RPC | Forno (`forno.celo.org`) + Alchemy free tier | $0 |
| LLM (agent) | Gemini 2.0 Flash free tier | $0 |
| x402 facilitator | Coinbase CDP (1,000 tx/mo free) | $0 |
| Celo deployer gas | ~$3–5 one-time | <$5 |
| Stacks deployer gas | ~1 STX per contract deploy | <$1 |