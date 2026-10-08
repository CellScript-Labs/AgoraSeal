#!/usr/bin/env bash
# Native clippy still compiles the RISC-V assembly dependency on macOS.
# Apple clang/ar cannot provide that archive. Executable release artifacts
# are built separately in Linux and checked against the original pins.
if test "$(uname -s)" = Darwin; then
  llvm_root=/opt/homebrew/opt/llvm/bin
  if ! test -x "$llvm_root/clang"; then llvm_root=/usr/local/opt/llvm/bin; fi
  if ! test -x "$llvm_root/clang" || ! test -x "$llvm_root/llvm-ar"; then
    echo 'macOS native RISC-V checks require Homebrew LLVM (clang and llvm-ar)' >&2
    return 1
  fi
  export CLANG="${CLANG:-$llvm_root/clang}"
  export AR_riscv64imac_unknown_none_elf="${AR_riscv64imac_unknown_none_elf:-$llvm_root/llvm-ar}"
fi
