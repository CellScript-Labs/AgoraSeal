# AgoraSeal engineering contract

Read README.md, docs/PROTOCOL.md and docs/PRODUCTION.md before changes.
The goal is a production-usable ZK governance and treasury application on CKB,
with actual CellScript source enforcing vote eligibility, proposal lifecycle,
settlement and payout. Rust may implement proof guests, cryptographic verifiers,
clients and test/reference models; it must not replace the application's `.cell`
business policies. `reference/vote` is an explicit non-production comparison
implementation, not the application's Vote Type Script. A native tally demonstration
does not complete that goal. Keep the production evidence ledger accurate.

Use official CKB sources for consensus, serialization, syscalls and deployment.
Pin proof systems, guest programs, verifier binaries, keys, codecs and compiler
identities. Never conflate CellScript ProofPlan with a cryptographic proof.
Never accept host-provided votes, eligibility, roots or results as authority.
Authenticated chain data and on-chain rules must establish every such fact.

Run `./scripts/gate.sh dev` before committing and `./scripts/gate.sh ci` before
claiming CI readiness. `./scripts/gate.sh production` must reject missing
production evidence. Do not weaken the gate to make a release pass.
Keep pure Rust protocol code usable without std. No interpreter tooling.
Do not import secret setup material, private keys or real voter witnesses.
Do not mutate sibling repositories without a concrete integration requirement.

Tests must include real proof failures, context substitution, chain omissions,
double voting, withdrawal/redeposit, payout replay and resource exhaustion as
the relevant layer is implemented. Model tests are not CKB-VM/node evidence.
