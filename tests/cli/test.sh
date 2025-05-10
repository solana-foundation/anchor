#!/bin/sh

set -e -u -o pipefail

script_dir=$(dirname "${0}")
output_dir="${script_dir}/output"

anchor_cli(){
  cargo run -p anchor-cli --bin anchor "$@"
}

setup_test() {
  test_dir="${output_dir}/${1}"
  rm -rf "${test_dir}"
  mkdir -p "${test_dir}"
  cd "${test_dir}"
}

diff_test() {
  actual_dir="${output_dir}/${1}"
  expected_dir="${script_dir}/${1}"
  diff --oneline -r "${actual_dir}" "${expected_dir}"
}

# init
{
  setup_test init
  anchor_cli init test-program --no-install
  diff_test init
}

# new

# build
# clean
# build

# airdrop
# cluster

# idl
# run
# test
# deploy
# keys
# account

# expand
# verify
# migrate
# upgrade
# shell
# login
# publish
# localnet
