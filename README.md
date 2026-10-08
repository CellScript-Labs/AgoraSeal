# AgoraSeal

**Development resumed on 2026-10-08 at the user's request.** The English
[handoff](docs/HANDOFF.md) records the previous WIP state, validation,
PLONK memory-pressure interruption and prerequisites for resuming. Current
acceptance evidence remains in [the production ledger](docs/PRODUCTION.md).

ZK voting settlement and treasury execution on CKB, with application policies
implemented in **CellScript**. AgoraSeal explores the ZK direction independently of the Nervos
treasury team's optimistic tally design.

**Development status: not production-ready.** The executable starting point is
a CellScript vote Type Script, treasury payment Lock and bounded native replay of authenticated
CKB block transaction commitments and per-deposit voting semantics. A real
release-circuit PLONK proof passes the actual CellScript statement-binding
parent and pinned CKB-VM verifier, with cryptographic and recovery mutations
in the normal gate. See [the component evidence](docs/evidence/PLONK-2026-10-08.md).
Node-backed voting eligibility,
proposal/escrow creation and release admission are
tracked in [the production ledger](docs/PRODUCTION.md). Native replay is not a
ZK proof or a consensus validator.

## Design

Each proposal fixes its vote program, chain domain, window, quorum, recipient
and requested amount. DAO deposit outpoints identify voting rights. The latest
ballot for a deposit wins; withdrawing that deposit invalidates its ballot.
Deposits must predate the proposal, enforced by the vote Type Script. The prover
must process every transaction in the exact anchored window, including spends.

The ZK guest seals the tally. The implemented CellScript parent binds its public
bytes to the successor Cell. Full settlement must additionally bind the proposal
and CKB chain context, then authorize a single-use treasury payment. Those
lifecycle policies remain pending. Funds can come from ordinary pre-funded
Cells; integration with secondary issuance requires a separately activated
CKB treasury provider. See [the protocol](docs/PROTOCOL.md).

This is proof-backed counting over public ballots; ballot secrecy is not claimed.

## Development

The sibling `../CellScript` checkout (or `CELLSCRIPT_ROOT`) must be at
`35bf983db30aae80281f97e30bbce08a878d7c58`. The gate builds the compiler from
that checkout, compiles the actual `.cell`, verifies its ELF/metadata/source
binding, and executes it in CKB-VM.

```sh
./scripts/gate.sh dev
./scripts/gate.sh ci
cargo run --locked -p agoraseal-cli -- replay PROPOSAL_SCRIPT_HASH START_HASH END_HASH block0.bin block1.bin
```

The gate verifies the committed real-proof fixture and its CKB/recovery attacks;
it does not generate a new proof. The separate
`./scripts/zk.sh test-guest` and `./scripts/zk.sh prove-core` commands require the
pinned guest toolchain described in [zk/README.md](zk/README.md).

Block files are canonical CKB Molecule blocks in chain order. Hash arguments
are 32-byte hexadecimal values. The command checks the full transaction roots
(including witnesses), proposal inclusion, contiguous range and tally. Its JSON
output is labeled native replay and must not be accepted as settlement proof.

`./scripts/gate.sh production` intentionally fails until all production layers
have their own executable evidence. The full goal includes deployable artifacts,
wallet/prover tooling, adversarial runtime tests, reproducible costs and an
operational runbook; a green native gate cannot satisfy it.

The executable application policy is
[`contracts/treasury/treasury.cell`](contracts/treasury/treasury.cell).
Its current VM evidence covers exact payment, authenticated settlement Type
identity, quorum, fee bounds and 15 substitution failures. The settlement
fixture uses an artificial Type Script: **this is component evidence, not a
verified ZK tally or a deployable treasury**. See the
[treasury boundary](contracts/treasury/README.md) and
[comparison with ckb-vote-poc](docs/COMPARISON.md).

The actual [vote policy](contracts/vote/src/main.cell) also compiles and executes
in CKB-VM, including its pinned generic context adapter. DAO eligibility,
owner authorisation, weight and snapshot comparison are all in CellScript.
See [its ABI, bounds and evidence](contracts/vote/README.md). The Rust reference
is not substituted for this policy. Complete ZK settlement remains pending.

Rust is used for proof/crypto infrastructure, clients and test models.
[`reference/vote`](reference/README.md) is a Rust eligibility baseline only,
not the application's authoritative voting implementation.
