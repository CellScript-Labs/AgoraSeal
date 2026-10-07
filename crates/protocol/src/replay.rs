use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};
use ckb_gen_types::{packed, prelude::*};
use merkle_cbt::{CBMT, merkle_tree::Merge};

use crate::{Ballot, Choice, Error, Hash, OutPoint, Proposal, hash};

struct MergeHash;
impl Merge for MergeHash {
    type Item = Hash;
    fn merge(left: &Hash, right: &Hash) -> Hash {
        let mut input = [0; 64];
        input[..32].copy_from_slice(left);
        input[32..].copy_from_slice(right);
        hash(&input)
    }
}

/// Development bounds. Production values must be measured and bound to the guest.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub blocks: u32,
    pub block_bytes: usize,
    pub total_bytes: u64,
    pub deposits: usize,
    pub ballots: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            blocks: 100_000,
            block_bytes: 4 * 1024 * 1024,
            total_bytes: 1024 * 1024 * 1024,
            deposits: 100_000,
            ballots: 1_000_000,
        }
    }
}

struct Record {
    ballot: Ballot,
    outpoint: OutPoint,
    voter: Hash,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tally {
    pub proposal: Proposal,
    pub proposal_script: Hash,
    pub proposal_outpoint: OutPoint,
    pub start_hash: Hash,
    pub end_hash: Hash,
    pub start_number: u64,
    pub end_number: u64,
    pub yes: u64,
    pub no: u64,
    pub counted: u64,
    pub counted_digest: Hash,
    pub passed: bool,
}

/// Streaming scanner. Any failure poisons the instance, preventing partial results.
pub struct Replay {
    expected_script: Hash,
    expected_start: Hash,
    expected_end: Hash,
    limits: Limits,
    proposal: Option<(Proposal, OutPoint)>,
    start_number: u64,
    last_number: u64,
    last_hash: Hash,
    blocks: u32,
    bytes: u64,
    observed: u64,
    records: BTreeMap<OutPoint, Record>,
    ballot_deposits: BTreeMap<OutPoint, OutPoint>,
    // Retain identities across ballot retraction; historical spends cannot be undone.
    identities: BTreeMap<OutPoint, (Hash, u64)>,
    spent_deposits: BTreeSet<OutPoint>,
    poisoned: bool,
}

impl Replay {
    pub fn new(proposal_script: Hash, start: Hash, end: Hash, limits: Limits) -> Self {
        Self {
            expected_script: proposal_script,
            expected_start: start,
            expected_end: end,
            limits,
            proposal: None,
            start_number: 0,
            last_number: 0,
            last_hash: [0; 32],
            blocks: 0,
            bytes: 0,
            observed: 0,
            records: BTreeMap::new(),
            ballot_deposits: BTreeMap::new(),
            identities: BTreeMap::new(),
            spent_deposits: BTreeSet::new(),
            poisoned: false,
        }
    }

    pub fn push(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        let result = self.push_inner(bytes);
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }

