# We don't want to build `multiple-errors` because it is expected to error
anchor build -p errors --skip-lint
anchor test --skip-build