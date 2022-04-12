#!/bin/bash

set -e

if [ $# -eq 0 ]; then
    echo "Usage $0 VERSION"
    exit 1
fi

echo "Bumping versions to $1"

# GNU/BSD compat
sedi=(-i)
case "$(uname)" in
  # For macOS, use two parameters
  Darwin*) sedi=(-i "")
esac

git grep -l $(cat VERSION) -- ':!**/yarn.lock' ':!CHANGELOG.md' ':!Cargo.lock' ':!package.json' | \
    xargs sed "${sedi[@]}" \
    -e "s/$(cat VERSION)/$1/g"

# Potential for collisions in package.json files, handle those separately
# Replace only matching "version": "x.xx.x" and "@project-serum/anchor": "x.xx.x"
git grep -l $(cat VERSION) -- '**/package.json' | \
    xargs sed "${sedi[@]}" \
    -e "s/@project-serum\/anchor\": \"$(cat VERSION)\"/@project-serum\/anchor\": \"$1\"/g" \
    -e "s/\"version\": \"$(cat VERSION)\"/\"version\": \"$1\"/g"

# Potential for collisions in Cargo.lock, use cargo update to update it
cargo update --workspace

# Insert version number into CHANGELOG.md
sed "${sedi[@]}" -e "s/## \[Unreleased\]/## [Unreleased]\n\n## [$1] - $(date '+%Y-%m-%d')/g" CHANGELOG.md

pushd ts && yarn && popd
pushd tests && yarn && popd
pushd examples && yarn && pushd tutorial && yarn && popd && popd

echo $1 > VERSION

echo "$(git diff --stat | tail -n1) files modified"

echo " $(cat VERSION) changeset generated, commit and tag"
