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
mkdir -p target/cellscript
"$compiler" contracts/treasury/treasury.cell --target riscv64-elf --target-profile ckb \
  --entry-lock pay_passed_proposal --primitive-strict 0.16 -o target/cellscript/treasury.elf
# This checker flag admits an artifact boundary, not the AgoraSeal application.
"$compiler" verify-artifact target/cellscript/treasury.elf --verify-sources \
  --expect-target-profile ckb --production --json > target/cellscript/treasury.checker.json
printf '%s\n' "$compiler_pin" > target/cellscript/compiler-commit.txt
