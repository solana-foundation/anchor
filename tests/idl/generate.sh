#!/usr/bin/env bash

# `$1` is the directory to generate the IDLs in, defaults to `./idls`
if [ $# = 1 ]; then
    dir=$1
else
    dir=$PWD/idls
fi

cd programs/idl
anchor idl build -o $dir/new.json

cd ../generics
anchor idl build -o $dir/generics.json

cd ../pda-init-seeds
anchor idl build -o $dir/pda-init-seeds.json

if command -v jq &> /dev/null; then
    pda_count=$(jq '[.instructions[].accounts[] | select(.pda.seeds != null)] | length' $dir/pda-init-seeds.json 2>/dev/null || echo "0")
    expected_count=3
    if [ "$pda_count" -ne "$expected_count" ]; then
        exit 1
    fi
fi

cd ../relations-derivation
anchor idl build -o $dir/relations.json
