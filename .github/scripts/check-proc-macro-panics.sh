#!/usr/bin/env bash

set -euo pipefail

readonly patterns='panic!|\.unwrap\(|unimplemented!'
readonly proc_macro_dirs=(
  "lang/attribute"
  "lang/derive"
  "lang/syn"
)

matches="$(rg -n "${patterns}" "${proc_macro_dirs[@]}" -g '*.rs' || true)"

if [[ -n "${matches}" ]]; then
  echo "Found banned panic-style constructs in proc-macro crates:"
  echo "${matches}"
  exit 1
fi
