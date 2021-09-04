.PHONY: test build build-deps build-dex

test: build
	anchor test

build: build-dex
	anchor build

build-dex:
	cd deps/serum-dex/dex/ && cargo build-bpf && cd ../../../

localnet:
	./scripts/localnet.sh
