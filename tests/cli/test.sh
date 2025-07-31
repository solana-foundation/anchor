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
# (
#   setup_test clean
#   output=$(
#     (
#       cd test-program
#       anchor_cli build

#       # Check that target exists and is non-empty
#       if [ -d target ] && [ "$(ls -A target)" ]; then
#         echo "target exists and is not empty before clean"
#       else
#         echo "target is missing or empty before clean"
#       fi

#       # Run anchor clean
#       anchor_cli clean

#       rm Cargo.lock

#       # Check that only test_program-keypair.json exists
#       if [ "$(find target -type f | wc -l)" -eq 1 ] && [ -f target/deploy/test_program-keypair.json ]; then
#         echo "clean successful: only test_program-keypair.json exists"
#       else
#         echo "clean failed or unexpected files remain in target"
#         find target
#       fi
#     )
#   )
  
#   # Remove the keypair file from both directories before diff comparison since it's randomly generated
#   rm -f "${output_dir}/clean/test-program/target/deploy/test_program-keypair.json"
#   rm -f "${expected_dir}/clean/test-program/target/deploy/test_program-keypair.json"
  
#   diff_test clean "${output}" "$?"
# )

# test
# (
#   setup_test test
  
#   # Set required environment variables for the test
#   export ANCHOR_PROVIDER_URL="http://127.0.0.1:8899"
#   export ANCHOR_WALLET="${workspace_dir}/tests/cli/keypairs/test-key.json"

#   cd test-program

#   # First build the program (redirect compilation output to /dev/null)
#   "${workspace_dir}/target/debug/anchor" build > /dev/null 2>&1

#   # Run test with local validator and capture output
#   test_output=$(timeout 60s "${workspace_dir}/target/debug/anchor" test 2>&1) || test_exit_code="$?"

#   # Check if test passed by looking for "1 passing" in the output
#   if echo "$test_output" | grep -q "1 passing"; then
#     echo "test test passed"
#   else
#     echo "test test failed"
#     echo "----- output ----"
#     echo "$test_output"
#     echo "----- end -----"
#     script_exit_code=1
#   fi
# )

# deploy
# (
#   setup_test deploy

#   # Set required environment variables for the deploy
#   export ANCHOR_PROVIDER_URL="http://127.0.0.1:8899"
#   export ANCHOR_WALLET="${workspace_dir}/tests/cli/keypairs/test-key.json"

#   cd test-program

#   # Build the program before deploying
#   build_output=$(anchor_cli build 2>&1) || build_exit_code="$?"

#   # Deploy the program to localnet
#   deploy_output=$(anchor_cli deploy 2>&1) || deploy_exit_code="$?"

#   # Check for 'Deploy success' in the deploy output
#   if echo "$deploy_output" | grep -q "Deploy success"; then
#     echo "test deploy passed"
#   else
#     echo "test deploy failed"
#     echo "----- output ----"
#     echo "$deploy_output"
#     echo "----- end -----"
#     script_exit_code=1
#   fi

#   # Clean up after deploy
#   anchor_cli clean > /dev/null 2>&1 || true
# )

