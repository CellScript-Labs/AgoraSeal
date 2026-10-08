# Fixed SP1 PLONK verifier for CKB

This Rust child implements cryptographic verification and a bounded witness
adapter. Application rules remain in CellScript. It accepts exactly four
arguments: a lowercase 64-character CKB statement hash and three empty strings.
The proof packet is loaded from GroupInput witness 0 `output_type`, with a
2,048-byte complete-witness bound applied before Molecule parsing. Packet size
is exactly 964 proof bytes plus 277 public bytes. `AGZKPV01` and the canonical
boolean byte are checked; every public byte is bound to the supplied hash.

The guest key comes from `zk/program-vkey.txt` at build time. No caller can
replace the guest key, PLONK VK, recursion VK root or expected exit code zero.
The parent pins the resulting child ELF by full CKB data hash. No key rotation
or verifier upgrade path is implicitly authorized.

The CKB port is pinned to
[`XuJiandong/sp1@f25435f62a49a9d443500a21f5de8d44f83d200d`](https://github.com/XuJiandong/sp1/tree/f25435f62a49a9d443500a21f5de8d44f83d200d/crates/verifier).
It adapts the SP1 verifier for `no_std` and uses `ckb-alt-bn128` optimized curve
arithmetic. This is a shared cryptographic dependency with the reference PoC;
CellScript does not establish a distinct cryptographic security advantage.
The lockfile fixes transitive dependencies. Independent review remains pending.

The embedded release PLONK VK is byte-identical to the official crates.io
`sp1-verifier 6.1.0` VK, with SHA-256
`5a093a2fcb46394f5cadfe55c44d4d572fad9cec7aeb38026b0278322ef07fac`.
The pinned recursion root is
`002f850ee998974d6cc00e50cd0814b098c05bfade466d28573240d057f25352`.
Source and key pins identify the implementation; they do not replace real
proof compatibility tests, a setup/SRS review or application admission.

| Child exit | Meaning |
| --- | --- |
| 10 | Argument count, lowercase hex or forbidden extra argument |
| 11 | Public statement domain/boolean encoding |
| 12 | Cryptographic verification failure |
| 13 | Witness read/bound, Molecule format, missing packet or packet size |
| 14 | Statement commitment mismatch |

The CellScript caller maps a nonzero child exit to its external-call failure.
Tests must distinguish that rejection from a VM infrastructure error or cycle
exhaustion. The current 3-billion-cycle test ceiling is a development limit,
not a measured production budget or a guarantee about maximum invalid cost.

`./scripts/gate.sh ci` rebuilds this verifier offline with a fresh target and
different source directory, then compares the entire ELF byte-for-byte. The
same host toolchain and dependency cache are reused; this is a source-root
reproducibility check, not a hermetic clean-room build. The observed matching
ELF SHA-256 is
`c216a687a84dbda2e9dcb3b6b7b347743d7551e8b92922d92e6b0348e9714cbe`.