    fn push_inner(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if bytes.len() > self.limits.block_bytes || self.blocks >= self.limits.blocks {
            return Err(Error::Bound);
        }
        self.bytes = self
            .bytes
            .checked_add(bytes.len() as u64)
            .ok_or(Error::Overflow)?;
        if self.bytes > self.limits.total_bytes {
            return Err(Error::Bound);
        }
        // CKB BlockV1 appends a consensus extension. The official compatible reader
        // handles it; vote authority depends on the header and committed transactions.
        let block =
            packed::BlockReader::from_compatible_slice(bytes).map_err(|_| Error::Encoding)?;
        let header = block.header();
        let header_hash = hash(header.as_slice());
        let number = u64::from_le_bytes(
            header
                .raw()
                .number()
                .as_slice()
                .try_into()
                .map_err(|_| Error::Encoding)?,
        );
        if self.blocks == 0 {
            if header_hash != self.expected_start {
                return Err(Error::StartAnchor);
            }
            self.start_number = number;
        } else if self.last_number.checked_add(1) != Some(number)
            || header.raw().parent_hash().as_slice() != self.last_hash
        {
            return Err(Error::BlockOrder);
        }

        let transactions = block.transactions();
        if transactions.is_empty() {
            return Err(Error::Encoding);
        }
        let mut raw_hashes = Vec::with_capacity(transactions.len());
        let mut witness_hashes = Vec::with_capacity(transactions.len());
        for tx in transactions.iter() {
            raw_hashes.push(hash(tx.raw().as_slice()));
            witness_hashes.push(hash(tx.as_slice()));
        }
        let root = CBMT::<Hash, MergeHash>::build_merkle_root(&[
            CBMT::<Hash, MergeHash>::build_merkle_root(&raw_hashes),
            CBMT::<Hash, MergeHash>::build_merkle_root(&witness_hashes),
        ]);
        if header.raw().transactions_root().as_slice() != root {
            return Err(Error::TransactionRoot);
        }

        for (tx_index, tx) in transactions.iter().enumerate() {
            if tx.raw().outputs().len() != tx.raw().outputs_data().len() {
                return Err(Error::Encoding);
            }
            for input in tx.raw().inputs().iter() {
                let outpoint: OutPoint = input
                    .previous_output()
                    .as_slice()
                    .try_into()
                    .map_err(|_| Error::Encoding)?;
                if self
                    .proposal
                    .as_ref()
                    .is_some_and(|(_, proposal)| proposal == &outpoint)
                {
                    return Err(Error::ProposalSpent);
                }
                if self.identities.contains_key(&outpoint) {
                    self.spent_deposits.insert(outpoint);
                    self.remove_record(&outpoint);
                }
                if let Some(deposit) = self.ballot_deposits.get(&outpoint).copied() {
                    self.remove_record(&deposit);
                }
            }
            let mut transaction_deposits = BTreeSet::new();
            for (index, output) in tx.raw().outputs().iter().enumerate() {
                let Some(script) = output.type_().to_opt() else {
                    continue;
                };
                let data = tx.raw().outputs_data().get(index).ok_or(Error::Encoding)?;
                let mut outpoint = [0; 36];
                outpoint[..32].copy_from_slice(&raw_hashes[tx_index]);
                outpoint[32..].copy_from_slice(
                    &u32::try_from(index)
                        .map_err(|_| Error::Bound)?
                        .to_le_bytes(),
                );
                if hash(script.as_slice()) == self.expected_script {
                    if self.proposal.is_some() || self.blocks != 0 {
                        return Err(Error::DuplicateProposal);
                    }
                    let proposal = Proposal::decode(data.raw_data())?;
                    if proposal
                        .duration
                        .checked_add(1)
                        .is_none_or(|n| n > self.limits.blocks)
                    {
                        return Err(Error::Bound);
                    }
                    self.start_number
                        .checked_add(u64::from(proposal.duration))
                        .ok_or(Error::Overflow)?;
                    self.proposal = Some((proposal, outpoint));
                    continue;
                }
                let Some((proposal, _)) = &self.proposal else {
                    continue;
                };
                // Exact immutable data2 code identity, full proposal Script hash args.
                if script.code_hash().as_slice() != proposal.vote_code
                    || script.hash_type().as_slice() != [4]
                    || script.args().raw_data() != self.expected_script
                {
                    continue;
                }
                if self.blocks == 0 {
                    continue;
                } // Voting interval is (S,E].
                let ballot = Ballot::decode(data.raw_data())?;
                if !transaction_deposits.insert(ballot.deposit) {
                    return Err(Error::DuplicateDeposit);
                }
                // The on-chain script validates the referenced deposit. Demand the
                // exact direct dependency here too; dep-group expansion is excluded.
                let matching_deps = tx
                    .raw()
                    .cell_deps()
                    .iter()
                    .filter(|dep| {
                        dep.dep_type().as_slice() == [0]
                            && dep.out_point().as_slice() == ballot.deposit
                    })
                    .count();
                if matching_deps != 1 || self.spent_deposits.contains(&ballot.deposit) {
                    return Err(Error::DepositIdentity);
                }
                let voter = hash(output.lock().as_slice());
                if let Some(identity) = self.identities.get(&ballot.deposit) {
                    if *identity != (voter, ballot.weight) {
                        return Err(Error::DepositIdentity);
                    }
                } else {
                    if self.identities.len() >= self.limits.deposits {
                        return Err(Error::Bound);
                    }
                    self.identities
                        .insert(ballot.deposit, (voter, ballot.weight));
                }
                self.observed = self.observed.checked_add(1).ok_or(Error::Overflow)?;
                if self.observed > self.limits.ballots {
                    return Err(Error::Bound);
                }
                self.remove_record(&ballot.deposit);
                self.ballot_deposits.insert(outpoint, ballot.deposit);
                self.records.insert(
                    ballot.deposit,
                    Record {
                        ballot,
                        outpoint,
                        voter,
                    },
                );
            }
        }
        let (proposal, _) = self.proposal.as_ref().ok_or(Error::ProposalMissing)?;
        if self.blocks > proposal.duration {
            return Err(Error::BlockOrder);
        }
        self.last_hash = header_hash;
        self.last_number = number;
        self.blocks = self.blocks.checked_add(1).ok_or(Error::Overflow)?;
        Ok(())
    }

    fn remove_record(&mut self, deposit: &OutPoint) {
        if let Some(record) = self.records.remove(deposit) {
            self.ballot_deposits.remove(&record.outpoint);
        }
    }

    pub fn finish(self) -> Result<Tally, Error> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        let (proposal, proposal_outpoint) = self.proposal.ok_or(Error::EmptyRange)?;
        if self.blocks != proposal.duration + 1 {
            return Err(Error::BlockOrder);
        }
        if self.last_hash != self.expected_end {
            return Err(Error::EndAnchor);
        }
        let mut yes = 0u64;
        let mut no = 0u64;
        let mut digest = ckb_hash::new_blake2b();
        digest.update(b"agoraseal-counted-v1");
        digest.update(&self.expected_script);
        digest.update(&self.expected_start);
        digest.update(&self.expected_end);
        digest.update(&(self.records.len() as u64).to_le_bytes());
        for (deposit, record) in &self.records {
            let total = match record.ballot.choice {
                Choice::Yes => &mut yes,
                Choice::No => &mut no,
            };
            *total = total
                .checked_add(record.ballot.weight)
                .ok_or(Error::Overflow)?;
            digest.update(deposit);
            digest.update(&record.outpoint);
            digest.update(&record.voter);
            digest.update(&record.ballot.weight.to_le_bytes());
            digest.update(&[record.ballot.choice as u8]);
        }
        let mut counted_digest = [0; 32];
        digest.finalize(&mut counted_digest);
        let total = yes.checked_add(no).ok_or(Error::Overflow)?;
        let passed = total >= proposal.quorum && yes > no;
        Ok(Tally {
            proposal,
            proposal_script: self.expected_script,
            proposal_outpoint,
            start_hash: self.expected_start,
            end_hash: self.expected_end,
            start_number: self.start_number,
            end_number: self.last_number,
            yes,
            no,
            counted: self.records.len() as u64,
            counted_digest,
            passed,
        })
    }
}
