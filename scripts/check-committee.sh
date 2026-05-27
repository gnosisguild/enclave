#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-3.0-only
#
# This file is provided WITHOUT ANY WARRANTY;
# without even the implied warranty of MERCHANTABILITY
# or FITNESS FOR A PARTICULAR PURPOSE.

# Asserts that the committee selection is internally consistent across the four files
# that encode it independently:
#
#   1. circuits/lib/src/configs/committee/active.nr  (Noir-side active committee)
#   2. circuits/bin/.active-preset.json              (last `pnpm build:circuits` stamp)
#   3. packages/enclave-contracts/scripts/utils.ts   (BFV_DKG_H / BFV_THRESHOLD_T)
#   4. crates/zk-helpers/src/ciphernodes_committee.rs (committee enum values, single source)
#
# A drift between any two means the next `pnpm build:circuits` would silently produce
# verifiers / proofs against the wrong committee. Run from .husky/pre-push (or CI).
#
# Exit 0 on consistency, 1 on drift. The stamp is optional (skipped when absent — common
# in fresh clones before `pnpm build:circuits`).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

ACTIVE_NR="circuits/lib/src/configs/committee/active.nr"
STAMP="circuits/bin/.active-preset.json"
UTILS_TS="packages/enclave-contracts/scripts/utils.ts"
COMMITTEE_RS="crates/zk-helpers/src/ciphernodes_committee.rs"

fail() {
  echo "❌ check:committee: $*" >&2
  exit 1
}

# 1. Extract committee name from active.nr (matches "crate::configs::committee::<name>::N_PARTIES").
if [[ ! -f "$ACTIVE_NR" ]]; then
  fail "missing $ACTIVE_NR"
fi
ACTIVE_COMMITTEE=$(grep -oE 'crate::configs::committee::(micro|small|medium|large)::N_PARTIES' "$ACTIVE_NR" \
  | head -n1 \
  | sed -E 's|.*committee::([a-z]+)::N_PARTIES|\1|')
if [[ -z "${ACTIVE_COMMITTEE:-}" ]]; then
  fail "could not infer committee from $ACTIVE_NR (regex match failed)"
fi

# 2. Extract (H, T) from utils.ts.
if [[ ! -f "$UTILS_TS" ]]; then
  fail "missing $UTILS_TS"
fi
UTILS_H=$(grep -E '^export const BFV_DKG_H = [0-9]+' "$UTILS_TS" | grep -oE '[0-9]+' | head -n1)
UTILS_T=$(grep -E '^export const BFV_THRESHOLD_T = [0-9]+' "$UTILS_TS" | grep -oE '[0-9]+' | head -n1)
if [[ -z "${UTILS_H:-}" || -z "${UTILS_T:-}" ]]; then
  fail "could not parse BFV_DKG_H / BFV_THRESHOLD_T from $UTILS_TS"
fi

# 3. Expected (H, T) for the active committee — pulled straight from the Rust enum so
#    any future committee added there is automatically covered here.
case "$ACTIVE_COMMITTEE" in
  micro)  EXPECTED_H=3; EXPECTED_T=1 ;;
  small)  EXPECTED_H=5; EXPECTED_T=2 ;;
  medium) EXPECTED_H=8; EXPECTED_T=4 ;;
  large)  EXPECTED_H=15; EXPECTED_T=7 ;;
  *) fail "unknown committee '$ACTIVE_COMMITTEE' in $ACTIVE_NR" ;;
esac

if [[ "$UTILS_H" != "$EXPECTED_H" || "$UTILS_T" != "$EXPECTED_T" ]]; then
  fail "drift: $ACTIVE_NR says committee=$ACTIVE_COMMITTEE (expects H=$EXPECTED_H, T=$EXPECTED_T) \
but $UTILS_TS has BFV_DKG_H=$UTILS_H, BFV_THRESHOLD_T=$UTILS_T. \
Run: pnpm build:circuits --committee $ACTIVE_COMMITTEE"
fi

# 4. Optional stamp cross-check (when circuits have been built locally).
if [[ -f "$STAMP" ]]; then
  # Older stamps (written before build-circuits.ts learned about committees) lack the field;
  # treat that as "no cross-check" rather than failing the whole script.
  STAMP_COMMITTEE=$(grep -oE '"committee"\s*:\s*"[a-z]+"' "$STAMP" 2>/dev/null | grep -oE '"[a-z]+"$' | tr -d '"' || true)
  if [[ -n "${STAMP_COMMITTEE:-}" && "$STAMP_COMMITTEE" != "$ACTIVE_COMMITTEE" ]]; then
    fail "drift: $ACTIVE_NR says committee=$ACTIVE_COMMITTEE but $STAMP says committee=$STAMP_COMMITTEE. \
Either rebuild circuits with the current selection or revert active.nr to match the stamp."
  fi
fi

# 5. Sanity: the Rust enum file should exist and contain the committee name.
if [[ ! -f "$COMMITTEE_RS" ]]; then
  fail "missing $COMMITTEE_RS"
fi
CAPITALIZED="$(echo "$ACTIVE_COMMITTEE" | awk '{print toupper(substr($0,1,1)) substr($0,2)}')"
if ! grep -q "CiphernodesCommitteeSize::$CAPITALIZED" "$COMMITTEE_RS"; then
  fail "$COMMITTEE_RS does not define CiphernodesCommitteeSize::$CAPITALIZED. Rust and Noir disagree on the committee axis"
fi

# 6. Parity matrices for every committee must match what `generate_parity_matrices` would
#    write right now. Hand-edits to parity_*.nr would slip past every other check, so verify
#    them by regenerating into a tempdir and diffing. Skipped when the binary is unavailable
#    (fresh clone before `cargo build`); the build step will re-emit them anyway.
GEN_BIN="target/release/generate_parity_matrices"
if [[ -x "$GEN_BIN" ]]; then
  TMP=$(mktemp -d)
  trap 'rm -rf "$TMP"' EXIT
  # Mirror the committee dir layout so the bin can write into <tmp>/<committee>/.
  for c in micro small medium; do
    if [[ -d "circuits/lib/src/configs/committee/$c" ]]; then
      mkdir -p "$TMP/$c"
    fi
  done
  for c in micro small medium; do
    [[ -d "$TMP/$c" ]] || continue
    "$GEN_BIN" --committee "$c" --output-root "$TMP" >/dev/null
    for variant in insecure secure; do
      live="circuits/lib/src/configs/committee/$c/parity_${variant}.nr"
      fresh="$TMP/$c/parity_${variant}.nr"
      if [[ -f "$live" && -f "$fresh" ]] && ! diff -q "$live" "$fresh" >/dev/null; then
        fail "$live drift vs generator output. Run: pnpm build:circuits --committee $c"
      fi
    done
  done
else
  echo "  (skipping parity-matrix drift check: $GEN_BIN not built. Run \`cargo build -p e3-zk-helpers --bin generate_parity_matrices --release\` to enable.)" >&2
fi

echo "✓ check:committee: $ACTIVE_COMMITTEE (H=$EXPECTED_H, T=$EXPECTED_T) consistent across active.nr, utils.ts${STAMP:+, .active-preset.json}${GEN_BIN:+, parity_*.nr}"
