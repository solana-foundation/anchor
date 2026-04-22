DICTIONARY_FILE ?= .cspell-anchor-dictionary.txt

.PHONY: build-cli
build-cli:
	cargo build -p anchor-cli --release
	cp target/release/anchor cli/npm-package/anchor

# Unified coverage for the v2 stack.
#
# Produces one HTML report combining:
#   1. SBF runtime coverage (lang-v2/spl-v2 as exercised inside the VM) —
#      collected via litesvm register tracing, mapped to source via DWARF.
#   2. Host-side coverage (derive macros, tests-v2 harness) — collected via
#      cargo-llvm-cov's LLVM instrumentation.
#
# Requirements: cargo-llvm-cov (cargo install cargo-llvm-cov), lcov (brew
# install lcov or apt install lcov — provides lcov + genhtml).
COVERAGE_DIR := target/coverage
COVERAGE_HTML := $(COVERAGE_DIR)/html

.PHONY: coverage-v2
coverage-v2: coverage-v2-sbf coverage-v2-host coverage-v2-merge coverage-v2-html
	@echo ""
	@echo "Coverage report: $(COVERAGE_HTML)/index.html"

# SBF runtime coverage: one-shot via `anchor coverage`, which sets up the
# DWARF build env (RUSTC_WRAPPER + CARGO_PROFILE_RELEASE_DEBUG=2 — same as
# `anchor debugger`), runs `cargo test` with register-tracing via
# SBF_TRACE_DIR, then maps PCs to source via addr2line and emits LCOV.
#
# `--skip-build` because tests-v2 isn't a cdylib itself — the test harness
# builds its programs per-manifest via `build_program()`, inheriting the
# wrapper from the parent env.
.PHONY: coverage-v2-sbf
coverage-v2-sbf: build-anchor-debug
	@echo "==> SBF runtime coverage"
	cd $(PWD)/tests-v2 && $(PWD)/target/debug/anchor coverage \
		--skip-build \
		--trace-dir $(PWD)/$(COVERAGE_DIR)/traces \
		--output $(PWD)/$(COVERAGE_DIR)/sbf.lcov

# Always invoke cargo (it handles incremental) so the anchor binary picks up
# any changes to coverage.rs / source_resolver / etc.
.PHONY: build-anchor-debug
build-anchor-debug:
	cargo build -p anchor-cli

# Host-side coverage (derive macros + integration test harness).
#
# Shares `CARGO_PROFILE_RELEASE_DEBUG=2` with the sbf step so the SBF programs
# rebuilt here by tests-v2 reuse the same cached artifacts (otherwise the
# profile flag diff would force a full SBF rebuild).
.PHONY: coverage-v2-host
coverage-v2-host:
	@echo "==> Host-side coverage (derive macros + test harness)"
	@command -v cargo-llvm-cov >/dev/null || { \
		echo "cargo-llvm-cov not installed. Run: cargo install cargo-llvm-cov"; \
		exit 1; \
	}
	cargo llvm-cov clean --workspace
	# First pass: `anchor-lang-v2` with its `testing` feature enabled — the
	# Miri-witnesses integration tests need the `anchor_lang_v2::testing`
	# scaffold. Split into its own invocation because `testing` is a
	# lang-v2-only feature, and passing it to a multi-package run would
	# fail when `anchor-spl-v2` / `tests-v2` don't define it.
	CARGO_PROFILE_RELEASE_DEBUG=2 \
	cargo llvm-cov --no-report -p anchor-lang-v2 --features testing
	# Second pass: the other v2 crates under default features.
	CARGO_PROFILE_RELEASE_DEBUG=2 \
	cargo llvm-cov --no-report -p anchor-spl-v2 -p tests-v2
	# Third pass: lang-v2 + spl-v2 with --features idl-build so the
	# IDL-emission branches in derive/src/idl.rs and the program-level IDL
	# assembler run. `--no-report` accumulates profile data across passes;
	# the merged report is assembled by the final `llvm-cov report` below.
	CARGO_PROFILE_RELEASE_DEBUG=2 \
	cargo llvm-cov --no-report --features idl-build \
		-p anchor-lang-v2 -p anchor-spl-v2
	cargo llvm-cov report \
		--lcov \
		--output-path $(COVERAGE_DIR)/host.lcov \
		--ignore-filename-regex='(\.cargo|target/(debug|release)/build)'

