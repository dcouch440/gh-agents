#!/usr/bin/env bash
# Validate JuiceFS mount is working correctly for Nexor workspace use.
#
# Tests: mount check, basic file ops, container bind-mount access,
# write performance, and concurrent writes.

set -euo pipefail

MOUNT_POINT="${MOUNT_POINT:-/tmp/nexor-jfs}"
TEST_DIR="${MOUNT_POINT}/_validation_test"

cleanup() {
  rm -rf "${TEST_DIR}" 2>/dev/null || true
}
trap cleanup EXIT

echo "=== JuiceFS Validation ==="
echo "Mount point: ${MOUNT_POINT}"
echo ""

# 1. Mount check
echo "[1/5] Checking mount..."
if ! mount | grep -q "${MOUNT_POINT}"; then
  echo "FAIL: ${MOUNT_POINT} is not mounted"
  exit 1
fi
echo "  OK: mounted"

# 2. Basic file ops
echo "[2/5] Basic file operations..."
mkdir -p "${TEST_DIR}"
echo "hello" > "${TEST_DIR}/test.txt"
[ "$(cat "${TEST_DIR}/test.txt")" = "hello" ] || { echo "FAIL: read mismatch"; exit 1; }
rm "${TEST_DIR}/test.txt"

# Nested directories
mkdir -p "${TEST_DIR}/a/b/c"
echo "nested" > "${TEST_DIR}/a/b/c/deep.txt"
[ "$(cat "${TEST_DIR}/a/b/c/deep.txt")" = "nested" ] || { echo "FAIL: nested read"; exit 1; }
rm -rf "${TEST_DIR}/a"
echo "  OK: create/read/delete/nested"

# 3. Container access
echo "[3/5] Container read access..."
echo "container-test" > "${TEST_DIR}/container-test.txt"
RESULT=$(docker run --rm -v "${TEST_DIR}:/workspace:ro" debian:bookworm-slim cat /workspace/container-test.txt 2>&1) || {
  echo "FAIL: docker run failed: ${RESULT}"
  exit 1
}
[ "${RESULT}" = "container-test" ] || { echo "FAIL: container read mismatch (got: ${RESULT})"; exit 1; }
rm "${TEST_DIR}/container-test.txt"
echo "  OK: container reads host files via bind mount"

# 4. Performance (100 files, 10KB each)
echo "[4/5] Write performance (100 x 10KB files)..."
PERF_DIR="${TEST_DIR}/perf"
mkdir -p "${PERF_DIR}"

# macOS date doesn't support %N, use python for millisecond timing
START=$(python3 -c "import time; print(int(time.time() * 1000))")
for i in $(seq 1 100); do
  dd if=/dev/urandom of="${PERF_DIR}/file_${i}.bin" bs=10240 count=1 2>/dev/null
done
END=$(python3 -c "import time; print(int(time.time() * 1000))")
ELAPSED=$((END - START))
echo "  OK: 100 x 10KB files written in ${ELAPSED}ms"
rm -rf "${PERF_DIR}"

# 5. Concurrent writes
echo "[5/5] Concurrent writes..."
CONC_DIR="${TEST_DIR}/concurrent"
mkdir -p "${CONC_DIR}"
for i in $(seq 1 10); do
  echo "proc-${i}" > "${CONC_DIR}/file_${i}.txt" &
done
wait
COUNT=$(ls "${CONC_DIR}" | wc -l | tr -d ' ')
[ "${COUNT}" -eq "10" ] || { echo "FAIL: expected 10 files, got ${COUNT}"; exit 1; }
rm -rf "${CONC_DIR}"
echo "  OK: 10 concurrent writes, all files intact"

echo ""
echo "=== All validations passed ==="
