#!/bin/bash

echo "Building programs"

#
# Build the UncheckedAccount variant.
#
pushd programs/unchecked-account/
output=$(anchor build 2>&1 > /dev/null)
if ! [[ $output =~ "Struct field \"unchecked\" in struct \"Initialize\" is unsafe" ]]; then
   echo "Error: expected /// CHECK error"
   exit 1
fi
popd

#
# Build the AccountInfo variant.
#
pushd programs/account-info/
output=$(anchor build 2>&1 > /dev/null)
if ! [[ $output =~ "Struct field \"unchecked\" in struct \"Initialize\" is unsafe" ]]; then
   echo "Error: expected /// CHECK error"
   exit 1
fi
popd

#
# Build the duplicate-names variant.
#
pushd programs/duplicate-names/
output=$(anchor build 2>&1 > /dev/null)
if ! [[ $output =~ "Struct field \"my_account\" in struct \"FuncOne\" is unsafe" ]]; then
   echo "Error: expected /// CHECK error for duplicate-names"
   exit 1
fi
popd

#
# Build the control variant.
#
pushd programs/ignore-non-accounts/
if ! anchor build ; then
   echo "Error: anchor build failed when it shouldn't have"
   exit 1
fi
popd

echo "Success. As expected, all builds failed that were supposed to fail."
