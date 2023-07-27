#!/usr/bin/env bash
set -e

# Run anchor test
anchor test --skip-lint

tmp_dir=$(mktemp -d)

# Generate IDLs
./generate.sh $tmp_dir

# Exit status
ret=0

# Compare IDLs. `$ret` will be non-zero in the case of a mismatch.
compare() {
    echo "----------------------------------------------------"
    echo "IDL $1 before > after changes"
    echo "----------------------------------------------------"
    diff -y --color=always --suppress-common-lines idls/$1.json $tmp_dir/$1.json
    ret=$(($ret+$?))

    if [ "$ret" = "0" ]; then
        echo "No changes"
    fi

    echo ""
}

compare "parse"
compare "build"
compare "generics_build"
compare "relations_build"

exit $ret
