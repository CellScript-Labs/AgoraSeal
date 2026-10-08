#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p .local
repro_root=$(mktemp -d "$PWD/.local/verifier-rebuild.XXXXXX")
# Keep the isolated build under ignored .local for inspection after failure.
mkdir -p "$repro_root/zk"
for component in ckb-context ckb-state-context sp1-plonk; do
  mkdir -p "$repro_root/verifiers/$component/src"
  cp "verifiers/$component/Cargo.toml" "verifiers/$component/Cargo.lock" "$repro_root/verifiers/$component/"
  cp "verifiers/$component/src/main.rs" "$repro_root/verifiers/$component/src/"
done
cp zk/program-vkey.txt "$repro_root/zk/"
# Fresh output directory and a different source root; the same pinned toolchain
# and cached dependencies are shared. This is not a full clean-room OS rebuild.
./scripts/canonical-build.sh rebuild-verifier "${repro_root#"$PWD/"}"
for component in ckb-context ckb-state-context sp1-plonk; do
  file="$repro_root/verifiers/$component/target/riscv64imac-unknown-none-elf/release/agoraseal-$component"
  cmp "verifiers/$component/target/riscv64imac-unknown-none-elf/release/agoraseal-$component" "$file"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file"
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file"
  else
    echo 'A SHA-256 implementation (sha256sum or shasum) is required' >&2
    exit 1
  fi
done
