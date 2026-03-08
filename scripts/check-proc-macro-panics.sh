#!/usr/bin/env bash                                                                                                                                                             
  # Checks that proc-macro crates contain no reachable panic paths.
  # Lines marked with "// safe-unwrap: <reason>" are explicitly exempted.                                                                                                         
  #
  # Run locally: bash scripts/check-proc-macro-panics.sh

  set -euo pipefail

  PROC_MACRO_DIRS=(
      "lang/derive/space/src"
      "lang/derive/accounts/src"
      "lang/derive/serde/src"
      "lang/attribute/access-control/src"
      "lang/attribute/account/src"
      "lang/attribute/constant/src"
      "lang/attribute/error/src"
      "lang/attribute/event/src"
      "lang/attribute/program/src"
  )

  PATTERN='panic!|\.unwrap\(\)|unimplemented!|\.expect\('
  EXEMPTION='safe-unwrap:'

  found=0
  for dir in "${PROC_MACRO_DIRS[@]}"; do
      if [ -d "$dir" ]; then
          results=$(grep -rn --include="*.rs" -E "$PATTERN" "$dir" \
              | grep -v "$EXEMPTION" || true)
          if [ -n "$results" ]; then
              echo "$results"
              found=1
          fi
      fi
  done

  if [ "$found" -eq 1 ]; then
      echo ""
      echo "ERROR: panic!/unwrap()/unimplemented!/expect() found in proc-macro crates."
      echo "Replace with syn::Error, or mark intentional ones with '// safe-unwrap: <reason>'."
      exit 1
  fi

  echo "OK: no unguarded panic paths found in proc-macro crates."