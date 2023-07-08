#!/usr/bin/env bash

cd programs/idl
anchor idl parse --file src/lib.rs > ../../tests/testdata/idl_parse_exp.json
anchor idl build > ../../tests/testdata/idl_build_exp.json

cd ../generics
anchor idl build > ../../tests/testdata/generics_build_exp.json

cd ../relations-derivation
anchor idl build > ../../tests/testdata/relations_build_exp.json