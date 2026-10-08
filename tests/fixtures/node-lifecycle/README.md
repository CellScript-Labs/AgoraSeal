# Public disposable-node lifecycle evidence

Generated 2026-10-08 using clean CKB `f7fa4436737756f97a24e254f22c13a36316ecea`
(`0.207.0`) and the exact pinned CellScript/SP1/adapter artifacts. Normal
`send_transaction` admitted the real standard-secp signed DAO deposit, funded
proposal, eligible ballot, complete PLONK settlement, failed refund and signed
payment/refund spends. `evidence.json` is the completed local harness report.
Dummy PoW, zero cellbase maturity and the always-success local faucet/code
deployment are disposable test configuration, not public-network custody.

`passed` and `failed` each contain 13 canonical Molecule block frames, actual
creation and settlement transactions, fixed guest input, 277 public bytes and
964-byte real PLONK proof/bundle. All signing used the explicitly public fixture
scalar `0x45` repeated. No user key, setup secret or confidential voter input is
included. `genesis-header.bin` was reread from this same isolated node.

The first settlement committed before a harness assertion incorrectly demanded
RPC status `dead`; the node returned `unknown` for the spent input. Recovery
verified the exact canonical committed tx/witness and reused the completed proof
after independent replay/key verification. The fixed harness then completed
both lifecycles. Recovered report rows are labeled `recovered_committed`, and
do not invent lost pre-crash dry-run measurements. The failed-refund row contains
the fresh actual node cycle estimate. Partial proofs are never checkpoints.

`ccc` contains 17 replacement-window frames, its actual creation/settlement,
new real PLONK proof and completed CCC report. `ccc/stale` preserves the old
17-block statement, valid proof and rejected settlement attempt. The unmodified
CCC standard signer used public scalar `0x46` repeated. Normal node admission
rejected the old noncanonical HeaderDep; substituting the new end HeaderDep
with the old proof explicitly failed Type error 1. A fresh exported proof then
settled and its payment was signed/spent. The report identifies recovery of
the original live proposal rather than inventing pre-recovery measurements.
Returning a known old block hash from `submit_block` did not reattach it to
the canonical chain; only reread canonical headers established recovery.

The normal gate verifies every hash, independently replays/verifies all four
node proofs and reexecutes their Type/Lock/crypto/context paths in CKB-VM. The
stale proof executes only with its old VM context; pairing it with the new end
header must reach explicit Type error 1, not merely exhaust a cycle budget. This
regression is reproducible execution evidence; it does not rerun the historical
node, prove PoW or attest a funded public deployment. `scripts/node.sh` generates
a fresh native and CCC node rehearsal when the pinned node/proving environment
is available. Public-network deployment and production release admission remain
closed in `docs/PRODUCTION.md`.
