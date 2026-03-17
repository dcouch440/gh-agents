#!/usr/bin/env bash
# Unmount the JuiceFS volume from the host.

set -euo pipefail

MOUNT_POINT="${MOUNT_POINT:-/tmp/nexor-jfs}"

if ! mount | grep -q "${MOUNT_POINT}"; then
  echo "JuiceFS is not mounted at ${MOUNT_POINT}"
  exit 0
fi

juicefs umount "${MOUNT_POINT}"
echo "JuiceFS unmounted from ${MOUNT_POINT}"
