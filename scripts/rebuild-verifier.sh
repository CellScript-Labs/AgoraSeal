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
cargo build --locked --offline --manifest-path "$repro_root/verifiers/sp1-plonk/Cargo.toml" \
  --target riscv64imac-unknown-none-elf --release
cmp verifiers/sp1-plonk/target/riscv64imac-unknown-none-elf/release/agoraseal-sp1-plonk \
  "$repro_root/verifiers/sp1-plonk/target/riscv64imac-unknown-none-elf/release/agoraseal-sp1-plonk"
sha256sum "$repro_root/verifiers/sp1-plonk/target/riscv64imac-unknown-none-elf/release/agoraseal-sp1-plonk"
