# CellScript treasury payment component

`treasury.cell` is the spending policy itself. No Rust function implements its
business checks. The pinned compiler emits a RISC-V ELF and the standalone
artifact checker validates its lowering/source-map/ELF binding. The test suite
executes those bytes in CKB-VM.

The current bounded transaction has exactly one escrow input and one output.
The output pays the whole committed amount to the full recipient Lock hash,
has empty data and no Type Script. The input may additionally contain a fee
reserve, but its excess over the amount must not exceed `max_fee`. Two escrow
inputs cannot share one payment output. No treasury change or general pooled
treasury support is claimed.

The escrow data is fixed packed fields (120 bytes): proposal hash, settlement
Type Script hash, recipient Lock hash, amount, quorum, maximum fee. Hashes are
32 bytes; integers are little-endian u64. CellDep 0 contains a fixed 104-byte
settlement: proposal hash, recipient hash, amount, quorum, YES, NO, passed.
The Lock checks the complete dependency Type hash, all matching commitments,
positive amount/quorum, passed exactly 1, YES > NO, checked total and
total >= quorum. No tally data is taken from an untrusted witness.

**Incomplete authentication chain:** escrow initialization must pin an admitted
immutable ZK settlement Type Script; that Type must authenticate the proof and
proposal/window/program identity. Unique proposal/escrow creation must prevent
duplicate funding claims. Those components are not implemented. Tests use an
always-success settlement Type solely to isolate the payment policy, and must
never be interpreted as production or cryptographic evidence. `--production`
on the artifact checker does not close these application dependencies.

Initial measured fixture: 5,472-byte ELF; 16,890 CKB-VM transaction cycles.
This includes the isolated payment fixture, not proof verification, signatures,
node validation or maximal-case measurement. It is not a comparison against
an SP1 proposal verifier. Run `./scripts/gate.sh dev` to rebuild and recheck.
