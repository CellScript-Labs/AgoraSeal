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
| Complete CellScript application | Real `.cell` proposal creation, voting, settlement, failure handling and payout; each transition exercised in CKB-VM | Proposal/settlement and full lifecycle missing |
| Governance admission | Exact chain/DAO/vote/guest/verifier identities; unique proposal and reserve creation; bounded duration/amount/quorum; no TODO admission paths | Creation and deployment admission missing |
| Proof soundness integration | Real release-circuit SNARK; exact codec/key/root/exit-code checks; wrong proof/key/context and malformed encodings rejected by actual CKB verifier | Core proof exists; SNARK/CKB boundary pending |
| History and eligibility | Full canonical anchored window; creation-header association; snapshot/deposit-spend/replacement/retraction tests; real DAO deposits and signatures | Native and isolated VM evidence only |
| Treasury safety | Single-use settlement/payment, reserve conservation, fees, failed proposal recovery, no arbitrary `passed` dependencies, replay and concurrent-spend rejection | Isolated payout component only |
| Wallet and SDK | Typed transaction builders, supported wallet signing, dependency/witness ordering, fee/capacity estimation, local preflight and clear error mapping | Native replay/prover CLI only |
| Operational recovery | Reorg, stale inputs, timeout, interrupted proving, retry idempotency and artifact recovery tests | Not implemented |
| Prover operations | Bounded input/resource controls, reproducible guest identity, local proving, resumable artifacts and explicit remote-prover trust/cost if ever used | Bounded guest and local core proof exist; recovery/SNARK path incomplete |
| Adversarial coverage | Native, guest, CKB-VM and node corpus; fuzz/property coverage where useful; real cryptographic failures and late-invalid costs | Component corpus only |
| Performance and cost | Matched semantics/data/hardware: proving time and peak memory, valid/late-invalid VM cycles, proof/tx/ELF sizes, fees, ballot occupancy; regression budgets | No matched benchmark; component observations only |
| Build and supply chain | Exact compiler, guest, adapter, dependencies, Go/circuit artifacts and keys; clean-room rebuild comparison; artifact provenance | Partial pins; full clean-room closure missing |
| CI and releases | Hosted checks run automatically, required branch/release checks, immutable versioned artifacts and independently replayable evidence | Local gates and inactive workflow template |
| Deployment | Local-node deployment rehearsal, network identity checks, version/upgrade policy, rollback or migration procedure, deterministic deployment manifests | Not implemented |
| Security readiness | Application-specific threat model, security review findings closed or explicitly resolved, setup/SRS disposition and incident response procedure | No complete application review or setup disposition |
| Documentation and operator experience | Fresh-machine quickstart, lifecycle examples, witness/codec references, troubleshooting, runbooks, recovery exercises and honest limitations | Development component docs only |

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

Implementation sequence remains adaptable: finish release SNARK and exact CKB
verification, close CellScript creation/settlement/payout, build complete SDK
and local-node flows, then exercise recovery, comparative benchmarks and release
operations. Keep `docs/PRODUCTION.md` synchronized with actual evidence.
