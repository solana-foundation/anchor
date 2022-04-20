#!/bin/bash

# Write a keypair for program deploy
mkdir -p target/deploy
cp keypairs/idl_commands_one-keypair.json target/deploy

echo "Building programs"

anchor build

echo "Starting local validator for test"

solana-test-validator --reset \
  -q \
  --mint tgyXxAhCkpgtKCEi4W6xWJSzqwVGs3uk2RodbZP2J49 \
  --bpf-program 2uA3amp95zsEHUpo8qnLMhcFAUsiKVEcKHXS1JetFjU5 target/deploy/idl_commands_one.so \
  --bpf-program DE4UbHnAcT6Kfh1fVTPRPwpiA3vipmQ4xR3gcLwX3wwS target/deploy/idl_commands_one.so \
  &

sleep 10

echo "Running tests"

anchor test --skip-deploy --skip-local-validator

trap "trap - SIGTERM && kill -- -$$" SIGINT SIGTERM EXIT
