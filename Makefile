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
	# First pass: default feature set (covers runtime + derive codegen paths).
	CARGO_PROFILE_RELEASE_DEBUG=2 \
	cargo llvm-cov --no-report -p anchor-lang-v2 -p anchor-spl-v2 -p tests-v2
	# Second pass: same crates with --features idl-build so the IDL-emission
	# branches in derive/src/idl.rs and the program-level IDL assembler run.
	# No `--no-clean` here — passing `--no-report` already preserves the
	# profile data across passes; the merged report is assembled by the
	# final `llvm-cov report` below.
	CARGO_PROFILE_RELEASE_DEBUG=2 \
	cargo llvm-cov --no-report --features idl-build \
		-p anchor-lang-v2 -p anchor-spl-v2
	cargo llvm-cov report \
		--lcov \
		--output-path $(COVERAGE_DIR)/host.lcov \
		--ignore-filename-regex='(\.cargo|target/debug/build|/runner/work/)'

# Merge the two LCOV files into one. The sbf tool already filters to files
# that exist on disk, so phantom dep paths are gone. We just drop stdlib and
# registry paths from the host LCOV.
.PHONY: coverage-v2-merge
coverage-v2-merge: $(COVERAGE_DIR)/sbf.lcov $(COVERAGE_DIR)/host.lcov
	@echo "==> Merging coverage"
	@command -v lcov >/dev/null || { \
		echo "lcov not installed. Run: brew install lcov  (or apt install lcov)"; \
		exit 1; \
	}
	lcov -a $(COVERAGE_DIR)/sbf.lcov -a $(COVERAGE_DIR)/host.lcov \
		-o $(COVERAGE_DIR)/combined.lcov
	lcov --remove $(COVERAGE_DIR)/combined.lcov \
		'/Users/runner/*' '/home/runner/*' '*/.cargo/*' '*/target/*' \
		-o $(COVERAGE_DIR)/combined.lcov \
		--ignore-errors unused

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
