# Benchmark tests

The bench program and its tests are used to measure the performance of Anchor programs.

## How

Create a program -> Write tests that measure usage -> Compare the results -> Save the new result

The script will check whether there is a difference between the current result and the last saved result(in `bench.json`) at the end of the tests. If the difference between the results is greater than 1%, the new data will be saved in `bench.json` and Markdown files in [/bench](https://github.com/coral-xyz/anchor/tree/master/bench) will be updated accordingly.

## Scripts

`anchor test --skip-lint`: Run all tests and update benchmark files when necessary. This is the only command that needs to be run for most use cases.

---

The following scripts are useful when making changes to how benchmarking works.

`anchor run update-bench`: Update Markdown files in [/bench](https://github.com/coral-xyz/anchor/tree/master/bench) based on the data from `bench.json`.

`anchor run generate-ix`: Generate instructions with repetitive accounts.

---

The following script is only for the maintainer(s) of Anchor.

`anchor run bump-version -- <VERSION>`: Bump the version in all benchmark files.
