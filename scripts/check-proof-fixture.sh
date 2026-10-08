#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
for file in input.bin block-000000.bin block-000001.bin block-000002.bin \
  block-000003.bin public-values.bin program-vkey.txt proof-plonk.bin proof-plonk-bundle.bin; do
  path="tests/fixtures/sp1-plonk/$file"
  test -f "$path" && ! test -L "$path" || {
    echo "Proof fixture must be a regular file: $path" >&2
    exit 1
  }
done
if command -v sha256sum >/dev/null 2>&1; then
  sha256sum --check tests/fixtures/sp1-plonk/artifacts.sha256
elif command -v shasum >/dev/null 2>&1; then
  shasum -a 256 --check tests/fixtures/sp1-plonk/artifacts.sha256
else
  echo 'A SHA-256 implementation (sha256sum or shasum) is required' >&2
  exit 1
fi
