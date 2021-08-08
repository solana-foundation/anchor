.PHONY: build-cli
build-cli:
	cargo build -p anchor-cli --release
	cp target/release/anchor cli/npm-package/anchor

.PHONY: build-example-bpf-%
build-example-bpf-%: export NAME=$(subst _,/,$($(strip @):build-example-bpf-%=%))
build-example-bpf-%:
	cd examples/${NAME} && cargo build-bpf

.PHONY: build-example-bpf-permissioned-markets
build-example-bpf-permissioned-markets:
	cd examples/permissioned-markets/deps/serum-dex/dex && cargo build-bpf
	cd examples/permissioned-markets && cargo build-bpf

.PHONY: build-example-bpf-swap
build-example-bpf-swap:
	cd examples/swap/deps/serum-dex/dex && cargo build-bpf
	cd examples/swap && cargo build-bpf

.PHONY: build-example-bpf-all
build-example-bpf-all: build-example-bpf-cashiers-check
build-example-bpf-all: build-example-bpf-cfo
build-example-bpf-all: build-example-bpf-chat
build-example-bpf-all: build-example-bpf-composite
build-example-bpf-all: build-example-bpf-errors
build-example-bpf-all: build-example-bpf-escrow
build-example-bpf-all: build-example-bpf-events
build-example-bpf-all: build-example-bpf-ido-pool
build-example-bpf-all: build-example-bpf-interface
build-example-bpf-all: build-example-bpf-lockup
build-example-bpf-all: build-example-bpf-misc
build-example-bpf-all: build-example-bpf-multisig
build-example-bpf-all: build-example-bpf-permissioned-markets
build-example-bpf-all: build-example-bpf-pyth
build-example-bpf-all: build-example-bpf-spl_token-proxy
build-example-bpf-all: build-example-bpf-swap
build-example-bpf-all: build-example-bpf-sysvars
build-example-bpf-all: build-example-bpf-tutorial_basic-0
build-example-bpf-all: build-example-bpf-tutorial_basic-1
build-example-bpf-all: build-example-bpf-tutorial_basic-2
build-example-bpf-all: build-example-bpf-tutorial_basic-3
build-example-bpf-all: build-example-bpf-tutorial_basic-4
build-example-bpf-all: build-example-bpf-tutorial_basic-5
build-example-bpf-all: build-example-bpf-typescript
build-example-bpf-all: build-example-bpf-zero-copy

.PHONY: clean
clean:
	find . -type d -name .anchor -print0 | xargs -0 rm -rf
	find . -type d -name target -print0 | xargs -0 rm -rf
