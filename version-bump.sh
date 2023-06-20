#!/bin/bash

set -e

if [ $# -eq 0 ]; then
    echo "Usage $0 VERSION"
    exit 1
fi

version=$1

echo "Bumping versions to $version"

# GNU/BSD compat
sedi=(-i)
case "$(uname)" in
  # For macOS, use two parameters
  Darwin*) sedi=(-i "")
esac

# Don't replace version with the following globs
skip_globs=":!**/yarn.lock :!Cargo.lock :!package.json :!tests/bench/bench.json :!bench/*.md"

git grep -l $(cat VERSION) -- $skip_globs |
    xargs sed "${sedi[@]}" \
    -e "s/$(cat VERSION)/$version/g"

# Potential for collisions in package.json files, handle those separately
# Replace only matching "version": "x.xx.x" and "@coral-xyz/anchor": "x.xx.x"
git grep -l $(cat VERSION) -- '**/package.json' | \
    xargs sed "${sedi[@]}" \
    -e "s/@coral-xyz\/anchor\": \"$(cat VERSION)\"/@coral-xyz\/anchor\": \"$version\"/g" \
    -e "s/\"version\": \"$(cat VERSION)\"/\"version\": \"$version\"/g"

# Potential for collisions in Cargo.lock, use cargo update to update it
cargo update --workspace

# Insert version number into CHANGELOG.md
sed "${sedi[@]}" -e "s/## \[Unreleased\]/## [Unreleased]\n\n## [$version] - $(date '+%Y-%m-%d')/g" CHANGELOG.md

pushd ts && yarn && popd
pushd tests && yarn && popd
pushd examples && yarn && pushd tutorial && yarn && popd && popd

# Bump benchmark files
pushd tests/bench && anchor run bump-version -- --anchor-version $version && popd

echo $version > VERSION

echo "$(git diff --stat | tail -n1) files modified"

echo " $(cat VERSION) changeset generated, commit and tag"
