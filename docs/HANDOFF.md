# WIP handoff — paused on 2026-10-08

Historical stop-state record: the user explicitly resumed work on 2026-10-08.
The pause instruction below describes the earlier handoff, not a new pause.
See `PRODUCTION.md` for current acceptance evidence.
The resumed run generated and verified the real PLONK/CKB binding component;
the earlier missing-proof result below is historical. See
[the 2026-10-08 observation](evidence/PLONK-2026-10-08.md).

## Stop state and user instruction

The user explicitly requested: pause all work, push a WIP commit, and write an
English handoff. **Do not resume implementation, proving, deployments or
background work until the user explicitly resumes the project.** No AgoraSeal
prover or gate process was running when this handoff was prepared. No new proof
attempt or expensive gate was started during the pause-and-publish operation.

Repository: <https://github.com/CellScript-Labs/AgoraSeal>, branch `main`.
Working directory: `~/RustRoverProjects/AgoraSeal`. The last committed milestone
before this WIP was `0ec10d11b3e8a71dc7076d309d26dd6eb6c2bd36` (pinned SP1 guest
and verified core proof). The commit containing this document preserves the
subsequent unfinished work; it is not a production release.

The broader objective is a ZK governance/treasury application with actual
CellScript business policies, more production-mature across the complete
application than `XuJiandong/ckb-vote-poc`. Rust may implement cryptography,
proof computation, generic CKB adapters, clients and reference models, but must
not replace the `.cell` application policies. Read `AGENTS.md`, `README.md`,
`docs/PROTOCOL.md`, `docs/PRODUCTION.md`, `docs/MATURITY.md` and
`docs/COMPARISON.md` before continuing. Overall superiority is not established.
The comparison baseline was rechecked on 2026-10-07 and remained
`c70421b45b930325a4dda558de12fcad5f8b7918`.

## What this WIP contains

- `verifiers/sp1-plonk`: a fixed-key, release-circuit SP1 6.1.0 cryptographic
  verifier for CKB. It pins the CKB port at
  `XuJiandong/sp1@f25435f62a49a9d443500a21f5de8d44f83d200d`, the release PLONK VK,
  recursion VK root, zero exit code and AgoraSeal guest key. It contains no DAO
  eligibility, quorum, canonical-chain admission or payout decisions.
- `contracts/proof-binding`: an actual CellScript parent, generated ELF and
  checker path. It binds the proof's 277-byte public statement to the output
  Cell's data hash, pins the child ELF, and requires child success. This is a
  **binding component**, not a complete proposal/settlement contract.
- `crates/protocol/tests/sp1_vm.rs`: malformed ABI and parent/child rejection
  tests, plus an ignored real-proof test. The latter includes 20 substitutions
  and a search for the minimum transaction cycle budget reaching explicit
  rejection of a changed YES total. It writes a component TOML report only
  after all checks pass; it has not passed or produced that report yet.
- `zk/host`: local PLONK proving through the official SDK's `native-gnark`
  feature; standalone official-verifier checks; shared bounded input replay;
  a separate `agoraseal-verify-plonk` binary for revalidating completed artifacts
  without running a prover. The fixed guest key, rather than a saved key file,
  is authority. Its real-proof recovery test is also still ignored/pending.
- `scripts/zk.sh`: release circuit file size/hash checks; rejection of
  `SP1_CIRCUIT_MODE`, `WITHOUT_VK_VERIFICATION` and `SP1_DUMP`; PLONK proving,
  persisted-proof verification and CKB-VM verification modes. The SDK's dump
  mode can exit successfully without proving, hence the explicit rejection.
- `scripts/rebuild-verifier.sh`: a fresh-target, different-source-root,
  offline ELF comparison, integrated into the local `ci` gate. It shares the
  host toolchain/dependency cache and is not a hermetic clean-room build.
- Generic CLI `verifier-pin` generation/checking and clearer exact error-code
  assertions in component tests. `docs/MATURITY.md` records the full acceptance
  surface requested by the user.

## Critical uncompleted result: no PLONK proof

**No successful AgoraSeal PLONK proof has been generated.** There is no
`target/guest-fixture/proof-plonk.bin`, no PLONK bundle, and no successful
`ckb-proof-binding.toml` report at handoff. The existing `proof-core.bin` is a
previously verified core proof, not a CKB-verifiable SNARK.

Two local PLONK attempts were interrupted. The latest attempt logged:

