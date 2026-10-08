#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
mode="${1:-dev}"
case "$mode" in
  dev|ci) ;;
  production)
    echo 'Production admission unavailable: maximal-resource/matched-cost, SRS/independent-review, hosted-release and public-network/custody evidence remain incomplete. See docs/PRODUCTION.md.' >&2
    exit 1
    ;;
  *) echo 'usage: scripts/gate.sh dev|ci|production' >&2; exit 2 ;;
esac
source ./scripts/ckb-build-env.sh
for script in scripts/*.sh; do bash -n "$script"; done
cargo fmt --all -- --check
cargo fmt --manifest-path zk/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path zk/Cargo.toml -p agoraseal-prover --all-targets -- -D warnings
./scripts/check-proof-fixture.sh
cargo build --locked --release --manifest-path zk/Cargo.toml -p agoraseal-prover --bin agoraseal-verify-plonk
zk/target/release/agoraseal-verify-plonk tests/fixtures/sp1-plonk
zk/target/release/agoraseal-verify-plonk tests/fixtures/funded-failed
zk/target/release/agoraseal-verify-plonk tests/fixtures/node-lifecycle/passed
zk/target/release/agoraseal-verify-plonk tests/fixtures/node-lifecycle/failed
zk/target/release/agoraseal-verify-plonk tests/fixtures/node-lifecycle/ccc
zk/target/release/agoraseal-verify-plonk tests/fixtures/node-lifecycle/ccc/stale
cargo fmt --manifest-path verifiers/ckb-context/Cargo.toml -- --check
cargo fmt --manifest-path verifiers/ckb-state-context/Cargo.toml -- --check
cargo fmt --manifest-path verifiers/sp1-plonk/Cargo.toml -- --check
cargo clippy --locked --manifest-path verifiers/sp1-plonk/Cargo.toml --target riscv64imac-unknown-none-elf --release -- -D warnings
./scripts/build-cellscript.sh
cargo clippy --locked --manifest-path verifiers/ckb-context/Cargo.toml --target riscv64imac-unknown-none-elf --release -- -D warnings
cargo clippy --locked --manifest-path verifiers/ckb-state-context/Cargo.toml --target riscv64imac-unknown-none-elf --release -- -D warnings
cargo fmt --manifest-path reference/vote/Cargo.toml -- --check
cargo build --locked --manifest-path reference/vote/Cargo.toml --target riscv64imac-unknown-none-elf --release
cargo clippy --locked --manifest-path reference/vote/Cargo.toml --target riscv64imac-unknown-none-elf --release -- -D warnings
cargo check --locked --workspace --all-targets
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo check --locked -p agoraseal-protocol --target riscv64imac-unknown-none-elf
npm ci --prefix sdk --ignore-scripts --no-audit --no-fund
npm run check --prefix sdk
npm test --prefix sdk
if test "$mode" = ci; then
  ./scripts/rebuild-verifier.sh
  ./scripts/rebuild-policies.sh
fi
git diff --check
