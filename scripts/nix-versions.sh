#!/usr/bin/env bash
set -euo pipefail
val() { grep "$1 = " flake.nix | head -1 | sed 's/.*"\(.*\)".*/\1/'; }
NOIR_REV=$(grep 'rev = ' flake.nix | head -1 | sed 's/.*"\(.*\)".*/\1/')
VERSIONS_JSON="./crates/zk-prover/versions.json"
BB_VER=$(jq -r '.required_bb_version' "$VERSIONS_JSON")
FAIL=0
check() {
  local name="$1" url="$2" expected="$3" unpack="${4:-}"
  printf "%-35s" "$name"
  got=$(nix hash convert --to sri --hash-algo sha256 "$(nix-prefetch-url $unpack --type sha256 "$url" 2>/dev/null)")
  if [[ "$got" == "$expected" ]]; then echo "$name is correct ✅"
  else echo "❌ expected $expected got $got"; FAIL=1; fi
}
check "noir" \
  "https://github.com/noir-lang/noir/archive/${NOIR_REV}.tar.gz" \
  "$(val noirHash)" --unpack

# COMMENTING OUT WHILE WE ARE MISSING THE CORRECT VERSION
# WAITING FOR UPGRADE OF BB VERSION
# for p in amd64-linux arm64-linux amd64-darwin arm64-darwin; do
for p in amd64-linux; do
  hex=$(jq -r ".bb_checksums[\"${p}\"]" "$VERSIONS_JSON")
  sri=$(nix hash convert --to sri --hash-algo sha256 "$hex")
  check "bb-${p}" \
    "https://github.com/AztecProtocol/aztec-packages/releases/download/v${BB_VER}/barretenberg-${p}.tar.gz" \
    "$sri"
done
exit $FAIL
