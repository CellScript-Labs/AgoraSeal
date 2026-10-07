#![no_std]
//! Deterministic voting semantics over a complete, anchored CKB block range.
//! Native success is not proof verification or historical consensus validation.
extern crate alloc;

mod codec;
mod replay;
mod statement;

pub use codec::{Ballot, Choice, Hash, OutPoint, Proposal};
pub use replay::{Limits, Replay, Tally};
pub use statement::{GuestInput, PublicStatement};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Encoding,
    Policy,
    Bound,
    Overflow,
    EmptyRange,
    BlockOrder,
    StartAnchor,
    EndAnchor,
    TransactionRoot,
    ProposalMissing,
    DuplicateProposal,
    ProposalSpent,
    DuplicateDeposit,
    DepositIdentity,
    Poisoned,
}

pub fn hash(data: &[u8]) -> Hash {
    ckb_hash::blake2b_256(data)
}
