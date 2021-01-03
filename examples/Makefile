.PHONY: examples

ANCHOR=$(PWD)/../target/debug/anchor

examples:
	cd tutorial/basic-0/program && cargo build-bpf && $(ANCHOR) idl -f src/lib.rs  -o ../idl.json
	cd basic && cargo build-bpf && $(ANCHOR) idl -f src/lib.rs  -o idl.json
