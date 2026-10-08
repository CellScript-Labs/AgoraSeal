//! Actual CellScript vote policy plus exact generic CKB context adapter.
//! Fake DAO/proposal types and always-success
//! owner Locks isolate this script; node, DAO and real signature evidence is separate.
use agoraseal_protocol::{Ballot, Choice, Proposal};
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use ckb_types::{
    bytes::Bytes,
    core::{
        EpochNumberWithFraction, HeaderBuilder, ScriptHashType, TransactionBuilder, TransactionView,
    },
    packed,
    prelude::*,
};

#[derive(Clone, Copy, Debug)]
enum Mutation {
    None,
    Weight,
    Owner,
    Age,
    DaoType,
    Withdrawal,
    Duplicate,
    MissingHeader,
    MissingProposal,
    BadCode,
    ContextCode,
    ClaimedHeight,
    DepositPoint,
    ProposalHeight,
    SpentDeposit,
    DepGroup,
    DuplicateDep,
    InvalidChoice,
    BallotTrailing,
    OwnerAtBound,
    TooManyInputs,
}

fn fixture(mutation: Mutation) -> (Context, TransactionView) {
    let elf = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/cellscript/vote.elf"
    ))
    .expect("build vote script with scripts/gate.sh first");
    let mut context = Context::default();
    let code = context.deploy_cell(elf.into());
    let success = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let adapter = context.deploy_cell(std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../verifiers/ckb-context/target/riscv64imac-unknown-none-elf/release/agoraseal-ckb-context"
    )).expect("build context adapter first").into());
    let owner = context
        .build_script(&success, Bytes::from_static(b"owner"))
        .unwrap();
    let other = context
        .build_script(&success, Bytes::from_static(b"other"))
        .unwrap();
    let proposal_type = context
        .build_script(&success, Bytes::from_static(b"proposal"))
        .unwrap();
    let dao_type = context
        .build_script(&success, Bytes::from_static(b"dao"))
        .unwrap();
    let proposal_hash = proposal_type.calc_script_hash();
    let vote_type = context
        .build_script_with_hash_type(&code, ScriptHashType::Data2, proposal_hash.as_bytes())
        .unwrap();
    let deposit_capacity = 100_000_000_000u64;
    let deposit_cell = packed::CellOutput::new_builder()
        .capacity(deposit_capacity)
        .lock(owner.clone())
        .type_(
            Some(if matches!(mutation, Mutation::DaoType) {
                other.clone()
            } else {
                dao_type.clone()
            })
            .pack(),
        )
        .build();
    let deposit = context.create_cell(
        deposit_cell,
        if matches!(mutation, Mutation::Withdrawal) {
            Bytes::from_static(&[1, 0, 0, 0, 0, 0, 0, 0])
        } else {
            Bytes::from_static(&[0; 8])
        },
    );
    let p = Proposal {
        genesis: [1; 32],
        vote_code: if matches!(mutation, Mutation::BadCode) {
            [7; 32]
        } else {
            vote_type.code_hash().as_slice().try_into().unwrap()
        },
        dao_script: dao_type.calc_script_hash().as_slice().try_into().unwrap(),
        duration: 10,
        quorum: 100,
        amount: 100,
        recipient: [3; 32],
        description: [4; 32],
    };
    let proposal_cell = context.create_cell(
        packed::CellOutput::new_builder()
            .capacity(100_000_000_000u64)
            .lock(owner.clone())
            .type_(Some(proposal_type).pack())
            .build(),
        Bytes::copy_from_slice(&p.encode()),
    );
    let header = |number| {
        HeaderBuilder::default()
            .number(number)
            .epoch(EpochNumberWithFraction::new(1, 0, 1000))
            .build()
    };
    let deposit_header = header(if matches!(mutation, Mutation::Age) {
        100
    } else {
        90
    });
    let proposal_header = header(100);
    context.insert_header(deposit_header.clone());
    context.insert_header(proposal_header.clone());
    context.link_cell_with_block(deposit.clone(), deposit_header.hash(), 0);
    context.link_cell_with_block(proposal_cell.clone(), proposal_header.hash(), 0);
    let funding = context.create_cell(
        packed::CellOutput::new_builder()
            .capacity(100_000_000_000u64)
            .lock(if matches!(mutation, Mutation::Owner) {
                other
            } else {
                owner.clone()
            })
            .build(),
        Bytes::new(),
    );
    let ballot = Ballot {
        deposit: if matches!(mutation, Mutation::DepositPoint) {
            [7; 36]
        } else {
            deposit.as_slice().try_into().unwrap()
        },
        weight: deposit_capacity + u64::from(matches!(mutation, Mutation::Weight)),
        choice: Choice::Yes,
    };
    let mut ballot_data = ballot.encode().to_vec();
    if matches!(mutation, Mutation::InvalidChoice) {
        ballot_data[52] = 2;
    }
    if matches!(mutation, Mutation::BallotTrailing) {
        ballot_data.push(0);
    }
    let vote_output = packed::CellOutput::new_builder()
        .capacity(20_000_000_000u64)
        .lock(owner.clone())
        .type_(Some(vote_type).pack())
        .build();
    let dep = |out_point| packed::CellDep::new_builder().out_point(out_point).build();
    let mut tx = TransactionBuilder::default()
        .input(
            packed::CellInput::new_builder()
                .previous_output(funding)
                .build(),
        )
        .cell_dep(if matches!(mutation, Mutation::DepGroup) {
            let group_data = packed::OutPointVec::new_builder()
                .push(deposit.clone())
                .build()
                .as_bytes();
            let group = context.create_cell(
                packed::CellOutput::new_builder()
                    .capacity(deposit_capacity)
                    .lock(owner.clone())
                    .build(),
                group_data,
            );
            packed::CellDep::new_builder()
                .out_point(group)
                .dep_type(1u8)
                .build()
        } else {
            dep(deposit.clone())
        })
        .output(vote_output.clone())
        .output_data(Bytes::from(ballot_data).pack());
    if !matches!(mutation, Mutation::MissingProposal) {
        tx = tx.cell_dep(dep(proposal_cell));
    }
    tx = tx.cell_dep(dep(if matches!(mutation, Mutation::ContextCode) {
        success.clone()
    } else {
        adapter
    }));
    let mut payload = b"CSARGv1\0".to_vec();
    let claimed_height = if matches!(mutation, Mutation::Age) {
        100u64
    } else if matches!(mutation, Mutation::ClaimedHeight) {
        89
    } else {
        90
    };
    payload.extend_from_slice(&claimed_height.to_le_bytes());
    payload.extend_from_slice(
        &(if matches!(mutation, Mutation::ProposalHeight) {
            101u64
        } else {
            100
        })
        .to_le_bytes(),
    );
    tx = tx.witness(
        packed::WitnessArgs::new_builder()
            .input_type(Some(Bytes::from(payload)).pack())
            .build()
            .as_bytes()
            .pack(),
    );
    if !matches!(mutation, Mutation::MissingHeader) {
        tx = tx
            .header_dep(deposit_header.hash())
            .header_dep(proposal_header.hash());
    }
    if matches!(mutation, Mutation::Duplicate) {
        tx = tx
            .output(vote_output)
            .output_data(Bytes::copy_from_slice(&ballot.encode()).pack());
    }
    if matches!(mutation, Mutation::SpentDeposit) {
        tx = tx.input(
            packed::CellInput::new_builder()
                .previous_output(deposit.clone())
                .build(),
        );
    }
    if matches!(mutation, Mutation::OwnerAtBound | Mutation::TooManyInputs) {
        // Replace the initial owner's funding input by unrelated funding, and
        // put the only authorising input at the final admitted index.
        let unrelated = context
            .build_script(&success, Bytes::from_static(b"unrelated"))
            .unwrap();
        let count = if matches!(mutation, Mutation::TooManyInputs) {
            129
        } else {
            128
        };
        let inputs: Vec<_> = (0..count)
            .map(|i| {
                let cell = context.create_cell(
                    packed::CellOutput::new_builder()
                        .capacity(deposit_capacity)
                        .lock(if i == count - 1 {
                            owner.clone()
                        } else {
                            unrelated.clone()
                        })
                        .build(),
                    Bytes::new(),
                );
                packed::CellInput::new_builder()
                    .previous_output(cell)
                    .build()
            })
            .collect();
        tx = tx.set_inputs(inputs);
    }
    let tx = context.complete_tx(tx.build());
    let tx = if matches!(mutation, Mutation::DuplicateDep) {
        tx.as_advanced_builder().cell_dep(dep(deposit)).build()
    } else {
        tx
    };
    (context, tx)
}

