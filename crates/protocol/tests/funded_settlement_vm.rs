//! Real proof + actual funded Type/Lock + unique-state/header adapter.
//! Synthetic headers isolate VM execution; full node evidence is separate.
#[path = "support/funded.rs"]
#[allow(dead_code)]
mod funded;
use agoraseal_protocol::{Proposal, PublicStatement, hash};
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use ckb_types::{
    bytes::Bytes,
    core::{Capacity, EpochNumberWithFraction, HeaderBuilder, TransactionBuilder, TransactionView},
    packed,
    prelude::*,
};

#[derive(Clone, Copy, Debug)]
enum Mutation {
    None,
    Claim,
    RecipientWitness,
    GenesisPublic,
    ProposalPublic,
    OutPointPublic,
    DataPublic,
    StartNumber,
    EndNumber,
    PublicNo,
    Passed,
    Proof,
    MissingPacket,
    VerifierCode,
    ContextCode,
    StartHeader,
    EndHeader,
    GenesisHeader,
    CreationAssociation,
    MissingHeaders,
    Prefix,
    ReceiptTrailing,
    ReceiptCapacity,
    ReceiptLock,
    PaymentLock,
    PaymentCapacity,
    PaymentData,
    PaymentType,
    ExtraOutput,
    ExtraInput,
    Burn,
    ReceiptReplay,
}
fn fixture(mutation: Mutation) -> (Context, TransactionView) {
    let (mut context, create, blocks) = funded::history(false);
    let root = std::env::var_os("AGORASEAL_FUNDED_FIXTURE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/funded-failed")
        });
    let mut public =
        std::fs::read(root.join("public-values.bin")).expect("committed real lifecycle fixture");
    let native_statement = PublicStatement::decode(&public).unwrap();
    assert!(!native_statement.passed);
    let mut proof =
        std::fs::read(root.join("proof-plonk.bin")).expect("real release PLONK proof required");
    assert_eq!(proof.len(), 964);
    let original = create.outputs().get(0).unwrap();
    let original_data = create.outputs_data().get(0).unwrap().raw_data();
    let proposal = Proposal::decode(&original_data).unwrap();
    let input_point = packed::OutPoint::new_builder()
        .tx_hash(create.hash())
        .build();
    assert_eq!(native_statement.proposal_outpoint, input_point.as_slice());
    let index = match mutation {
        Mutation::GenesisPublic => Some(8),
        Mutation::ProposalPublic => Some(40),
        Mutation::OutPointPublic => Some(72),
        Mutation::DataPublic => Some(108),
        Mutation::StartNumber => Some(204),
        Mutation::EndNumber => Some(212),
        Mutation::PublicNo => Some(228),
        Mutation::Passed => Some(276),
        _ => None,
    };
    if let Some(index) = index {
        public[index] ^= 1;
    }
    if matches!(mutation, Mutation::Proof) {
        proof[100] ^= 1;
    }
    let mut data = original_data.to_vec();
    data.extend_from_slice(&public);
    if matches!(mutation, Mutation::Prefix) {
        data[124] ^= 1;
    }
    if matches!(mutation, Mutation::ReceiptTrailing) {
        data.push(0);
    }
    let mut claim = hash(&public);
    if matches!(mutation, Mutation::Claim) {
        claim[31] ^= 1;
    }
    let mut entry = b"CSARGv1\0".to_vec();
    entry.extend_from_slice(&claim);
    entry.extend_from_slice(&[9; 20]);
    entry.extend_from_slice(&proposal.recipient);
    if matches!(mutation, Mutation::RecipientWitness) {
        entry[59] ^= 1;
    }
    let mut packet = proof;
    packet.extend_from_slice(&public);
    let witness = packed::WitnessArgs::new_builder()
        .input_type(Some(Bytes::from(entry)).pack())
        .output_type(
            if matches!(mutation, Mutation::MissingPacket) {
                None
            } else {
                Some(Bytes::from(packet))
            }
            .pack(),
        )
        .build();
    let success = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let refund = create.inputs().get(0).unwrap().previous_output();
    let refund_lock = context.get_cell(&refund).unwrap().0.lock();
    let other = context
        .build_script(&success, Bytes::from_static(b"substitution"))
        .unwrap();
    let mut receipt = original
        .clone()
        .as_builder()
        .capacity(
            original
                .occupied_capacity(Capacity::bytes(465).unwrap())
                .unwrap()
                .as_u64()
                + u64::from(matches!(mutation, Mutation::ReceiptCapacity)),
        )
        .lock(if matches!(mutation, Mutation::ReceiptLock) {
            other.clone()
        } else {
            original.lock()
        })
        .build();
    let receipt_capacity: u64 = receipt.capacity().unpack();
    let capacity: u64 = original.capacity().unpack();
    let mut paid = capacity - receipt_capacity - 10_000;
    if matches!(mutation, Mutation::PaymentCapacity) {
        paid = proposal.amount - 1;
    }
    let payment = packed::CellOutput::new_builder()
        .capacity(paid)
        .lock(if matches!(mutation, Mutation::PaymentLock) {
            other.clone()
        } else {
            refund_lock.clone()
        })
        .type_(
            if matches!(mutation, Mutation::PaymentType) {
                Some(other.clone())
            } else {
                None
            }
            .pack(),
        )
        .build();
    let (sp1,adapter)=(
        context.get_cell_by_data_hash(&packed::Byte32::new(hash(&funded::artifact("verifiers/sp1-plonk/target/riscv64imac-unknown-none-elf/release/agoraseal-sp1-plonk")))).unwrap(),
        context.get_cell_by_data_hash(&packed::Byte32::new(hash(&funded::artifact("verifiers/ckb-state-context/target/riscv64imac-unknown-none-elf/release/agoraseal-ckb-state-context")))).unwrap());
    let dep = |point| packed::CellDep::new_builder().out_point(point).build();
    let mut headers = vec![
        blocks[0].hash(),
        blocks[2].hash(),
        create.header_deps().get(0).unwrap(),
    ];
    if matches!(
        mutation,
        Mutation::StartHeader
            | Mutation::EndHeader
            | Mutation::GenesisHeader
            | Mutation::CreationAssociation
    ) {
        let position = match mutation {
            Mutation::EndHeader => 1,
            Mutation::GenesisHeader => 2,
            _ => 0,
        };
        let number = if position == 2 {
            0
        } else {
            100 + 2 * position as u64
        };
        let fake = HeaderBuilder::default()
            .number(number)
            .nonce(999u128)
            .epoch(if number == 0 {
                EpochNumberWithFraction::new_unchecked(0, 0, 0)
            } else {
                EpochNumberWithFraction::new(1, number, 1000)
            })
            .build();
        context.insert_header(fake.clone());
        if matches!(mutation, Mutation::CreationAssociation) {
            context.link_cell_with_block(input_point.clone(), fake.hash(), 0);
        } else {
            headers[position] = fake.hash();
        }
    }
    let mut input_point = input_point;
    if matches!(mutation, Mutation::ReceiptReplay) {
        let replay = packed::OutPoint::new_builder().tx_hash([88u8; 32]).build();
        context.create_cell_with_out_point(
            replay.clone(),
            receipt.clone(),
            Bytes::copy_from_slice(&data),
        );
        input_point = replay;
    }
    if matches!(mutation, Mutation::Burn) {
        receipt = receipt
            .as_builder()
            .type_(None::<packed::Script>.pack())
            .build();
    }
    let mut tx = TransactionBuilder::default()
        .input(
            packed::CellInput::new_builder()
                .previous_output(input_point.clone())
                .build(),
        )
        .output(receipt.clone())
        .output_data(Bytes::copy_from_slice(&data).pack())
        .output(payment)
        .output_data(
            if matches!(mutation, Mutation::PaymentData) {
                Bytes::from_static(&[1])
            } else {
                Bytes::new()
            }
            .pack(),
        )
        .cell_dep(dep(if matches!(mutation, Mutation::VerifierCode) {
            success.clone()
        } else {
            sp1
        }))
        .cell_dep(dep(if matches!(mutation, Mutation::ContextCode) {
            success
        } else {
            adapter
        }))
        .witness(witness.as_bytes().pack());
    if !matches!(mutation, Mutation::MissingHeaders) {
        tx = tx.set_header_deps(headers);
    }
    if matches!(mutation, Mutation::ExtraOutput) {
        tx = tx.output(receipt).output_data(Bytes::from(data).pack());
    }
    if matches!(mutation, Mutation::ExtraInput) {
        let (cell, data) = context.get_cell(&input_point).unwrap();
        let second = context.create_cell(cell, data);
        tx = tx.input(
            packed::CellInput::new_builder()
                .previous_output(second)
                .build(),
        );
    }
    (context.clone(), context.complete_tx(tx.build()))
}
#[test]
fn real_plonk_releases_failed_reserve_once_through_actual_cellscript() {
    let (context, tx) = fixture(Mutation::None);
    let cycles = context
        .verify_tx(&tx, 70_000_000)
        .expect("full real proof/Type/Lock/header path");
    println!(
        "real funded failed settlement cycles: {cycles}; transaction bytes: {}",
        tx.data().as_slice().len()
    );
}
#[test]
fn full_settlement_rejects_proof_context_payout_and_receipt_substitutions() {
    for mutation in [
        Mutation::Claim,
        Mutation::RecipientWitness,
        Mutation::GenesisPublic,
        Mutation::ProposalPublic,
        Mutation::OutPointPublic,
        Mutation::DataPublic,
        Mutation::StartNumber,
        Mutation::EndNumber,
        Mutation::PublicNo,
        Mutation::Passed,
        Mutation::Proof,
        Mutation::MissingPacket,
        Mutation::VerifierCode,
        Mutation::ContextCode,
        Mutation::StartHeader,
        Mutation::EndHeader,
        Mutation::GenesisHeader,
        Mutation::CreationAssociation,
        Mutation::MissingHeaders,
        Mutation::Prefix,
        Mutation::ReceiptTrailing,
        Mutation::ReceiptCapacity,
        Mutation::ReceiptLock,
        Mutation::PaymentLock,
        Mutation::PaymentCapacity,
        Mutation::PaymentData,
        Mutation::PaymentType,
        Mutation::ExtraOutput,
        Mutation::ExtraInput,
        Mutation::Burn,
        Mutation::ReceiptReplay,
    ] {
        let (context, tx) = fixture(mutation);
        let error = context
            .verify_tx(&tx, 140_000_000)
            .expect_err("lifecycle substitution rejected");
        let text = format!("{error:?}");
        let code = match mutation {
            Mutation::PublicNo
            | Mutation::Proof
            | Mutation::MissingPacket
            | Mutation::StartHeader
            | Mutation::EndHeader
            | Mutation::GenesisHeader
            | Mutation::CreationAssociation
            | Mutation::MissingHeaders => 1,
            Mutation::VerifierCode | Mutation::ContextCode => 41,
            Mutation::PaymentLock => 47,
            _ => 5,
        };
        assert!(
            text.contains(&format!("error code {code} on page")),
            "{mutation:?}: {text}"
        );
    }
}
