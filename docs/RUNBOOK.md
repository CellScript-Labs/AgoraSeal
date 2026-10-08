# Build, rehearse and recover

This runbook operates on public fixtures and an owned disposable node. It does
not authorize a funded public-network deployment. Application policy, resource,
setup/custody and release evidence are listed in [PRODUCTION.md](PRODUCTION.md).

## Fresh checkout and build

Use the exact CellScript revision
`35bf983db30aae80281f97e30bbce08a878d7c58` in a dedicated checkout; set
`CELLSCRIPT_ROOT` if the sibling `../CellScript` contains other work. Rust is
1.97.1. Initialize that compiler's required pinned dependencies as documented
there; the adapter builder example uses `ckb-sdk-rust@v5.1.0`. Never reset or
patch another active checkout to satisfy the pin. Node 26.5.1 was used for the
typed CCC scripts; `--experimental-strip-types` must be supported. npm package
versions/integrities are locked. macOS RISC-V clippy needs Homebrew LLVM; exact
executable verifiers use the pinned Linux/amd64 canonical Docker build on both
Linux and macOS; Docker must be available for these gates.

```sh
./scripts/gate.sh dev
./scripts/gate.sh ci
```

Both gates verify committed public real proofs, replay complete frames and run
the actual `.cell` policies in CKB-VM. They do not regenerate proofs or start a
node. The CI-mode verifier rebuild changes source root and uses a fresh output
directory for all three verifier/context children and both funded policies with
the same toolchain/cache; it is not hermetic OS isolation. Read
[zk/README.md](../zk/README.md) for exact guest/Go/circuit installation and hashes.
Normal builds only check template/SDK pins; a deliberate change requires
reviewed artifact repinning and new package locks/evidence, not auto-selection.

## Full local-node acceptance

Use clean official CKB revision
`f7fa4436737756f97a24e254f22c13a36316ecea` (0.207.0), independently of any active
sibling node checkout. Build its binary with Rust 1.97.1 and its locked manifest.
The harness verifies source cleanliness and the binary version/revision before
starting. Node database and local ports are owned by the run, with normal
`send_transaction` admission, Dummy PoW and test-only faucet/cellbase maturity.

```sh
cargo build --locked --manifest-path PATH_TO_PINNED_CKB/Cargo.toml --release --bin ckb --jobs 2
./scripts/node.sh PATH_TO_PINNED_CKB PATH_TO_PINNED_CKB/target/release/ckb target/NEW-node-run
```

The new run directory must not exist. Native acceptance deploys exact artifacts,
creates a real signed standard DAO deposit, votes and proves both passed and
failed proposals, settles, spends payment/refund with real signatures and
rejects replay/invalid proof/signature. It writes incremental `journal.json` and
final `evidence.json` only when both lifecycles pass. The CCC acceptance then
owns that stopped node, uses a different public fixture key, builds/signs the
complete flow and exercises canonical-anchor removal/replacement. Its output
is `target/NEW-node-run-sdk/evidence.json` only after all checks pass.

The native and CCC proving settings use one worker/buffer at each SP1 pipeline
stage, eight Rayon threads, four Go processors, `GOGC=50` and `GOMEMLIMIT=32GiB`.
The last limit is a soft Go GC target, not a total-process cap. A measured
synthetic funded proof reached a 45.53GB peak footprint on this 64GiB machine;
other processes and swap affect observations. Preserve logs and provision actual
headroom. The owned prover aborts after one hour; no paid/remote prover is used.

## Client / prover ordering

1. Verify deployment genesis and all six actual live, committed code Cells.
2. Select an ordinary empty funding Cell. Review recipient, amount, quorum,
   duration and the permanent 603 CKB receipt cost. Prepare policy transaction,
   wallet dependencies and signature placeholder, then freeze it before signing.
3. Await canonical creation. A DAO deposit must predate this creation block.
   Vote with the deposit owner's signer and a separate ordinary funding Cell;
   do not consume the referenced deposit in the casting transaction.
4. After E = creation height + duration, export every canonical block S..E
   into a new proof attempt. Recheck all exported header hashes; fail on reorg.
5. Generate/independently verify the fixed-guest proof. Build the proof-bound
   settlement and recheck live input, actual creation association and canonical
   anchors immediately before node preflight and normal submission.
