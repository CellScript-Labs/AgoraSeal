# Unique state and header coordinate adapter

Generic CKB adapter only: no DAO, vote, amount, window, quorum or payout policy.
The parent remains actual CellScript. Cargo.lock and the exact ELF are pinned;
the template checker admits no automatic repin.

argv has four strings. The first is exactly 226 lowercase hex characters
(113 bytes); the remaining three are empty. The current parent Script must be
exactly 85 serialized bytes with exactly 32 argument bytes. The child calls
ckb-std 1.1.0 validate_type_id using those current arguments. Creation derives
the identifier from the first global CellInput and group output's global index.
Transfers allow at most one group input/output; the parent's .cell additionally
forbids burns and receipt consumption.

Tuple layouts:

| Range | Creation mode 0 | Settlement mode 1 |
| --- | --- | --- |
| 0 | 0 | 1 |
| 1..33 | Genesis hash | Genesis hash |
| 33..65 | All zero | Start hash |
| 65..97 | All zero | End hash |
| 97..105 | All zero | Start number, little-endian u64 |
| 105..113 | All zero | End number, little-endian u64 |

Creation requires HeaderDep 0 to be the exact claimed hash with number zero.
Settlement requires HeaderDeps 0/1/2 to be the exact start/end/genesis tuple and
the actual GroupInput 0 creation header to equal start. Every syscall loads
an exact 208-byte Header; no host-provided number or unrelated input is authority.

CKB consensus checks HeaderDeps for main-chain membership before resolving
a transaction. The pinned node source implements this in
[VerifyContext::check_valid](https://github.com/nervosnetwork/ckb/blob/f7fa4436737756f97a24e254f22c13a36316ecea/verification/contextual/src/contextual_block_verifier.rs)
and
[Snapshot::check_valid](https://github.com/nervosnetwork/ckb/blob/f7fa4436737756f97a24e254f22c13a36316ecea/util/snapshot/src/lib.rs).
The VM syscall alone and artificial Context headers do not prove membership
or finality. A reorg can invalidate the transaction's anchors; clients must
refresh them and the proof before retrying.

Errors: 10 malformed argv/mode, 11 current Script encoding/size, 12 Type-ID
failure, 13 unavailable/incorrectly sized header, 14 tuple mismatch. The parent
SPAWN/WAIT wrapper propagates child failure as its own child-failure code.
