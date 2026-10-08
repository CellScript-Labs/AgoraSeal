# Application threat model and review disposition

This is an implementation review record, not an independent audit or release
approval. Scope: the funded proposal/vote/treasury `.cell`, generic context
children, fixed SP1 guest/verifier, native replay/prover and CCC transaction
client. The earlier isolated payment and proof-binding components remain test
boundaries; neither is the funded application's settlement authority.

Assets are funded proposal capacity, immutable recipient/refund commitments,
DAO voting weight, latest ballot state, the canonical proof window and signing
intent. An attacker can fund proposals, create/reclaim owned ballots, submit
arbitrary transaction/proof/witness bytes, race spends, offer substituted code
or stale RPC responses and choose malformed/expensive proving input. They cannot
forge an owner signature, break CKB hash/PLONK assumptions or rewrite the
canonical chain under the assumed node consensus/finality model.

| Boundary / attack | Enforcement and evidence | Remaining trust or limit |
| --- | --- | --- |
| Arbitrary proposal identity, chain, DAO/vote/key configuration | Creation `.cell` fixes exact hashes, genesis header zero, bounds and reserve; generic child validates unique Type-ID; creation VM mutations | Code deployment availability/custody and external compiler/checker trust |
| Foreign deposits, invented weight, late deposits | Vote `.cell` checks actual live direct DAO dependency, standard Type/data, whole capacity, same owner-authorized input and creation headers | Historical eligibility depends on normal node consensus executing the fixed vote Script |
| Missing/reordered blocks or selected vote list | Guest replay authenticates contiguous headers and full transaction/witness roots, scans every input/output and checks endpoints | HeaderDeps establish canonical chain; guest does not independently validate PoW or historical Scripts |
| Duplicate, replaced, withdrawn or retracted ballots | Per-deposit replay state and checked totals; native adversarial corpus; actual vote VM admission | Public ballots, capacity weighting and deposit cutoff are governance policy choices |
| Fake pass flag, wrong proposal or creation association | Fixed release circuit/program/key, real public-byte proof binding, `.cell` input/receipt/quorum checks, exact context child | SP1/PLONK correctness, recursion root and SRS provenance remain external cryptographic assumptions |
| Redirected payment, arbitrary fee, duplicate settlement | One input/two outputs; immutable 465-byte receipt; exact recipient/refund full Lock; bounded fee; receipt cannot spend again | Receipt permanently occupies 603 CKB; no early cancel/reclaim path |
| Replay or concurrent spend | Unique state transition plus normal node unspent-input admission; retry resolves exact committed tx before treating input as spent | A dry-run can execute an already spent input; it does not establish spendability |
| Stale RPC, reorg, wallet mutation | CCC rereads actual code, committed/canonical creation and live inputs without caches; freezes policy transaction, permits signature Lock only; rechecks before dry-run/send | Trusted node availability and network confirmation choice; browser wallet connector not supplied |
| Interrupted proving or fabricated recovery | Completed artifacts replay input, enforce compiled key and verify real proof; partial/stale/missing frames reject | No mid-computation checkpoint; preserve partial attempt and prove in a new directory |
| Resource exhaustion | Input file/read/replay bounds, 1-hour owned prover bound, constrained worker settings, 70M client/node rehearsal budget and VM regression ceilings | 16-block policy is not a measured maximal populated workload; Go memory limit is soft and not whole-process memory control |

Review traced the proposal receipt/payout rules, full raw Script hashes,
unique-group/header association, replay spend ordering, signing message/witness
ordering, normal node admission and owned process cleanup. VM mutation tests
and SDK invariant tests provide executable evidence. Node and CCC evidence is
reported only by a completed rehearsal report; pending runs are not accepted.

Observed integration issues were corrected: packed-domain hashes were replaced
with explicit raw/full Script bindings; nominal witness Hash conversion was
checked against constructed recipient Script; the unused-verifier declaration
gate required a separate funding Lock source directory; the spent-cell assertion
now combines exact canonical committed spending with non-live status instead
of assuming the RPC always returns `dead`. These are development corrections,
not claims of independently discovered or disclosed vulnerabilities.

Unresolved release risks: no independent application/circuit review; no SRS
ceremony/provenance disposition (source lineage is recorded in `zk/SRS.md`);
no maximal populated-window proof/RSS
measurement or matched baseline benchmark; no activated hosted release checks;
no public network/custody/confirmation parameters or protected immutable code
deployment. Ordinary pre-funded Cells are supported; CKB secondary issuance
treasury activation is a separate network/provider integration. Keep production
admission closed until those exact evidence gaps are resolved.

Incident response: stop creating new funded proposals when code/key/proof or
chain identity is uncertain; preserve receipts, prepared tx hashes, node logs
and proof attempts. Query canonical transaction status before retrying. Reject
changed recipient/choice/anchors and create a reviewed new deployment identity
for policy/guest/verifier changes. There is no privileged upgrade or unilateral
reserve rescue in the immutable funded policy. Funds can remain locked if the
admitted prover/verifier becomes unavailable; do not conceal that liveness risk.
