#!/bin/bash
# Deployment and testing script - combines steps 4-6 of iteration workflow
# Runs: deploy, endpoint testing, log checking
# Usage: ./scripts/deploy-test.sh

set -e

WORKER_URL="https://iso.rob-gilks.workers.dev"
LOG_CHECK_DURATION=${1:-10}

echo "🚀 Deployment and Testing Workflow"
echo "==================================="
echo ""

# Step 4: Deploy to Cloudflare
echo "4️⃣  Deploying to Cloudflare..."
if npx wrangler deploy; then
    echo "   ✅ Deployment successful"
else
    echo "   ❌ Deployment failed"
    exit 1
fi
echo ""

# Wait a moment for deployment to propagate
echo "   ⏳ Waiting for deployment to propagate..."
sleep 3
echo ""

# Step 5: Test deployed endpoints
echo "5️⃣  Testing deployed endpoints..."

# Test root endpoint
echo "   Testing root endpoint (/)..."
ROOT_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" "${WORKER_URL}/" || echo "000")
if [ "${ROOT_RESPONSE}" = "200" ]; then
    echo "   ✅ Root endpoint responding"
else
    echo "   ❌ Root endpoint failed (HTTP ${ROOT_RESPONSE})"
    exit 1
fi

# Test create endpoint
echo "   Testing /create endpoint..."
CREATE_RESPONSE=$(curl -s "${WORKER_URL}/create" || echo "")
if echo "${CREATE_RESPONSE}" | grep -q "Match created:"; then
    MATCH_CODE=$(echo "${CREATE_RESPONSE}" | grep -oE "Match created: [A-Z0-9]{5}" | cut -d' ' -f3)
    echo "   ✅ Create endpoint working (match: ${MATCH_CODE})"
    
    # Test join endpoint with the created match
    if [ -n "${MATCH_CODE}" ]; then
        echo "   Testing /join/${MATCH_CODE} endpoint..."
        JOIN_RESPONSE=$(curl -s "${WORKER_URL}/join/${MATCH_CODE}" || echo "")
        if echo "${JOIN_RESPONSE}" | grep -q "Match.*found"; then
            echo "   ✅ Join endpoint working"
        else
            echo "   ⚠️  Join endpoint response unexpected: ${JOIN_RESPONSE}"
        fi
    fi
else
    echo "   ❌ Create endpoint failed or unexpected response"
    exit 1
fi
echo ""

# Step 6: Check Cloudflare logs
echo "6️⃣  Checking Cloudflare logs (${LOG_CHECK_DURATION} seconds)..."
if ./scripts/check-logs.sh "${LOG_CHECK_DURATION}"; then
    echo "   ✅ No errors in logs"
else
    echo "   ⚠️  Errors detected in logs (see above)"
    # Don't exit 1 here - log errors might be expected during testing
    # But we should still report them
fi
echo ""

echo "✅ Deployment and testing complete!"
echo ""
echo "Next steps:"
echo "  - Review any log warnings/errors above"
echo "  - Update documentation if needed"
echo "  - Commit and push changes"
exit 0

