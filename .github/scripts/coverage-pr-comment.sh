#!/usr/bin/env bash
#
# Compare two LCOV tracefiles. When the line-coverage delta is significant,
# post a fresh PR comment with the report. On every push that moves
# coverage, a new comment appears — no in-place update, no deletion of
# prior bot comments. On fork PRs the `GITHUB_TOKEN` is read-only, so the
# comment POST is best-effort; the full report is always emitted to the
# job summary so the data is visible either way.
#
# Inputs (env):
#   GH_TOKEN   — GitHub token (provided by Actions).
#   PR_NUMBER  — Pull request number.
#   REPO       — `<owner>/<name>` of the repository.
#   BASE_SHA   — Base commit SHA (display only).
#   HEAD_SHA   — PR head commit SHA (display only).
#
# Inputs (args): <base.lcov> <pr.lcov>
#
# Negligible threshold: the change is "negligible" when both the
# line-coverage percentage delta is under LINES_EPSILON AND the
# covered-line count delta is under HIT_EPSILON.
set -euo pipefail

LINES_EPSILON=${LINES_EPSILON:-0.05}
HIT_EPSILON=${HIT_EPSILON:-2}
MARKER='<!-- coverage-bot:v1 -->'

if [[ $# -ne 2 ]]; then
    echo "usage: $0 <base.lcov> <pr.lcov>" >&2
    exit 2
fi

BASE_LCOV=$1
PR_LCOV=$2
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

load_totals() {
    local lcov=$1 prefix=$2
    # shellcheck disable=SC2046
    eval $("$SCRIPT_DIR/coverage-totals.sh" "$lcov" | sed "s/^/${prefix}_/")
}
load_totals "$BASE_LCOV" BASE
load_totals "$PR_LCOV" PR

delta_pct() { awk -v a="$1" -v b="$2" 'BEGIN { printf "%+.2f", b - a }'; }
delta_int() { awk -v a="$1" -v b="$2" 'BEGIN { printf "%+d", b - a }'; }
abs_lt()    { awk -v a="$1" -v b="$2" 'BEGIN { if ((a < 0 ? -a : a) < b) print 1; else print 0 }'; }

LINES_DELTA=$(delta_pct "$BASE_LINES_PCT" "$PR_LINES_PCT")
LINES_HIT_DELTA=$(delta_int "$BASE_LINES_HIT" "$PR_LINES_HIT")
FUNCTIONS_DELTA=$(delta_pct "$BASE_FUNCTIONS_PCT" "$PR_FUNCTIONS_PCT")
FUNCTIONS_HIT_DELTA=$(delta_int "$BASE_FUNCTIONS_HIT" "$PR_FUNCTIONS_HIT")
BRANCHES_DELTA=$(delta_pct "$BASE_BRANCHES_PCT" "$PR_BRANCHES_PCT")
BRANCHES_HIT_DELTA=$(delta_int "$BASE_BRANCHES_HIT" "$PR_BRANCHES_HIT")

echo "Coverage delta summary:"
echo "  Lines:     ${BASE_LINES_PCT}% -> ${PR_LINES_PCT}% (${LINES_DELTA} pp, ${LINES_HIT_DELTA} lines)"
echo "  Functions: ${BASE_FUNCTIONS_PCT}% -> ${PR_FUNCTIONS_PCT}% (${FUNCTIONS_DELTA} pp, ${FUNCTIONS_HIT_DELTA} fns)"
echo "  Branches:  ${BASE_BRANCHES_PCT}% -> ${PR_BRANCHES_PCT}% (${BRANCHES_DELTA} pp, ${BRANCHES_HIT_DELTA} branches)"

if [[ $(abs_lt "$LINES_DELTA" "$LINES_EPSILON") == 1 \
   && $(abs_lt "$LINES_HIT_DELTA" "$HIT_EPSILON") == 1 ]]; then
    echo "Delta is negligible (eps=${LINES_EPSILON}pp, hit_eps=${HIT_EPSILON}); not posting."
    exit 0
fi

: "${GH_TOKEN:?GH_TOKEN not set}"
: "${PR_NUMBER:?PR_NUMBER not set}"
: "${REPO:?REPO not set}"

BASE_SHORT=${BASE_SHA:-unknown}
HEAD_SHORT=${HEAD_SHA:-unknown}
BASE_SHORT=${BASE_SHORT:0:7}
HEAD_SHORT=${HEAD_SHORT:0:7}

format_row() {
    local label=$1 base_pct=$2 pr_pct=$3 delta=$4 base_hit=$5 pr_hit=$6 total=$7 hit_delta=$8
    echo "| ${label} | ${base_pct}% (${base_hit} / ${total}) | ${pr_pct}% (${pr_hit} / ${total}) | ${delta} pp (${hit_delta}) |"
}

BODY=$(cat <<EOF
${MARKER}
## 📊 Coverage report (v2 stack)

| Metric | Base (\`${BASE_SHORT}\`) | PR (\`${HEAD_SHORT}\`) | Δ |
|---|---|---|---|
$(format_row "Lines"     "$BASE_LINES_PCT"     "$PR_LINES_PCT"     "$LINES_DELTA"     "$BASE_LINES_HIT"     "$PR_LINES_HIT"     "$PR_LINES_TOTAL"     "$LINES_HIT_DELTA")
$(format_row "Functions" "$BASE_FUNCTIONS_PCT" "$PR_FUNCTIONS_PCT" "$FUNCTIONS_DELTA" "$BASE_FUNCTIONS_HIT" "$PR_FUNCTIONS_HIT" "$PR_FUNCTIONS_TOTAL" "$FUNCTIONS_HIT_DELTA")
$(format_row "Branches"  "$BASE_BRANCHES_PCT"  "$PR_BRANCHES_PCT"  "$BRANCHES_DELTA"  "$BASE_BRANCHES_HIT"  "$PR_BRANCHES_HIT"  "$PR_BRANCHES_TOTAL"  "$BRANCHES_HIT_DELTA")

<sub>Generated from \`make coverage-v2\`. Threshold for posting: line coverage must move by ≥ ${LINES_EPSILON} pp OR ≥ ${HIT_EPSILON} lines.</sub>
EOF
)

# Always surface in the job summary so fork PRs (read-only token) still
# get a visible report on the workflow run page.
if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
    printf '%s\n' "$BODY" >> "$GITHUB_STEP_SUMMARY"
fi

# Post the comment. A failed POST — 403 included — is a real error the
# check should surface loudly. The report is still in the job summary
# above so the data isn't lost, but a failed post means the pipeline
# isn't delivering on its contract and needs attention (grant the token
# pull-requests:write scope, or move this step into a workflow triggered
# by `workflow_run: completed` so it runs with base-branch permissions).
echo "Posting coverage comment on PR #${PR_NUMBER}"
if ! POST_ERR=$(gh api -X POST "repos/${REPO}/issues/${PR_NUMBER}/comments" \
        -f body="$BODY" 2>&1 >/dev/null); then
    if grep -q "Resource not accessible by integration" <<<"$POST_ERR"; then
        echo "::error::PR comment POST returned 403 — GITHUB_TOKEN lacks pull-requests:write. Likely the fork-PR read-only-token case."
    else
        printf '%s\n' "$POST_ERR" >&2
    fi
    exit 1
fi
