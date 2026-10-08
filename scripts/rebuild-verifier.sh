#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p .local
repro_root=$(mktemp -d "$PWD/.local/verifier-rebuild.XXXXXX")
# Keep the isolated build under ignored .local for inspection after failure.
mkdir -p "$repro_root/verifiers/sp1-plonk/src" "$repro_root/zk"
cp verifiers/sp1-plonk/Cargo.toml verifiers/sp1-plonk/Cargo.lock "$repro_root/verifiers/sp1-plonk/"
cp verifiers/sp1-plonk/src/main.rs "$repro_root/verifiers/sp1-plonk/src/"
cp zk/program-vkey.txt "$repro_root/zk/"
# Fresh output directory and a different source root; the same pinned toolchain
# and cached dependencies are shared. This is not a full clean-room OS rebuild.
if test "$(uname -s)" = Darwin; then
  ./scripts/canonical-build.sh rebuild-verifier "${repro_root#"$PWD/"}"
else
  cargo build --locked --offline --manifest-path "$repro_root/verifiers/sp1-plonk/Cargo.toml" \
    --target riscv64imac-unknown-none-elf --release
fi
cmp verifiers/sp1-plonk/target/riscv64imac-unknown-none-elf/release/agoraseal-sp1-plonk \
  "$repro_root/verifiers/sp1-plonk/target/riscv64imac-unknown-none-elf/release/agoraseal-sp1-plonk"
if command -v sha256sum >/dev/null 2>&1; then
  sha256sum "$repro_root/verifiers/sp1-plonk/target/riscv64imac-unknown-none-elf/release/agoraseal-sp1-plonk"
elif command -v shasum >/dev/null 2>&1; then
  shasum -a 256 "$repro_root/verifiers/sp1-plonk/target/riscv64imac-unknown-none-elf/release/agoraseal-sp1-plonk"
else
  echo 'A SHA-256 implementation (sha256sum or shasum) is required' >&2
  exit 1
fi
