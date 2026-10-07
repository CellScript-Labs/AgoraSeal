# Production acceptance ledger

Objective: independently named ZK DAO implementation in
`~/RustRoverProjects/AgoraSeal`, published under CellScript-Labs and made usable
in production. Completion means the entire application and its operational
evidence work. Repository creation and a native tally are initial steps only.

| Requirement | Current evidence | Status |
| --- | --- | --- |
| Independent repository, reproducible native workspace | Workspace, lockfile, native gates | In progress |
| Voting policy and canonical authenticated block replay | Protocol crate and adversarial tests | Native layer only |
| On-chain DAO eligibility and proposal lifecycle | Specification | Not implemented |
| Real ZK guest/prover with exact program identity | SP1 candidate under evaluation | Not implemented |
| Exact verifier executed in CKB-VM | None | Not implemented |
| CellScript source/ELF/checker settlement binding | Integration contract specified | Not implemented |
| Funded treasury lifecycle and single-use payouts | Specification | Not implemented |
| SDK, wallet/prover ordering and stale-state recovery | Native replay CLI only | Not implemented |
| End-to-end node acceptance with real signatures | None | Not implemented |
| Wrong chain/window/vote/key/proof and payout attacks | Native subset only | Incomplete |
| Maximal and late-invalid proof/runtime cost measurements | None | Not implemented |
| Reproducible guest/verifier/parent build and supply chain pins | Native Cargo.lock only | Incomplete |
| Setup/key custody and security disposition | No setup or review performed | Pending |
| Deployment tooling, rehearsal and operator runbook | None | Not implemented |
| CI and immutable release evidence | Native CI workflow | Incomplete |
| GitHub push verified against local commit | Pending initial push | Pending |

Next implementation sequence: authenticated native protocol and mutation corpus;
real vote/proposal scripts and CKB-VM eligibility tests; real SP1 guest/prover
evaluation; typed CellScript settlement integration; treasury and SDK lifecycle;
full node/adversarial/resource/reproducibility work; production admission.
Adapt the sequence to measured blockers while retaining every outcome above.

No production approval is inherited from CellScript's counter circuit. No
public-network deployment, consensus treasury activation, private ballot
confidentiality or independent security review has occurred. Test setup keys
must never be promoted as production material. Deployment to funded public
networks will require actual network/custody parameters; implementation and
local-node rehearsal can proceed independently.
