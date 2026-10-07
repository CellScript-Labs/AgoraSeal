# AgoraSeal voting protocol — development specification

Status: application design and native executable core. This document defines
the intended complete application, not a claim that missing layers exist.

## Sources and identity

This is CKB Script/application work. Official sources consulted:

- https://docs.nervos.org/llms.txt
- https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0022-transaction-structure/0022-transaction-structure.md
- https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0023-dao-deposit-withdraw/0023-dao-deposit-withdraw.md
- ckb-gen-types/ckb-hash 1.1.1 and merkle-cbt 0.3.2 (Cargo.lock is authoritative).
- XuJiandong/ckb-vote-poc at c70421b45b930325a4dda558de12fcad5f8b7918,
  inspected as a design reference; its code is not copied into this project.
- CellScript at 35bf983db30aae80281f97e30bbce08a878d7c58, exact Groth16
  profile and accepted #22 boundary. SP1 is a separate integration, not a
  compatible replacement for that profile's 128-byte proof.

## Eligibility and voting semantics

1. The proposal is uniquely identified by its complete Type Script hash and
   creation OutPoint. Its immutable creation data fixes the chain genesis,
   immutable vote code hash (`data2`), full DAO Type Script hash, duration, quorum, recipient full Lock
   Script hash, requested shannons and description digest.
2. Let S be the proposal's creation block and E = S + duration. Ballots count
   in (S, E], in canonical block/transaction/output order. S is included in
   replay to prove proposal creation. Deposits must be created before S.
3. A ballot refers to exactly one live Nervos DAO deposit OutPoint. Its weight
   is the deposit's whole raw capacity in shannons (including occupied
   capacity, excluding unrealized DAO compensation). This is an explicit
   governance policy, not a DAO withdrawal-value calculation.
4. The vote Type Script must validate the pinned Nervos DAO Type identity,
   deposit-phase data, direct nonduplicated dependency, creation HeaderDep,
   block-age cutoff and equal voter/deposit Lock. An input with that Lock must
   authorize creation. The proposal must be an authenticated live dependency.
   Deposit/vote Script code must use immutable data hashes. These checks are
   not supplied by a caller's JSON or the scanner's native tests.
5. The latest valid ballot replaces the previous ballot for that deposit.
   Disjoint deposits remain independent even under the same voter Lock.
   Repeated deposit references within one transaction reject. A new ballot
   cannot alter the deposit weight or owner established by on-chain checking.
6. Spending a deposit removes its voting weight. Spending the latest ballot
   retracts it; spending an obsolete ballot has no effect. New votes in the
   same transaction are processed after spends. Withdraw/redeposit cannot
   manufacture snapshot-eligible deposits. Votes may be reclaimed after E
   without changing this historical result.
7. Final totals require checked arithmetic. Passing means YES > NO and
   YES + NO >= quorum. Missing votes are not interpreted as YES or NO.
   This policy does not claim Sybil resistance beyond deposit ownership.

## Authenticated replay and proof statement

The guest must verify each serialized block's header hash, parent link, number
and complete transaction root using raw and full-transaction witness hashes.
It must scan all inputs and outputs, not a prover-selected vote list. A window
has exactly duration + 1 blocks; proposal discovery is restricted to S.
The proposal cannot be consumed within that range. Matching malformed ballots
reject rather than silently disappear from the tally.

The statement commits to the protocol version/domain, chain genesis, proposal
Type hash and creation OutPoint, proposal data digest, start/end hashes and
numbers, final YES/NO and count, ordered counted-record digest, and pass result.
The counted digest covers deposit, latest ballot OutPoint, voter, weight and
direction, sorted by deposit OutPoint, using domain-separated CKB hashes.

On-chain settlement must reconstruct/check the expected statement, authenticate
start/end HeaderDeps and bind the start header to proposal creation. Checking
only a prover-selected start/end pair is insufficient. Node-validated header
references establish canonical-chain membership; the guest does not redo PoW
or all historical Script execution. Reorgs require new proof preparation and
the chosen confirmation policy. The deployment's genesis binding must be
checked by clients and contracts; a host-provided genesis string is not enough.

Native replay assumes the anchored history passed normal CKB consensus and
the exact vote script enforced eligibility. It proves neither fact on its own.
Its result must only become chain authority through the admitted proof path.

## CellScript and treasury boundary

The actual application policies must be `.cell` source: vote eligibility,
proposal creation/transitions, settlement and treasury spending. Rust is
reserved for the proof guest, cryptographic verifier, clients and reference
models. A Rust business Script with a thin CellScript wrapper does not meet
this architecture.

Use a proposal state successor for settlement so the same proof cannot settle
twice. The CellScript parent must authenticate exact verifier/program/key and
canonical statement bytes, enforce every failure, and preserve immutable
proposal fields. Circuit/guest execution and proof generation stay outside the
compiler. An explicit new SP1 adapter/profile is required if that route is
selected; generic `spawn` alone is not typed proof admission.

A passed result permits a separate consume-once payout to the full committed
recipient Lock hash, for the exact requested amount. Treasury inputs, outputs,
change and fees must be accounted for, with no diversion or duplicate claims.
The application must handle failed proposals, cancellation policy, stale input
recovery and verifier lifecycle with explicit authorization. Ordinary funding
allows complete testing before consensus-level treasury activation.

The first implemented component is the CellScript payment Lock, described in
[`contracts/treasury/README.md`](../contracts/treasury/README.md). It supports
one earmarked reserve input and one exact recipient output, with an explicit
fee ceiling. It reads an exactly typed settlement dependency. The settlement
Type Script, unique escrow creation and full proof authentication are not yet
implemented; synthetic settlement fixtures cannot authorize real funding.

## Bounds and decisions still requiring evidence

Native development caps: 100,000 blocks; 4 MiB per block; 1 GiB total encoded
range; 100,000 distinct deposits; 1,000,000 observed matching ballot records.
The guest and host must apply allocation/read bounds before parsing input.
These are development ceilings, not measured production throughput claims.
Streaming and, if measurements demand it, recursive chunk aggregation must
preserve complete range and deposit-state continuity across boundaries.

Proof system/version, actual guest/verifier costs, setup disposition, maximum
production window and deployment custody require executable decisions. The
existing SP1 prototype is the first candidate to evaluate; no production SP1
claim is made before program/verifier compatibility and real CKB-VM costs pass.
