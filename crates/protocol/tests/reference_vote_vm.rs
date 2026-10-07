//! Rust reference ONLY; the application's authoritative vote policy must be CellScript.
//! Real CKB-VM reference eligibility tests. Fake DAO/proposal types and always-success
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

#[derive(Clone, Copy)]
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
}

fn fixture(mutation: Mutation) -> (Context, TransactionView) {
    let elf = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../reference/vote/target/riscv64imac-unknown-none-elf/release/agoraseal-vote"
    ))
    .expect("build vote script with scripts/gate.sh first");
    let mut context = Context::default();
    let code = context.deploy_cell(elf.into());
    let success = context.deploy_cell(ALWAYS_SUCCESS.clone());
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
        deposit: deposit.as_slice().try_into().unwrap(),
        weight: deposit_capacity + u64::from(matches!(mutation, Mutation::Weight)),
        choice: Choice::Yes,
    };
    let vote_output = packed::CellOutput::new_builder()
        .capacity(20_000_000_000u64)
        .lock(owner)
        .type_(Some(vote_type).pack())
        .build();
    let dep = |out_point| packed::CellDep::new_builder().out_point(out_point).build();
    let mut tx = TransactionBuilder::default()
        .input(
            packed::CellInput::new_builder()
                .previous_output(funding)
                .build(),
        )
        .cell_dep(dep(deposit))
        .output(vote_output.clone())
        .output_data(Bytes::copy_from_slice(&ballot.encode()).pack());
    if !matches!(mutation, Mutation::MissingProposal) {
        tx = tx.cell_dep(dep(proposal_cell));
    }
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
    let tx = context.complete_tx(tx.build());
    (context, tx)
}

#[test]
fn eligible_ballot_executes_real_riscv_script() {
    let (context, tx) = fixture(Mutation::None);
    let cycles = context
        .verify_tx(&tx, 3_000_000)
        .expect("eligible ballot must execute");
    assert!(cycles > 0);
    println!("vote eligibility cycles: {cycles}");
}

#[test]
fn eligibility_substitutions_reach_exact_rejection_codes() {
    for (mutation, code) in [
        (Mutation::Weight, 16),
        (Mutation::Owner, 15),
        (Mutation::Age, 17),
        (Mutation::DaoType, 14),
        (Mutation::Withdrawal, 14),
        (Mutation::Duplicate, 18),
        (Mutation::MissingHeader, 10),
        (Mutation::MissingProposal, 13),
        (Mutation::BadCode, 13),
    ] {
        let (context, tx) = fixture(mutation);
        let error = context
            .verify_tx(&tx, 3_000_000)
            .expect_err("mutation must reject");
        let text = format!("{error:?}");
        assert!(
            text.contains(&format!("error code {code}")),
            "expected vote rejection {code}, got {text}"
        );
    }
}
