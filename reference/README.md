# Reference code, not application contracts

`vote/` is the initial Rust eligibility reference. It remains a useful
differential-test baseline, including real CKB-VM syscall behaviour. It is
deliberately outside `contracts/`: the production voting policy must be
implemented in CellScript. The native replay currently relies on the
eligibility contract described in the protocol; that contract is not yet
satisfied by a deployed CellScript Vote Type Script.

The reference checks exact proposal/DAO identity, deposit phase and creation
height, owner authorisation, weight and duplicate deposits. It accepts only
direct CellDeps before the first DepGroup, preserving the raw/resolved index
correspondence. Its VM fixtures use artificial DAO/proposal types and
always-success owner Locks, so they do not establish signatures, actual DAO
validation, consensus or ZK admission.
