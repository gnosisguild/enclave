#!/usr/bin/env bash
set -euo pipefail

val() { grep "$1 = " flake.nix | head -1 | sed 's/.*"\(.*\)".*/\1/'; }

NOIR_REV=$(grep 'rev = ' flake.nix | head -1 | sed 's/.*"\(.*\)".*/\1/')
BB_VER=$(val bbVersion)
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

for p in amd64-linux arm64-linux amd64-darwin arm64-darwin; do
  check "bb-${p}" \
    "https://github.com/AztecProtocol/aztec-packages/releases/download/v${BB_VER}/barretenberg-${p}.tar.gz" \
    "$(grep "\"${p}\" = " flake.nix | sed 's/.*"\(sha256-[^"]*\)".*/\1/')"
done

exit $FAIL
