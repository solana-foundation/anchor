#!/bin/bash

set +e  # Don't exit on error

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

total=0
passed=0
failed=0

echo "Running reload checker tests..."
echo ""

# Test bad examples (should fail with reload errors)
for dir in bad-one bad-two double-cpi-test; do
    if [ -d "$dir" ]; then
        total=$((total + 1))
        cd "$dir"
        output=$(anchor build 2>&1)
        if echo "$output" | grep -q "Missing reload"; then
            echo "✓ $dir: failed as expected"
            passed=$((passed + 1))
        else
            echo "✗ $dir: should have failed with reload error"
            failed=$((failed + 1))
        fi
        cd ..
    fi
done

# Test good examples (should pass)
for dir in good-one good-two multi-cpi-test; do
    if [ -d "$dir" ]; then
        total=$((total + 1))
        cd "$dir"
        output=$( anchor build 2>&1)
        if echo "$output" | grep -q "Finished"; then
            echo "✓ $dir: passed"
            passed=$((passed + 1))
        else
            echo "✗ $dir: should have passed"
            failed=$((failed + 1))
        fi
        cd ..
    fi
done

echo ""
echo "Results: $passed/$total passed"

if [ $failed -gt 0 ]; then
    exit 1
fi

