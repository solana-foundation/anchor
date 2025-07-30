#!/usr/bin/env bash

# This script checks that all our Docker images are pinned to a specific SHA256 hash.
#
# References as to why...
#   - https://nickjanetakis.com/blog/docker-tip-18-please-pin-your-docker-image-versions
#   - https://snyk.io/blog/10-docker-image-security-best-practices/ (Specifically: USE FIXED TAGS FOR IMMUTABILITY)
#
# Explanation of regex ignore choices
#   - We ignore sha256 because it suggests that the image dep is pinned

git ls-files -z | grep -z "Dockerfile*" | xargs -r -0 grep -s "FROM" | egrep -v 'sha256'
if [ $? -eq 0 ]; then
   echo "[!] Unpinned docker files" >&2
   exit 1
else
   echo "[+] No unpinned docker files"
fi