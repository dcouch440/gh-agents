# JuiceFS Workspace Setup

JuiceFS provides the POSIX filesystem backing for agent workspaces. It stores metadata in Postgres and file data in MinIO (S3-compatible).

## Prerequisites

```bash
# JuiceFS binary
brew install juicedata/tap/juicefs

# macFUSE (required for FUSE mounts on macOS)
brew install --cask macfuse

# MinIO client (for bucket creation)
brew install minio/stable/mc
```

After installing macFUSE, you may need to reboot and allow the kernel extension in System Settings > Privacy & Security.

## Setup

### 1. Start infrastructure

```bash
docker-compose up -d
```

This creates the `juicefs` database automatically on first Postgres startup (via `docker/init-juicefs-db.sql`).

**Existing dev environments** (Postgres volume already initialized):

```bash
docker exec gh-agents-postgres-1 psql -U nexor -c "CREATE DATABASE juicefs"
```

### 2. Format the volume (one-time)

```bash
./scripts/juicefs-format.sh
```

Creates the `juicefs-data` MinIO bucket and formats the JuiceFS volume `nexor-workspace`.

### 3. Mount

```bash
./scripts/juicefs-mount.sh
```

Mounts JuiceFS at `/tmp/nexor-jfs` (configurable via `MOUNT_POINT` env var).

### 4. Validate

```bash
./scripts/juicefs-validate.sh
```

Runs 5 checks: mount status, file ops, container bind-mount access, write performance, and concurrent writes.

### 5. Unmount (when done)

```bash
./scripts/juicefs-umount.sh
```

## Configuration

| Env Var | Default | Description |
|---------|---------|-------------|
| `WORKSPACE_MOUNT_POINT` | `/tmp/nexor-jfs` | JuiceFS mount path on host |
| `PG_HOST` | `localhost` | Postgres host (scripts only) |
| `PG_PORT` | `5432` | Postgres port (scripts only) |
| `PG_USER` | `nexor` | Postgres user (scripts only) |
| `PG_PASS` | `nexor` | Postgres password (scripts only) |
| `PG_DB` | `juicefs` | JuiceFS metadata database (scripts only) |
| `MINIO_ENDPOINT` | `http://localhost:9000` | MinIO endpoint (scripts only) |

## Directory Layout

```
/tmp/nexor-jfs/                          <- mount point
  workflows/
    {workflow_id}/
      runs/
        {run_id}/                        <- per-run workspace root
          ...agent-produced files...
      pinned/                            <- sealed files (future)
```

## Troubleshooting

**"mount point does not exist"**: Run `mkdir -p /tmp/nexor-jfs` or check that the mount script ran.

**"database juicefs does not exist"**: The init SQL only runs on first Postgres startup. Create it manually:
```bash
docker exec gh-agents-postgres-1 psql -U nexor -c "CREATE DATABASE juicefs"
```

**macFUSE not loading**: Reboot after install, then allow the kernel extension in System Settings > Privacy & Security.

**Permission denied on mount**: macFUSE requires the current user to have access. Check `ls -la /tmp/nexor-jfs`.
