# Real funded settlement fixture

Public synthetic fixture generated on 2026-10-08. The actual proposal .cell
creation policy and unique-state/header adapter executed before these frames
were proved with the original pinned guest and release PLONK circuit. The full
failed settlement then executes the actual proposal Type, funding Lock, exact
SP1 verifier and header adapter in CKB-VM. It passes at 66,518,748 cycles; 31
proof/context/payout/state substitutions reject. The transaction is 2,481 bytes.

The three synthetic blocks and always-success funding Lock do not establish
node consensus, wallet signatures or real DAO historical admission. Empty
voting history fails quorum and refunds the funding Lock; no recipient signature
or confidential ballot is included. All files are public test material, not
setup secrets or real voter witnesses.

Reproduce frames after compiling the exact pinned application artifacts:

```sh
cargo run --locked -p agoraseal-protocol --example funded_fixture -- failed target/NEW-fixture
```

The destination must be new. Existing proofs are never overwritten by fixture
export. `scripts/gate.sh` verifies hashes, replays frames, verifies the completed
proof independently, and runs real settlement tests without ignoring them.
