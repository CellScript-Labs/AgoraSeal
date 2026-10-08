# SP1 replay guest — development boundary

The guest runs the shared bounded block replay algorithm. Voting eligibility,
treasury payment and the planned proposal/settlement policies are CellScript
application code. This Rust guest implements the off-chain proof computation;
it does not replace those on-chain policies.

SP1 SDK, guest and transitive SP1/slop crates are locked at 6.1.0. The initial
lock resolution was seeded from the design reference's SP1 lockfile at
`c70421b45b930325a4dda558de12fcad5f8b7918` and resolved for this workspace's own
packages. No reference voting implementation is used by this guest.

## Exact toolchain

The host uses the repository's Rust 1.97.1. The guest uses the official
Succinct Rust `succinct-1.96.0-64bit-v2` release. SP1 6.1's default Rust 1.93
cannot compile the pinned CKB libraries (MSRV 1.95). The portable protocol
crate declares MSRV 1.96; the CellScript compiler toolchain is unchanged.

The admitted guest is built with Linux x86-64 tools. Download and extract these
official assets into ignored `.local/` for a native Linux build:

| Asset | SHA-256 |
| --- | --- |
| [SP1 v6.1.0 cargo-prove archive](https://github.com/succinctlabs/sp1/releases/download/v6.1.0/cargo_prove_v6.1.0_linux_amd64.tar.gz) | `99f0087f581798ec510573baedcd51b97a32c6f6a1f5942612ed252e5285954b` |
| Extracted `.local/sp1/cargo-prove` | `639e1101649a4c03b6a3e9f0e93f1dc8b884039852c48ab003a504f67d5b6b1f` |
| [Succinct Rust 1.96 v2 archive](https://github.com/succinctlabs/rust/releases/download/succinct-1.96.0-64bit-v2/rust-toolchain-x86_64-unknown-linux-gnu.tar.gz) | `ff3afc3a6f22af93d162652972f254fcecf443ca4b2de18897312f19997fb94b` |
| Extracted toolchain `bin/rustc` | `9b889df8d4591fbc324a6db88fe7fc8b6830821c89a99472666c04943f215511` |
| Rust 1.97.1 `bin/cargo`, linked into the private guest toolchain | `828980723df339d62434390e9fb8ef8831036583343ae2316b7ab5646b5c1953` |

The guest toolchain does not contain Cargo. Link the repository's pinned host
Cargo into that private toolchain before registering it, avoiding rustup's
mutable stable-Cargo fallback:

```sh
ln -s "$(rustup which --toolchain 1.97.1 cargo)" .local/sp1/toolchain-1.96.0-v2/bin/cargo
rustup toolchain link succinct "$PWD/.local/sp1/toolchain-1.96.0-v2"
```

On Apple Silicon, the proof script builds the guest and CKB child verifiers in
a local Linux/amd64 Docker environment. It uses the same Linux tool hashes,
guest ELF hash, program key and CKB verifier pins. The host SDK/prover still
runs natively on macOS, outside Docker's memory allocation. Docker Desktop and
Homebrew LLVM are required for the development gate; native clippy uses real
LLVM clang/ar rather than Apple's incompatible RISC-V tools.

`./scripts/install-canonical-guest.sh` downloads and verifies the Linux guest
tools under `.local/sp1/linux`; `./scripts/canonical-build.sh guest` runs them
inside Docker. The script registers `succinct` only in the repository-local
container tool directory. It does not replace the user's normal toolchain.
The Docker base is pinned in `docker/Dockerfile.canonical`; Rust Cargo must
also match the Linux executable hash above. Apt provisioning is development
infrastructure, not a claim of hermetic release closure. Existing complete
Cargo registry/Git caches are mounted read-only, and artifact compilation uses
`--locked --offline`; missing cached dependencies fail explicitly.
The guest embeds panic source locations. The container therefore reproduces
the original `/home/arthur/RustRoverProjects/AgoraSeal` source root and
`/home/arthur/.cargo` dependency root. Using `/workspace` changes both the ELF
and program key even with the same pinned Rust executables.

Observed on 2026-10-08: a native macOS guest build produced SHA-256
`25fe68c2209c7d371403f47ed5e37f9477b438228c767e1075f9d0ce4f21c145`
and program key
`0x0069767b4a8878aecf36d142bd997b68a94a3ff4a4c1a6f6e92c49caeb3ba5ff`.
Both were rejected against the original pins. That build is not admitted.
Native macOS child ELFs also differ; a Linux rebuild reproduced the existing
CKB context adapter's exact `5739d9d4...` data hash without repinning it.
Using Linux clang 19.1.7 also reproduced the SP1 child ELF SHA-256
`c216a687a84dbda2e9dcb3b6b7b347743d7551e8b92922d92e6b0348e9714cbe`.
Clang 14 emits different ELF symbol metadata; the admitted build uses clang 19.
The guest rebuilt at the original container paths reproduces its complete
`e0f9eb4c...` ELF hash and original `0x00e6a250...` program key.

The gate verifies the cargo-prove, guest rustc and Cargo executable hashes and uses
`--locked`; it never ignores rust-version checks.
It also checks `guest.sha256` after compilation and checks the program
key against `program-vkey.txt` before execution or proof generation. Guest changes require an
intentional repin and new evidence; these pins are not on-chain admission yet.

The guest's prebuilt standard library needs GCC atomic ABI functions. The
target-specific `ckb-std = 1.1.0` dependency provides only its `dummy-atomic`
single-thread runtime implementation. It supplies no voting rules or CKB
entrypoint. Those atomic operations require wrapping arithmetic, explicitly
configured for that dependency. This compatibility choice needs to remain in
the build/review boundary; linking alone is not proof of execution correctness.

## Input and statement

Input starts with the fixed 108-byte `AGINPUT1` frame: proposal Script hash,
start hash, end hash and u32 block count. Then come exactly that many complete
Molecule blocks. The guest checks frame size before allocation, per-block and
total bounds, rejects extra frames, and runs the full commitment/range replay.
It does not accept caller-provided votes or totals.

Public output is exactly 277 bytes, `AGZKPV01`, defined by
`crates/protocol/src/statement.rs`: genesis, proposal Script hash and OutPoint,
proposal data hash, start/end hashes and heights, YES/NO totals, record count,
record digest and canonical boolean result. On-chain header admission and the
exact vote policy must establish canonicality and eligibility; this guest
does not revalidate consensus or historical Script execution.

## Commands and evidence levels

```sh
./scripts/zk.sh build
./scripts/zk.sh test-guest
./scripts/zk.sh inspect-guest
./scripts/zk.sh prove-core
./scripts/zk.sh prove-plonk
./scripts/zk.sh verify-plonk
./scripts/zk.sh verify-ckb
./scripts/zk.sh verify-circuits
```

`test-guest` executes a synthetic four-block fixture and compares all public
bytes to native replay, then sends eight malformed inputs directly to the guest,
including oversized header/frame and excessive block-count rejection.
These cover a wrong anchor, missing block, extra frame, corrupt block and
truncated input header. Host prechecks do not substitute for these guest tests.
SP1 6.1's execution API can return `Ok` with a failed guest exit code. The host
therefore requires exit 0 for success, and the rejection corpus requires exit 1
with no committed public statement. An executor/service error does not count
as a successful guest rejection test.

`prove-core` requests a real local CPU proof, verifies it, and requires rejection
of 14 public-field substitutions, a changed program key and an empty proof. It saves the proof,
public values and program key hash under ignored `target/guest-fixture/`.
This is not a compressed PLONK/Groth16 proof and is not CKB-VM verification.
No remote prover or paid service is used. Cold release builds and real proving
are substantially heavier than the native gate.

The fixture has artificial block and Script context. It is not a live-node
voting lifecycle. Proof generation, exact CKB verifier admission, maximal cost,
setup disposition and full hermetic release evidence must be tracked separately
in `docs/PRODUCTION.md`; these commands do not grant production admission.

Observed on 2026-10-07: the pinned guest ELF SHA-256 is
`e0f9eb4cae3f2b73be1d50f6f47fd507a165658dfdef26418ff576f82176cdb2`.
The four-block fixture executes 222,561 SP1 instructions and emits 277 bytes
identical to native replay. All eight direct guest rejection cases pass with
exit 1 and empty public output. This instruction count is not CKB cycles or
a proof-generation timing benchmark.

A real local SP1 core proof for this fixture was generated and verified. The
verifier rejected all 14 public-field substitutions, the changed program key
and an empty proof. The pinned program key is in `program-vkey.txt`; the guest
ELF is 293,880 bytes. The observed serialized core proof is 2,781,716 bytes with
SHA-256 `d141299e24c9af701e9f4ac5e732668a994981a6657e535b2ea01b1b1d52c78e`.
Proof bytes may vary across runs; that hash records this observation, not a
deterministic proof requirement. Public output SHA-256 is
`20477696c3bc1e1b69a051712af1f17ee6ffde27110f8d5de800a13c24464990`.
The proving run used `SP1_WORKER_NUM_CORE_WORKERS=1` and
`SP1_WORKER_CORE_BUFFER_SIZE=1` to limit concurrent CPU shard work. Proof bytes
remain ignored local artifacts and can be regenerated. No compressed SNARK,
real-node lifecycle or CKB verifier performance is implied by this core proof.

## Release PLONK circuit installation

`prove-plonk` uses the official SDK's `native-gnark` feature and a local Go/CGO
build, without a Docker daemon or remote proving service. The development host
used `go1.26.8-X:nodwarf5 linux/amd64`; upstream builds the Go library with its
`debug` tag. The lockfile fixes the Rust/Go wrapper source and its bundled Go
module checksums. Full OS/C toolchain and clean-room reproducibility are still
release requirements, not established by the lockfile alone.

The [official SP1 v6.1.0 PLONK archive](https://sp1-circuits.s3-us-east-2.amazonaws.com/v6.1.0-plonk.tar.gz)
is 2,305,113,029 bytes, with observed SHA-256
`b3e2f5b5dd5ca89675c73975d2d1cc4a0f9e7cde048ec5354dabc7b0694aa39b`.
Download it to ignored `.local/sp1/v6.1.0-plonk.tar.gz`, verify this hash, and
extract into a staging directory before renaming it to
`.local/sp1/circuits/plonk/v6.1.0`. The required files are `constraints.json`,
`plonk_circuit.bin`, `plonk_pk.bin` and `plonk_vk.bin`. Do not treat an existing
directory as evidence of a complete installation. `scripts/zk.sh prove-plonk`
checks all four files against `zk/plonk-artifacts.sha256` before building or
proving. `SP1_PLONK_CIRCUIT_PATH` may select another base directory; the version
suffix and content hashes remain mandatory.

The archive's VK matches the official standalone verifier and the pinned CKB
port. Pinning downloaded artifacts is a supply-chain identity check; it is not
an independent circuit build or a review of the trusted setup/SRS ceremony.
`SP1_CIRCUIT_MODE`, `WITHOUT_VK_VERIFICATION` and `SP1_DUMP` must be unset.
The SDK's dump mode exits successfully before proving, so it is explicitly
rejected by both the entry script and host. Development circuits and skipped
VK verification cannot produce accepted evidence here. `RUST_LOG=info` enables
the SDK's stage logs for subsequent runs; unset logging stays quiet.

The PLONK mode verifies the proof with the SDK and the standalone official
verifier, checks public-field/program-key substitutions, and exercises raw-proof
mutation/length failures. It writes raw `proof-plonk.bin`, the SDK bundle,
public bytes and program key under `target/guest-fixture/` on success.
`verify-ckb` then executes that raw proof through the actual CellScript parent
and CKB verifier, including adversarial substitutions. Its fixture still uses
synthetic history and owner Locks; full proposal/chain/treasury admission is
outside this component. Record actual successful runs separately from this
command description.

`verify-plonk` revalidates saved artifacts without generating a new proof. It
replays the original bounded block input, requires the stored public bytes to
match that replay, enforces the compiled guest-key pin, and verifies the exact
964-byte proof with the official standalone verifier. It does not trust the
saved key file as authority. The command also runs recovery mutation tests for
changed/truncated/extended proof, changed/extended public bytes, a substituted
key and a different input anchor. The executable accepts an arbitrary input
directory as `agoraseal-verify-plonk INPUT_DIRECTORY`; the script uses the
development fixture directory. This recovers verification of a completed
artifact, not a checkpoint of an unfinished recursive proving computation.

## Completed 2026-10-08 component acceptance

A real release-circuit PLONK proof was generated locally and verified by the
SDK, standalone verifier and actual CellScript/CKB parent-child transaction.
All original guest/key/verifier pins were reproduced without repinning.
The 964-byte proof, synthetic block inputs and public bytes are committed under
`tests/fixtures/sp1-plonk`; normal gates verify this fixture and run real-proof
substitution, late-invalid cycle and persisted-artifact recovery tests.
The real-proof tests are no longer ignored. Fresh proving still writes ignored
`target/guest-fixture` artifacts, selected by `verify-plonk`/`verify-ckb`.

See [the observation and resource limits](../docs/evidence/PLONK-2026-10-08.md).
The separate funded application now supplies proposal/chain/treasury admission
and signed disposable-node pass/refund evidence; its public proofs are in
`tests/fixtures/node-lifecycle` and its complete policies in
`contracts/funded-proposal` / `contracts/funded-treasury`. Those samples do not
establish maximal workloads, unfinished-computation checkpointing, SRS/security
review or complete release closure. See `docs/PRODUCTION.md`.
