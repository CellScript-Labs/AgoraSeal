# Funded CellScript lifecycle and CCC recovery observation

The resumed application now executes creation, eligible voting, real PLONK
settlement and single-use payment/refund through actual `.cell` policies.
Normal transaction admission on an owned disposable CKB node completed both
native lifecycles and the CCC wallet/reorg/reproof path. This is local
development evidence. Production and overall comparative maturity remain
unadmitted under [PRODUCTION.md](../PRODUCTION.md).

## Exact identities and policy

Compiler `35bf983db30aae80281f97e30bbce08a878d7c58`, Rust 1.97.1,
SP1 SDK 6.1.0, guest/key/release circuit and cryptographic child identities
remain those recorded in [the PLONK observation](PLONK-2026-10-08.md).
CKB is clean official `f7fa4436737756f97a24e254f22c13a36316ecea`, version
`0.207.0`; the locally built node SHA-256 is
`6758ead98ac8f8deee67e4a2aea32c61801480e70199b52f463bbb7661c3d6b1`.
The host is the same Apple M4 Pro / 64 GiB machine; Node is 26.5.1 and
CCC shell 1.3.15. No paid or remote prover was used.

| Artifact | ELF bytes | CKB data hash |
| --- | ---: | --- |
| SP1 PLONK | 284,560 | `a14f73480477fb0d2bddec5b9d7df38ad03c2ca861441d66de00862043e85b02` |
| Vote context | 35,552 | `5739d9d4928ee4ed0abe51456f708fb155e7d59cfe3d76f06980b1b3060255c8` |
| State context | 49,632 | `bab53b4c8d28033a9194b8b5acd08a062ae712ae03bdd1d1d8196106b8ccfe0d` |
| Vote `.cell` | 27,104 | `e5090fbbe406884866f57b51a22e41e6087fdb5195d09fc9b957e188f8a9bb74` |
| Funded proposal `.cell` | 72,344 | `1c5cfaeaee7d318c13d5882d079fb39a58ba503ebb20efc355da346c2c0568cd` |
| Funding Lock `.cell` | 15,416 | `426fc417f5402a7d5ca78363357b9de31fbd9496806ad59249d2c5f3fc3a78d9` |

The generic state child validates unique Type-ID and exact actual
creation/start/end/genesis header coordinates. Admission, immutable parameters,
proof commitment, quorum, unique receipt and capacity/payment/refund decisions
belong to CellScript. The funding Lock resides in its own source directory;
it does not inherit unused external-verifier declarations from the proposal
package. The separate-source extraction preserved its ELF byte-for-byte.

The proposal is 188 bytes, the real public statement 277 and the raw proof 964.
The immutable receipt contains 465 data bytes and permanently occupies 603 CKB.
Duration is 1..16 blocks; amount covers at least a 61 CKB standard payment and
the committed funder's refund Lock capacity. Fees are bounded at 100,000
shannons. There is no early cancellation, administrative rescue or receipt
reclamation. This implements ordinary pre-funded capacity, not consensus
secondary issuance treasury activation or confidential voting.

## Real signed node acceptance

The committed [native report](../../tests/fixtures/node-lifecycle/evidence.json)
records deployment of the six exact code Cells, real standard DAO deposit and
owner signatures, actual funded creation and an eligible YES ballot, passing
settlement/payment spend, failed settlement/refund spend, invalid signature,
proof substitution and spent-proposal replay rejection. Both ranges contain
13 complete canonical Molecule blocks. The explicitly public native fixture
scalar is `0x45` repeated; no user keys or secret setup material were imported.

The first passing settlement had already committed when a harness assertion
incorrectly required RPC status `dead`, while the node returned `unknown`.
Recovery verified exact canonical transaction/witness bytes before accepting
that spend and independently verified/reused its completed proof. Lost pre-crash
dry-run measurements are not reconstructed: those report rows explicitly say
`recovered_committed`. The fresh refund estimate is 66,503,507 cycles.

The [CCC report](../../tests/fixtures/node-lifecycle/ccc/evidence.json) records
the complete recovered CCC acceptance. The unmodified standard CCC signer used
public scalar `0x46` repeated. Its fresh path created a signed DAO deposit,
funded proposal and eligible YES ballot, then exported/proved 17 blocks.
End-block removal and permanent replacement caused old-proof rejection in the
client and normal node; substituting the new HeaderDep with the old proof
explicitly returned proposal Type error 1. Merely resubmitting a known old
block hash did not restore canonical membership; canonical rereads prevented
mistaking that response for recovery.

