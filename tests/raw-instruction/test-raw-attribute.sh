#!/bin/bash

set -e

FAIL_PROGRAM="test-programs/raw-instruction-fail"
PASS_PROGRAM="programs/raw-instruction"

cd "$FAIL_PROGRAM"
if anchor build --ignore-keys 2>&1 | grep -q "Functions with &\[u8\] or &mut \[u8\] arguments must be marked with #\[raw\] attribute"; then
    echo "PASS: Wrong program correctly fails"
else
    echo "FAIL: Wrong program did not fail as expected"
    exit 1
fi
cd - > /dev/null

if anchor build --program-name raw-instruction --ignore-keys 2>&1 | tail -3 | grep -q "Finished"; then
    echo "PASS: Right program builds successfully"
else
    echo "FAIL: Right program build failed"
    exit 1
fi

if anchor test --skip-build 2>&1 | grep -q "passing"; then
    echo "PASS: All tests pass"
else
    echo "FAIL: Some tests failed"
    exit 1
fi

if node validate.js | grep -q "passed"; then
    echo "PASS: IDL validation passed"
else
    echo "FAIL: IDL validation failed"
    exit 1
fi
