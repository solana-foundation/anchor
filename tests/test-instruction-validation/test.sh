#!/bin/bash

set -e

echo "Test 1: Running FAIL-ARGS-COUNT case (expects compilation error)..."
cd fail-args-count
output=$(anchor build --ignore-keys 2>&1) || true
if echo "$output" | grep -q "expects MORE args"; then
    echo "PASS: FAIL-ARGS-COUNT case correctly caught parameter mismatch at compile time"
else
    echo "FAIL: Expected compilation error but build succeeded or wrong error"
    echo "$output"
    exit 1
fi
cd ..

echo "Test 2: Running PASS-ARGS-COUNT case (expects successful compilation)..."
cd pass-args-count
output=$(anchor build --ignore-keys 2>&1) || true
if echo "$output" | grep -q "Finished"; then
    echo "PASS: PASS-ARGS-COUNT case compiled successfully"
else
    echo "FAIL: Expected successful compilation but build failed"
    echo "$output"
    exit 1
fi
cd ..

echo "Test 3: Running FAIL-TYPE case (expects compilation error)..."
cd fail-type
output=$(anchor build --ignore-keys 2>&1) || true
if echo "$output" | grep -q "IsSameType"; then
    echo "PASS: Fail-TYPE case correctly caught type mismatch at compile time"
else
    echo "FAIL: Expected compilation error but build succeeded or wrong error"
    echo "$output"
    exit 1
fi
cd ..

echo "Test 4: Running PASS-TYPE case (expects successful compilation)..."
cd pass-type
output=$(anchor build --ignore-keys 2>&1) || true
if echo "$output" | grep -q "Finished"; then
    echo "PASS: PASS-TYPE case compiled successfully"
else
    echo "FAIL: Expected successful compilation but build failed"
    echo "$output"
    exit 1
fi
cd ..

echo "Test 5: Running PASS-PARTIAL-ARGS case (in-order partial args with runtime tests)..."
cd pass-partial-args
output=$(anchor test --ignore-keys 2>&1) || true
if echo "$output" | grep -q "passing"; then
    echo "PASS: PASS-PARTIAL-ARGS compiled and all runtime tests passed"
else
    echo "FAIL: Expected all tests to pass"
    echo "$output"
    exit 1
fi
cd ..
