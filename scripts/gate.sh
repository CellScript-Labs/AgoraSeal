#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
mode="${1:-dev}"
case "$mode" in
  dev|ci) ;;
  production)
    echo 'Production admission unavailable: real proof, CKB-VM, CellScript settlement, treasury and node evidence are incomplete. See docs/PRODUCTION.md.' >&2
    exit 1
    ;;
  *) echo 'usage: scripts/gate.sh dev|ci|production' >&2; exit 2 ;;
esac
cargo fmt --all -- --check
cargo fmt --manifest-path zk/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path zk/Cargo.toml -p agoraseal-prover --all-targets -- -D warnings
cargo fmt --manifest-path verifiers/ckb-context/Cargo.toml -- --check
./scripts/build-cellscript.sh
cargo clippy --locked --manifest-path verifiers/ckb-context/Cargo.toml --target riscv64imac-unknown-none-elf --release -- -D warnings
cargo fmt --manifest-path reference/vote/Cargo.toml -- --check
cargo build --locked --manifest-path reference/vote/Cargo.toml --target riscv64imac-unknown-none-elf --release
cargo clippy --locked --manifest-path reference/vote/Cargo.toml --target riscv64imac-unknown-none-elf --release -- -D warnings
cargo check --locked --workspace --all-targets
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo check --locked -p agoraseal-protocol --target riscv64imac-unknown-none-elf
git diff --check
