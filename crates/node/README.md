# Local node tools

`agoraseal-node` owns a disposable CKB instance and exercises actual `.cell`
policies with public fixture signatures and normal transaction admission.
`--resume` reopens only its stopped database, recovers exact canonical committed
transactions and independently verifies completed proofs. No application rule
is implemented here as an on-chain Rust Script.

`agoraseal-node-serve` controls that owned database's lifetime through stdin for
the CCC test. `agoraseal-export` writes a new bounded complete canonical-history
attempt; `agoraseal-prove-local` runs the shared one-hour local proving/verification
path. See `docs/RUNBOOK.md` for supported commands, pins and recovery limits.

Generic devnet utilities are adapted from CellScript at exact revision
35bf983db30aae80281f97e30bbce08a878d7c58, specifically
`crates/cellscript-tools/src/ckb_devnet.rs`. Its original MIT notice is retained
in `LICENSE-CELLSCRIPT`. Signing follows the standard CKB SDK v5.1.0 message
layout and uses only explicitly public fixture scalar 0x45 repeated; CCC's
independent fixture uses 0x46 repeated. These are not user wallet keys.
