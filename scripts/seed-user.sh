#!/usr/bin/env bash
# Create the first user account against a running nexor backend.
#
# There's no signup UI — the login page only logs in. This hits
# POST /api/auth/register directly so a fresh checkout has an account
# to log in with.
#
# Usage:
#   ./scripts/seed-user.sh
#   EMAIL=me@example.com PASSWORD=supersecret1 ./scripts/seed-user.sh

set -euo pipefail

API_URL="${API_URL:-http://localhost:3000}"
EMAIL="${EMAIL:-dev@nexor.local}"
PASSWORD="${PASSWORD:-development}"

if [ "${#PASSWORD}" -lt 8 ]; then
  echo "FAIL: PASSWORD must be at least 8 characters" >&2
  exit 1
fi

echo "Registering ${EMAIL} at ${API_URL}..."

RESPONSE=$(curl -s -w '\n%{http_code}' -X POST "${API_URL}/api/auth/register" \
  -H 'Content-Type: application/json' \
  -d "{\"email\":\"${EMAIL}\",\"password\":\"${PASSWORD}\"}")

STATUS="${RESPONSE##*$'\n'}"
BODY="${RESPONSE%$'\n'*}"

case "${STATUS}" in
  201)
    echo "  OK: account created"
    echo ""
    echo "Log in at the frontend with:"
    echo "  email:    ${EMAIL}"
    echo "  password: ${PASSWORD}"
    ;;
  409)
    echo "  ${EMAIL} is already registered — log in with the password you set for it."
    ;;
  *)
    echo "FAIL: registration failed (HTTP ${STATUS})" >&2
    echo "${BODY}" >&2
    exit 1
    ;;
esac
