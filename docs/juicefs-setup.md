# JuiceFS Workspace Setup

JuiceFS provides the POSIX filesystem backing for agent workspaces. It stores metadata in Postgres and file data in MinIO (S3-compatible).

## Dockerized Dev (Recommended)

The server runs inside Docker with JuiceFS mounted by a dedicated container. No macFUSE or JuiceFS binary needed on the host.

### 1. Build the server binary

```bash
~/.cargo/bin/cargo build
```

### 2. Build the frontend

```bash
cd frontend && npm run build && cd ..
```

### 3. Create the JuiceFS database (first time only)

If your Postgres volume already exists (database was created before `init-juicefs-db.sql` was added):

```bash
docker exec gh-agents-postgres-1 psql -U nexor -c "CREATE DATABASE juicefs"
```

New environments get this automatically from the init script.

### 4. Format the JuiceFS volume (first time only)

```bash
docker compose up -d postgres minio
./scripts/juicefs-format.sh --docker
```

### 5. Start everything

```bash
docker compose --profile server up
```

This starts: postgres, minio, juicefs (FUSE mount), and nexor-server.

The JuiceFS container mounts the filesystem at `/mnt/jfs` with `rshared` propagation. The server container and agent containers access it as a normal directory.

### 6. Verify

The server is at `http://localhost:3000`. JuiceFS workspace files appear in `./jfs/` on the host (via mount propagation).

## Host-Based Dev (Alternative)

Run the server directly on macOS. Requires macFUSE and JuiceFS binary on the host.

### Prerequisites

```bash
brew install juicedata/tap/juicefs
brew install --cask macfuse       # reboot + allow kernel extension
brew install minio/stable/mc
```

### Setup

```bash
docker-compose up -d                  # postgres + minio
./scripts/juicefs-format.sh           # one-time
./scripts/juicefs-mount.sh            # mount at /tmp/nexor-jfs
./scripts/juicefs-validate.sh         # verify
cargo run -- serve                    # start server on host
```

## Configuration

| Env Var | Default | Description |
|---------|---------|-------------|
| `WORKSPACE_MOUNT_POINT` | `/tmp/nexor-jfs` (host) or `/mnt/jfs` (docker) | JuiceFS mount path |

## Directory Layout

```
{mount_point}/
  workflows/
    {workflow_id}/
      runs/
        {run_id}/                        <- per-run workspace root
          ...agent-produced files...
      pinned/                            <- sealed files (future)
```

## Troubleshooting

**"database juicefs does not exist"**: The init SQL only runs on first Postgres startup. Create manually:
```bash
docker exec gh-agents-postgres-1 psql -U nexor -c "CREATE DATABASE juicefs"
```

**JuiceFS container won't start**: Check that the `juicefs` database exists and that the format step completed. Run `./scripts/juicefs-format.sh --docker` if needed.

**Agent containers can't see workspace files**: The `rshared` mount propagation must be working. Check `ls ./jfs/` on the host — if empty, the propagation isn't working. Try restarting Docker Desktop.

**macFUSE not loading (host mode)**: Reboot after install, then allow the kernel extension in System Settings > Privacy & Security.