The corrected continuation recovered the still-live canonical proposal,
reverified its completed old proof, preserved that attempt, exported the new
canonical window and generated a new real proof. Permissionless CCC settlement
then committed, retry of the spent proposal rejected, and the standard signer
spent the payment. The final report identifies proposal recovery, rather than
inventing missing initial measurements. This validates the resumed sequence;
it is not a claim that the final one-command fresh rehearsal was rerun after
every harness correction.

| Proof attempt | Observed local proving command seconds |
| --- | ---: |
| Native passing | 833.484386958 |
| Native failed | 971.4000755 |
| CCC original window | 766.243837541 |
| CCC replacement window | 751.462969966 |

These intervals include local setup/proving/SDK checks but exclude the later
standalone verifier. Worker/buffer counts are one, Rayon threads eight, Go
processors four, `GOGC=50`, `GOMEMLIMIT=32GiB`. The Go memory setting is soft.
A separately timed synthetic funded proof took 826.74 seconds, with maximum
RSS 24,903,499,776 bytes and peak footprint 45,531,466,936 bytes as reported by
BSD time. Other builds ran concurrently. These are contended observations,
not maximal-memory bounds or matched benchmarks against another application.

## Fixed regression and build evidence

The node fixtures preserve creation/settlement transactions, complete frames,
input/public/key/proof bytes and SHA-256 manifests. `ccc/stale` preserves the
old valid proof and rejected settlement attempt. The normal gate independently
replays/verifies all six committed real proofs (component, synthetic funded,
native pass/refund, CCC old/new), then executes actual policies in CKB-VM.

| Golden full settlement | CKB-VM cycles | Transaction bytes |
| --- | ---: | ---: |
| Native passed | 66,308,424 | 2,501 |
| Native refund | 66,503,507 | 2,501 |
| CCC replacement window | 66,409,909 | 2,501 |
| CCC historical old context | 66,502,352 | 2,501 |
| Synthetic funded refund | 66,518,748 | 2,481 |

The historical CCC proof executes only with its own old VM headers. Pairing it
with the replacement end header must return explicit Type error 1 under a
140M rejection budget. Positive fixtures have a 70M regression ceiling. VM
replay does not rerun historical node consensus or prove signature admission;
the separate completed node reports establish those exercises.

Coverage includes 24 creation admission mutations, 31 complete funded
settlement mutations, the existing vote/replay/cryptographic corpus and 11 CCC
invariant tests. Eight direct guest mutations independently reject with exit 1
and no public output, including oversize header, block frame and count. Those
bound rejections do not measure a maximally populated legal voting window.
The unchanged guest ELF/key pins are retained.

All three generic/cryptographic children and both funded policies have also
been rebuilt from different fresh source/output roots and compared byte-for-byte.
The canonical Linux/amd64 toolchain and executable hashes are pinned. Shared
caches and apt provisioning remain, so this is not full hermetic OS/circuit
clean-room reproduction. The SRS source lineage and unresolved independent
disposition are documented in [zk/SRS.md](../../zk/SRS.md).

Validation commands are the repository's `./scripts/gate.sh dev`,
`./scripts/gate.sh ci`, `./scripts/zk.sh test-guest`, native node resume and CCC
rehearsal `--resume` in [RUNBOOK.md](../RUNBOOK.md). Final local results on
2026-10-08 are:

| Check | Result |
| --- | --- |
| `./scripts/gate.sh dev` | Exit 0; six committed proof checks, complete workspace/VM tests, clippy/no-std and 11 CCC tests |
| `./scripts/gate.sh ci` | Exit 0; dev coverage plus all three fresh verifier and both fresh funded policy byte comparisons/checkers |
| Direct guest `test-guest` on the unchanged pinned ELF | Exit 0; native/public agreement and all eight direct guest failures, each exit 1 with no public statement |
| Native owned-node `--resume` | Exit 0; completed pass/refund and signature spends; exact canonical transaction and completed-proof recovery |
| CCC owned-node `--resume` | Exit 0; preserved old proof, reorg/context rejection, new proof, permissionless settlement, retry rejection and signed payment spend |
| `./scripts/gate.sh production` | Expected exit 1 identifying the unresolved release evidence |

Native/CCC/prover children stopped after their completed runs. Build/prover
logs and the owned database remain under ignored `target`/`.local` for local
inspection; only public fixture evidence is committed. The hosted workflow
remains an inactive template, and no public deployment or remote publication
was performed by this continuation. The production gate must keep rejecting
missing maximal/matched cost, independent SRS/application/circuit review,
hosted release and public network/custody evidence.
