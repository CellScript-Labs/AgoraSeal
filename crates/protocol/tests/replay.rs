//! Synthetic CKB block fixtures exercise commitments and deterministic policy.
//! They are not consensus-valid transactions and do not test on-chain eligibility.
use agoraseal_protocol::{Ballot, Choice, Error, Hash, Limits, OutPoint, Proposal, Replay, hash};
use ckb_types::{
    bytes::Bytes,
    core::{BlockBuilder, BlockView, TransactionBuilder, TransactionView},
    packed,
    prelude::*,
};

fn owner(id: u8) -> packed::Script {
    packed::Script::new_builder()
        .code_hash([id; 32])
        .hash_type(4u8)
        .build()
}
fn proposal_script() -> packed::Script {
    owner(41)
}
fn proposal() -> Proposal {
    Proposal {
        genesis: [1; 32],
        vote_code: [2; 32],
        dao_script: [5; 32],
        duration: 3,
        quorum: 100,
        amount: 100,
        recipient: [3; 32],
        description: [4; 32],
    }
}
fn output(script: packed::Script, lock: packed::Script) -> packed::CellOutput {
    packed::CellOutput::new_builder()
        .capacity(100_000_000_000u64)
        .lock(lock)
        .type_(Some(script).pack())
        .build()
}
fn proposal_tx(p: &Proposal) -> TransactionView {
    TransactionBuilder::default()
        .output(output(proposal_script(), owner(7)))
        .output_data(Bytes::copy_from_slice(&p.encode()).pack())
        .build()
}
fn deposit(id: u8) -> OutPoint {
    let mut outpoint = [0; 36];
    outpoint[..32].fill(id);
    outpoint
}
fn outpoint(tx: &TransactionView, index: u32) -> OutPoint {
    let mut bytes = [0; 36];
    bytes[..32].copy_from_slice(tx.hash().as_slice());
    bytes[32..].copy_from_slice(&index.to_le_bytes());
    bytes
}
fn ballot_tx(ballots: &[(u8, u64, Choice)], voter: u8) -> TransactionView {
    let script = packed::Script::new_builder()
        .code_hash(proposal().vote_code)
        .hash_type(4u8)
        .args(Bytes::copy_from_slice(&hash(proposal_script().as_slice())).pack())
        .build();
    let mut builder = TransactionBuilder::default();
    for (id, weight, choice) in ballots {
        let ballot = Ballot {
            deposit: deposit(*id),
            weight: *weight,
            choice: *choice,
        };
        builder = builder
            .output(output(script.clone(), owner(voter)))
            .output_data(Bytes::copy_from_slice(&ballot.encode()).pack())
            .cell_dep(
                packed::CellDep::new_builder()
                    .out_point(packed::OutPoint::from_slice(&ballot.deposit).unwrap())
                    .build(),
            );
    }
    builder.build()
}
fn spend(outpoints: &[OutPoint]) -> TransactionView {
    let mut builder = TransactionBuilder::default();
    for outpoint in outpoints {
        builder = builder.input(
            packed::CellInput::new_builder()
                .previous_output(packed::OutPoint::from_slice(outpoint).unwrap())
                .build(),
        );
    }
    builder.build()
}
fn chain_with(p: &Proposal, rows: Vec<Vec<TransactionView>>) -> Vec<BlockView> {
    let mut blocks = Vec::new();
    let mut rows = rows;
    rows.insert(0, vec![proposal_tx(p)]);
    for (offset, mut transactions) in rows.into_iter().enumerate() {
        if transactions.is_empty() {
            transactions.push(TransactionBuilder::default().build());
        }
        let parent: packed::Byte32 = blocks
            .last()
            .map_or_else(Default::default, |b: &BlockView| b.hash());
        blocks.push(
            BlockBuilder::default()
                .number(100u64 + offset as u64)
                .epoch(ckb_types::core::EpochNumberWithFraction::new(
                    1,
                    offset as u64,
                    1000,
                ))
                .parent_hash(parent)
                .transactions(transactions)
                .build(),
        );
    }
    blocks
}
fn scanner(blocks: &[BlockView], limits: Limits) -> Replay {
    Replay::new(
        hash(proposal_script().as_slice()),
        blocks[0].hash().as_slice().try_into().unwrap(),
        blocks.last().unwrap().hash().as_slice().try_into().unwrap(),
        limits,
    )
}
fn tally(blocks: &[BlockView]) -> Result<agoraseal_protocol::Tally, Error> {
    let mut replay = scanner(blocks, Limits::default());
    for block in blocks {
        replay.push(block.data().as_slice())?;
    }
    replay.finish()
}

#[test]
fn complete_range_counts_independent_deposits_and_latest_vote() {
    let blocks = chain_with(
        &proposal(),
        vec![
            vec![ballot_tx(
                &[(10, 120, Choice::Yes), (11, 50, Choice::No)],
                8,
            )],
            vec![ballot_tx(&[(11, 50, Choice::Yes)], 8)],
            vec![],
        ],
    );
    let result = tally(&blocks).unwrap();
    assert_eq!(
        (result.yes, result.no, result.counted, result.passed),
        (170, 0, 2, true)
    );
    assert_eq!((result.start_number, result.end_number), (100, 103));
    assert_eq!(result, tally(&blocks).unwrap());
}

