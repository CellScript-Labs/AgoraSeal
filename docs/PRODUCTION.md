# Production acceptance ledger

Resumed 2026-10-08 from the public handoff in an independent sibling checkout,
branch `arthur/resume-handoff`. The funded `.cell` application, real PLONK proofs
and signed disposable-node pass/refund lifecycles now work. This is a development
implementation with local evidence, not a production release. The broader
maturity target against ckb-vote-poc remains subject to every row below.

| Requirement | Current authoritative evidence | Status / remaining gap |
| --- | --- | --- |
| Repository and locks | Independent AgoraSeal checkout, exact Rust/CellScript/Cargo/npm pins | Resumed work is local; no new remote publication |
| Canonical replay and voting semantics | Protocol guest authenticates complete transaction/witness roots, replacement/retraction/spend ordering and checked tally; adversarial corpus | Implemented; native tests alone are non-consensus |
| Actual `.cell` business rules | `contracts/vote`, `contracts/funded-proposal`, `contracts/funded-treasury`; source/metadata/lowering/ELF checking | Implemented for stated bounded policy |
| Governance admission | Exact vote/DAO/funding Lock/genesis identities, unique Type-ID, amount/quorum/window/reserve and recipient/refund commitments; 24 creation VM mutations | Implemented; actual deployment custody pending |
| Real cryptographic binding | Fixed SP1 6.1.0 guest/key/release circuit and CKB port; real public/key/proof mutations reject; normal gates independently verify six committed proofs | Implemented; independent circuit/SRS disposition pending |
| Complete settlement/payment/refund | Actual input/outpoint/data/creation/header association; immutable receipt and exact payment/refund; 31 funded VM mutations and node goldens | Implemented; 603 CKB permanently occupied receipt, no early cancellation/reclaim |
| Signed node admission | Clean CKB f7fa443 0.207.0; normal send_transaction for DAO deposit, vote, passed payment, failed refund and both signature spends; replay/invalid proof/signature reject | Disposable-node acceptance completed; no public network deployment |
| CCC wallet/client | Typed builders and fixed artifact pins; ordinary funding, capacity/fee, witness/dependency freezing, no-cache state/chain checks, preflight and confirmation; 11 client tests | Signed DAO/creation/vote, permissionless settlement and signed payment node path completed; no browser connector/UI |
| Recovery | Exact canonical committed tx/witness recovery; independent completed-proof reuse; stale/partial mutations; CCC removes/replaces E, rejects old proof in SDK/node/context, reproves and settles | Native and CCC recovery exercised; no mid-computation checkpoint or automatic public-network finality policy |
| Resource costs | Node golden passed 66,308,424 / failed 66,503,507 / CCC new-window 66,409,909 cycles, each 2,501 bytes; eight direct guest rejection cases include frame/count/allocation bounds; observed proof time/RSS | Small workload observations only; maximal populated-window and matched comparative benchmarks pending |
| Reproducibility | Original Linux guest/key/SP1/context pins reproduced on macOS; all three children and both funded policies compare byte-identical from fresh source/output roots; exact canonical tool hashes | Validated; no hermetic OS/circuit clean-room claim |
| Setup/key custody | Public fixture scalars only; no user key or secret setup imported; release file hashes and upstream SRS source lineage in `zk/SRS.md` | Independent SRS/key linkage and production custody disposition pending |
| Security readiness | Application threat model, source review and executable attack corpus in `THREAT-MODEL.md` | Implementation review only; independent application/circuit review pending |
| Operator experience | `RUNBOOK.md`, CCC docs, canonical export/prove/revalidate tools, owned-node lifetime and native resume | Implemented local workflow; actual network/finality/custody parameters missing |
| CI/release | Local dev/ci gates and pinned workflow template | Hosted workflow inactive; no immutable release or required branch checks |
| Maturity comparison | Upstream HEAD rechecked 2026-10-08, still c70421b; concrete admission closure and visible policy costs | No overall production, speed or security superiority claim |

Public fixed evidence is in `tests/fixtures/funded-failed` and
`tests/fixtures/node-lifecycle`; reports distinguish original normal node
admission from later VM replay and recovered committed rows. The first resumed
component milestone is documented in `evidence/PLONK-2026-10-08.md`. The node
harness recovered from an overly strict spent-status assertion without changing
chain/proof authority or reproving its completed first statement.
The complete application and CCC reorg/reproof observation is recorded in
[the lifecycle evidence](evidence/LIFECYCLE-2026-10-08.md). The old CCC proof
remains cryptographically valid for its historical frames; it cannot authorize
settlement with the replacement canonical end header.

## Admission remains closed

`./scripts/gate.sh production` rejects unresolved maximal-resource/matched-cost,
SRS/review, hosted release and network/custody evidence. A green local gate,
artifact-checker `--production` flag or disposable node cannot substitute for
those requirements. Public ballots are not confidential; ordinary pre-funding
is not consensus secondary issuance treasury activation. No production approval
is inherited from a compiler circuit/profile or the upstream PoC.

Release preparation still requires a measured populated workload and an agreed
operating envelope, matching reference semantics before cost comparison, SRS
provenance/disposition, independent review, activated required hosted checks,
versioned immutable artifacts, protected code Cell custody and actual network
confirmation parameters. None of those statuses may be inferred from repository
creation or a successful synthetic proof. Public funding requires concrete
network/custody instructions; implementation and local rehearsal were carried
out without importing user secrets or modifying active sibling repositories.
