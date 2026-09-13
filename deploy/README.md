# Phenotype Fabric Daemon - Deployment Guide

This directory contains production deployment artifacts for `fabric-daemon`.

## Quick Start

| Method | Best for | Time to running |
|--------|----------|-----------------|
| Docker Compose | Development, single-node | ~2 minutes |
| Systemd | Linux servers, production | ~10 minutes |
| Manual | Custom environments | ~15 minutes |

---

## Option 1: Docker Compose (Recommended for Development)

```bash
# Build and start the daemon.
docker compose up -d

# Follow logs.
docker compose logs -f fabric-daemon

# Check health (sends a wire-protocol health_check message).
docker compose exec fabric-daemon fabric-cli health --connect 127.0.0.1:9400

# Stop.
docker compose down
```

State is persisted to `./data` on the host (SQLite database).

### Custom Configuration

Mount a config file and override the entrypoint command:

```yaml
# In docker-compose.yml, under fabric-daemon:
volumes:
  - ./data:/var/lib/fabric
  - ./config.toml:/etc/fabric/config.toml:ro
command: ["start", "--config", "/etc/fabric/config.toml"]
```

---

## Option 2: Systemd (Production Linux Servers)

### Prerequisites

- Linux with systemd
- `fabric-daemon` binary installed to `/usr/local/bin/fabric-daemon`
- `fabric-cli` binary installed to `/usr/local/bin/fabric-cli` (for health checks)

### Install

```bash
# Create the fabric user.
sudo useradd --system --shell /bin/false --home-dir /var/lib/fabric fabric
sudo mkdir -p /var/lib/fabric
sudo chown fabric:fabric /var/lib/fabric

# Install the config file.
sudo mkdir -p /etc/fabric
sudo cp deploy/systemd/fabric-daemon.conf /etc/fabric/config.toml
sudo chown root:fabric /etc/fabric/config.toml
sudo chmod 640 /etc/fabric/config.toml

# Install the systemd unit.
sudo cp deploy/systemd/fabric-daemon.service /etc/systemd/system/
sudo systemctl daemon-reload
```

### Operate

```bash
# Enable and start.
sudo systemctl enable --now fabric-daemon

# Check status.
sudo systemctl status fabric-daemon

# View logs.
sudo journalctl -u fabric-daemon -f

# Restart after config change.
sudo systemctl restart fabric-daemon
```

---

## Option 3: Manual / Other Init Systems

```bash
# Build in release mode.
cargo build --release --bin fabric-daemon

# Run with config.
./target/release/fabric-daemon start --config deploy/systemd/fabric-daemon.conf

# Or with CLI overrides.
./target/release/fabric-daemon start \
    --listen 0.0.0.0:9400 \
    --db /var/lib/fabric/state.db \
    --log-level info
```

---

## Architecture Notes

- **Wire Protocol**: `fabric-daemon` speaks a TCP JSON wire protocol on port 9400 (not HTTP). Health checks use the same port via `{"type":"health_check"}` messages.
- **State**: SQLite with WAL mode, stored in `state.db` under the state directory.
- **Health**: Use `fabric-cli health --connect <address>` for health checks (Docker HEALTHCHECK, systemd watchdog, etc.).
- **Signals**: `fabric-daemon` catches SIGINT (Ctrl+C) for graceful shutdown and flushes state to disk.

## Files in This Directory

```
deploy/
  systemd/
    fabric-daemon.service   # systemd unit file
    fabric-daemon.conf      # Example TOML configuration
  README.md                 # This file
```
