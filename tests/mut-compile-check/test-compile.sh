#!/bin/bash
# Test script for mut-compile-check
#
# This script verifies that:
# 1. Program with #[account(mut)] compiles successfully
# 2. Program with ReadOnlyAccount that tries to mutate FAILS to compile
# 3. Program demonstrating old behavior (Account without #[account(mut)]) compiles
#    (demonstrating the bug that ReadOnlyAccount solves)

set -e

cd "$(dirname "$0")"

echo "=== Mut Compile Check Tests ==="
echo ""

# Test 1: With #[account(mut)] and ReadOnlyAccount for reading - should compile
echo "Test 1: Program with correct usage (#[account(mut)] and ReadOnlyAccount)"
echo "        Expected: COMPILE SUCCESS"
cargo check --manifest-path programs/mut-compile-check/Cargo.toml 2>/dev/null
if [ $? -eq 0 ]; then
    echo "        Result: PASS ✓"
else
    echo "        Result: FAIL ✗"
    echo "        Program with #[account(mut)] should compile"
    exit 1
fi
echo ""

# Test 2: ReadOnlyAccount with attempted mutation - should fail
echo "Test 2: Program that tries to mutate ReadOnlyAccount"
echo "        Expected: COMPILE FAILURE (E0594 or E0599)"
COMPILE_OUTPUT=$(cargo check --manifest-path programs/mut-compile-check-fail/Cargo.toml 2>&1 || true)
if echo "$COMPILE_OUTPUT" | grep -qE "error\[E0594\]|error\[E0599\]"; then
    echo "        Result: PASS ✓ (correctly fails to compile)"
else
    echo "        Result: FAIL ✗"
    echo "        Program with ReadOnlyAccount mutation should fail to compile"
    echo "        Compile output:"
    echo "$COMPILE_OUTPUT"
    exit 1
fi
echo ""

# Test 3: Old behavior (Account without #[account(mut)]) - should compile
# This demonstrates the bug: mutation compiles but doesn't persist
echo "Test 3: Program demonstrating old behavior (Account without #[account(mut)])"
echo "        Expected: COMPILE SUCCESS (demonstrates the bug)"
echo "        Note: This is the problematic behavior that ReadOnlyAccount fixes"
cargo check --manifest-path programs/mut-compile-check-old-behavior/Cargo.toml 2>/dev/null
if [ $? -eq 0 ]; then
    echo "        Result: PASS ✓ (compiles, demonstrating the bug)"
else
    echo "        Result: FAIL ✗"
    echo "        Old behavior program should compile (to demonstrate the bug)"
    exit 1
fi
echo ""

echo "=== All tests passed! ==="
echo ""
echo "Summary:"
echo "  - Account with #[account(mut)] allows mutation (correct)"
echo "  - ReadOnlyAccount prevents mutation at compile time (fixed)"
echo "  - Account without #[account(mut)] still allows mutation (old behavior/bug)"
echo ""
echo "Users should use ReadOnlyAccount for accounts that shouldn't be mutated"
echo "to get compile-time safety instead of silent runtime failures."
