#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Check that every commit subject in <base>..<head> follows Conventional Commits,
# `type(scope)!: subject` (spec ci-checks, "Commit messages follow Conventional Commits").
# Merge commits made by Git (subject "Merge ...") are skipped. A base of all zeros (the
# first push of a branch) checks only <head>.
#
# Usage: scripts/check-commits.sh <base> <head>
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <base> <head>" >&2
  exit 2
fi
base=$1
head=$2

types='feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert'
pattern="^($types)(\([a-z0-9,._-]+\))?!?: [^ ].*$"

if [[ $base =~ ^0+$ ]]; then
  range=(--max-count=1 "$head")
else
  range=("$base..$head")
fi
commits=$(git log --format='%H %s' "${range[@]}")

bad_count=0
while read -r hash subject; do
  if [[ -z $hash || $subject == "Merge "* ]]; then
    continue
  fi
  if [[ ! $subject =~ $pattern ]]; then
    echo "Not a Conventional Commit: ${hash:0:12} $subject"
    bad_count=$((bad_count + 1))
  fi
done <<< "$commits"

if [[ $bad_count -gt 0 ]]; then
  echo "Expected: type(scope)!: subject, where type is one of ${types//|/, }; (scope) and ! are optional."
  exit 1
fi
echo "Every commit subject follows Conventional Commits."
