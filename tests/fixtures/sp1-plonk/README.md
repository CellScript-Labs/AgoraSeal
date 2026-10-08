# Real release-circuit PLONK component fixture

Generated locally on 2026-10-08 from
`crates/protocol/tests/replay.rs::guest_public_statement_is_canonical_and_context_bound`.
The four blocks, proposal, Locks and votes are synthetic public test data.
There are no private voter witnesses or secret setup keys. This fixture proves
the cryptographic statement-binding component, not a live-node lifecycle,
proposal admission, treasury authorization or production readiness.

| Identity | Pin |
| --- | --- |
| Guest ELF SHA-256 | `e0f9eb4cae3f2b73be1d50f6f47fd507a165658dfdef26418ff576f82176cdb2` |
| Guest key | `0x00e6a25086d9a6ba917d12d14e857f93f57b1cbfd828466c2e352c419f848502` |
| SP1 release circuit | `6.1.0`, all four artifacts checked against `zk/plonk-artifacts.sha256` |
| Compiler | `35bf983db30aae80281f97e30bbce08a878d7c58` |
| CKB verifier ELF SHA-256 | `c216a687a84dbda2e9dcb3b6b7b347743d7551e8b92922d92e6b0348e9714cbe` |
| Raw proof SHA-256 | `c0985e25755f95a1980758aca61b8ce3b09093f579661d6693d51a371277b832` |
| Public statement SHA-256 | `20477696c3bc1e1b69a051712af1f17ee6ffde27110f8d5de800a13c24464990` |

The raw proof is 964 bytes and the public statement is 277 bytes. The SDK
bundle is included for inspection; stored keys and bundles never supply
verification authority. `artifacts.sha256` covers every binary input and output.
Proof generation is randomized; this is one verified observation, not a
requirement that future proofs have identical bytes.

`scripts/gate.sh dev|ci` verifies artifact checksums, replays the complete input
with the standalone verifier, and runs the real-proof CKB-VM test and recovery
mutation test without `#[ignore]`. The CKB fixture uses actual generated
CellScript and the pinned cryptographic child. The report is regenerated in
ignored `target/guest-fixture/ckb-proof-binding.toml`.

Observed valid transaction cost: 66,012,150 CKB cycles. The 20 proof/public/length
substitutions reject. A changed YES total requires a minimum complete-transaction
budget of 129,217,167 cycles to reach explicit rejection, measured by binary
search on the unchanged ELF. Cycle exhaustion does not count as rejection.
Regression ceilings are 70,000,000 valid cycles and 135,000,000 late-invalid
cycles for this fixture; they are not maximum application or network budgets.

To verify newly generated artifacts instead, `scripts/zk.sh verify-plonk` and
`scripts/zk.sh verify-ckb` select `target/guest-fixture`. The test-only
`AGORASEAL_PROOF_FIXTURE` override changes input files, not the admitted guest,
verifier, codec or rejection requirements.
