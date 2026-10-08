#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
root="$PWD"
mode="${1:-test-guest}"
case "$mode" in
  build|execute|test-guest|prove-core|prove-plonk|verify-plonk|verify-ckb) ;;
  *) echo 'usage: scripts/zk.sh build|execute|test-guest|prove-core|prove-plonk|verify-plonk|verify-ckb' >&2; exit 2 ;;
esac
for unsafe_mode in SP1_CIRCUIT_MODE WITHOUT_VK_VERIFICATION SP1_DUMP; do
  if test "${!unsafe_mode+x}" = x; then
    echo "$unsafe_mode must be unset for proof evidence" >&2
    exit 1
  fi
done
if test "$mode" = verify-plonk; then
  cargo build --locked --release --manifest-path zk/Cargo.toml -p agoraseal-prover --bin agoraseal-verify-plonk
  zk/target/release/agoraseal-verify-plonk target/guest-fixture
  cargo test --locked -p agoraseal-protocol --test recovery_cli -- --include-ignored --nocapture
  exit 0
fi
if test "$mode" = prove-plonk; then
  export SP1_PLONK_CIRCUIT_PATH="${SP1_PLONK_CIRCUIT_PATH:-$root/.local/sp1/circuits/plonk}"
  # The SDK checks only directory existence; verify every consumed release
  # artifact before any expensive proving or automatic SDK download can occur.
  (
    cd "$SP1_PLONK_CIRCUIT_PATH/v6.1.0"
    for artifact in \
      '111997113 constraints.json' \
      '663838329 plonk_circuit.bin' \
      '2147518120 plonk_pk.bin' \
      '34368 plonk_vk.bin'; do
      read -r expected_size file <<< "$artifact"
      if ! test -f "$file" || test -L "$file" || \
        ! test "$(stat -c %s "$file")" = "$expected_size"; then
        echo "Circuit artifact must be a regular file of exactly $expected_size bytes: $file" >&2
        exit 1
      fi
    done
    sha256sum --check "$root/zk/plonk-artifacts.sha256"
  )
fi
if test "$mode" = verify-ckb; then
  ./scripts/build-cellscript.sh
  cargo test --locked -p agoraseal-protocol --test sp1_vm -- --include-ignored --nocapture
  exit 0
fi
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
features=()
if test "$mode" = prove-plonk; then features=(--features native-gnark); fi
cargo build --locked --release --manifest-path zk/Cargo.toml -p agoraseal-prover "${features[@]}"
if test "$mode" = build; then exit 0; fi
AGORASEAL_EXPORT_GUEST_FIXTURE="$root/target/guest-fixture" cargo test --locked -p agoraseal-protocol --test replay guest_public_statement_is_canonical_and_context_bound
zk/target/release/agoraseal-prover "$mode" target/sp1/agoraseal-guest target/guest-fixture
if test "$mode" = prove-core || test "$mode" = prove-plonk; then
  test "$(cat target/guest-fixture/program-vkey.txt)" = "$(cat zk/program-vkey.txt)" || {
    echo 'Guest program key differs from the reviewed pin' >&2
    exit 1
  }
fi
