# Direct-prefix CKB context adapter v1

This small no_std RISC-V program supplies CKB context facts not directly
available through the current CellScript source surface. It is policy-neutral:
there are no DAO, voting, owner, capacity, proposal, quorum, age-comparison or
payment decisions. It is not a ZK verifier.

The exact `hex4-v1` argv contract is four lowercase hexadecimal strings:

1. First CellDep OutPoint: 36 bytes.
2. Claimed first CellDep creation block number: 8-byte little-endian.
3. Claimed second CellDep creation block number: 8-byte little-endian.
4. Empty string, reserved and required empty.

The child loads and validates the complete Molecule Transaction within 65,536
bytes. It permits 2..128 raw CellDeps, requires the first two to be direct and
rejects repeated raw dependency OutPoints. Thus the resolved first two entries
correspond to these raw entries. It matches the first OutPoint and uses actual
`LOAD_HEADER(index, Source::CellDep)` results to authenticate both numbers.
These syscalls require the corresponding headers in HeaderDeps. Each returned
header must be exactly 208 bytes; borrowed readers avoid unsupported atomics.

Errors: 10 arguments/hex, 11 transaction/size/count, 12 dependency identity or
prefix, 13 header loading/encoding, 14 claimed height mismatch. CellScript's
bounded spawn adapter fails closed on any nonzero child status; parent tests
also verify hash substitution rejection before invocation.

Source of syscall semantics: CKB RFC0009 and ckb-std 1.1.0; Cargo.lock pins
the exact generated Molecule implementation. This program is part of the
application's trusted computing base and requires review and full resource
evidence before production. It does not substitute for native proposal/DAO
validation, the node, signatures or proof admission.
