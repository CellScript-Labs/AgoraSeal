//! Executes the CellScript-generated treasury ELF, not a Rust policy substitute.
//! An always-success settlement Type isolates this component. It is NOT ZK evidence.
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use ckb_types::{
    bytes::Bytes,
    core::{ScriptHashType, TransactionBuilder, TransactionView},
    packed,
    prelude::*,
};

#[derive(Clone, Copy, Debug)]
enum Mutation {
    None,
    Recipient,
    Amount,
    Fee,
    Proposal,
    SettlementType,
    Quorum,
    Tie,
    NotPassed,
    Overflow,
    ExtraInput,
    ExtraOutput,
    PaymentData,
    PaymentType,
    EscrowTrailing,
    SettlementTrailing,
}

fn fixture(mutation: Mutation) -> (Context, TransactionView) {
    let elf = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/cellscript/treasury.elf"
    ))
    .expect("compile the actual CellScript source with scripts/gate.sh first");
    let mut context = Context::default();
    let code = context.deploy_cell(elf.into());
    let success = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let lock = context
        .build_script_with_hash_type(&code, ScriptHashType::Data2, Bytes::new())
        .unwrap();
    let recipient = context
        .build_script(&success, Bytes::from_static(b"recipient"))
        .unwrap();
    let other = context
        .build_script(&success, Bytes::from_static(b"other"))
        .unwrap();
    let settlement_type = context
        .build_script(&success, Bytes::from_static(b"test-settlement-only"))
        .unwrap();
    let amount = 100_000_000_000u64;
    let max_fee = 1000u64;
    let quorum = 80u64;
    let mut escrow = vec![1; 32];
    escrow.extend_from_slice(settlement_type.calc_script_hash().as_slice());
    escrow.extend_from_slice(recipient.calc_script_hash().as_slice());
    for value in [amount, quorum, max_fee] {
        escrow.extend_from_slice(&value.to_le_bytes());
    }
    if matches!(mutation, Mutation::EscrowTrailing) {
        escrow.push(0);
    }
    let reserve = context.create_cell(
        packed::CellOutput::new_builder()
            .capacity(amount + max_fee + u64::from(matches!(mutation, Mutation::Fee)))
            .lock(lock)
            .build(),
        escrow.into(),
    );
    let mut result = vec![
        if matches!(mutation, Mutation::Proposal) {
            2
        } else {
            1
        };
        32
    ];
    result.extend_from_slice(recipient.calc_script_hash().as_slice());
    let yes = if matches!(mutation, Mutation::Overflow) {
        u64::MAX
    } else if matches!(mutation, Mutation::Tie) {
        20
    } else if matches!(mutation, Mutation::Quorum) {
        59
    } else {
        60
    };
    for value in [
        amount,
        quorum,
        yes,
        20,
        u64::from(!matches!(mutation, Mutation::NotPassed)),
    ] {
        result.extend_from_slice(&value.to_le_bytes());
    }
    if matches!(mutation, Mutation::SettlementTrailing) {
        result.push(0);
    }
    let settlement = context.create_cell(
        packed::CellOutput::new_builder()
            .capacity(amount)
            .lock(recipient.clone())
            .type_(
                Some(if matches!(mutation, Mutation::SettlementType) {
                    other.clone()
                } else {
                    settlement_type
                })
                .pack(),
            )
            .build(),
        result.into(),
    );
    let payment = packed::CellOutput::new_builder()
        .capacity(amount - u64::from(matches!(mutation, Mutation::Amount)))
        .lock(if matches!(mutation, Mutation::Recipient) {
            other.clone()
        } else {
            recipient
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
    let witness = packed::WitnessArgs::new_builder().build();
    let mut tx = TransactionBuilder::default()
        .input(
            packed::CellInput::new_builder()
                .previous_output(reserve.clone())
                .build(),
        )
        .cell_dep(packed::CellDep::new_builder().out_point(settlement).build())
        .output(payment)
        .output_data(
            if matches!(mutation, Mutation::PaymentData) {
                Bytes::from_static(&[0])
            } else {
                Bytes::new()
            }
            .pack(),
        )
        .witness(witness.as_bytes().pack());
    if matches!(mutation, Mutation::ExtraInput) {
        // A second independently created escrow cannot share the same payout.
        let (cell, data) = context.get_cell(&reserve).unwrap();
        let second = context.create_cell(cell, data);
        tx = tx.input(
            packed::CellInput::new_builder()
                .previous_output(second)
                .build(),
        );
    }
    if matches!(mutation, Mutation::ExtraOutput) {
        tx = tx
            .output(
                packed::CellOutput::new_builder()
                    .capacity(1u64)
                    .lock(other)
                    .build(),
            )
            .output_data(Bytes::new().pack());
    }
    (context.clone(), context.complete_tx(tx.build()))
}

#[test]
fn cellscript_pays_exact_recipient_and_quorum_boundary() {
    let (context, tx) = fixture(Mutation::None);
    let cycles = context
        .verify_tx(&tx, 3_000_000)
        .expect("CellScript payout");
    println!("CellScript treasury payout cycles: {cycles}");
}

#[test]
fn cellscript_rejects_payout_substitutions() {
    for mutation in [
        Mutation::Recipient,
        Mutation::Amount,
        Mutation::Fee,
        Mutation::Proposal,
        Mutation::SettlementType,
        Mutation::Quorum,
        Mutation::Tie,
        Mutation::NotPassed,
        Mutation::Overflow,
        Mutation::ExtraInput,
        Mutation::ExtraOutput,
        Mutation::PaymentData,
        Mutation::PaymentType,
        Mutation::EscrowTrailing,
        Mutation::SettlementTrailing,
    ] {
        let (context, tx) = fixture(mutation);
        let error = context
            .verify_tx(&tx, 3_000_000)
            .expect_err("mutation must reject");
        let expected = match mutation {
            Mutation::Recipient => 47,
            Mutation::SettlementType => 17,
            _ => 5,
        };
        assert!(
            format!("{error:?}").contains(&format!("error code {expected} on page")),
            "{mutation:?}: {error:?}"
        );
    }
}
