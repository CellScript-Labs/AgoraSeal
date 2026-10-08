//! Actual proposal creation and Type-ID/header adapter in CKB-VM.
//! Funding Lock is isolated with ALWAYS_SUCCESS; this is not node/signature evidence.
use agoraseal_protocol::{Proposal, hash};
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use ckb_types::{
    bytes::Bytes,
    core::{
        Capacity, EpochNumberWithFraction, HeaderBuilder, ScriptHashType, TransactionBuilder,
        TransactionView,
    },
    packed,
    prelude::*,
};

pub const SIGHASH: [u8; 32] = [
    0x9b, 0xd7, 0xe0, 0x6f, 0x3e, 0xcf, 0x4b, 0xe0, 0xf2, 0xfc, 0xd2, 0x18, 0x8b, 0x23, 0xf1, 0xb9,
    0xfc, 0xc8, 0x8e, 0x5d, 0x4b, 0x65, 0xa8, 0x63, 0x7b, 0x17, 0x72, 0x3b, 0xbd, 0xa3, 0xcc, 0xe8,
];
pub const DAO: [u8; 32] = [
    0x82, 0xd7, 0x6d, 0x1b, 0x75, 0xfe, 0x2f, 0xd9, 0xa2, 0x7d, 0xfb, 0xaa, 0x65, 0xa0, 0x39, 0x22,
    0x1a, 0x38, 0x0d, 0x76, 0xc9, 0x26, 0xf3, 0x78, 0xd3, 0xf8, 0x1c, 0xf3, 0xe7, 0xe1, 0x3f, 0x2e,
];

pub fn artifact(path: &str) -> Bytes {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .unwrap_or_else(|error| panic!("{path}: run scripts/gate.sh first: {error}"))
    .into()
}

#[derive(Clone, Copy, Debug)]
pub enum CreationMutation {
    None,
    Genesis,
    GenesisNumber,
    MissingHeader,
    Vote,
    Dao,
    DurationZero,
    DurationBound,
    Quorum,
    Amount,
    Recipient,
    Refund,
    TreasuryCode,
    TreasuryHashType,
    ShortArgs,
    CapacityLow,
    CapacityHigh,
    FirstInputData,
    FirstInputType,
    WrongId,
    Duplicate,
    ProposalTrailing,
    ContextCode,
    WitnessShort,
    WitnessTrailing,
}

