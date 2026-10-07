# Production acceptance ledger

Objective: independently named ZK DAO implementation in
`~/RustRoverProjects/AgoraSeal`, published under CellScript-Labs and made usable
in production. Completion means the entire application and its operational
evidence work. Repository creation and a native tally are initial steps only.

| Requirement | Current evidence | Status |
| --- | --- | --- |
| Independent repository, reproducible native workspace | Public AgoraSeal repository, lockfiles, native gates | Repository published |
| Voting policy and canonical authenticated block replay | Protocol crate and adversarial tests | Native layer only |
| CellScript DAO eligibility and proposal lifecycle | Actual `.cell` vote policy, exact context adapter, parent-child VM tests (normal/bound/reclaim + 19 negative cases) | Vote component implemented; proposal creation/lifecycle pending |
| Real ZK guest/prover with exact program identity | Pinned SP1 6.1.0 guest: real local core proof generated/verified, 14 public-field substitutions + wrong key + empty proof rejected; five direct guest rejection cases | Core proof component implemented; SNARK compression and on-chain admission pending |
| Exact verifier executed in CKB-VM | None | Not implemented |
| CellScript source/ELF/checker settlement binding | Integration contract specified | Not implemented |
| Funded treasury lifecycle and single-use payouts | Actual `.cell` payment Lock: ELF/checker and CKB-VM positive + 15 negative cases | Payment component only; creation and authenticated ZK settlement pending |
| SDK, wallet/prover ordering and stale-state recovery | Native replay CLI only | Not implemented |
| End-to-end node acceptance with real signatures | None | Not implemented |
| Wrong chain/window/vote/key/proof and payout attacks | Native/CellScript component corpus, direct guest failures and real core-proof context/key substitutions | Chain-level and compressed-proof coverage incomplete |
| Maximal and late-invalid proof/runtime cost measurements | None | Not implemented |
| Reproducible guest/verifier/parent build and supply chain pins | Native and SP1 Cargo locks; exact compiler/context ELF and guest rustc/Cargo/tool hashes | Full verifier and hermetic release closure incomplete |
| Setup/key custody and security disposition | No setup or review performed | Pending |
| Deployment tooling, rehearsal and operator runbook | None | Not implemented |
| CI and immutable release evidence | Local gate and `ci/github-actions.yml` template; token lacks workflow scope | Hosted CI not active |
| GitHub push verified against local commit | CellScript vote/context milestone `5b6106a7b86d51b736a7df772ded548fba4c9a4e` verified | Component publication complete |

Next implementation sequence: authenticated native protocol and mutation corpus;
CellScript proposal creation/lifecycle and node-backed eligibility tests; real SP1 guest/prover
evaluation; typed CellScript settlement integration; treasury and SDK lifecycle;
full node/adversarial/resource/reproducibility work; production admission.
Adapt the sequence to measured blockers while retaining every outcome above.

No production approval is inherited from CellScript's counter circuit. No
public-network deployment, consensus treasury activation, private ballot
confidentiality or independent security review has occurred. Test setup keys
must never be promoted as production material. Deployment to funded public
networks will require actual network/custody parameters; implementation and
local-node rehearsal can proceed independently.

Application business rules must live in `.cell`; moving them into Rust does
not satisfy this ledger. The Rust eligibility reference is outside the
application contract directory. The treasury checker passing `--production`
means its artifact has no unsupported execution paths; it does not certify
this application's missing proof/creation/network layers.
