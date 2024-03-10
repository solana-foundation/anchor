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

cd ../relations-derivation
anchor idl build -o $dir/relations.json
