#!/bin/bash
set -e

cd "$(dirname "$0")"

# Test 1: Program with #[account(mut)] should compile
cargo check --manifest-path programs/mut-compile-check/Cargo.toml >/dev/null 2>&1 || {
    exit 1
}

# Test 2: Program with ReadOnlyAccount mutation should fail to compile
COMPILE_OUTPUT=$(cargo check --manifest-path programs/mut-compile-check-fail/Cargo.toml 2>&1 || true)
if ! echo "$COMPILE_OUTPUT" | grep -qE "error\[E0594\]|error\[E0599\]"; then
    echo "$COMPILE_OUTPUT"
    exit 1
fi

# Test 3: Old behavior (Account without #[account(mut)]) should compile
cargo check --manifest-path programs/mut-compile-check-old-behavior/Cargo.toml >/dev/null 2>&1 || {
    exit 1
}
