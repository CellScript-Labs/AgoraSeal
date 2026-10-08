# CellScript public-statement binding

This component executes the actual `.cell` parent and a pinned SP1 PLONK child.
It is not a proposal lifecycle or a treasury authorization contract. In
particular, it does not yet authenticate the proposal's creation, chain
anchors, immutable governance parameters or a one-time settlement transition.
The implemented [funded proposal](../funded-proposal/README.md) handles those
application checks; this component remains the smaller proof-binding regression.

The parent requires one input and one output in its Type Script group. Output
data must contain exactly 277 bytes. The `statement_hash: Hash` entry argument
must equal the output Cell's CKB data hash. The parent forwards these exact 32
bytes to the pinned verifier at CellDep index 0 and requires successful exit.
The compiler records this as a **trusted external verifier** boundary.

GroupInput witness 0 uses canonical Molecule `WitnessArgs`:

| Field | Encoding |
| --- | --- |
| `input_type` | Exactly 40 bytes: `CSARGv1\0` followed by the 32-byte statement hash |
| `output_type` | Exactly 1,241 bytes: 964-byte SP1 PLONK proof followed by the 277-byte public statement |
| Complete witness | At most 2,048 bytes, including Molecule framing and any Lock witness |

The child checks the packet's public-statement hash against the parent's
argument before cryptographic verification. A different packet statement cannot
authenticate the output. The current compiler's `hex4-v1` interface bounds
argument payloads to 256 bytes; large proof bytes are therefore read from the
fixed witness source instead of passed as arguments.

`src/main.cell.in` and `Cell.toml.in` are templates. An intentional verifier
change requires `agoraseal-cli verifier-pin VERIFIER_ELF contracts/proof-binding`,
followed by the package's `cellc lock` and fresh evidence. Normal builds use
`--check` and reject drift without rewriting source or manifest pins.

Run `./scripts/build-cellscript.sh` to build and independently check the parent.
The regular `sp1_vm` integration tests exercise malformed ABI, wrong verifier,
wrong statement, missing/truncated packet and oversized witness rejection.
The regular gate also executes a committed real release-circuit PLONK proof
and 20 cryptographic substitutions; this test is no longer ignored.
`./scripts/zk.sh verify-ckb` selects a newly generated proof under
`target/guest-fixture` instead. See [the synthetic fixture](../../tests/fixtures/sp1-plonk/README.md)
and [the observed evidence](../../docs/evidence/PLONK-2026-10-08.md).

The real-proof test writes `target/guest-fixture/ckb-proof-binding.toml` only after
all cases pass, invalidating any prior report at the start. It records ELF,
proof and public hashes, serialized sizes, valid transaction cycles and the
minimum transaction budget that reaches explicit rejection for a changed YES
total. The latter is found by cycle-budget search with the exact release ELF;
cycle exhaustion or an infrastructure failure cannot count as rejection.
This sample is not a worst-case cost bound over every possible invalid proof.