pub fn creation(mutation: CreationMutation) -> (Context, TransactionView) {
    let mut context = Context::new_with_deterministic_rng();
    let parent = context.deploy_cell(artifact("target/cellscript/funded-proposal.elf"));
    let treasury = context.deploy_cell(artifact("target/cellscript/funded-treasury.elf"));
    let adapter = context.deploy_cell(artifact("verifiers/ckb-state-context/target/riscv64imac-unknown-none-elf/release/agoraseal-ckb-state-context"));
    let verifier = context.deploy_cell(artifact(
        "verifiers/sp1-plonk/target/riscv64imac-unknown-none-elf/release/agoraseal-sp1-plonk",
    ));
    let success = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let funding_lock = context.build_script(&success, Bytes::new()).unwrap();
    let input_point = packed::OutPoint::new_builder().tx_hash([77u8; 32]).build();
    context.create_cell_with_out_point(
        input_point.clone(),
        packed::CellOutput::new_builder()
            .capacity(200_000_000_000u64)
            .lock(funding_lock.clone())
            .type_(
                if matches!(mutation, CreationMutation::FirstInputType) {
                    Some(funding_lock.clone())
                } else {
                    None
                }
                .pack(),
            )
            .build(),
        if matches!(mutation, CreationMutation::FirstInputData) {
            Bytes::from_static(&[1])
        } else {
            Bytes::new()
        },
    );
    let input = packed::CellInput::new_builder()
        .previous_output(input_point)
        .build();
    let mut id_material = input.as_slice().to_vec();
    id_material.extend_from_slice(&0u64.to_le_bytes());
    let mut id = hash(&id_material);
    if matches!(mutation, CreationMutation::WrongId) {
        id[31] ^= 1;
    }
    let proposal_type = context
        .build_script_with_hash_type(&parent, ScriptHashType::Data2, Bytes::copy_from_slice(&id))
        .unwrap();
    let mut refund: Vec<u8> = funding_lock.calc_script_hash().as_slice().to_vec();
    if matches!(mutation, CreationMutation::Refund) {
        refund[31] ^= 1;
    }
    if matches!(mutation, CreationMutation::ShortArgs) {
        refund.pop();
    }
    let treasury_lock = context
        .build_script_with_hash_type(
            if matches!(mutation, CreationMutation::TreasuryCode) {
                &success
            } else {
                &treasury
            },
            if matches!(mutation, CreationMutation::TreasuryHashType) {
                ScriptHashType::Data1
            } else {
                ScriptHashType::Data2
            },
            refund.into(),
        )
        .unwrap();
    let header = HeaderBuilder::default()
        .number(u64::from(matches!(
            mutation,
            CreationMutation::GenesisNumber
        )))
        .epoch(if matches!(mutation, CreationMutation::GenesisNumber) {
            EpochNumberWithFraction::new(1, 0, 1000)
        } else {
            EpochNumberWithFraction::new_unchecked(0, 0, 0)
        })
        .nonce(13u128)
        .build();
    context.insert_header(header.clone());
    let payee = packed::Script::new_builder()
        .code_hash(SIGHASH)
        .hash_type(1u8)
        .args(Bytes::from(vec![9; 20]).pack())
        .build();
    let dao = packed::Script::new_builder()
        .code_hash(DAO)
        .hash_type(1u8)
        .build();
    let mut proposal = Proposal {
        genesis: header.hash().as_slice().try_into().unwrap(),
        vote_code: hash(&artifact("target/cellscript/vote.elf")),
        dao_script: hash(dao.as_slice()),
        duration: 2,
        quorum: 1,
        amount: 10_000_000_000,
        recipient: hash(payee.as_slice()),
        description: [12; 32],
    };
    match mutation {
        CreationMutation::Genesis => proposal.genesis[31] ^= 1,
        CreationMutation::Vote => proposal.vote_code[31] ^= 1,
        CreationMutation::Dao => proposal.dao_script[31] ^= 1,
        CreationMutation::DurationZero => proposal.duration = 0,
        CreationMutation::DurationBound => proposal.duration = 17,
        CreationMutation::Quorum => proposal.quorum = 0,
        CreationMutation::Amount => proposal.amount = 6_099_999_999,
        CreationMutation::Recipient => proposal.recipient[31] ^= 1,
        _ => {}
    }
    let mut data = proposal.encode().to_vec();
    if matches!(mutation, CreationMutation::ProposalTrailing) {
        data.push(0);
    }
    let output = packed::CellOutput::new_builder()
        .lock(treasury_lock)
        .type_(Some(proposal_type).pack())
        .build();
    let receipt_capacity = output
        .occupied_capacity(Capacity::bytes(465).unwrap())
        .unwrap()
        .as_u64();
    let mut capacity = receipt_capacity + proposal.amount + 100_000;
    if matches!(mutation, CreationMutation::CapacityLow) {
        capacity = receipt_capacity + proposal.amount - 1;
    }
    if matches!(mutation, CreationMutation::CapacityHigh) {
        capacity += 1;
    }
    let output = output.as_builder().capacity(capacity).build();
    let mut entry = b"CSARGv1\0".to_vec();
    entry.extend_from_slice(&[0; 32]);
    entry.extend_from_slice(&[9; 20]);
    entry.extend_from_slice(payee.calc_script_hash().as_slice());
    if matches!(mutation, CreationMutation::WitnessShort) {
        entry.pop();
    }
    if matches!(mutation, CreationMutation::WitnessTrailing) {
        entry.push(0);
    }
    let witness = packed::WitnessArgs::new_builder()
        .input_type(Some(Bytes::from(entry)).pack())
        .build();
    let dep = |point| packed::CellDep::new_builder().out_point(point).build();
    let mut tx = TransactionBuilder::default()
        .input(input)
        .output(output.clone())
        .output_data(Bytes::from(data.clone()).pack())
        .cell_dep(dep(verifier))
        .cell_dep(dep(if matches!(mutation, CreationMutation::ContextCode) {
            success
        } else {
            adapter
        }))
        .witness(witness.as_bytes().pack());
    if !matches!(mutation, CreationMutation::MissingHeader) {
        tx = tx.header_dep(header.hash());
    }
    if matches!(mutation, CreationMutation::Duplicate) {
        tx = tx.output(output).output_data(Bytes::from(data).pack());
    }
    (context.clone(), context.complete_tx(tx.build()))
}
pub fn history(passing: bool) -> (Context, TransactionView, Vec<ckb_types::core::BlockView>) {
    use agoraseal_protocol::{Ballot, Choice};
    use ckb_types::core::BlockBuilder;
    let (mut context, creation) = creation(CreationMutation::None);
    context
        .verify_tx(&creation, 10_000_000)
        .expect("fixture must execute real proposal policy");
    let genesis = creation.header_deps().get(0).unwrap();
    let mut blocks = Vec::new();
    let proposal_point = packed::OutPoint::new_builder()
        .tx_hash(creation.hash())
        .build();
    let proposal = creation.outputs().get(0).unwrap();
    let proposal_data = creation.outputs_data().get(0).unwrap().raw_data();
    let proposal_hash = proposal.type_().to_opt().unwrap().calc_script_hash();
    context.create_cell_with_out_point(proposal_point.clone(), proposal, proposal_data);
    let start = BlockBuilder::default()
        .number(100u64)
        .epoch(EpochNumberWithFraction::new(1, 100, 1000))
        .parent_hash(genesis)
        .transaction(creation.clone())
        .build();
    context.insert_header(start.header());
    context.link_cell_with_block(proposal_point.clone(), start.hash(), 0);
    blocks.push(start);

    let middle_tx = if passing {
        let success = context.deploy_cell(ALWAYS_SUCCESS.clone());
        let vote = context.deploy_cell(artifact("target/cellscript/vote.elf"));
        let adapter = context.deploy_cell(artifact("verifiers/ckb-context/target/riscv64imac-unknown-none-elf/release/agoraseal-ckb-context"));
        let owner = context.build_script(&success, Bytes::new()).unwrap();
        let dao = packed::Script::new_builder()
            .code_hash(DAO)
            .hash_type(1u8)
            .build();
        let deposit = packed::OutPoint::new_builder().tx_hash([66u8; 32]).build();
        context.create_cell_with_out_point(
            deposit.clone(),
            packed::CellOutput::new_builder()
                .capacity(100_000_000_000u64)
                .lock(owner.clone())
                .type_(Some(dao).pack())
                .build(),
            Bytes::from(vec![0; 8]),
        );
        let deposit_header = HeaderBuilder::default()
            .number(61u64)
            .epoch(EpochNumberWithFraction::new(1, 61, 1000))
            .build();
        context.insert_header(deposit_header.clone());
        context.link_cell_with_block(deposit.clone(), deposit_header.hash(), 0);
        let signing_input = packed::OutPoint::new_builder().tx_hash([67u8; 32]).build();
        context.create_cell_with_out_point(
            signing_input.clone(),
            packed::CellOutput::new_builder()
                .capacity(100_000_000_000u64)
                .lock(owner.clone())
                .build(),
            Bytes::new(),
        );
        let vote_type = context
            .build_script_with_hash_type(&vote, ScriptHashType::Data2, proposal_hash.as_bytes())
            .unwrap();
        let ballot = Ballot {
            deposit: deposit.as_slice().try_into().unwrap(),
            weight: 100_000_000_000,
            choice: Choice::Yes,
        };
        let mut entry = b"CSARGv1\0".to_vec();
        entry.extend_from_slice(&61u64.to_le_bytes());
        entry.extend_from_slice(&100u64.to_le_bytes());
        let witness = packed::WitnessArgs::new_builder()
            .input_type(Some(Bytes::from(entry)).pack())
            .build();
        let dep = |point| packed::CellDep::new_builder().out_point(point).build();
        let tx = context.complete_tx(
            TransactionBuilder::default()
                .input(
                    packed::CellInput::new_builder()
                        .previous_output(signing_input)
                        .build(),
                )
                .output(
                    packed::CellOutput::new_builder()
                        .capacity(20_000_000_000u64)
                        .lock(owner)
                        .type_(Some(vote_type).pack())
                        .build(),
                )
                .output_data(Bytes::copy_from_slice(&ballot.encode()).pack())
                .cell_dep(dep(deposit))
                .cell_dep(dep(proposal_point))
                .cell_dep(dep(adapter))
                .header_dep(deposit_header.hash())
                .header_dep(blocks[0].hash())
                .witness(witness.as_bytes().pack())
                .build(),
        );
        context
            .verify_tx(&tx, 10_000_000)
            .expect("fixture ballot must execute actual vote and context");
        tx
    } else {
        TransactionBuilder::default().build()
    };
    for (number, tx) in [
        (101, middle_tx),
        (102, TransactionBuilder::default().build()),
    ] {
        let block = BlockBuilder::default()
            .number(number)
            .epoch(EpochNumberWithFraction::new(1, number, 1000))
            .parent_hash(blocks.last().unwrap().hash())
            .transaction(tx)
            .build();
        context.insert_header(block.header());
        blocks.push(block);
    }
    (context, creation, blocks)
}
