#!/usr/bin/env bash
#
# Print totals for an LCOV tracefile as shell-sourceable key=value lines:
#
#     LINES_HIT=NNN
#     LINES_TOTAL=NNN
#     LINES_PCT=XX.XX
#     FUNCTIONS_HIT=NNN
#     FUNCTIONS_TOTAL=NNN
#     FUNCTIONS_PCT=XX.XX
#     BRANCHES_HIT=NNN
#     BRANCHES_TOTAL=NNN
#     BRANCHES_PCT=XX.XX
#
# Parses the LCOV trace directly rather than delegating to `lcov --summary`
# so the output is stable across lcov versions (1.14 / 1.16 / 2.x all
# format their summary text differently, and we want machine-readable
# fields for the PR-comment delta math).
#
# Usage: coverage-totals.sh path/to/file.lcov
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "usage: $0 <lcov-file>" >&2
    exit 2
fi

LCOV=$1
if [[ ! -s "$LCOV" ]]; then
    echo "lcov file missing or empty: $LCOV" >&2
    exit 1
fi

awk -v out_prefix="" '
    /^LH:/ { lh += substr($0, 4) }
    /^LF:/ { lf += substr($0, 4) }
    /^FNH:/ { fnh += substr($0, 5) }
    /^FNF:/ { fnf += substr($0, 5) }
    /^BRH:/ { brh += substr($0, 5) }
    /^BRF:/ { brf += substr($0, 5) }
    END {
        printf "LINES_HIT=%d\n",       lh
        printf "LINES_TOTAL=%d\n",     lf
        printf "LINES_PCT=%.2f\n",     (lf > 0) ? 100.0 * lh / lf : 0.0
        printf "FUNCTIONS_HIT=%d\n",   fnh
        printf "FUNCTIONS_TOTAL=%d\n", fnf
        printf "FUNCTIONS_PCT=%.2f\n", (fnf > 0) ? 100.0 * fnh / fnf : 0.0
        printf "BRANCHES_HIT=%d\n",    brh
        printf "BRANCHES_TOTAL=%d\n",  brf
        printf "BRANCHES_PCT=%.2f\n",  (brf > 0) ? 100.0 * brh / brf : 0.0
    }
' "$LCOV"
