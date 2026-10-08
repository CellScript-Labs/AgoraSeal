#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
compiler_root="${CELLSCRIPT_ROOT:-../CellScript}"
test "$(git -C "$compiler_root" rev-parse HEAD)" = 35bf983db30aae80281f97e30bbce08a878d7c58
repro_root=$(mktemp -d "$PWD/.local/policy-rebuild.XXXXXX")
mkdir -p "$repro_root/contracts/funded-proposal/src" "$repro_root/contracts/funded-treasury"
cp contracts/funded-proposal/Cell.toml contracts/funded-proposal/Cell.lock "$repro_root/contracts/funded-proposal/"
cp contracts/funded-proposal/src/main.cell "$repro_root/contracts/funded-proposal/src/"
cp contracts/funded-treasury/treasury.cell "$repro_root/contracts/funded-treasury/"
"$compiler_root/target/debug/cellc" "$repro_root/contracts/funded-proposal" \
  --target riscv64-elf --target-profile ckb --entry-action settle --primitive-strict 0.16 -o "$repro_root/proposal.elf"
"$compiler_root/target/debug/cellc" "$repro_root/contracts/funded-treasury/treasury.cell" \
  --target riscv64-elf --target-profile ckb --entry-lock release --primitive-strict 0.16 -o "$repro_root/treasury.elf"
for artifact in proposal treasury; do
  cmp "target/cellscript/funded-$artifact.elf" "$repro_root/$artifact.elf"
  "$compiler_root/target/debug/cellc" verify-artifact "$repro_root/$artifact.elf" \
    --verify-sources --expect-target-profile ckb --production --json > "$repro_root/$artifact.checker.json"
done
