#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
root="$PWD"
mode="${1:-test-guest}"
case "$mode" in
  build|execute|test-guest|inspect-guest|prove-core|prove-plonk|verify-plonk|verify-ckb|verify-circuits) ;;
  *) echo 'usage: scripts/zk.sh build|execute|test-guest|inspect-guest|prove-core|prove-plonk|verify-plonk|verify-ckb|verify-circuits' >&2; exit 2 ;;
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
  AGORASEAL_PROOF_FIXTURE="$root/target/guest-fixture" \
    cargo test --locked -p agoraseal-protocol --test recovery_cli -- --nocapture
  exit 0
fi
if test "$mode" = prove-plonk || test "$mode" = verify-circuits; then
  export SP1_PLONK_CIRCUIT_PATH="${SP1_PLONK_CIRCUIT_PATH:-$root/.local/sp1/circuits/plonk}"
  # The SDK checks only directory existence; verify every consumed release
  # artifact before any expensive proving or automatic SDK download can occur.
  ./scripts/check-plonk-circuits.sh
  if test "$mode" = verify-circuits; then exit 0; fi
fi
if test "$mode" = verify-ckb; then
  ./scripts/build-cellscript.sh
  AGORASEAL_PROOF_FIXTURE="$root/target/guest-fixture" \
    cargo test --locked -p agoraseal-protocol --test sp1_vm -- --nocapture
  exit 0
fi
prove="$root/.local/sp1/cargo-prove"
check_hash() {
  local expected="$1" file="$2" actual
  if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$file")"
  elif command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "$file")"
  else
    echo 'A SHA-256 implementation (sha256sum or shasum) is required' >&2
    exit 1
  fi
  test "${actual%% *}" = "$expected" || {
    echo "Toolchain hash mismatch: $file; see zk/README.md" >&2
    exit 1
  }
}
cargo fmt --manifest-path zk/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path zk/Cargo.toml -p agoraseal-prover --all-targets -- -D warnings
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)
    prove_hash=639e1101649a4c03b6a3e9f0e93f1dc8b884039852c48ab003a504f67d5b6b1f
    rustc_hash=9b889df8d4591fbc324a6db88fe7fc8b6830821c89a99472666c04943f215511
    cargo_hash=828980723df339d62434390e9fb8ef8831036583343ae2316b7ab5646b5c1953
    check_hash "$prove_hash" "$prove"
    check_hash "$rustc_hash" "$(rustup which --toolchain succinct rustc)"
    check_hash "$cargo_hash" "$(rustup which --toolchain succinct cargo)"
    (
      cd zk/program
      "$prove" prove build --locked --binaries agoraseal-guest --output-directory "$root/target/sp1"
    )
    ;;
  Darwin-arm64)
    # The native macOS guest has a different program key. Reproduce the
    # original Linux ELF instead of silently changing the admitted identity.
    ./scripts/install-canonical-guest.sh
    ./scripts/canonical-build.sh guest
    ;;
  *) echo 'No reviewed SP1 toolchain pins for this platform; see zk/README.md' >&2; exit 1 ;;
esac
check_hash "$(cut -d ' ' -f 1 zk/guest.sha256)" target/sp1/agoraseal-guest
if test "$mode" = prove-plonk; then
  cargo build --locked --release --manifest-path zk/Cargo.toml -p agoraseal-prover --features native-gnark
else
  cargo build --locked --release --manifest-path zk/Cargo.toml -p agoraseal-prover
fi
if test "$mode" = build; then exit 0; fi
AGORASEAL_EXPORT_GUEST_FIXTURE="$root/target/guest-fixture" cargo test --locked -p agoraseal-protocol --test replay guest_public_statement_is_canonical_and_context_bound
zk/target/release/agoraseal-prover "$mode" target/sp1/agoraseal-guest target/guest-fixture
if test "$mode" = prove-core || test "$mode" = prove-plonk; then
  test "$(cat target/guest-fixture/program-vkey.txt)" = "$(cat zk/program-vkey.txt)" || {
    echo 'Guest program key differs from the reviewed pin' >&2
    exit 1
  }
fi
