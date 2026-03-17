#!/usr/bin/env bash
# Mount the JuiceFS volume on the host.
# Requires: juicefs binary, macFUSE (macOS) or fuse3 (Linux).
#
# The mount point becomes the root of all workflow workspaces.
# Directory layout: {mount}/workflows/{workflow_id}/runs/{run_id}/

set -euo pipefail

JUICEFS_BIN="${JUICEFS_BIN:-juicefs}"
MOUNT_POINT="${MOUNT_POINT:-/tmp/nexor-jfs}"
PG_HOST="${PG_HOST:-localhost}"
PG_PORT="${PG_PORT:-5432}"
PG_USER="${PG_USER:-nexor}"
PG_PASS="${PG_PASS:-nexor}"
PG_DB="${PG_DB:-juicefs}"
CACHE_DIR="${CACHE_DIR:-/tmp/nexor-jfs-cache}"

META_URL="postgres://${PG_USER}:${PG_PASS}@${PG_HOST}:${PG_PORT}/${PG_DB}?sslmode=disable"

mkdir -p "${MOUNT_POINT}" "${CACHE_DIR}"

# Check if already mounted
if mount | grep -q "${MOUNT_POINT}"; then
  echo "JuiceFS already mounted at ${MOUNT_POINT}"
  exit 0
fi

echo "Mounting JuiceFS at ${MOUNT_POINT}..."

"${JUICEFS_BIN}" mount \
  --cache-dir "${CACHE_DIR}" \
  --cache-size 1024 \
  --buffer-size 64 \
  --prefetch 1 \
  -d \
  "${META_URL}" \
  "${MOUNT_POINT}"

echo "JuiceFS mounted at ${MOUNT_POINT}"
