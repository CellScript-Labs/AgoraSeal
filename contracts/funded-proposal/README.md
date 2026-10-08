# Funded proposal lifecycle

The authoritative policies are src/main.cell (proposal Type) and
../funded-treasury/treasury.cell
(funding Lock). The trusted children contain cryptography or generic CKB
coordinates only. The original vote/DAO codecs and fixed SP1 guest/key remain
unchanged.

Creation mints one Type-ID style unique proposal, with a 32-byte identifier
derived from the first funding input and output index. The proposal is global
output 0 and contains exactly 188 bytes. Admission fixes the actual vote ELF,
standard DAO full Script hash, actual genesis header (number 0), funding Lock
ELF, first funding input's full refund Lock, and a standard secp256k1 recipient.
Duration is 1..16 blocks; quorum is positive. Amount is at least 61 CKB and the
refund Lock's occupied capacity. Up to 128 funding inputs/outputs are allowed.
Additional inputs are explicit contributions refunded to the first input's
Lock; clients must make that choice visible.

Reserve capacity covers the future receipt, requested payment and at most
100,000 shannons in fees. Creation cannot choose an arbitrary proposal/DAO/vote
or treasury identity. Only the actual input Locks authorize funding.

Settlement consumes exactly one funded proposal and produces global output 0
as a 465-byte immutable receipt (188 proposal bytes plus 277 public bytes) and
global output 1 as the payment/refund. The Type preserves every proposal byte,
binds the real proof to the actual input's Type hash, OutPoint and data hash,
checks genesis, exact duration and quorum, and invokes the pinned SP1 child.
The generic context child then binds the actual proposal creation header and
end header. HeaderDeps are start, end, genesis at indexes 0,1,2.

The funding Lock pays the exact amount to the committed recipient when passed.
Failure refunds all available payment capacity (at least the requested amount)
to the first funding input's committed Lock. Neither path permits payment data,
payment Type, extra inputs/outputs, alternate recipient or an excessive fee.
The receipt keeps the same Type and Lock and exactly its occupied capacity.
A receipt cannot transition or be burned, so it cannot authorize a second release.

This closure costs 603 CKB of permanently occupied receipt capacity for the
current 32-byte Type and Lock arguments, plus the requested amount and fees.
It deliberately has no early cancellation or receipt reclamation. This cost
and the 16-block development window are visible tradeoffs, not production
resource or maturity claims.

## Witness and dependency order

The Type action settle has one entry for creation and settlement. Its
WitnessArgs.input_type is exactly 92 bytes:

- CSARGv1 followed by a zero byte (8 bytes).
- Statement hash (32 bytes; ignored during creation).
- Recipient's standard secp256k1 blake160 arguments (20 bytes).
- Full recipient Script hash (32 bytes), independently checked against those
  arguments and the immutable proposal.

Settlement WitnessArgs.output_type is exactly 964 proof bytes followed by
277 public bytes; the full witness must fit the SP1 child's 2,048-byte bound.
Creation can include the funding wallet's real signature in WitnessArgs.lock.
CellDeps 0 and 1 must resolve the exact SP1 child and state-context child.
Vote transactions retain their separate DAO/proposal/context ordering.

main.cell.in and Cell.toml.in contain exactly six reviewed substitutions.
agoraseal lifecycle-pin checks the state adapter, proof verifier, vote and
funding Lock ELF hashes. Normal gates use --check and never repin. Any deliberate
manifest repin requires cellc lock in this package before review.

## Evidence

Actual CKB-VM creation passes at 239,581 cycles; 24 admission/encoding/identity
mutations reject. The committed real PLONK failed-history fixture executes
the complete Type/Lock/crypto/context path at 66,518,748 cycles. Its 31
proof/context/payout/receipt mutations reject, including input creation-header
association and attempted second receipt consumption.

Those fixtures use synthetic headers and an always-success funding Lock.
The disposable-node rehearsal is a separate command with real signatures and
DAO deposits; its completed report is required before claiming node acceptance.
Artifact-checker --production describes the executable artifact boundary,
not production approval of the complete application.