6. Await chosen confirmations and preserve the transaction hash/receipt. A
   passed proposal pays exact amount; failure refunds available payment capacity
   to the first funder's full Lock. Both payment Cells use normal owner spending.

The bounded exporter/prover commands are:

```sh
cargo build --locked -p agoraseal-node --bins
target/debug/agoraseal-export RPC_URL PROPOSAL_TX_HASH OUTPUT_INDEX target/NEW-proof-attempt
target/debug/agoraseal-prove-local target/NEW-proof-attempt
zk/target/release/agoraseal-verify-plonk target/NEW-proof-attempt
```

Before proving, build the pinned guest and native-gnark host with the commands
in `zk/README.md`, and check release circuit files. The exporter writes input
last, after native authenticated replay and canonical rereads. Native tally
output cannot authorize payment. Never use a saved key/public/JSON file as
authority over the compiled guest identity or replayed canonical frames.

## Recovery and failure policy

| Failure | Required action |
| --- | --- |
| Submitted tx times out | Query exact prepared hash and canonical committed block. Do not silently select new inputs/recipient or infer failure from timeout. |
| Proposal input returns `unknown` or `dead` | Resolve known canonical spending transaction. Non-live status alone is not evidence that the intended settlement succeeded. |
| Complete proof exists | Recompute public bytes from every frame, enforce compiled guest/key pin and verify the persisted proof before reuse. |
| Partial/missing/mutated proof/frame | Preserve attempt; no computation checkpoint exists. Export/prove a fresh directory explicitly. |
| Incomplete attempt has existing prover logs | Preserve logs and choose a new directory; a new prover must not overwrite the interrupted run's evidence. |
| Start/end or creation header changed | Old proof cannot be sent. Export new canonical range and regenerate proof; select confirmation policy appropriate to the network. |
| Another settlement wins | Refresh canonical receipt/spending tx. Never retry a spent proposal with changed outputs. |
| Fee/capacity or 70M budget failure | Stop and review the fixed policy/actual execution cost. Do not bypass client/node checks or weaken cryptographic verification. |
| Code Cell unavailable or key changed | Stop new funding. Preserve evidence; changes need new reviewed immutable deployment/guest/verifier identity. |

An interrupted native harness can resume its owned database without creating
new proposal identities or reproving completed proofs:

```sh
target/debug/agoraseal-node PATH_TO_PINNED_CKB PATH_TO_PINNED_CKB/target/release/ckb target/EXISTING-node-run --resume
```

Recovery scans only the bounded disposable chain, requires live identical code
Cells, compares exact already-committed tx and witnesses, checks canonical
height/hash, and compares every saved frame/input byte before proof reuse.
It rejects an occupied saved RPC port instead of attaching to another service.
Closing the owned CCC server's stdin gracefully stops only its node child.
The CCC test can resume from an exported complete original attempt:

```sh
node --experimental-strip-types sdk/src/rehearse.ts PATH_TO_PINNED_CKB PATH_TO_PINNED_CKB/target/release/ckb target/EXISTING-node-run target/EXISTING-node-run-sdk --resume
```

It requires the exact proposal still be live and canonical, revalidates the
completed original proof, then exercises stale-anchor rejection and writes the
new canonical proof in a separate `proof-after-reorg` directory. If that new
attempt is itself partially proved, preserve it and choose a new attempt;
it is not an execution checkpoint. `submit_block` returning an already-known
hash does not establish main-chain reinclusion; always reread canonical height.
Do not truncate public chains or substitute IntegrationTest no-verification
methods for ordinary transaction/block admission.

## Deployment and upgrade boundary

Current code deployments use the disposable faucet's always-success Lock.
Production code Cell custody/protection, actual genesis and confirmation policy
must be selected and reviewed before public funding. Data2 pins make application
code immutable: no administrative upgrade can rewrite an existing funded
proposal. A policy/compiler/guest/verifier change requires a new identity and
explicit migration; prior reserves retain their old verification path. There
is no privileged cancellation, rescue or receipt reclamation. If proof service
becomes unavailable, reserve liveness remains at risk. The example spends
ordinary funding; consensus secondary-issuance integration is separate.
