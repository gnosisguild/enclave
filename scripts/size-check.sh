#!/usr/bin/env bash

set -e

git diff -w origin/$1...HEAD | \
  grep -v -E '(\.lock$|lock\.yaml$)' | \
  grep "^[+-]" | \
  grep -v "^[+-]\s*$" | \
  grep -v "^[+-]\s*(//|#|\*|/\*|\*/)" | \
  grep -v "^(---|\+\+\+|@@)" | \
  wc -l
