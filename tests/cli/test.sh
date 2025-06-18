#!/bin/sh

set -e -u

script_dir=$(realpath "$(dirname "${0}")")
workspace_dir=$(realpath "${script_dir}/../../")
expected_dir="${script_dir}/expected"
initialize_dir="${script_dir}/initialize"
output_dir="${script_dir}/output"

anchor_cli() {
  "${workspace_dir}/target/debug/anchor" "$@" 2>&1
}

setup_test() {
  test_dir="${output_dir}/${1}"
  rm -rf "${test_dir}"
  cp -r "${initialize_dir}/${1}" "${test_dir}"
  cd "${test_dir}"
}

patch_program() {
  program="${1}"
  rm -r "${program}/app" "${program}/target"
}

patch_program_id() {
  program_name="${1}"
  program_rust_name=$(printf "%s" "${program_name}" | sed "s/-/_/g")
  program_regex=$(printf "%s" "${program_name}" | sed "s/-/./g")

  new_program_id="${2:-aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x}"

  # fix declare_id!()
  for f in *"/programs/${program_name}/src/lib.rs"; do
    [ -f "${f}" ] || continue
    sed -i "s/declare_id!.*/declare_id!(\"${new_program_id}\");/" \
      "${f}"
  done

  # fix Anchor.toml
  for f in *"/Anchor.toml"; do
    [ -f "${f}" ] || continue
    sed -i "s/\(${program_regex}\) = .*/\1 = \"${new_program_id}\"/" \
      "${f}"
  done

  # delete keypair, if exists
  rm -f *"/target/deploy/${program_rust_name}-keypair.json"
}

script_exit_code=0
diff_test() {
  test_name="${1}"
  test_output="${2}"
  test_exit_code="${3}"
  diff_output=$(
    diff -u -r \
      "${expected_dir}/${test_name}" \
      "${output_dir}/${test_name}" \
        2>&1
  ) || diff_exit_code="$?" 

  if [ "${diff_exit_code:-0}" = "0" ] && [ "${test_exit_code}" = "0" ]; then
    echo "test ${test_name} passed"
  else
    echo
    echo "test ${test_name} failed with code ${test_exit_code}"
    echo "----- output ----"
    echo "${test_output}"
    echo "----- diff ----"
    echo "${diff_output}"
    echo "----- end -----"
    echo
    exit_code=1
  fi
}

# build binary
(
  cd "${workspace_dir}"
  cargo build -p anchor-cli --bin anchor
)

# init
# (
#   setup_test init
#   output=$(
#     anchor_cli init test-program --no-install --no-git
#     patch_program test-program
#     patch_program_id test-program
#   ) || exit_code="$?"
#   diff_test init "${output}" "${exit_code:-0}"
# )

# new
# (
#   setup_test new
#   output=$(
#     (
#       cd test-program
#       anchor_cli new another-program --solidity
#     )
#     patch_program test-program
#     patch_program_id another-program bbHgTM8c4goW91FVeYMUUE8bQgGaqNZLNRLaoK4HqnJ
#   )
#   diff_test new "${output}" "$?"
# )

# build # just check binary, commit the generated idl for expected
# (
#   setup_test build
#   output=$(
#     (
#       cd test-program
#       anchor_cli build

#       if [ -f target/deploy/test_program.so ]; then
#         echo "test build passed"
#       else
#         echo "test build failed"
#       fi
#     )
#   )
# )

# clean
(
  setup_test clean
  output=$(
    (
      cd test-program
      anchor_cli build

      # Check that target exists and is non-empty
      if [ -d target ] && [ "$(ls -A target)" ]; then
        echo "target exists and is not empty before clean"
      else
        echo "target is missing or empty before clean"
      fi

      # Run anchor clean
      anchor_cli clean

      rm Cargo.lock

      # Check that only test_program-keypair.json exists
      if [ "$(find target -type f | wc -l)" -eq 1 ] && [ -f target/deploy/test_program-keypair.json ]; then
        echo "clean successful: only test_program-keypair.json exists"
      else
        echo "clean failed or unexpected files remain in target"
        find target
      fi
    )
  )
  
  # Remove the keypair file from both directories before diff comparison since it's randomly generated
  rm -f "${output_dir}/clean/test-program/target/deploy/test_program-keypair.json"
  rm -f "${expected_dir}/clean/test-program/target/deploy/test_program-keypair.json"
  
  diff_test clean "${output}" "$?"
)




# build

# airdrop
# cluster
(
  expected_output="Cluster Endpoints:

* Mainnet - https://api.mainnet-beta.solana.com
* Devnet  - https://api.devnet.solana.com
* Testnet - https://api.testnet.solana.com"
  output=$(
    anchor_cli cluster list
  ) || exit_code="$?"

  echo "${expected_output}" > "${output_dir}/expected_cluster.txt"
  echo "${output}" > "${output_dir}/actual_cluster.txt"
  if diff_output=$(diff "${output_dir}/expected_cluster.txt" "${output_dir}/actual_cluster.txt"); then
    echo "test cluster passed"
  else
    echo "test cluster failed"
    echo "----- diff ----"
    echo "${diff_output}"
    echo "----- end -----"
    script_exit_code=1
  fi
)


# idl
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

exit "${script_exit_code}"
