# VPN Integration — WireGuard Tunneling for Agent Containers

## Overview

Agent containers can route all traffic through a per-execution WireGuard VPN tunnel. When a workflow step has `vpn_enabled = true`, the system creates a dedicated VPN sidecar container, pairs it with a unique WireGuard peer, and attaches the agent container to the sidecar's network namespace. All agent traffic — LLM calls, git operations, web requests — exits through the VPN with no possibility of leakage.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Docker Host                                            │
│                                                         │
│  ┌───────────────────┐    ┌──────────────────────────┐  │
│  │  Agent Container   │    │  VPN Sidecar Container   │  │
│  │                    │    │                           │  │
│  │  git, curl, LLM   │    │  WireGuard (wg0)         │  │
│  │  calls, etc.       │◄──►│  iptables kill switch    │  │
│  │                    │    │  IPv6 disabled            │  │
│  │  --network=        │    │  DNS via VPN gateway     │  │
│  │  container:sidecar │    │                           │  │
│  └───────────────────┘    └──────────┬───────────────┘  │
│                                      │ UDP :51820       │
└──────────────────────────────────────┼──────────────────┘
                                       │
                              ┌────────▼────────┐
                              │  wg-easy Server  │
                              │  (WireGuard +    │
                              │   REST API)      │
                              └────────┬────────┘
                                       │
                                   Internet
```

The agent container has **no direct network interface** — it shares the sidecar's network namespace via `--network=container:<sidecar_id>`. The sidecar's iptables kill switch blocks all traffic except through `wg0`, so if the tunnel drops, traffic is blocked rather than leaked.

## Components

### wg-easy Server (`docker-compose.yml`)

Self-hosted WireGuard server with a REST API for peer management. Runs as a Docker Compose service under the `vpn` profile.

```bash
docker compose --profile vpn up -d
```

Environment variables:
- `WGEASY_API_URL` — Base URL (default: `http://localhost:51821`)
- `WGEASY_PASSWORD` — API password

### wg-easy Client (`src/execution/vpn/mod.rs`)

REST client for the wg-easy API. Handles:
- Session-based authentication with lazy init and re-auth on 401/403
- Peer CRUD: `create_peer`, `get_peer_config`, `delete_peer`, `list_peers`
- Orphan reaper: `reap_orphaned_peers` — cleans up peers with no matching sidecar

### VPN Sidecar Manager (`src/execution/vpn_sidecar/mod.rs`)

Manages the WireGuard sidecar container lifecycle:

1. `docker create` with `NET_ADMIN` capability, IPv6 disabled, log suppression
2. `docker start`
3. Write WireGuard config into container
4. `wg-quick up wg0`
5. Apply iptables kill switch
6. Health check (wg show + ping + IP leak verification)

Also provides `reap_orphaned_sidecars` for startup cleanup.

### Tunnel Watchdog (`src/execution/vpn_sidecar/watchdog/mod.rs`)

Background monitor that runs alongside agent execution via `tokio::select!`. Polls `wg show wg0` every 5 seconds. After 3 consecutive failures (~15s), aborts execution with a clear error instead of letting the agent hang for 300 seconds with no network.

### Retry Logic (`src/execution/vpn/retry/mod.rs`)

VPN-specific retry with exponential backoff. Classifies errors:
- **Retryable**: API unreachable, HTTP timeouts, 5xx server errors
- **Not retryable**: Auth failures, config validation, sidecar failures, 4xx errors

### Config Validation (`src/execution/vpn/mod.rs::validate_wg_config`)

Validates WireGuard configs before applying them:
- Requires `AllowedIPs = 0.0.0.0/0` (full tunnel — no split-tunnel allowed)
- Requires `DNS =` line (prevents DNS leaks)

Rejects configs that would allow traffic outside the VPN.

## Security Measures

| Layer | Protection | Implementation |
|-------|-----------|----------------|
| **Kill switch** | Traffic blocked if tunnel drops | iptables OUTPUT/INPUT default DROP, only wg0 + loopback + UDP 51820 allowed |
| **IPv6** | Prevents VPN bypass via IPv6 | `--sysctl=net.ipv6.conf.all.disable_ipv6=1` on sidecar |
| **DNS** | Prevents DNS leaks to host | `WG_DEFAULT_DNS=10.8.0.1` routes DNS through VPN gateway |
| **Config validation** | Prevents split-tunnel misconfiguration | Rejects configs without `0.0.0.0/0` or DNS |
| **IP verification** | Proves traffic exits through VPN | Checks external IP via `api.ipify.org` during health check |
| **Log suppression** | Prevents handshake metadata in logs | `--log-driver=none` on sidecar |
| **Privilege hardening** | Minimizes attack surface | `--cap-drop=ALL` on agent, `--security-opt=no-new-privileges` on both |
| **Tunnel watchdog** | Fast failure on tunnel drop | Aborts execution within ~15s instead of 300s silent hang |

