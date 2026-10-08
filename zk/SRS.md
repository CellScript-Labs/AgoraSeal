# PLONK setup provenance and disposition

Disposition: pinned upstream release artifacts are accepted for development
evidence. Production acceptance is unresolved; no new ceremony, toxic-waste
handling or independent ceremony/circuit audit was performed in this project.
The downloaded PLONK proving key is public proving material, not a wallet key
or a secret ceremony contribution.

The installed SP1 6.1.0 crate records upstream source commit
`d714f028a22963e6173d57eb616363182296838c`. Source inspected:

- [gnark circuit build](https://github.com/succinctlabs/sp1/blob/d714f028a22963e6173d57eb616363182296838c/crates/recursion/gnark-ffi/go/sp1/build.go):
  its non-development branch loads the Aztec Ignition SRS, converts it to
  Lagrange form and calls PLONK setup. A directory containing `dev` uses unsafe
  test setup. AgoraSeal consumes prebuilt pinned release circuit files and never
  invokes that development setup path.
- [transcript loader](https://github.com/succinctlabs/sp1/blob/d714f028a22963e6173d57eb616363182296838c/crates/recursion/gnark-ffi/go/sp1/trusted_setup/trusted_setup.go):
  selects MAIN IGNITION, reads public contributions from the Aztec endpoint and
  checks contribution succession. The build starts at participant index 174.
  This source path is provenance information; it does not independently prove
  that a downloaded release archive was built from a reviewed full transcript.
- [Go dependency lock](https://github.com/succinctlabs/sp1/blob/d714f028a22963e6173d57eb616363182296838c/crates/recursion/gnark-ffi/go/go.mod):
  fixes ignition verifier `10693546ab33`, gnark replacement `cd7874155e26` and
  gnark-crypto `022ec58e8c19`; `go.sum` remains unchanged by local module fetching.
- [release artifact installation](https://github.com/succinctlabs/sp1/blob/d714f028a22963e6173d57eb616363182296838c/crates/prover/src/build.rs):
  forms the versioned PLONK archive URL from the official SP1 circuits endpoint.
  AgoraSeal checks the archive/file hashes recorded in `README.md` and
  `plonk-artifacts.sha256`, rather than trusting directory existence.

Local preflight checks exact regular files: constraints 111,997,113 bytes,
circuit 663,838,329 bytes, proving key 2,147,518,120 bytes and verifying key
34,368 bytes. CKB and standalone verifiers bind the exact release VK, recursion
root, zero exit code and compiled AgoraSeal guest key. Real proofs and
public/key/raw-proof mutations exercise that binding; they do not establish
ceremony honesty or an absence of circuit implementation defects.

Release disposition must identify the accepted transcript/root and rationale,
check the release key's linkage to that setup, document upstream/dependency
review and resolve independent application/circuit findings. A fresh circuit
build or new setup would change admitted artifacts and requires explicit
key/verifier/guest compatibility review and new evidence. Do not repin to local
test keys or promote a successful public fixture into ceremony certification.