| Event | Timestamp, UTC, 2026-10-07 |
| --- | --- |
| CPU prover initialization | 07:00:24 |
| PLONK proving started | 07:01:02 |
| Shrink stage completed | 07:03:16 |
| Wrap stage completed | 07:09:46 |
| Release PLONK artifacts selected | 07:09:46 |
| System memory warning, 85.23% | 07:11:04 |
| System memory warning, 97.44% | 07:11:14 |
| System memory warning, 99.64% | 07:11:24 |

The host is an Intel i9-13900H with approximately 31 GiB RAM and 15 GiB swap.
A final live observation showed approximately 30 GiB RAM and essentially all
15 GiB swap in use. These are system observations, not an accurate process
peak measurement. `target/plonk-resources.log` is empty because the timed
command did not complete. There is no recorded terminal exit status proving
an OOM kill; do not label the interruption as a diagnosed OOM or a successful
benchmark. Do not blindly repeat this configuration on the same memory budget.

The last attempt used:

```text
SP1_WORKER_NUM_CORE_WORKERS=1
SP1_WORKER_CORE_BUFFER_SIZE=1
SP1_WORKER_NUM_PREPARE_REDUCE_WORKERS=1
SP1_WORKER_PREPARE_REDUCE_BUFFER_SIZE=1
SP1_WORKER_NUM_RECURSION_EXECUTOR_WORKERS=1
SP1_WORKER_RECURSION_EXECUTOR_BUFFER_SIZE=1
SP1_WORKER_NUM_RECURSION_PROVER_WORKERS=1
SP1_WORKER_RECURSION_PROVER_BUFFER_SIZE=1
RAYON_NUM_THREADS=8
GOMAXPROCS=8
RUST_LOG=info
```

The release SDK uses an in-memory local proving pipeline. The new standalone
verification command rechecks a **completed** proof; it does not checkpoint or
resume unfinished recursion/PLONK work. Investigate a suitably provisioned local
execution environment or a validated memory-control/checkpoint approach before
another full attempt. No paid remote prover, cloud machine or external service
has been authorized or used.

## Validation actually observed

- Latest `./scripts/gate.sh dev` passed after the final executable changes in
  this WIP. The native, vote, treasury, context, pinning, ABI and malformed-proof
  component tests passed. The two tests requiring a real PLONK proof were
  explicitly ignored; their presence does not count as successful evidence.
- `./scripts/gate.sh ci` passed at an earlier intermediate state, including a
  byte-identical isolated verifier rebuild. Later additions included recovery
  verification and the late-invalid budget measurement. Do not claim that the
  final WIP received a fresh complete CI run.
- Host clippy with `native-gnark` passed before the shared replay/recovery
  refactor. Default-feature host clippy and the standalone verifier release
  build passed after that refactor.
- Circuit substitution and forbidden environment-mode checks rejected before
  proving. The CellScript parent passed strict compilation and independent
  artifact checking. Both first-byte and last-byte statement-hash substitutions
  were rejected in actual CKB-VM tests.
- The pinned CKB verifier rebuilt byte-identically in another source root with
  an empty target directory. Full OS/toolchain clean-room closure is pending.
- The previous milestone's real SP1 core proof and five direct guest rejection
  cases remain valid evidence at their documented scope. The guest executes
  222,561 instructions for the synthetic four-block fixture and emits 277 bytes.

No full node lifecycle, real wallet signatures, complete treasury settlement,
successful PLONK verification in CKB-VM, worst-case resource bound, independent
security review or production admission has been established.

## Exact boundary and integration traps

The parent uses GroupInput witness 0, canonical Molecule `WitnessArgs`:

| Field | Required encoding |
| --- | --- |
| `input_type` | Exactly `CSARGv1\0` plus the 32-byte statement hash: 40 bytes |
| `output_type` | Exactly 964 proof bytes plus 277 public bytes: 1,241 bytes |
| Complete witness | At most 2,048 bytes |
| Child arguments | Lowercase hex of the 32-byte statement hash; three empty strings |
| Child dependency | CellDep index 0, exact pinned ELF data hash |

The parent requires one group input and one group output; output data is exactly
277 bytes. It checks the witness hash against the actual output data hash. The
child checks the packet's statement hash and then the real proof. The metadata
classifies this as a trusted external verifier, not a compiler-proven circuit.

CellScript's generic `hex4-v1` ABI permits at most 256 input bytes, so the proof
cannot be passed directly through argv. `witness::bounded_entry` is the bounded
byte view used by the parent; `witness::input_type` is not that byte view.
An attempted `Hash::from_bytes` conversion of a variable `[u8; 32]` hit checker
V2419; the current witness Hash plus bounded byte view avoids that boundary.
Do not weaken the compiler/checker or move application policies into Rust to
bypass an integration issue.

