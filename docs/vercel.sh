#!/bin/bash

git diff --quiet HEAD^ HEAD ./
has_changes=$?
echo ">> Diff status $has_changes"

if [[ $VERCEL_GIT_COMMIT_REF == "master" ]] || [ $has_changes == 1 ]; then
  echo ">> Proceeding with deployment."
  exit 1;
else
  echo ">> Skipping deployment."
  exit 0;
fi