#[test]
fn eligible_ballot_executes_cellscript_and_exact_context_child() {
    let (context, tx) = fixture(Mutation::None);
    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("eligible ballot must execute");
    assert!(cycles > 0);
    println!("CellScript vote eligibility cycles: {cycles}");
}

#[test]
fn eligibility_substitutions_reach_exact_rejection_codes() {
    for mutation in [
        Mutation::Weight,
        Mutation::Owner,
        Mutation::Age,
        Mutation::DaoType,
        Mutation::Withdrawal,
        Mutation::Duplicate,
        Mutation::MissingHeader,
        Mutation::MissingProposal,
        Mutation::BadCode,
        Mutation::ContextCode,
        Mutation::ClaimedHeight,
        Mutation::DepositPoint,
        Mutation::ProposalHeight,
        Mutation::SpentDeposit,
        Mutation::DepGroup,
        Mutation::DuplicateDep,
        Mutation::InvalidChoice,
        Mutation::BallotTrailing,
        Mutation::TooManyInputs,
    ] {
        let (context, tx) = fixture(mutation);
        let error = context
            .verify_tx(&tx, 10_000_000)
            .expect_err("mutation must reject");
        let text = format!("{error:?}");
        let expected = match mutation {
            Mutation::MissingHeader
            | Mutation::ClaimedHeight
            | Mutation::DepositPoint
            | Mutation::ProposalHeight
            | Mutation::DepGroup
            | Mutation::DuplicateDep => 1,
            Mutation::ContextCode => 41,
            _ => 5,
        };
        assert!(
            text.contains(&format!("error code {expected} on page")),
            "{mutation:?}: {text}"
        );
    }
}

#[test]
fn owner_at_last_bounded_input_still_authorizes() {
    let (context, tx) = fixture(Mutation::OwnerAtBound);
    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("owner at index 127");
    println!("CellScript maximum-input vote fixture cycles: {cycles}");
}

#[test]
fn ballot_can_be_reclaimed_after_proposal_dependency_is_gone() {
    let (mut context, cast) = fixture(Mutation::None);
    context.verify_tx(&cast, 10_000_000).unwrap();
    let ballot = context.create_cell(
        cast.outputs().get(0).unwrap(),
        cast.outputs_data().get(0).unwrap().raw_data(),
    );
    let mut payload = b"CSARGv1\0".to_vec();
    payload.extend_from_slice(&[0; 16]);
    let tx = TransactionBuilder::default()
        .input(
            packed::CellInput::new_builder()
                .previous_output(ballot)
                .build(),
        )
        .witness(
            packed::WitnessArgs::new_builder()
                .input_type(Some(Bytes::from(payload)).pack())
                .build()
                .as_bytes()
                .pack(),
        )
        .build();
    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, 3_000_000)
        .expect("reclaim without proposal or context adapter");
}
