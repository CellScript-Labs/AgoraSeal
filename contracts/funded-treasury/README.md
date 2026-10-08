# Funded proposal spending Lock

`treasury.cell` is the authoritative spending policy for the
[funded lifecycle](../funded-proposal/README.md). It lives separately from the
proposal package because it performs no trusted external call; inheriting the
proposal's proof/context declarations would fail the compiler's unused-verifier
gate. The normal gate compiles this standalone `.cell` with entry Lock `release`
and checks its executable/source/metadata boundary. Moving the source did not
change the machine-code hash. The proposal fixes that exact Lock hash at mint.

The Lock enforces one reserve input, exact immutable receipt capacity, one
empty untyped payment, checked quorum/pass arithmetic, exact successful payment
or first-funder refund, capacity conservation and bounded fee. The proposal
Type authenticates the real tally and unique transition in the same transaction.
Neither policy accepts a host-supplied `passed` dependency.
