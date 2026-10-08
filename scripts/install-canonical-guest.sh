#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
root="$PWD"
check_hash() {
  local expected="$1" file="$2" actual
  if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$file")"
  elif command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "$file")"
  else
    echo 'A SHA-256 implementation (sha256sum or shasum) is required' >&2
    exit 1
  fi
  test "${actual%% *}" = "$expected" || {
    echo "Canonical guest tool hash mismatch: $file" >&2
    exit 1
  }
}
if test -d .local/sp1/linux; then
  check_hash 639e1101649a4c03b6a3e9f0e93f1dc8b884039852c48ab003a504f67d5b6b1f \
    .local/sp1/linux/prove/cargo-prove
  check_hash 9b889df8d4591fbc324a6db88fe7fc8b6830821c89a99472666c04943f215511 \
    .local/sp1/linux/toolchain/bin/rustc
  echo 'Existing canonical guest tools match the reviewed Linux pins'
  exit 0
fi
mkdir -p .local/sp1
prove_archive=.local/sp1/cargo-prove-linux-amd64.tar.gz
rust_archive=.local/sp1/toolchain-linux-amd64.tar.gz
if ! test -f "$prove_archive"; then
  curl --http1.1 -fL --retry 5 --retry-all-errors --output "$prove_archive.part" \
    https://github.com/succinctlabs/sp1/releases/download/v6.1.0/cargo_prove_v6.1.0_linux_amd64.tar.gz
  check_hash 99f0087f581798ec510573baedcd51b97a32c6f6a1f5942612ed252e5285954b "$prove_archive.part"
  mv "$prove_archive.part" "$prove_archive"
fi
check_hash 99f0087f581798ec510573baedcd51b97a32c6f6a1f5942612ed252e5285954b "$prove_archive"
if ! test -f "$rust_archive"; then
  curl --http1.1 -fL --retry 5 --retry-all-errors --output "$rust_archive.part" \
    https://github.com/succinctlabs/rust/releases/download/succinct-1.96.0-64bit-v2/rust-toolchain-x86_64-unknown-linux-gnu.tar.gz
  check_hash ff3afc3a6f22af93d162652972f254fcecf443ca4b2de18897312f19997fb94b "$rust_archive.part"
  mv "$rust_archive.part" "$rust_archive"
fi
check_hash ff3afc3a6f22af93d162652972f254fcecf443ca4b2de18897312f19997fb94b "$rust_archive"
stage="$(mktemp -d "$root/.local/sp1/linux-install.XXXXXX")"
mkdir -p "$stage/prove" "$stage/toolchain"
tar -xzf "$prove_archive" -C "$stage/prove"
tar -xzf "$rust_archive" -C "$stage/toolchain"
check_hash 639e1101649a4c03b6a3e9f0e93f1dc8b884039852c48ab003a504f67d5b6b1f "$stage/prove/cargo-prove"
check_hash 9b889df8d4591fbc324a6db88fe7fc8b6830821c89a99472666c04943f215511 "$stage/toolchain/bin/rustc"
mv "$stage" .local/sp1/linux
echo 'Installed canonical Linux guest tools; registration occurs only inside Docker'
