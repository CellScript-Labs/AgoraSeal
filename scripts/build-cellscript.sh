#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
compiler_root="${CELLSCRIPT_ROOT:-../CellScript}"
compiler_pin='35bf983db30aae80281f97e30bbce08a878d7c58'
if [[ "$(git -C "$compiler_root" rev-parse HEAD)" != "$compiler_pin" ]]; then
  echo "CellScript checkout must match $compiler_pin" >&2
  exit 1
fi
# Local documentation edits do not affect the compiler build. Executable inputs do.
git -C "$compiler_root" diff --exit-code HEAD -- src crates Cargo.toml Cargo.lock rust-toolchain.toml build.rs
if [[ -n "$(git -C "$compiler_root" ls-files --others --exclude-standard -- src crates)" ]]; then
  echo 'Untracked compiler sources prevent a pinned build' >&2
  exit 1
fi
cargo build --locked --manifest-path "$compiler_root/Cargo.toml" -p cellscript --bin cellc
compiler="$compiler_root/target/debug/cellc"
# Download only locked dependencies before the read-only offline container.
# Linux host clang versions can also change the admitted SP1 ELF.
for component in ckb-context ckb-state-context sp1-plonk; do
  cargo fetch --locked --manifest-path "verifiers/$component/Cargo.toml"
done
./scripts/canonical-build.sh ckb
cargo run --locked -p agoraseal-cli -- context-pin \
  verifiers/ckb-context/target/riscv64imac-unknown-none-elf/release/agoraseal-ckb-context \
  contracts/vote --check
cargo run --locked -p agoraseal-cli -- verifier-pin \
  verifiers/sp1-plonk/target/riscv64imac-unknown-none-elf/release/agoraseal-sp1-plonk \
  contracts/proof-binding --check
mkdir -p target/cellscript
"$compiler" contracts/treasury/treasury.cell --target riscv64-elf --target-profile ckb \
  --entry-lock pay_passed_proposal --primitive-strict 0.16 -o target/cellscript/treasury.elf
# This checker flag admits an artifact boundary, not the AgoraSeal application.
"$compiler" verify-artifact target/cellscript/treasury.elf --verify-sources \
  --expect-target-profile ckb --production --json > target/cellscript/treasury.checker.json
"$compiler" contracts/vote --target riscv64-elf --target-profile ckb \
  --entry-action cast --primitive-strict 0.16 -o target/cellscript/vote.elf
"$compiler" verify-artifact target/cellscript/vote.elf --verify-sources \
  --expect-target-profile ckb --production --json > target/cellscript/vote.checker.json
"$compiler" contracts/proof-binding --target riscv64-elf --target-profile ckb \
  --entry-action verify --primitive-strict 0.16 -o target/cellscript/proof-binding.elf
"$compiler" verify-artifact target/cellscript/proof-binding.elf --verify-sources \
  --expect-target-profile ckb --production --json > target/cellscript/proof-binding.checker.json
"$compiler" contracts/funded-treasury/treasury.cell --target riscv64-elf --target-profile ckb \
  --entry-lock release --primitive-strict 0.16 -o target/cellscript/funded-treasury.elf
"$compiler" verify-artifact target/cellscript/funded-treasury.elf --verify-sources \
  --expect-target-profile ckb --production --json > target/cellscript/funded-treasury.checker.json
cargo run --locked -p agoraseal-cli -- lifecycle-pin \
  verifiers/ckb-state-context/target/riscv64imac-unknown-none-elf/release/agoraseal-ckb-state-context \
  verifiers/sp1-plonk/target/riscv64imac-unknown-none-elf/release/agoraseal-sp1-plonk \
  target/cellscript/vote.elf target/cellscript/funded-treasury.elf contracts/funded-proposal --check
"$compiler" contracts/funded-proposal --target riscv64-elf --target-profile ckb \
  --entry-action settle --primitive-strict 0.16 -o target/cellscript/funded-proposal.elf
"$compiler" verify-artifact target/cellscript/funded-proposal.elf --verify-sources \
  --expect-target-profile ckb --production --json > target/cellscript/funded-proposal.checker.json
cargo run --locked -p agoraseal-cli -- sdk-pin "$PWD" --check
printf '%s\n' "$compiler_pin" > target/cellscript/compiler-commit.txt
