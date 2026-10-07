#![no_std]
#![no_main]

use agoraseal_protocol::{Ballot, Proposal};
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{packed, prelude::*},
    error::SysError,
    high_level::{load_cell_capacity, load_cell_lock_hash, load_cell_type_hash},
    syscalls,
};

ckb_std::entry!(entry);
ckb_std::default_alloc!(4096, 262144, 64);

const MAX_TX_BYTES: usize = 65_536;
const MAX_DEPS: usize = 128;
const MAX_INPUTS: usize = 128;
const MAX_OUTPUTS: usize = 128;

#[derive(Debug)]
#[repr(i8)]
enum Error {
    Syscall = 10,
    Encoding = 11,
    Bound = 12,
    Proposal = 13,
    Deposit = 14,
    Owner = 15,
    Weight = 16,
    Age = 17,
    Duplicate = 18,
}
impl From<SysError> for Error {
    fn from(_: SysError) -> Self {
        Self::Syscall
    }
}

fn exact_data<const N: usize>(index: usize, source: Source) -> Result<[u8; N], Error> {
    let mut bytes = [0; N];
    let len =
        syscalls::load_cell_data(&mut bytes, 0, index, source).map_err(|_| Error::Encoding)?;
    if len != N {
        return Err(Error::Encoding);
    }
    Ok(bytes)
}

fn creation_number(index: usize) -> Result<u64, Error> {
    // Borrow the fixed header buffer instead of constructing an owned Molecule
    // Header/Bytes. Owned Bytes would introduce unsupported RV64 atomic opcodes.
    let mut bytes = [0; packed::Header::TOTAL_SIZE];
    let len = syscalls::load_header(&mut bytes, 0, index, Source::CellDep)?;
    if len != bytes.len() {
        return Err(Error::Encoding);
    }
    let header = packed::HeaderReader::from_slice(&bytes).map_err(|_| Error::Encoding)?;
    Ok(u64::from_le_bytes(
        header
            .raw()
            .number()
            .as_slice()
            .try_into()
            .map_err(|_| Error::Encoding)?,
    ))
}

fn run() -> Result<(), Error> {
    let mut script_bytes = [0; 128];
    let len = syscalls::load_script(&mut script_bytes, 0).map_err(|_| Error::Encoding)?;
    let script =
        packed::ScriptReader::from_slice(&script_bytes[..len]).map_err(|_| Error::Encoding)?;
    let args = script.args().raw_data();
    let expected_proposal: [u8; 32] = args.try_into().map_err(|_| Error::Encoding)?;
    // Retracting/recycling a ballot remains controlled by its own input Lock.
    match load_cell_capacity(0, Source::GroupOutput) {
        Err(SysError::IndexOutOfBound) => return Ok(()),
        Err(error) => return Err(error.into()),
        Ok(_) => (),
    }
    let mut tx_bytes = alloc::vec![0; MAX_TX_BYTES];
    let len = syscalls::load_transaction(&mut tx_bytes, 0).map_err(|_| Error::Bound)?;
    let tx =
        packed::TransactionReader::from_slice(&tx_bytes[..len]).map_err(|_| Error::Encoding)?;
    let raw = tx.raw();
    if raw.cell_deps().len() > MAX_DEPS
        || raw.inputs().len() > MAX_INPUTS
        || raw.outputs().len() > MAX_OUTPUTS
    {
        return Err(Error::Bound);
    }
    // Only the direct prefix is addressable without dep-group expansion. Wallet
    // dep groups may follow it, but cannot supply proposal/deposit authority.
    let prefix = raw
        .cell_deps()
        .iter()
        .take_while(|dep| dep.dep_type().as_slice() == [0])
        .count();
    let mut seen = alloc::collections::BTreeSet::new();
    for dep in raw.cell_deps().iter() {
        if !seen.insert(dep.out_point().as_slice().to_vec()) {
            return Err(Error::Duplicate);
        }
    }
    let mut proposal_index = None;
    for index in 0..prefix {
        if load_cell_type_hash(index, Source::CellDep)? == Some(expected_proposal)
            && proposal_index.replace(index).is_some()
        {
            return Err(Error::Duplicate);
        }
    }
    let proposal_index = proposal_index.ok_or(Error::Proposal)?;
    let proposal = Proposal::decode(&exact_data::<{ Proposal::LEN }>(
        proposal_index,
        Source::CellDep,
    )?)
    .map_err(|_| Error::Proposal)?;
    if script.code_hash().as_slice() != proposal.vote_code || script.hash_type().as_slice() != [4] {
        return Err(Error::Proposal);
    }
    let proposal_number = creation_number(proposal_index)?;
    let mut ballot_deposits = alloc::collections::BTreeSet::new();
    for group_index in 0..MAX_OUTPUTS {
        let voter = match load_cell_lock_hash(group_index, Source::GroupOutput) {
            Err(SysError::IndexOutOfBound) => return Ok(()),
            Err(error) => return Err(error.into()),
            Ok(voter) => voter,
        };
        let ballot = Ballot::decode(&exact_data::<{ Ballot::LEN }>(
            group_index,
            Source::GroupOutput,
        )?)
        .map_err(|_| Error::Encoding)?;
        if !ballot_deposits.insert(ballot.deposit) {
            return Err(Error::Duplicate);
        }
        let dep_index = raw
            .cell_deps()
            .iter()
            .take(prefix)
            .position(|dep| dep.out_point().as_slice() == ballot.deposit)
            .ok_or(Error::Deposit)?;
        // A voting transaction cannot simultaneously withdraw its backing deposit.
        if raw
            .inputs()
            .iter()
            .any(|input| input.previous_output().as_slice() == ballot.deposit)
        {
            return Err(Error::Deposit);
        }
        if load_cell_type_hash(dep_index, Source::CellDep)? != Some(proposal.dao_script)
            || exact_data::<8>(dep_index, Source::CellDep)? != [0; 8]
        {
            return Err(Error::Deposit);
        }
        if load_cell_capacity(dep_index, Source::CellDep)? != ballot.weight {
            return Err(Error::Weight);
        }
        if load_cell_lock_hash(dep_index, Source::CellDep)? != voter {
            return Err(Error::Owner);
        }
        let mut authorized = false;
        for index in 0..raw.inputs().len() {
            if load_cell_lock_hash(index, Source::Input)? == voter {
                authorized = true;
                break;
            }
        }
        if !authorized {
            return Err(Error::Owner);
        }
        let deposit_number = creation_number(dep_index)?;
        if deposit_number >= proposal_number {
            return Err(Error::Age);
        }
    }
    // If there were exactly MAX_OUTPUTS entries, ensure the bounded loop did not
    // silently skip a further output. The raw transaction bound already limits it.
    match load_cell_lock_hash(MAX_OUTPUTS, Source::GroupOutput) {
        Err(SysError::IndexOutOfBound) => Ok(()),
        _ => Err(Error::Bound),
    }
}

pub fn entry() -> i8 {
    match run() {
        Ok(()) => 0,
        Err(error) => error as i8,
    }
}
