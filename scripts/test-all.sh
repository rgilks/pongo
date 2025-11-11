#!/bin/bash
# Unified test script - combines steps 1-2 of iteration workflow
# Runs: cargo fmt, cargo check, cargo clippy, cargo test
# Usage: ./scripts/test-all.sh

set -e

echo "🧪 Running unified test suite..."
echo "================================"
echo ""

# Step 1: Format check
echo "1️⃣  Checking code formatting..."
if cargo fmt --check; then
    echo "   ✅ Code is properly formatted"
else
    echo "   ❌ Code formatting issues found"
    echo "   Run 'cargo fmt' to fix"
    exit 1
fi
echo ""

# Step 2: Compilation check
echo "2️⃣  Checking compilation..."
if cargo check --workspace; then
    echo "   ✅ Code compiles successfully"
else
    echo "   ❌ Compilation errors found"
    exit 1
fi
echo ""

# Step 3: Clippy check
echo "3️⃣  Running clippy (linter)..."
if cargo clippy --workspace -- -D warnings; then
    echo "   ✅ No clippy warnings or errors"
else
    echo "   ❌ Clippy found warnings or errors"
    exit 1
fi
echo ""

# Step 4: Run tests
echo "4️⃣  Running unit tests..."
if cargo test --workspace; then
    echo "   ✅ All tests passed"
else
    echo "   ❌ Some tests failed"
    exit 1
fi
echo ""

echo "✅ All checks passed! Ready for next steps."
exit 0

