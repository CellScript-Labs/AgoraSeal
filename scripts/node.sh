#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
test "$#" = 3 || { echo 'usage: scripts/node.sh PINNED_CKB_REPO PINNED_CKB_BINARY NEW_RUN_DIRECTORY' >&2; exit 2; }
./scripts/build-cellscript.sh
./scripts/zk.sh build
cargo build --locked --release --manifest-path zk/Cargo.toml -p agoraseal-prover --features native-gnark --bins
cargo build --locked -p agoraseal-node --bins
./scripts/check-plonk-circuits.sh
# A running harness must not observe a subsequent cargo build replacing itself.
harness=$(mktemp "$PWD/target/agoraseal-node.XXXXXX")
trap 'rm -f "$harness"' EXIT
cp target/debug/agoraseal-node "$harness"
"$harness" "$1" "$2" "$3"
npm ci --prefix sdk --ignore-scripts --no-audit --no-fund
node --experimental-strip-types sdk/src/rehearse.ts "$1" "$2" "$3" "$3-sdk"
