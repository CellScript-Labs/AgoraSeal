#!/usr/bin/env bash
set -euo pipefail
root="$PWD"
export RUSTUP_TOOLCHAIN=1.97.1
test "$(sha256sum "$(rustup which cargo)" | cut -d ' ' -f 1)" = \
  828980723df339d62434390e9fb8ef8831036583343ae2316b7ab5646b5c1953
mode="${1:?guest or ckb required}"
case "$mode" in
  guest)
    tools="$root/.local/sp1/linux"
    test "$(sha256sum "$tools/prove/cargo-prove" | cut -d ' ' -f 1)" = \
      639e1101649a4c03b6a3e9f0e93f1dc8b884039852c48ab003a504f67d5b6b1f
    test "$(sha256sum "$tools/toolchain/bin/rustc" | cut -d ' ' -f 1)" = \
      9b889df8d4591fbc324a6db88fe7fc8b6830821c89a99472666c04943f215511
    host_cargo="$(rustup which cargo)"
    if ! test -e "$tools/toolchain/bin/cargo"; then
      ln -s "$host_cargo" "$tools/toolchain/bin/cargo"
    fi
    test "$(sha256sum "$tools/toolchain/bin/cargo" | cut -d ' ' -f 1)" = \
      828980723df339d62434390e9fb8ef8831036583343ae2316b7ab5646b5c1953
    env RUSTUP_HOME="$tools/rustup" rustup toolchain link succinct "$tools/toolchain"
    cd zk/program
    env RUSTUP_HOME="$tools/rustup" RUSTUP_TOOLCHAIN=succinct \
      CARGO_TARGET_DIR="$root/.local/canonical-target/guest" \
      CARGO_NET_OFFLINE=true \
      "$tools/prove/cargo-prove" prove build --locked \
      --binaries agoraseal-guest --output-directory "$root/target/sp1"
    ;;
  ckb)
    for component in ckb-context sp1-plonk; do
      # ckb-alt-bn128 does not track its custom CLANG selector for Cargo cache
      # invalidation. Keep this reviewed assembler environment in a new target.
      export CARGO_TARGET_DIR="$root/.local/canonical-target/$component-linux-clang19"
      cargo build --locked --offline --manifest-path "verifiers/$component/Cargo.toml" \
        --target riscv64imac-unknown-none-elf --release
      mkdir -p "verifiers/$component/target/riscv64imac-unknown-none-elf/release"
      cp "$CARGO_TARGET_DIR/riscv64imac-unknown-none-elf/release/agoraseal-$component" \
        "verifiers/$component/target/riscv64imac-unknown-none-elf/release/agoraseal-$component"
    done
    ;;
  rebuild-verifier)
    rebuild_root="$root/${2:?isolated source root required}"
    cargo build --locked --offline \
      --manifest-path "$rebuild_root/verifiers/sp1-plonk/Cargo.toml" \
      --target riscv64imac-unknown-none-elf --release
    ;;
  *) echo 'usage: canonical-inside.sh guest|ckb|rebuild-verifier SOURCE_ROOT' >&2; exit 2 ;;
esac