# Merge the two LCOV files into one.
#
# `cargo-llvm-cov` instruments every line of every file it compiles, including
# SBF-runtime code pulled in as a path dep of `tests-v2` (lang-v2/src/*.rs,
# spl-v2/src/*.rs). Some of those lines get erased by LLVM's `#[inline(always)]`
# handling — SBF's DWARF sees the line executed, but host-side instrumentation
# records `DA:N,0` because the machine code lives at the callsite, not the
# source site. Naive merge ends up displaying those lines as uncovered.
#
# The filter scrubs host's `DA:N,0` entries **only when SBF recorded a hit at
# the same (file, line)**. Zero-hit lines on both sides remain in the report
# as genuinely uncovered — which matters: earlier versions of this filter
# stripped every zero-hit line for any file that appeared in sbf.lcov, which
# made files with genuine uncovered regions (e.g. spl-v2/src/token.rs's many
# untested CPI branches) look artificially at 100%. See analysis in
# `ci.lcov` vs Codecov comparison (spl-v2/src/token.rs was reported as
# 82/82 locally vs. 82/347 on CI, all because of this scrub). Line-level
# matching preserves the SBF-only-executed fix without over-reaching.
.PHONY: coverage-v2-merge
coverage-v2-merge: $(COVERAGE_DIR)/sbf.lcov $(COVERAGE_DIR)/host.lcov
	@echo "==> Merging coverage"
	@command -v lcov >/dev/null || { \
		echo "lcov not installed. Run: brew install lcov  (or apt install lcov)"; \
		exit 1; \
	}
	awk -F'[:,]' ' \
		FNR == NR { \
			if ($$1 == "SF") { f = $$0; sub("^SF:", "", f) } \
			else if ($$1 == "DA" && $$3+0 > 0) sbf_hit[f "," $$2+0] = 1; \
			next \
		} \
		$$1 == "SF" { f = $$0; sub("^SF:", "", f); print; next } \
		$$1 == "DA" { \
			if ($$3+0 == 0 && (f "," $$2+0) in sbf_hit) next; \
			print; next; \
		} \
		{ print } \
		' $(COVERAGE_DIR)/sbf.lcov $(COVERAGE_DIR)/host.lcov \
		> $(COVERAGE_DIR)/host.filtered.lcov
	# Recompute LF/LH for files we mutated — lcov -a keeps the original
	# counts if they don't agree with the DA lines. Drop stale summary
	# lines and let lcov rebuild them on load.
	sed -i.bak '/^LF:/d;/^LH:/d' $(COVERAGE_DIR)/host.filtered.lcov
	rm -f $(COVERAGE_DIR)/host.filtered.lcov.bak
	lcov -a $(COVERAGE_DIR)/sbf.lcov -a $(COVERAGE_DIR)/host.filtered.lcov \
		-o $(COVERAGE_DIR)/combined.lcov \
		--ignore-errors empty
	# Drop cargo-registry and target-generated paths only. The prior
	# `/home/runner/*` / `/Users/runner/*` blanket filters matched the
	# CI workspace root (`/home/runner/work/<repo>/<repo>/...`) and
	# stripped every source file, producing an empty LCOV on CI. Path
	# globs of `*/.cargo/*` and `*/target/*` are content-based so they
	# work identically on local and CI runners.
	lcov --remove $(COVERAGE_DIR)/combined.lcov \
		'*/.cargo/*' '*/target/*' \
		-o $(COVERAGE_DIR)/combined.lcov \
		--ignore-errors unused \
		--ignore-errors empty

$(COVERAGE_DIR)/sbf.lcov: coverage-v2-sbf
$(COVERAGE_DIR)/host.lcov: coverage-v2-host

# Generate HTML report. Cleans output dir first so stale files from a prior
# run don't leak into the tree.
.PHONY: coverage-v2-html
coverage-v2-html: $(COVERAGE_DIR)/combined.lcov
	@echo "==> Generating HTML report"
	@command -v genhtml >/dev/null || { \
		echo "genhtml not installed (ships with lcov)."; \
		exit 1; \
	}
	rm -rf $(COVERAGE_HTML)
	genhtml $(COVERAGE_DIR)/combined.lcov \
		--output-directory $(COVERAGE_HTML) \
		--prefix $(PWD) \
		--ignore-errors source,unmapped,category \
		--quiet

$(COVERAGE_DIR)/combined.lcov: coverage-v2-merge

.PHONY: coverage-v2-clean
coverage-v2-clean:
	rm -rf $(COVERAGE_DIR)

.PHONY: clean
clean:
	find . -type d -name .anchor -print0 | xargs -0 rm -rf
	find . -type d -name node_modules -print0 | xargs -0 rm -rf
	find . -type d -name target -print0 | xargs -0 rm -rf

.PHONY: publish
publish:
	cd lang/syn/ && cargo publish && cd ../../
	sleep 25
	cd lang/derive/accounts/ && cargo publish && cd ../../../
	sleep 25
	cd lang/derive/serde/ && cargo publish && cd ../../../
	sleep 25
	cd lang/derive/space/ && cargo publish && cd ../../../
	sleep 25
	cd lang/attribute/access-control/ && cargo publish && cd ../../../
	sleep 25
	cd lang/attribute/account/ && cargo publish && cd ../../../
	sleep 25
	cd lang/attribute/constant/ && cargo publish && cd ../../../
	sleep 25
	cd lang/attribute/error/ && cargo publish && cd ../../../
	sleep 25
	cd lang/attribute/program/ && cargo publish && cd ../../..
	sleep 25
	cd lang/attribute/event/ && cargo publish && cd ../../../
	sleep 25
	cd lang/ && cargo publish && cd ../
	sleep 25
	cd spl/ && cargo publish && cd ../
	sleep 25
	cd client/ && cargo publish && cd ../
	sleep 25
	cd cli/ && cargo publish && cd ../
	sleep 25

.PHONY: spellcheck
spellcheck:
	cspell *.md **/*.md **/*.mdx --config cspell.config.yaml --no-progress

.PHONY: update-dictionary
update-dictionary:
	echo $(DICTIONARY_FILE)
	cspell *.md **/*.md **/*.mdx --config cspell.config.yaml --words-only --unique --no-progress --quiet 2>/dev/null | sort --ignore-case > .new-dictionary-words
	cat .new-dictionary-words $(DICTIONARY_FILE) | sort --ignore-case > .new-$(DICTIONARY_FILE)
	mv .new-$(DICTIONARY_FILE) $(DICTIONARY_FILE)
	rm -f .new-dictionary-words
