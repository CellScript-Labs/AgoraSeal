# Production acceptance ledger

Work resumed at the user's request on 2026-10-08 in a fresh sibling checkout.
The previous stop state remains in [HANDOFF.md](HANDOFF.md). The resumed run
generated a real release-circuit PLONK proof and verified it through the actual
CellScript parent and CKB-VM child. See [the component evidence](evidence/PLONK-2026-10-08.md)
and [the committed synthetic fixture](../tests/fixtures/sp1-plonk/README.md).
Complete application admission remains unavailable.

Objective: independently named ZK DAO implementation in
`~/RustRoverProjects/AgoraSeal`, published under CellScript-Labs and made usable
in production. Completion means the entire application and its operational
evidence work. Repository creation and a native tally are initial steps only.
The additional acceptance target is production maturity beyond the reference
PoC across all dimensions in [MATURITY.md](MATURITY.md), with matched cost
measurements and explicit remaining tradeoffs.

| Requirement | Current evidence | Status |
| --- | --- | --- |
| Independent repository, reproducible native workspace | Public AgoraSeal repository, lockfiles, native gates | Repository published |
| Voting policy and canonical authenticated block replay | Protocol crate and adversarial tests | Native layer only |
| CellScript DAO eligibility and proposal lifecycle | Actual `.cell` vote policy, exact context adapter, parent-child VM tests (normal/bound/reclaim + 19 negative cases) | Vote component implemented; proposal creation/lifecycle pending |
| Real ZK guest/prover with exact program identity | Original guest/key reproduced; real release-circuit PLONK proof generated and verified by SDK and standalone verifier; public/key/raw-proof mutations reject | Cryptographic component implemented; complete application admission pending |
| Exact verifier executed in CKB-VM | Real 964-byte PLONK proof passes actual CellScript parent and pinned child; 20 substitutions reject; valid/late-invalid costs measured | Proof-binding component verified; proposal/chain admission pending |
| CellScript source/ELF/checker settlement binding | Actual proof-binding `.cell` compiles and passes artifact checking | Binding component only; proposal/chain/settlement admission pending |
| Funded treasury lifecycle and single-use payouts | Actual `.cell` payment Lock: ELF/checker and CKB-VM positive + 15 negative cases | Payment component only; creation and authenticated ZK settlement pending |
| SDK, wallet/prover ordering and stale-state recovery | Standalone real-proof revalidation and seven stale/partial artifact cases pass; input is replayed, saved keys are not authority | Completed-artifact recovery implemented; wallet/SDK and chain recovery pending |
| End-to-end node acceptance with real signatures | None | Not implemented |
| Wrong chain/window/vote/key/proof and payout attacks | Native/CellScript/guest corpus plus real PLONK public/key/proof substitutions and recovery mutations | Cryptographic component coverage present; chain/payout lifecycle incomplete |
| Maximal and late-invalid proof/runtime cost measurements | Four-block PLONK run completed; 66,012,150 valid CKB cycles and 129,217,167 changed-YES minimum rejection budget; explicit fixture regression ceilings | Component observation only; maximal and matched application benchmarks pending |
| Reproducible guest/verifier/parent build and supply chain pins | Original guest/key/context/SP1 ELF reproduced on Apple Silicon through pinned Linux tools; fresh-target verifier rebuild matches; committed proof fixture | Full hermetic release closure incomplete |
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
