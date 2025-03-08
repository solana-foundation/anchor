#!/bin/bash

set -e

if [ $# -eq 0 ]; then
    echo "Usage $0 VERSION"
    exit 1
fi

old_version=$(cat VERSION)
version=$1

echo "Bumping versions to $version"

# GNU/BSD compat
sedi=(-i)
case "$(uname)" in
  # For macOS, use two parameters
  Darwin*) sedi=(-i "")
esac

# Only replace version with the following globs
allow_globs=":**/Cargo.toml **/Makefile client/src/lib.rs lang/attribute/program/src/lib.rs"
git grep -l $old_version -- $allow_globs |
    xargs sed "${sedi[@]}" \
    -e "s/$old_version/$version/g"

# Separately handle docs because blindly replacing the old version with the new
# might break certain examples/links
pushd docs/content/docs
git grep -l $old_version -- "./*.md*" | \
    xargs sed "${sedi[@]}" \
    -e "s/\"$old_version\"/\"$version\"/g"
allow_globs="installation.mdx quickstart/local.mdx references/verifiable-builds.mdx"
git grep -l $old_version -- $allow_globs |
    xargs sed "${sedi[@]}" \
    -e "s/$old_version/$version/g"
# Replace `solana_version` with the current version
solana_version=$(solana --version | awk '{print $2;}')
sed $sedi "s/solana_version.*\"/solana_version = \"$solana_version\"/g" references/anchor-toml.mdx
# Keep release notes and changelog the same
git restore updates
popd

# Potential for collisions in `package.json` files, handle those separately
# Replace only matching "version": "x.xx.x" and "@coral-xyz/anchor": "x.xx.x"
git grep -l $old_version -- "**/package.json" | \
    xargs sed "${sedi[@]}" \
    -e "s/@coral-xyz\/anchor\": \"$old_version\"/@coral-xyz\/anchor\": \"$version\"/g" \
    -e "s/\"version\": \"$old_version\"/\"version\": \"$version\"/g"

# Insert version number into CHANGELOG
sed "${sedi[@]}" -e \
    "s/## \[Unreleased\]/## [Unreleased]\n\n### Features\n\n### Fixes\n\n### Breaking\n\n## [$version] - $(date '+%Y-%m-%d')/g" \
    CHANGELOG.md

# Update lock files
pushd ts && yarn && popd
pushd tests && yarn && popd
pushd examples && yarn && pushd tutorial && yarn && popd && popd

# Bump benchmark files
pushd tests/bench && anchor run bump-version -- --anchor-version $version && popd

echo $version > VERSION

echo "$(git diff --stat | tail -n1) files modified"

echo "$version changeset generated, commit and tag"
