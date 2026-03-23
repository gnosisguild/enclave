#!/usr/bin/env bash
# CRISP fold public key_hash = compute_vk_hash(ude, crisp, ct0, ct1). Needs pnpm compile:circuits.
set -euo pipefail

CRISP="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO="$(cd "$CRISP/../.." && pwd)"
R="$REPO/circuits/bin/threshold/target/recursive_vk"
VK=(
  "$R/user_data_encryption/vk_hash"
  "$CRISP/circuits/bin/crisp/target/recursive_vk/crisp/vk_hash"
  "$R/user_data_encryption_ct0/vk_hash"
  "$R/user_data_encryption_ct1/vk_hash"
)
for f in "${VK[@]}"; do
  [[ -f "$f" ]] || { echo "missing $f (run pnpm compile:circuits in examples/CRISP)" >&2; exit 1; }
done
(cd "$REPO" && cargo run -q -p e3-zk-helpers --bin compute-vk-hash -- "${VK[@]}")
