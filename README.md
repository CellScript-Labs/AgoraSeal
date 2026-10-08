# AgoraSeal

ZK governance and treasury execution on CKB, with application business policies
in **CellScript**. Development resumed from the [historical handoff](docs/HANDOFF.md)
on 2026-10-08 in a dedicated sibling checkout.

**Development implementation; production admission remains closed.** The funded
application now has actual `.cell` proposal creation, vote eligibility, real
PLONK settlement and single-use payment/refund policies. A clean pinned local
CKB node admitted real DAO deposits, signatures and both funded lifecycles.
The CCC client completed signed creation/voting, reorg rejection, fresh proving,
permissionless settlement and signed payment on that disposable node.
Committed public node proofs reexecute in the normal gate. This is ordinary
pre-funded governance; secondary-issuance treasury activation is separate.

Each proposal fixes the chain genesis, exact vote/DAO/verifier identities,
unique Type-ID, window, quorum, full recipient/refund Locks and amount. DAO
deposit outpoints identify voting rights. The latest ballot per deposit wins;
spending that deposit or latest ballot retracts its weight. Deposits must predate
proposal creation. The fixed guest authenticates every transaction/witness root
in the exact canonical window and seals the tally. The proposal Type binds the
proof to its actual input/creation header and immutable successor; the funding
Lock pays the recipient or refunds the first funder. A receipt cannot spend again.

Current bounds/tradeoffs: 1..16 block duration, standard secp256k1 recipient,
603 CKB permanently occupied receipt, no early cancellation/reclamation, public
ballots and one deposit per ballot. Maximal populated proof cost is unmeasured.
The [production ledger](docs/PRODUCTION.md) records remaining resource, matched
benchmark, setup/review, release and network/custody evidence. Overall maturity
or speed superiority over [ckb-vote-poc](docs/COMPARISON.md) is not established.

## Build and use

Use a dedicated CellScript checkout at
`35bf983db30aae80281f97e30bbce08a878d7c58` through `CELLSCRIPT_ROOT` or the sibling
`../CellScript`. The normal gate builds the compiler, compiles `.cell` artifacts,
checks source/metadata/ELF bindings, verifies real proofs and executes CKB-VM
mutation/positive tests. Rust 1.97.1 and exact dependency locks are required.

```sh
./scripts/gate.sh dev
./scripts/gate.sh ci
```

Neither gate starts a new prover or node. To run fresh signed native and CCC
acceptance with the pinned local node/proving environment:

```sh
./scripts/node.sh PATH_TO_PINNED_CKB PATH_TO_PINNED_CKB/target/release/ckb target/NEW-node-run
```

See [the operator runbook](docs/RUNBOOK.md) for setup, exact history export,
local proving, confirmation, reorg/timeout recovery and deployment boundaries.
The [CCC client](sdk/README.md) prepares/finalizes policy transactions before
wallet signing, rereads live/canonical state and performs normal node preflight
and admission. It supplies typed builders, not a browser UI or wallet connector.
Only explicitly public test keys are used in the isolated node harness.

Policies and evidence:

- [Funded proposal Type and funding Lock](contracts/funded-proposal/README.md).
- [Vote policy and direct DAO/header context ABI](contracts/vote/README.md).
- [Protocol and canonical replay](docs/PROTOCOL.md).
- [Cryptographic component evidence](docs/evidence/PLONK-2026-10-08.md).
- [Full lifecycle and CCC recovery evidence](docs/evidence/LIFECYCLE-2026-10-08.md).
- [Public real funded proof](tests/fixtures/funded-failed/README.md) and
  [signed node lifecycle fixtures](tests/fixtures/node-lifecycle/README.md).
- [Threat model and review disposition](docs/THREAT-MODEL.md).

Rust owns cryptography, guests, generic syscall adapters, clients and test models.
The older isolated treasury/proof-binding components remain regression examples;
the funded application uses its complete Type/Lock path. [reference/vote](reference/README.md)
is a Rust comparison baseline, not the authoritative application vote policy.

`./scripts/gate.sh production` rejects absent release evidence. Artifact-checker
`--production`, native tally, a green local gate or a disposable-node report
cannot independently admit a public funded deployment or certify cryptography.
