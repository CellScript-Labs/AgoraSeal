#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
root="$PWD"
mode="${1:-test-guest}"
case "$mode" in
  build|execute|test-guest|prove-core) ;;
  *) echo 'usage: scripts/zk.sh build|execute|test-guest|prove-core' >&2; exit 2 ;;
esac
prove="$root/.local/sp1/cargo-prove"
check_hash() {
  local expected="$1" file="$2" actual
  actual="$(sha256sum "$file")"
  test "${actual%% *}" = "$expected" || {
    echo "Toolchain hash mismatch: $file; see zk/README.md" >&2
    exit 1
  }
}
check_hash 639e1101649a4c03b6a3e9f0e93f1dc8b884039852c48ab003a504f67d5b6b1f "$prove"
check_hash 9b889df8d4591fbc324a6db88fe7fc8b6830821c89a99472666c04943f215511 "$(rustup which --toolchain succinct rustc)"
check_hash 828980723df339d62434390e9fb8ef8831036583343ae2316b7ab5646b5c1953 "$(rustup which --toolchain succinct cargo)"
cargo fmt --manifest-path zk/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path zk/Cargo.toml -p agoraseal-prover --all-targets -- -D warnings
(
  cd zk/program
  "$prove" prove build --locked --binaries agoraseal-guest --output-directory "$root/target/sp1"
)
sha256sum --check zk/guest.sha256
cargo build --locked --release --manifest-path zk/Cargo.toml -p agoraseal-prover
if test "$mode" = build; then exit 0; fi
AGORASEAL_EXPORT_GUEST_FIXTURE="$root/target/guest-fixture" cargo test --locked -p agoraseal-protocol --test replay guest_public_statement_is_canonical_and_context_bound
zk/target/release/agoraseal-prover "$mode" target/sp1/agoraseal-guest target/guest-fixture
if test "$mode" = prove-core; then
  test "$(cat target/guest-fixture/program-vkey.txt)" = "$(cat zk/program-vkey.txt)" || {
    echo 'Guest program key differs from the reviewed pin' >&2
    exit 1
  }
fi