# idl build
(
  setup_test idl

  # Set required environment variables
  export ANCHOR_PROVIDER_URL="http://127.0.0.1:8899"
  export ANCHOR_WALLET="${workspace_dir}/tests/cli/keypairs/test-key.json"

  cd test-program

# ------ start: build ------
# Build the IDL
#   idl_output=$(anchor_cli idl build 2>&1)
#   idl_exit_code="$?"

#   # Extract the JSON block from the output, ignoring build lines
#   idl_json=$(printf '%s\n' "$idl_output" | awk 'BEGIN{in_json=0} /^\s*\{/ {in_json=1} in_json{print} /^\s*\}/{if(in_json){exit}}')

#   if [ -z "$idl_json" ]; then
#     echo "test idl build failed: no JSON output found"
#     echo "----- output ----"
#     echo "$idl_output"
#     echo "----- end -----"
#     script_exit_code=1
#   fi

#   cat > expected_idl.json <<EOF
# {
#   "address": "aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x",
#   "metadata": {
#     "name": "test_program",
#     "version": "0.1.0",
#     "spec": "0.1.0",
#     "description": "Created with Anchor"
#   },
#   "instructions": [
#     {
#       "name": "initialize",
#       "discriminator": [
#         175,
#         175,
#         109,
#         31,
#         13,
#         152,
#         155,
#         237
#       ],
#       "accounts": [],
#       "args": []
#     }
#   ]
# }
# EOF

#   echo "$idl_json" > actual_idl.json

#   # remove whitespaces
#   jq . expected_idl.json > expected_idl.normalized.json
#   jq . actual_idl.json > actual_idl.normalized.json

#   if diff_output=$(diff -u expected_idl.normalized.json actual_idl.normalized.json); then
#     echo "test idl build passed"
#   else
#     echo "test idl build failed"
#     echo "----- diff ----"
#     echo "$diff_output"
#     echo "----- output ----"

#     printf '%s\n' "$idl_output" | grep -vE '^(\s*Compiling|\s*Finished|\s*Running|\s*Downloading|\s*Installing|\s*Updating)' || true
#     echo "----- end -----"
#     script_exit_code=1
#   fi

  # ------ end: build ------
  solana-test-validator --reset > validator.log 2>&1 &
  validator_pid=$!

  # Wait a bit for validator to start
  sleep 8


  anchor_cli deploy
  # wait for confirmation
  sleep 3

  # ------ start: init ------
  idl_init_output=$(anchor_cli idl init \
    --filepath target/idl/test_program.json \
    aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x 2>&1)
  idl_init_exit_code="$?"
  echo "Command exit code: $idl_init_exit_code"
  echo "Command output:"
  echo "$idl_init_output"

  if echo "$idl_init_output" | grep -q "Idl account created:"; then
    echo "test idl init passed"
  else
    echo "test idl init failed"
    echo "----- output ----"
    echo "$idl_init_output"
    echo "----- end -----"
    script_exit_code=1
  fi
  # ------ end: init ------

  # ------ start: fetch ------
  echo "testing fetch"
  idl_fetch_output=$(anchor_cli idl fetch \
    -o fetched_idl.json \
    aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x 2>&1)
  idl_fetch_exit_code="$?"
  echo "Fetch command exit code: $idl_fetch_exit_code"
  echo "Fetch command output:"
  echo "$idl_fetch_output"

  # Check if the fetched IDL file exists and has content
  if [ -s fetched_idl.json ]; then
    echo "test idl fetch passed"
  else
    echo "test idl fetch failed"
    echo "----- output ----"
    echo "$idl_fetch_output"
    echo "----- end -----"
    script_exit_code=1
  fi
  # ------ end: fetch ------

  # ----- start: authority ------
  idl_authority_output=$(anchor_cli idl authority \
  aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x 2>&1)
  idl_authority_exit_code="$?"
  echo "Command exit code: $idl_authority_exit_code"
  echo "Command output:"
  echo "$idl_authority_output"

  if echo "$idl_authority_output" | grep -q "9GSqbQeLFQaa49wfsKjxi4Q6v2xUH9pynowkTeCCG2Xr"; then
    echo "test idl authority passed"
  else
    echo "test idl authority failed"
    echo "----- output ----"
    echo "$idl_authority_output"
    echo "----- end -----"
    script_exit_code=1
  fi
  # ----- end: authority -----

  # ----- start: upgrade ------
  sed -i "s/Created with Anchor/Test Program/" target/idl/test_program.json

  sleep 3

  idl_upgrade_output=$(anchor_cli idl upgrade aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x \
  --filepath target/idl/test_program.json 2>&1)
  idl_upgrade_exit_code="$?"
  echo "Command exit code: $idl_upgrade_exit_code"
  echo "Command output:"
  echo "$idl_upgrade_output"

  if echo "$idl_upgrade_output" | grep -q "successfully upgraded"; then
    echo "test idl upgrade passed"
  else
    echo "test idl upgrade failed"
    echo "----- output ----"
    echo "$idl_upgrade_output"
    echo "----- end -----"
    script_exit_code=1
  fi
  # ----- end: upgrade -----

  # ----- start: erase-authority ------
  idl_erase_authority_output=$(echo "y" | anchor_cli idl erase-authority --program-id \
  aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x 2>&1)
  idl_erase_authority_exit_code="$?"
  echo "Command exit code: $idl_erase_authority_exit_code"
  echo "Command output:"
  echo "$idl_erase_authority_output"

  if echo "$idl_erase_authority_output" | grep -q "Authority update complete."; then
    echo "test idl erase-authority passed"
  else
    echo "test idl erase-authority failed"
    echo "----- output ----"
    echo "$idl_erase_authority_output"
    echo "----- end -----"
    script_exit_code=1
  fi
  # ----- end: erase-authority -----

  # Kill the validator
  kill $validator_pid || true
  # ------ end: build ------
)

  # ----- start: migrate ------
  setup_test idl

  cd test-program

  anchor_migrate_output=$(anchor_cli migrate 2>&1)
  anchor_migrate_exit_code="$?"
  echo "Command exit code: $anchor_migrate_exit_code"
  echo "Command output:"
  echo "$anchor_migrate_output"

  if echo "$anchor_migrate_output" | grep -q "Deploy complete."; then
    echo "test migrate passed"
  else
    echo "test idl migrate failed"
    echo "----- output ----"
    echo "$anchor_migrate_output"
    echo "----- end -----"
    script_exit_code=1
  fi
  # ----- end: migrate -----

# cluster
# (
#   expected_output="Cluster Endpoints:

# * Mainnet - https://api.mainnet-beta.solana.com
# * Devnet  - https://api.devnet.solana.com
# * Testnet - https://api.testnet.solana.com"
#   output=$(
#     anchor_cli cluster list
#   ) || exit_code="$?"

#   echo "${expected_output}" > "${output_dir}/expected_cluster.txt"
#   echo "${output}" > "${output_dir}/actual_cluster.txt"
#   if diff_output=$(diff "${output_dir}/expected_cluster.txt" "${output_dir}/actual_cluster.txt"); then
#     echo "test cluster passed"
#   else
#     echo "test cluster failed"
#     echo "----- diff ----"
#     echo "${diff_output}"
#     echo "----- end -----"
#     script_exit_code=1
#   fi
# )

# keys list
# (
#   cd "${initialize_dir}/build/test-program"
#   output=$(anchor_cli keys list)
#   expected_output="test_program: aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x"
#   if [ "$output" = "$expected_output" ]; then
#     echo "test keys list passed"
#   else
#     echo "test keys list failed"
#     echo "----- output ----"
#     echo "$output"
#     echo "----- end -----"
#     script_exit_code=1
#   fi
# )

# keys sync
# (
#   cd "${initialize_dir}/build/test-program"
#   output=$(anchor_cli keys sync)
#   expected_output="All program id declarations are synced."
#   if [ "$output" = "$expected_output" ]; then
#     echo "test keys sync passed"
#   else
#     echo "test keys sync failed"
#     echo "----- output ----"
#     echo "$output"
#     echo "----- end -----"
#     script_exit_code=1
#   fi
# )

exit "${script_exit_code}"