## Execution Flow

```
Workflow step (vpn_enabled=true)
  │
  ├─ 1. Create wg-easy peer (with retry)
  ├─ 2. Fetch peer WireGuard config
  ├─ 3. Validate config (full tunnel + DNS)
  ├─ 4. Create sidecar container
  │     ├─ docker create (NET_ADMIN, IPv6 off, no logs, no-new-privileges)
  │     ├─ docker start
  │     ├─ Write wg config → /etc/wireguard/wg0.conf
  │     ├─ wg-quick up wg0
  │     ├─ Apply iptables kill switch
  │     └─ Health check (wg show → ping → IP leak check)
  │
  ├─ 5. Create agent container (--network=container:<sidecar>)
  │
  ├─ 6. Execute step (with tunnel watchdog via tokio::select!)
  │     ├─ run_step_via_engine(...)   ← agent does its work
  │     └─ monitor_vpn_tunnel(...)    ← watchdog polls every 5s
  │         (whichever finishes first wins, loser is cancelled)
  │
  └─ 7. Cleanup
        ├─ Destroy agent container
        ├─ Destroy VPN sidecar (wg-quick down → docker stop → docker rm)
        └─ Delete wg-easy peer (with retry)
```

## Configuration

### Environment Variables

```bash
# Required when workflows have vpn_enabled=true
WGEASY_API_URL=http://localhost:51821
WGEASY_PASSWORD=changeme
```

### Constants (`src/constants.rs`)

| Constant | Value | Purpose |
|----------|-------|---------|
| `VPN_SIDECAR_IMAGE` | `lscr.io/linuxserver/wireguard:latest` | Sidecar Docker image |
| `VPN_HEALTH_CHECK_TIMEOUT_SECS` | 30 | Max wait for tunnel establishment |
| `VPN_HEALTH_CHECK_INTERVAL_SECS` | 2 | Poll interval during tunnel setup |
| `VPN_HEALTH_CHECK_GATEWAY` | `10.8.0.1` | WireGuard gateway for ping check |
| `VPN_WATCHDOG_INTERVAL_SECS` | 5 | Watchdog poll interval during execution |
| `VPN_WATCHDOG_MAX_FAILURES` | 3 | Consecutive failures before abort (~15s) |
| `VPN_RETRY_MAX_ATTEMPTS` | 3 | wg-easy API retry limit |
| `VPN_RETRY_INITIAL_BACKOFF_MS` | 200 | Initial retry delay |
| `VPN_REAPER_MAX_AGE_SECS` | 3600 | Orphan sidecar/peer age threshold (1h) |

## File Map

```
src/execution/
├── vpn/
│   ├── mod.rs              # wg-easy client, config validation
│   ├── tests.rs            # Client + validation tests
│   ├── integration_tests.rs # End-to-end tests (requires wg-easy)
│   └── retry/
│       ├── mod.rs           # Retry logic, error classification
│       └── tests.rs         # Retry behavior tests
├── vpn_sidecar/
│   ├── mod.rs              # Sidecar lifecycle, health check, reaper
│   ├── tests.rs            # Sidecar + hardening tests
│   └── watchdog/
│       ├── mod.rs           # Tunnel watchdog (monitor_vpn_tunnel)
│       └── tests.rs         # Watchdog tests
└── container/
    └── mod.rs              # Agent container (--cap-drop=ALL, etc.)

src/server/hub/dag/
└── mod.rs                  # Integration: create_optional_container,
                            # run_with_vpn_watchdog, destroy_optional_container

docker-compose.yml          # wg-easy service (profile: vpn)
```

## Testing

```bash
# Unit tests (no external dependencies)
cargo test execution::vpn::
cargo test execution::vpn_sidecar::

# Integration tests (requires running wg-easy)
docker compose --profile vpn up -d
cargo test execution::vpn::integration_tests -- --ignored
```

51 tests across 6 test files covering error classification, retry behavior, config validation, constants verification, and watchdog bounds.
