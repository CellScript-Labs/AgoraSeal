# CCC transaction client

Exact dependency versions and integrity hashes are in `package-lock.json`.
`src/pins.ts` is generated from the compiled application/verifier artifacts;
normal gates only check it and never silently repin a deployment.

```sh
npm ci --prefix sdk --ignore-scripts --no-audit --no-fund
npm run check --prefix sdk
npm test --prefix sdk
```

Import `prepareCreation`, `prepareVote`, `prepareSettlement` and the `Deployment`
type from `src/client.ts`. Supply a CCC Client and, for creation/voting, a CCC
Signer connected to that same Client. This library does not collect wallet keys.
Supported recipient descriptors are standard secp256k1 Type Scripts with
20-byte arguments. Funding must be an ordinary empty Cell; ballot funding uses
the DAO deposit owner's Lock. Wallet support is conditional on that signer's
ability to authorize its selected funding Lock. The executable rehearsal uses
the unmodified CCC standard secp signer, not a browser connector claim.

Deployment identifies genesis, six actual live code outpoints and any wallet
dependencies. Every preparation and final send rereads genesis/code bytes and
selected live Cells without caches. Code data hashes come from fixed SDK pins;
caller-supplied descriptors cannot select another verifier or policy. Prepare
the final policy outputs/dependency/header/witness order first; wallet preparation
may add required wallet dependencies and a signature placeholder, then the
transaction is frozen. Signing may fill the Lock witness only. Changes to raw
transactions, proof/policy witnesses or witness counts reject.

`PreparedTransaction.transaction` returns a clone for review. `signAndSend`
signs the frozen creation/vote, reruns freshness checks, executes a node dry-run,
enforces a 70M cycle ceiling, submits through normal `send_transaction` and
checks canonical confirmation. `sendSettlement` does the same without a wallet
signature: the fixed funding Lock authorizes the proof-bound payout. Default
confirmation depth is one subsequent block; callers must select a suitable
network confirmation policy. Dry-run is preflight; normal admission determines
unspent-input validity. Never interpret a dry-run of an already spent Cell as
permission to spend it again.

`AgoraError.code` distinguishes wrong chain/code, stale inputs, unconfirmed or
reorganized creation, closed window, changed wallet transaction, codec/identity,
capacity/fee, proof execution and cycle-budget failures. On stale inputs, select
new live funding and prepare again. On changed start/end anchors, preserve the
old attempt, export the new canonical range and generate a new proof. A network
timeout is not proof of failure: query the exact prepared transaction hash
before retrying. No operation automatically changes recipient or voted choice.

Live test: `scripts/node.sh` runs the native signed pass/refund lifecycles then
`src/rehearse.ts` against that stopped disposable node. The latter owns the
node child, uses an explicitly public scalar `0x46` repeated, creates a real DAO
deposit and proposal, signs a vote, exports/proves the exact history, settles and
spends its payment. It also rewinds/replaces the end block, rejects the old
proof in the SDK and normal node, and regenerates a proof over the changed
canonical window before settling. Old frames/proof remain preserved in `proof`;
the new attempt is `proof-after-reorg`. A partially completed CCC rehearsal can
resume its live canonical proposal and independently verified original proof
using the script's `--resume` option. It must never run
against a public node or an existing user wallet.

Unit tests include Rust/CKB-VM golden codec agreement, wallet mutations, capacity,
direct dependency slots, spent funding, closed voting windows, canonical-anchor
changes and broadcast prevention on failed/expensive dry-run. They are client
invariant tests; the separate node rehearsal supplies consensus/signature
evidence. There is no browser UI or connected wallet provider in this package.
