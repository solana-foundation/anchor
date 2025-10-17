#!/bin/bash

set -e

echo "Test 1: Running FAIL case (expects compilation error)..."
cd fail
if cargo build 2>&1 | grep -q "expects MORE args"; then
    echo "PASS: Fail case correctly caught parameter mismatch at compile time"
else
    echo "FAIL: Expected compilation error but build succeeded or wrong error"
    exit 1
fi
cd ..

echo "Test 2: Running PASS case (expects successful compilation)..."
cd pass
if cargo build 2>&1 | grep -q "Finished"; then
    echo "PASS: Pass case compiled successfully"
else
    echo "FAIL: Expected successful compilation but build failed"
    exit 1
fi
cd ..

