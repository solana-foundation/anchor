#!/usr/bin/env bash

idls_dir=$PWD/idls

cd programs/idl
anchor idl parse --file src/lib.rs -o $idls_dir/idl_parse_exp.json
anchor idl build -o $idls_dir/idl_build_exp.json

cd ../generics
anchor idl build -o $idls_dir/generics_build_exp.json

cd ../relations-derivation
anchor idl build -o $idls_dir/relations_build_exp.json