# CellScript Vote Type Script

`src/main.cell` contains the application's voting policy and compiles into the
actual vote Type Script. `main.cell.in` and `Cell.toml.in` are its reviewable
templates; the only substitution is the exact generic context adapter hash.
The build gate checks that the committed generated files match both templates
and the locally rebuilt adapter. It never repins the identity automatically.

The `.cell` policy checks proposal/DAO identity, fixed encodings and magic,
deposit phase, positive and exact raw-capacity weight, ballot direction,
proposal-committed vote code, matching owner Lock and an authorising input,
no simultaneous spending of the backing deposit, and
`deposit_height < proposal_height`. It allows one ballot output in the Type
group, at most 128 transaction inputs and 128 outputs. A voting transaction
must put the deposit and proposal at direct CellDeps 0 and 1; the context
adapter resolves at CellDep 2. Later dependencies may include wallet DepGroups.

Creation heights are witness hints, not authority. CellScript constructs a
52-byte request from the ballot's actual OutPoint bytes and those hints, pins
the adapter ELF data hash, invokes SPAWN/WAIT, and requires child success.
The adapter checks only generic CKB coordinates (see its own README). It does
not contain the DAO identity, owner, vote weight, direction, eligibility cutoff,
proposal policy or a ZK verifier. All such voting decisions remain in `.cell`.
The manifest labels this boundary `trusted-external`; the compiler does not
prove the adapter's implementation. VM tests execute the real child.

The action's witness ABI is `CSARGv1\0` followed by deposit/proposal heights as
two little-endian u64 values, inside `WitnessArgs.input_type`. Use `cellc
entry-witness` to encode it. When there are no group outputs, reclamation
requires only a well-formed entry witness (zero heights are sufficient); no
proposal or adapter dependency is needed. The input Lock still authorises the
spend. The replay layer decides whether that spend retracts a latest ballot.

Current evidence includes normal casting, owner at input 127, reclamation
without a live proposal, and 19 negative cases: weight, owner, age, DAO type,
withdrawal phase, repeated group output, missing header/proposal, vote code,
adapter substitution, both falsified heights, deposit reference substitution,
deposit spending, DepGroup remapping, duplicate deps, malformed direction,
trailing data and a 129th input. Normal casting costs approximately 243,000
cycles including the child; the 128-input fixture costs approximately 940,000.
Fixtures use artificial proposal/DAO types and always-success owner Locks.
They establish component execution, not real DAO/signature/node/ZK acceptance.

Missing production dependencies include proposal creation that admits only the
intended immutable vote/DAO identities, the real ZK guest/verifier/settlement,
full lifecycle and SDK, maximal transaction/dependency/late-failure costs,
reproducible cross-root artifacts and independent review. A passed component
checker is not an application production certificate.
