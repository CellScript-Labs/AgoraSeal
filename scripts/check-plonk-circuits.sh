#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
root="$PWD"
circuit_root="${SP1_PLONK_CIRCUIT_PATH:-$root/.local/sp1/circuits/plonk}/v6.1.0"
cd "$circuit_root"
for artifact in \
  '111997113 constraints.json' \
  '663838329 plonk_circuit.bin' \
  '2147518120 plonk_pk.bin' \
  '34368 plonk_vk.bin'; do
  read -r expected_size file <<< "$artifact"
  if ! test -f "$file" || test -L "$file" || \
    ! test "$(wc -c < "$file" | tr -d '[:space:]')" = "$expected_size"; then
    echo "Circuit artifact must be a regular file of exactly $expected_size bytes: $file" >&2
    exit 1
  fi
done
if command -v sha256sum >/dev/null 2>&1; then
  sha256sum --check "$root/zk/plonk-artifacts.sha256"
elif command -v shasum >/dev/null 2>&1; then
  shasum -a 256 --check "$root/zk/plonk-artifacts.sha256"
else
  echo 'A SHA-256 implementation (sha256sum or shasum) is required' >&2
  exit 1
fi
