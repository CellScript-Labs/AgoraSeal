#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if command -v sha256sum >/dev/null 2>&1; then
  checker=(sha256sum)
elif command -v shasum >/dev/null 2>&1; then
  checker=(shasum -a 256)
else
  echo 'A SHA-256 implementation (sha256sum or shasum) is required' >&2
  exit 1
fi
for fixture in sp1-plonk funded-failed; do
  for file in input.bin block-000000.bin block-000001.bin block-000002.bin \
    public-values.bin program-vkey.txt proof-plonk.bin proof-plonk-bundle.bin; do
    path="tests/fixtures/$fixture/$file"
    test -f "$path" && ! test -L "$path" || {
      echo "Proof fixture must be a regular file: $path" >&2
      exit 1
    }
  done
  if test "$fixture" = sp1-plonk; then
    test -f tests/fixtures/sp1-plonk/block-000003.bin && ! test -L tests/fixtures/sp1-plonk/block-000003.bin
  else
    test -f tests/fixtures/funded-failed/creation.bin && ! test -L tests/fixtures/funded-failed/creation.bin
    test -f tests/fixtures/funded-failed/refund-script.bin && ! test -L tests/fixtures/funded-failed/refund-script.bin
  fi
  "${checker[@]}" --check "tests/fixtures/$fixture/artifacts.sha256"
done
for case_name in passed failed ccc ccc/stale; do
  for file in input.bin public-values.bin program-vkey.txt proof-plonk.bin \
    proof-plonk-bundle.bin creation.bin settlement.bin; do
    path="tests/fixtures/node-lifecycle/$case_name/$file"
    test -f "$path" && ! test -L "$path" || exit 1
  done
  count=13
  if [[ "$case_name" = ccc* ]]; then count=17; fi
  for ((index=0; index<count; index++)); do
    printf -v file 'tests/fixtures/node-lifecycle/%s/block-%06d.bin' "$case_name" "$index"
    test -f "$file" && ! test -L "$file" || exit 1
  done
done
for file in genesis-header.bin evidence.json; do
  test -f "tests/fixtures/node-lifecycle/$file" && ! test -L "tests/fixtures/node-lifecycle/$file" || exit 1
done
"${checker[@]}" --check tests/fixtures/node-lifecycle/artifacts.sha256
