# anchor-bench

Small benchmark workspace for measuring Anchor program compute usage and binary size.

## What it does

- Builds a simple hello-world Anchor program under `programs/hello-world`
- Compiles the program to SBF with `cargo build-sbf`
- Loads the compiled `.so` into LiteSVM `0.10`
- Executes the `hello` instruction
- Writes benchmark results to `results.json`

## Run

```sh
cargo run --manifest-path bench/Cargo.toml
```

## Check

```sh
cargo run --manifest-path bench/Cargo.toml -- check
```

`check` exits with an error when the recorded benchmark history is out of date or when any
recorded snapshot commit is not present on `master`.