#[test]
fn spending_old_ballot_does_not_retract_latest_ballot() {
    let old = ballot_tx(&[(10, 120, Choice::Yes)], 8);
    let newer = ballot_tx(&[(10, 120, Choice::No)], 8);
    let blocks = chain_with(
        &proposal(),
        vec![
            vec![old.clone()],
            vec![newer],
            vec![spend(&[outpoint(&old, 0)])],
        ],
    );
    let result = tally(&blocks).unwrap();
    assert_eq!((result.yes, result.no, result.counted), (0, 120, 1));
}

#[test]
fn spending_latest_ballot_retracts_only_its_deposit() {
    let votes = ballot_tx(&[(10, 120, Choice::Yes), (11, 50, Choice::Yes)], 8);
    let blocks = chain_with(
        &proposal(),
        vec![
            vec![votes.clone()],
            vec![],
            vec![spend(&[outpoint(&votes, 0)])],
        ],
    );
    let result = tally(&blocks).unwrap();
    assert_eq!(
        (result.yes, result.no, result.counted, result.passed),
        (50, 0, 1, false)
    );
}

#[test]
fn withdrawn_deposit_invalidates_weight_and_cannot_be_revived() {
    let vote = ballot_tx(&[(10, 120, Choice::Yes)], 8);
    let blocks = chain_with(
        &proposal(),
        vec![vec![vote.clone()], vec![spend(&[deposit(10)])], vec![]],
    );
    assert_eq!(tally(&blocks).unwrap().counted, 0);
    let blocks = chain_with(
        &proposal(),
        vec![vec![vote.clone()], vec![spend(&[deposit(10)])], vec![vote]],
    );
    assert_eq!(tally(&blocks), Err(Error::DepositIdentity));
}

#[test]
fn omit_or_change_transaction_or_witness_rejects() {
    let vote = ballot_tx(&[(10, 120, Choice::Yes)], 8);
    let blocks = chain_with(&proposal(), vec![vec![vote.clone()], vec![], vec![]]);
    for tampered in [
        TransactionBuilder::default().build(),
        vote.as_advanced_builder()
            .witness(Bytes::from_static(b"forged").pack())
            .build(),
    ] {
        let mut replay = scanner(&blocks, Limits::default());
        replay.push(blocks[0].data().as_slice()).unwrap();
        // Preserve original header: the prover cannot repair the chain anchor.
        let altered = blocks[1]
            .data()
            .as_builder()
            .transactions(vec![tampered.data()].pack())
            .build();
        assert_eq!(replay.push(altered.as_slice()), Err(Error::TransactionRoot));
        assert_eq!(replay.finish(), Err(Error::Poisoned));
    }
}

#[test]
fn skip_reorder_truncate_extend_or_substitute_chain_rejects() {
    let blocks = chain_with(&proposal(), vec![vec![], vec![], vec![]]);
    let mut replay = scanner(&blocks, Limits::default());
    replay.push(blocks[0].data().as_slice()).unwrap();
    assert_eq!(
        replay.push(blocks[2].data().as_slice()),
        Err(Error::BlockOrder)
    );
    let mut replay = scanner(&blocks, Limits::default());
    for block in &blocks[..3] {
        replay.push(block.data().as_slice()).unwrap();
    }
    assert_eq!(replay.finish(), Err(Error::BlockOrder));
    let mut replay = scanner(&blocks, Limits::default());
    assert_eq!(
        replay.push(blocks[1].data().as_slice()),
        Err(Error::StartAnchor)
    );
    let mut replay = Replay::new(
        hash(proposal_script().as_slice()),
        blocks[0].hash().as_slice().try_into().unwrap(),
        [99; 32],
        Limits::default(),
    );
    for block in &blocks {
        replay.push(block.data().as_slice()).unwrap();
    }
    assert_eq!(replay.finish(), Err(Error::EndAnchor));
    let mut replay = scanner(&blocks, Limits::default());
    for block in &blocks {
        replay.push(block.data().as_slice()).unwrap();
    }
    let extra = BlockBuilder::default()
        .number(104u64)
        .epoch(ckb_types::core::EpochNumberWithFraction::new(1, 4, 1000))
        .parent_hash(blocks[3].hash())
        .transaction(TransactionBuilder::default().build())
        .build();
    assert_eq!(replay.push(extra.data().as_slice()), Err(Error::BlockOrder));
}

