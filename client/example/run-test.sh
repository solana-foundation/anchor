#!/bin/bash

################################################################################
#
# A script to run the example as an integration test. It starts up a localnet
# and executes the current directory's rust binary.
#
# Usage:
#
# ./run.sh
#
# Run this script from within the `example/` directory in which it is located.
# The anchor cli must be installed.
#
# cargo install --git https://github.com/coral-xyz/anchor anchor-cli --locked
#
################################################################################

set -euox pipefail

main() {
    #
    # Build programs.
    #
    local composite_pid="EHthziFziNoac9LBGxEaVN47Y3uUiRoXvqAiR6oes4iU"
    local basic_2_pid="Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
    local basic_4_pid="CwrqeMj2U8tFr1Rhkgwc84tpAsqbt9pTt2a4taoTADPr"
    local events_pid="2dhGsWUzy5YKUsjZdLHLmkNpUDAXkNa9MYWsPc4Ziqzy"
    local optional_pid="FNqz6pqLAwvMSds2FYjR4nKV3moVpPNtvkfGFrqLKrgG"

    cd ../../tests/composite && anchor build && cd -
    [ $? -ne 0 ] && exit 1
    cd ../../examples/tutorial/basic-2 && anchor build && cd -
    [ $? -ne 0 ] && exit 1
    cd ../../examples/tutorial/basic-4 && anchor build && cd -
    [ $? -ne 0 ] && exit 1
    cd ../../tests/events && anchor build && cd -
    [ $? -ne 0 ] && exit 1
    cd ../../tests/optional && anchor build && cd -
    [ $? -ne 0 ] && exit 1

    #
    # Bootup validator.
    #
    solana-test-validator -r \
				--bpf-program $composite_pid ../../tests/composite/target/deploy/composite.so \
				--bpf-program $basic_2_pid ../../examples/tutorial/basic-2/target/deploy/basic_2.so \
				--bpf-program $basic_4_pid ../../examples/tutorial/basic-4/target/deploy/basic_4.so \
				--bpf-program $events_pid ../../tests/events/target/deploy/events.so \
				--bpf-program $optional_pid ../../tests/optional/target/deploy/optional.so \
				> test-validator.log &
    test_validator_pid=$!

    sleep 5
    check_solana_validator_running $test_validator_pid

    #
    # Run single threaded test.
    #
    cargo run -- \
        --composite-pid $composite_pid \
        --basic-2-pid $basic_2_pid \
        --basic-4-pid $basic_4_pid \
        --events-pid $events_pid \
        --optional-pid $optional_pid

    #
    # Restart validator for multithreaded test
    #
    cleanup
    solana-test-validator -r \
				--bpf-program $composite_pid ../../tests/composite/target/deploy/composite.so \
				--bpf-program $basic_2_pid ../../examples/tutorial/basic-2/target/deploy/basic_2.so \
				--bpf-program $basic_4_pid ../../examples/tutorial/basic-4/target/deploy/basic_4.so \
				--bpf-program $events_pid ../../tests/events/target/deploy/events.so \
				--bpf-program $optional_pid ../../tests/optional/target/deploy/optional.so \
				> test-validator.log &
    test_validator_pid=$!

    sleep 5
    check_solana_validator_running $test_validator_pid

    #
    # Run multi threaded test.
    #
    cargo run -- \
        --composite-pid $composite_pid \
        --basic-2-pid $basic_2_pid \
        --basic-4-pid $basic_4_pid \
        --events-pid $events_pid \
        --optional-pid $optional_pid \
        --multithreaded

    #
    # Restart validator for async test
    #
    cleanup
    solana-test-validator -r \
				--bpf-program $composite_pid ../../tests/composite/target/deploy/composite.so \
				--bpf-program $basic_2_pid ../../examples/tutorial/basic-2/target/deploy/basic_2.so \
				--bpf-program $basic_4_pid ../../examples/tutorial/basic-4/target/deploy/basic_4.so \
				--bpf-program $events_pid ../../tests/events/target/deploy/events.so \
				--bpf-program $optional_pid ../../tests/optional/target/deploy/optional.so \
				> test-validator.log &
    test_validator_pid=$!

    sleep 5
    check_solana_validator_running $test_validator_pid

    #
    # Run async test.
    #
    cargo run --features async -- \
        --composite-pid $composite_pid \
        --basic-2-pid $basic_2_pid \
        --basic-4-pid $basic_4_pid \
        --events-pid $events_pid \
        --optional-pid $optional_pid \
        --multithreaded

}

cleanup() {
    pkill -P $$ || true
    wait || true
}

trap_add() {
    trap_add_cmd=$1; shift || fatal "${FUNCNAME} usage error"
    for trap_add_name in "$@"; do
        trap -- "$(
            extract_trap_cmd() { printf '%s\n' "${3:-}"; }
            eval "extract_trap_cmd $(trap -p "${trap_add_name}")"
            printf '%s\n' "${trap_add_cmd}"
        )" "${trap_add_name}" \
            || fatal "unable to add to trap ${trap_add_name}"
    done
}

check_solana_validator_running() {
    local pid=$1
    exit_state=$(kill -0 "$pid" && echo 'living' || echo 'exited')
    if [ "$exit_state" == 'exited' ]; then
        echo "Cannot start test validator, see ./test-validator.log"
        exit 1
    fi
}

declare -f -t trap_add
trap_add 'cleanup' EXIT
main
