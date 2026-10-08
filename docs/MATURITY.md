# Production maturity acceptance against ckb-vote-poc

User acceptance target: AgoraSeal must be more production-mature than the
reference PoC across the complete application, using real CellScript business
policies. This extends the implementation objective; it is not a claim that
the rows below are already satisfied. The comparison baseline is
`XuJiandong/ckb-vote-poc@c70421b45b930325a4dda558de12fcad5f8b7918`.
`docs/COMPARISON.md` records observed source-level differences. Recheck upstream
before a final release comparison and explicitly identify the compared revision.

| Dimension | Required acceptance evidence | Current gap |
| --- | --- | --- |
| Complete CellScript application | Real `.cell` proposal creation, voting, settlement, failure handling and payout; each transition exercised in CKB-VM | Implemented; actual node pass/refund golden settlements in the normal gate |
| Governance admission | Exact chain/DAO/vote/guest/verifier identities; unique proposal and reserve creation; bounded duration/amount/quorum; no TODO admission paths | Implemented creation/settlement; public deployment custody missing |
| Proof soundness integration | Real release-circuit SNARK; exact codec/key/root/exit-code checks; wrong proof/key/context and malformed encodings rejected by actual CKB verifier | Complete fixed-guest funded path passes; independent review/SRS disposition missing |
| History and eligibility | Full canonical anchored window; creation-header association; snapshot/deposit-spend/replacement/retraction tests; real DAO deposits and signatures | Signed DAO/vote node paths verified; spend/replacement/retraction corpus also covers native/VM layers, not every node scenario |
| Treasury safety | Single-use settlement/payment, reserve conservation, fees, failed proposal recovery, no arbitrary `passed` dependencies, replay and concurrent-spend rejection | Complete signed pass/refund node lifecycles; permanent 603 CKB receipt and no early cancellation/reclaim |
| Wallet and SDK | Typed transaction builders, supported wallet signing, dependency/witness ordering, fee/capacity estimation, local preflight and clear error mapping | CCC typed client / 11 tests; signed DAO/creation/vote, permissionless settlement and payment node path complete; no browser UI/connector |
| Operational recovery | Reorg, stale inputs, timeout, interrupted proving, retry idempotency and artifact recovery tests | Native exact committed-tx recovery; CCC rejects removed/replaced end header, preserves old proof, reproves and settles; no interrupted-computation checkpoint |
| Prover operations | Bounded input/resource controls, reproducible guest identity, local proving, resumable artifacts and explicit remote-prover trust/cost if ever used | Real local PLONK and completed-artifact recovery pass; unfinished computations have no checkpoint |
| Adversarial coverage | Native, guest, CKB-VM and node corpus; fuzz/property coverage where useful; real cryptographic failures and late-invalid costs | Creation, complete settlement, client and reorg corpus implemented; maximal populated resource coverage incomplete |
| Performance and cost | Matched semantics/data/hardware: proving time and peak memory, valid/late-invalid VM cycles, proof/tx/ELF sizes, fees, ballot occupancy; regression budgets | Component and full node lifecycle observations; maximal/matched benchmarks missing |
| Build and supply chain | Exact compiler, guest, adapter, dependencies, Go/circuit artifacts and keys; clean-room rebuild comparison; artifact provenance | Exact pins and fresh-root policy/verifier byte comparisons; full hermetic OS/circuit closure missing |
| CI and releases | Hosted checks run automatically, required branch/release checks, immutable versioned artifacts and independently replayable evidence | Local gates and inactive workflow template |
| Deployment | Local-node deployment rehearsal, network identity checks, version/upgrade policy, rollback or migration procedure, deterministic deployment manifests | Disposable-node deployment and immutable-identity runbook implemented; actual public network/custody missing |
| Security readiness | Application-specific threat model, security review findings closed or explicitly resolved, setup/SRS disposition and incident response procedure | Implementation threat/review/incident record and executable corpus; independent review/SRS disposition missing |
| Documentation and operator experience | Fresh-machine quickstart, lifecycle examples, witness/codec references, troubleshooting, runbooks, recovery exercises and honest limitations | Full local runbook/client/policy/ABI/recovery examples; resource/public release qualifications remain visible |

Each row needs authoritative artifacts and commands, not a checked box in this
document. Production admission must run the relevant checks and reject absent,
stale, substituted or merely simulated evidence. A component's artifact-checker
`--production` result does not certify the whole application.

Maturity and raw speed are separate measurements. No unconditional speed,
cost or security-superiority claim follows from the choice of CellScript.
Use equivalent policies for cost comparisons, publish tradeoffs (including
per-deposit ballot occupancy), and improve measured weaknesses. A real
performance regression must remain visible even when operational completeness
improves. Independent review must be identified as such only if it occurred.

Remaining acceptance follows measured gaps: measure maximal and matched
populated workloads, resolve independent review/SRS
and protected network/custody parameters, then activate required hosted checks
and publish immutable release evidence. Keep `docs/PRODUCTION.md` synchronized
with completed artifacts and exact validation commands.