#[test]
fn deposit_weight_owner_and_duplicate_mutations_reject() {
    for altered in [
        ballot_tx(&[(10, 121, Choice::Yes)], 8),
        ballot_tx(&[(10, 120, Choice::Yes)], 9),
    ] {
        let blocks = chain_with(
            &proposal(),
            vec![
                vec![ballot_tx(&[(10, 120, Choice::Yes)], 8)],
                vec![altered],
                vec![],
            ],
        );
        assert_eq!(tally(&blocks), Err(Error::DepositIdentity));
    }
    let duplicate = ballot_tx(&[(10, 120, Choice::Yes), (10, 120, Choice::No)], 8);
    let blocks = chain_with(&proposal(), vec![vec![duplicate], vec![], vec![]]);
    // Duplicate direct deps already reject before any second vote can count.
    assert_eq!(tally(&blocks), Err(Error::DepositIdentity));
}

#[test]
fn ballot_codecs_fail_closed_for_every_short_length_and_trailing_bytes() {
    let ballot = Ballot {
        deposit: deposit(7),
        weight: 120,
        choice: Choice::Yes,
    };
    let bytes = ballot.encode();
    assert_eq!(Ballot::decode(&bytes), Ok(ballot));
    for length in 0..bytes.len() {
        assert_eq!(Ballot::decode(&bytes[..length]), Err(Error::Encoding));
    }
    let mut extra = bytes.to_vec();
    extra.push(0);
    assert_eq!(Ballot::decode(&extra), Err(Error::Encoding));
    let mut bad = bytes;
    bad[52] = 2;
    assert_eq!(Ballot::decode(&bad), Err(Error::Encoding));
    let bytes = proposal().encode();
    assert_eq!(Proposal::decode(&bytes), Ok(proposal()));
    for length in 0..bytes.len() {
        assert_eq!(Proposal::decode(&bytes[..length]), Err(Error::Encoding));
    }
}

#[test]
fn bounds_are_enforced_and_failed_scanners_cannot_resume() {
    let blocks = chain_with(
        &proposal(),
        vec![
            vec![ballot_tx(&[(10, 120, Choice::Yes)], 8)],
            vec![],
            vec![],
        ],
    );
    for limits in [
        Limits {
            block_bytes: 1,
            ..Limits::default()
        },
        Limits {
            total_bytes: 1,
            ..Limits::default()
        },
        Limits {
            blocks: 3,
            ..Limits::default()
        },
    ] {
        let mut replay = scanner(&blocks, limits);
        assert_eq!(replay.push(blocks[0].data().as_slice()), Err(Error::Bound));
        assert_eq!(
            replay.push(blocks[0].data().as_slice()),
            Err(Error::Poisoned)
        );
    }
    for limits in [
        Limits {
            deposits: 0,
            ..Limits::default()
        },
        Limits {
            ballots: 0,
            ..Limits::default()
        },
    ] {
        let mut replay = scanner(&blocks, limits);
        replay.push(blocks[0].data().as_slice()).unwrap();
        assert_eq!(replay.push(blocks[1].data().as_slice()), Err(Error::Bound));
    }
}

#[test]
fn malformed_matching_ballot_rejects_instead_of_disappearing() {
    let vote = ballot_tx(&[(10, 120, Choice::Yes)], 8);
    let tx = vote
        .data()
        .as_builder()
        .raw(
            vote.data()
                .raw()
                .as_builder()
                .outputs_data(vec![Bytes::from_static(b"malformed").pack()].pack())
                .build(),
        )
        .build()
        .into_view();
    let blocks = chain_with(&proposal(), vec![vec![tx], vec![], vec![]]);
    assert_eq!(tally(&blocks), Err(Error::Encoding));
}

#[test]
fn proposal_cannot_be_consumed_or_duplicated_during_window() {
    let tx = proposal_tx(&proposal());
    let blocks = chain_with(
        &proposal(),
        vec![vec![spend(&[outpoint(&tx, 0)])], vec![], vec![]],
    );
    assert_eq!(tally(&blocks), Err(Error::ProposalSpent));
    let blocks = chain_with(&proposal(), vec![vec![tx], vec![], vec![]]);
    assert_eq!(tally(&blocks), Err(Error::DuplicateProposal));
}

#[test]
fn total_overflow_and_ties_cannot_pass() {
    let blocks = chain_with(
        &proposal(),
        vec![
            vec![ballot_tx(
                &[(10, u64::MAX, Choice::Yes), (11, 1, Choice::No)],
                8,
            )],
            vec![],
            vec![],
        ],
    );
    assert_eq!(tally(&blocks), Err(Error::Overflow));
    let blocks = chain_with(
        &proposal(),
        vec![
            vec![ballot_tx(&[(10, 50, Choice::Yes), (11, 50, Choice::No)], 8)],
            vec![],
            vec![],
        ],
    );
    assert!(!tally(&blocks).unwrap().passed);
}

#[test]
fn counted_digest_binds_ballot_direction_and_voter() {
    let run = |choice, voter| {
        tally(&chain_with(
            &proposal(),
            vec![vec![ballot_tx(&[(10, 120, choice)], voter)], vec![], vec![]],
        ))
        .unwrap()
        .counted_digest
    };
    let yes: Hash = run(Choice::Yes, 8);
    assert_ne!(yes, run(Choice::No, 8));
    assert_ne!(yes, run(Choice::Yes, 9));
}
