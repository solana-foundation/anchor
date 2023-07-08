#!/usr/bin/env bash
set -x
set -e

TMPDIR=$(mktemp -d)

cd programs/idl
anchor idl parse --file src/lib.rs > $TMPDIR/idl_parse_act.json
anchor idl build > $TMPDIR/idl_build_act.json

cd ../generics
anchor idl build > $TMPDIR/generics_build_act.json

cd ../relations-derivation
anchor idl build > $TMPDIR/relations_build_act.json

cd ../..
echo "----------------------------------------------------"
echo "idl parse before > after"
echo "----------------------------------------------------"
echo ""
diff -y --color tests/testdata/idl_parse_exp.json $TMPDIR/idl_parse_act.json
PARSE_RETCODE=$?

echo ""
echo ""
echo "----------------------------------------------------"
echo "idl build before > after"
echo "----------------------------------------------------"
echo ""
diff -y --color tests/testdata/idl_build_exp.json $TMPDIR/idl_build_act.json
GEN_RETCODE=$?

echo ""
echo ""
echo "----------------------------------------------------"
echo "idl generics build before > after"
echo "----------------------------------------------------"
echo ""
diff -y --color tests/testdata/generics_build_exp.json $TMPDIR/generics_build_act.json
GEN_GENERICS_RETCODE=$?

echo ""
echo ""
echo "----------------------------------------------------"
echo "idl relations build before > after"
echo "----------------------------------------------------"
echo ""
diff -y --color tests/testdata/relations_build_exp.json $TMPDIR/relations_build_act.json
GEN_RELATIONS_RETCODE=$?

# returns 0 when ok, or a positive integer when there are differences
exit $((PARSE_RETCODE+GEN_RETCODE+GEN_GENERICS_RETCODE+GEN_RELATIONS_RETCODE))
