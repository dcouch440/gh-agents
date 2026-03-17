#!/usr/bin/env bash
# Format the JuiceFS volume using Postgres metadata + MinIO data.
# Run once after docker-compose up.
#
# Prerequisites:
#   - juicefs binary installed
#   - mc (MinIO client) installed
#   - Postgres running with 'juicefs' database
#   - MinIO running

set -euo pipefail

JUICEFS_BIN="${JUICEFS_BIN:-juicefs}"
PG_HOST="${PG_HOST:-localhost}"
PG_PORT="${PG_PORT:-5432}"
PG_USER="${PG_USER:-nexor}"
PG_PASS="${PG_PASS:-nexor}"
PG_DB="${PG_DB:-juicefs}"
MINIO_ENDPOINT="${MINIO_ENDPOINT:-http://localhost:9000}"
MINIO_ACCESS_KEY="${MINIO_ACCESS_KEY:-minioadmin}"
MINIO_SECRET_KEY="${MINIO_SECRET_KEY:-minioadmin}"
MINIO_BUCKET="${MINIO_BUCKET:-juicefs-data}"
VOLUME_NAME="${VOLUME_NAME:-nexor-workspace}"

META_URL="postgres://${PG_USER}:${PG_PASS}@${PG_HOST}:${PG_PORT}/${PG_DB}?sslmode=disable"
STORAGE_URL="http://localhost:9000/${MINIO_BUCKET}"

# Create MinIO bucket if it doesn't exist
echo "Ensuring MinIO bucket '${MINIO_BUCKET}' exists..."
mc alias set nexor-minio "${MINIO_ENDPOINT}" "${MINIO_ACCESS_KEY}" "${MINIO_SECRET_KEY}" 2>/dev/null || true
mc mb --ignore-existing "nexor-minio/${MINIO_BUCKET}"

# Format the volume
echo "Formatting JuiceFS volume '${VOLUME_NAME}'..."
"${JUICEFS_BIN}" format \
  --storage minio \
  --bucket "${STORAGE_URL}" \
  --access-key "${MINIO_ACCESS_KEY}" \
  --secret-key "${MINIO_SECRET_KEY}" \
  "${META_URL}" \
  "${VOLUME_NAME}"

echo "JuiceFS volume '${VOLUME_NAME}' formatted successfully."
