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
# cargo install --git https://github.com/project-serum/anchor anchor-cli --locked
#
################################################################################

set -euox pipefail

main() {
    #
    # Bootup validator.
    #
    solana-test-validator > test-validator.log &
    sleep 5

    #
    # Deploy programs.
    #
    pushd ../../examples/composite/
    anchor build
    anchor deploy
    local composite_pid=$(cat target/idl/composite.json | jq -r .metadata.address)
    popd
    pushd ../../examples/tutorial/basic-2/
    anchor build
    anchor deploy
    local basic_2_pid=$(cat target/idl/basic_2.json | jq -r .metadata.address)
    popd
    pushd ../../examples/tutorial/basic-4/
    anchor build
    anchor deploy
    local basic_4_pid=$(cat target/idl/basic_4.json | jq -r .metadata.address)
    popd
    pushd ../../examples/events
    anchor build
    anchor deploy
    local events_pid=$(cat target/idl/events.json | jq -r .metadata.address)
    popd

    #
    # Run Test.
    #
    cargo run -- --composite-pid $composite_pid --basic-2-pid $basic_2_pid --basic-4-pid $basic_4_pid --events-pid $events_pid
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

declare -f -t trap_add
trap_add 'cleanup' EXIT
main
