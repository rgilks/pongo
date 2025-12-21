#!/bin/bash
# Check Cloudflare Workers logs for errors and warnings
# Usage: ./scripts/check-logs.sh [duration_seconds]

set -e

DURATION=${1:-10}
WORKER_NAME="pongo"

echo "🔍 Checking Cloudflare Workers logs for ${DURATION} seconds..."
echo "Press Ctrl+C to stop early"
echo ""

# Start tailing logs in background and capture output
LOG_FILE=$(mktemp)
npx wrangler tail "${WORKER_NAME}" --format json > "${LOG_FILE}" 2>&1 &
TAIL_PID=$!

# Wait for specified duration
sleep "${DURATION}"

# Kill the tail process
kill "${TAIL_PID}" 2>/dev/null || true
wait "${TAIL_PID}" 2>/dev/null || true

echo ""
echo "📊 Log Summary:"
echo "================"

# Count different log types
ERROR_COUNT=$(grep -c '"outcome":"error"' "${LOG_FILE}" 2>/dev/null || echo "0")
OK_COUNT=$(grep -c '"outcome":"ok"' "${LOG_FILE}" 2>/dev/null || echo "0")
EXCEPTION_COUNT=$(grep -ci "exception\|error\|panic" "${LOG_FILE}" 2>/dev/null || echo "0")

echo "✅ Successful requests: ${OK_COUNT}"
echo "❌ Failed requests: ${ERROR_COUNT}"
echo "⚠️  Exceptions/Errors found: ${EXCEPTION_COUNT}"

if [ "${ERROR_COUNT}" -gt 0 ] || [ "${EXCEPTION_COUNT}" -gt 0 ]; then
    echo ""
    echo "🔴 ERRORS DETECTED:"
    echo "==================="
    grep -i "exception\|error\|panic" "${LOG_FILE}" | head -10 || true
    echo ""
    echo "Full log file: ${LOG_FILE}"
    exit 1
else
    echo ""
    echo "✅ No errors detected in logs"
    rm -f "${LOG_FILE}"
    exit 0
fi