The parent intentionally does not yet bind an actual proposal input, proposal
creation header, canonical end header, immutable governance parameters, unique
settlement or payout. Valid cryptographic statement binding alone cannot
authorize treasury spending.

## Pins and local artifacts

| Item | Identity |
| --- | --- |
| CellScript compiler | `35bf983db30aae80281f97e30bbce08a878d7c58` |
| Host Rust | `1.97.1` |
| Guest toolchain | Succinct Rust `1.96.0-64bit-v2`, SDK `6.1.0` |
| Go used for native Gnark build | `go1.26.8-X:nodwarf5 linux/amd64` |
| Guest ELF SHA-256 | `e0f9eb4cae3f2b73be1d50f6f47fd507a165658dfdef26418ff576f82176cdb2` |
| Guest key | `0x00e6a25086d9a6ba917d12d14e857f93f57b1cbfd828466c2e352c419f848502` |
| CKB verifier ELF SHA-256 | `c216a687a84dbda2e9dcb3b6b7b347743d7551e8b92922d92e6b0348e9714cbe` |
| CKB verifier ELF data hash | `a14f73480477fb0d2bddec5b9d7df38ad03c2ca861441d66de00862043e85b02` |
| Core proof SHA-256 | `d141299e24c9af701e9f4ac5e732668a994981a6657e535b2ea01b1b1d52c78e` |
| Public bytes SHA-256 | `20477696c3bc1e1b69a051712af1f17ee6ffde27110f8d5de800a13c24464990` |

`zk/README.md` documents toolchain installation and the official 2,305,113,029
byte PLONK archive. The downloaded archive SHA-256 is
`b3e2f5b5dd5ca89675c73975d2d1cc4a0f9e7cde048ec5354dabc7b0694aa39b`.
The four consumed circuit files are pinned in `zk/plonk-artifacts.sha256`.
The release VK matches the official standalone verifier and the CKB port;
setup/SRS provenance review and independently rebuilding the circuit remain open.

The following are ignored local files and **are not included in the WIP push**:

- `.local/sp1/cargo-prove`, guest toolchain, PLONK archive/range-download parts,
  `.local/sp1/circuits/plonk/v6.1.0/`, and the inspected verifier source checkout.
- `target/sp1/agoraseal-guest`, `target/guest-fixture/`, compiled CellScript
  artifacts/checker reports, and all Cargo target directories.
- `target/plonk-proof.log`, `target/plonk-resources.log`,
  `target/native-gate.log`, `target/ci-gate.log`, `target/recovery-build.log`,
  and `target/verifier-rebuild.log`.

All committed keys here are public verifier/program identities, not private
signing keys or secret setup material. Large binaries and proof artifacts are
regenerated; a fresh checkout must follow the documented toolchain/circuit
installation instructions.

## Resume order, only after explicit user resumption

1. Inspect the WIP diff and current repository state. Preserve unrelated work
   in the sibling CellScript checkout; this application work did not modify it.
   Check that no earlier proving process remains before launching another.
2. Resolve the observed proving memory pressure. Keep release circuits and VK
   verification enabled. Do not interpret a timeout, interruption or empty
   resource log as successful proof evidence.
3. On an adequate execution environment, run `./scripts/zk.sh prove-plonk`.
   Require complete SDK and standalone verification, all mutation checks and
   the expected raw proof/public/key artifacts. Then run:

   ```sh
   ./scripts/zk.sh verify-plonk
   ./scripts/zk.sh verify-ckb
   ```

   The latter's positive parent/child path and rejection-budget search are
   untested against a valid proof. Diagnose real failures without replacing
   the verifier with an always-success fixture or loosening admission.
4. Run the applicable dev/CI gates and record actual evidence. Re-run direct
   guest tests when validating the shared host replay refactor. Update the
   production ledger and comparison only to the scope demonstrated.
5. Implement complete CellScript proposal creation, canonical header admission,
   one-time settlement, payment and failed-proposal recovery. Connect the
   currently separate vote, proof-binding and treasury components; synthetic
   settlement dependencies must never authorize real funds.
6. Continue every remaining dimension in `docs/MATURITY.md`: wallet/SDK, real
   node/signature flows, reorg/stale-input recovery, bounded prover operations,
   matched benchmarks, reproducibility, deployment and independent review.

Hosted CI is not active. `ci/github-actions.yml` is only a template; the current
GitHub token previously lacked `workflow` scope. Do not bypass that permission
boundary or claim hosted checks ran. No public-network deployment is authorized
by this WIP handoff. `./scripts/gate.sh production` must continue to reject the
incomplete application.
