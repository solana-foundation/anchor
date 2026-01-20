#!/bin/bash
set +e

cd "$(dirname "$0")"

# Test 1: With #[account(mut)] - should compile
anchor build --program-name mut-compile-check >/dev/null 2>&1
if [ $? -ne 0 ]; then
    echo "FAIL: Program with #[account(mut)] should compile"
    exit 1
fi

# Test 2: Without #[account(mut)] - should fail
anchor build --program-name mut-compile-check-fail 2>&1 | grep -qE "error\[E0594\]|error\[E0599\].*set_inner"
if [ $? -ne 0 ]; then
    echo "FAIL: Program without #[account(mut)] should fail to compile"
    exit 1
fi

# Test 3: Old behavior (uses Anchor 0.32.1) - should compile (demonstrates the bug)
anchor build --program-name mut-compile-check-old-behavior >/dev/null 2>&1
if [ $? -ne 0 ]; then
    echo "FAIL: Old behavior program should compile with Anchor 0.32.1 (demonstrates the bug)"
    exit 1
fi
