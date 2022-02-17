#!/bin/bash

echo "Building programs"

#
# Build the UncheckedAccount variant.
#
pushd programs/unchecked-account/
anchor build
if [ $? -eq 0 ]; then
   echo "Error: expected failure"
   exit 1
fi
popd

#
# Build the AccountInfo variant.
#
pushd programs/account-info/
anchor build
if [ $? -eq 0 ]; then
   echo "Error: expected failure"
   exit 1
fi
popd

echo "Success. All builds failed."
