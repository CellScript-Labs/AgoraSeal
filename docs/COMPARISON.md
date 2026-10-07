# AgoraSeal (CellScript) vs ckb-vote-poc

Reviewed 2026-10-07 against
[`ckb-vote-poc@c70421b`](https://github.com/XuJiandong/ckb-vote-poc/tree/c70421b45b930325a4dda558de12fcad5f8b7918)
and CellScript `35bf983`. This is a source/design comparison, not an independent
security audit or a head-to-head proving benchmark.

**Conclusion: a significant overall advantage is not established.** The
upstream PoC already has a guest, prover path, SP1 verifier integration,
proposal/vote contracts, SDK and devnet tooling. AgoraSeal currently has actual
CellScript payment policy plus native replay and a Rust eligibility reference;
its full ZK settlement path is missing. CellScript's strongest prospective
benefit is explicit, inspectable application policy and linked build evidence,
not an automatic improvement in cryptography or proving speed.

Upstream is not production-complete either: at this revision,
[`verify_sp1_vk_hash`, `verify_vote_cell_script`, `verify_duration`,
`verify_amount`, and `verify_minimal_requirement`](https://github.com/XuJiandong/ckb-vote-poc/blob/c70421b45b930325a4dda558de12fcad5f8b7918/contracts/proposal-type-script/src/main.rs#L181)
are TODO functions returning `Ok(())`. This leaves governance admission
configuration unfinished. It is a concrete integration requirement, not proof
that CellScript is intrinsically safer or that AgoraSeal has already solved it.

| Dimension | ckb-vote-poc at the pinned revision | AgoraSeal now / intended |
| --- | --- | --- |
| Application contracts | Rust proposal and vote Scripts | Actual `.cell` treasury payment Lock; `.cell` vote/proposal/settlement still required |
| ZK path | SP1 guest, prover tooling and modified PLONK verifier present | Native replay only; exact SP1 profile/admission not implemented |
| Public statement | Proposal, Script, start/end headers, YES/NO and pass | Intended additional explicit protocol/genesis/outpoint/record commitments; not yet proved |
| Policy audit artifacts | Rust source/tests/build artifacts | `.cell` policy, structured metadata, source map, lowering record and independent artifact checker; does not prove whole-app correctness |
| Treasury | Proposal specifies receiver/amount; funds/provider integration must be assessed separately | Isolated earmarked payout component exists; full lifecycle missing |
| Maturity | More complete implementation and SDK/devnet surface; remains a PoC | Earlier-stage; no production or security superiority claim |
| Cost | No matched measurements performed here | 16,890-cycle payment fixture excludes all ZK work; not comparable to upstream proof verification |

## Policy differences are tradeoffs

The [upstream vote specification](https://github.com/XuJiandong/ckb-vote-poc/blob/c70421b45b930325a4dda558de12fcad5f8b7918/docs/vote-type-script.md)
allows multiple deposits per vote, replaces the last vote per owner, and lets
voters immediately reclaim vote Cells without cancelling the vote. Its
[proposal specification](https://github.com/XuJiandong/ckb-vote-poc/blob/c70421b45b930325a4dda558de12fcad5f8b7918/docs/proposal-type-script.md)
allows deposits created during the voting window. These are explicit choices.

AgoraSeal's current model has one deposit per ballot, replaces per deposit,
requires the deposit to predate proposal creation, and retracts the latest
ballot if its Cell is consumed during the window. It gives finer-grained
withdrawal/replacement behaviour and a fixed eligibility cutoff, but requires
more ballot Cells and prevents immediate capacity recovery for an active vote.
These are not benefits provided by the programming language. Identical rules
can be implemented in Rust; comparison benchmarks must first align semantics.

Our quorum is in shannons and includes equality (`total >= quorum`). Upstream
currently uses a CKB-denominated threshold and strict `total > threshold`.
Neither rule is inherently better. The exact boundary must be specified.

## What the proof actually authenticates

Both designs must authenticate a complete anchored window, include spend
events, bind the exact proposal/program identity and enforce payout rules.
Those properties are not novel just because the parent is CellScript.

Upstream's
[guest](https://github.com/XuJiandong/ckb-vote-poc/blob/c70421b45b930325a4dda558de12fcad5f8b7918/sp1/ckb-vote-verification/program/src/main.rs)
uses a supplied witness-root component when checking block transaction roots.
AgoraSeal's native replay recomputes full transaction witness hashes. That is
stronger binding of the supplied witness bytes, but the current tally reads
raw transaction data. It is not by itself evidence of a vote-forgery flaw in
upstream, and hashing more bytes can increase proving cost.

The upstream
[native benchmark](https://github.com/XuJiandong/ckb-vote-poc/blob/c70421b45b930325a4dda558de12fcad5f8b7918/docs/native-count-vote-benchmark.md)
measures `count_vote`; it is not SP1 proof generation time or CKB verification
cost. No speedup claim should use that number as an end-to-end baseline.

## Concrete dependencies before claiming an advantage

1. Implement all application policies in `.cell`, with real VM mutation tests.
2. Pin and execute the real voting guest/prover/verifier. CellScript's accepted
   Groth16 transition profile is not an SP1 PLONK ABI; a new exact profile (or
   a separately specified compatible proof construction) must cover statement
   bytes, program/key identity, verifier artifact, bounds and failure handling.
3. Close creation, canonical header association, unique settlement and payout,
   including actual node/signature tests. An arbitrary claimed header or
   `passed = 1` dependency must never substitute for admission.
4. Measure identical workloads: proving time and memory, proof/witness bytes,
   CKB-VM valid and late-invalid cycles, code size, fees and ballot occupancy.
5. Compare maintenance and review effort using equivalent rule changes and
   regression corpora. Extra compiler/checker/profile machinery is also a cost.

Do not wait generically for every ZK issue to close. Ordinary governance and
payment rules can be developed now. The immediate ZK blocker is exact proof
integration and application evidence. Bounded multi-Cell proof support matters
only if the chosen statement needs it; scanning many historical votes off-chain
does not automatically require a multi-Cell on-chain proof statement.
