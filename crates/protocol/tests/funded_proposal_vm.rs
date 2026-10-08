#[path = "support/funded.rs"]
mod funded;
use funded::{CreationMutation, creation};

#[test]
fn synthetic_histories_execute_actual_creation_and_voting_policies() {
    use agoraseal_protocol::{Limits, Replay, hash};
    use ckb_types::prelude::*;
    for passing in [false, true] {
        let (_, creation, blocks) = funded::history(passing);
        let script = creation.outputs().get(0).unwrap().type_().to_opt().unwrap();
        let mut replay = Replay::new(
            hash(script.as_slice()),
            blocks[0].hash().as_slice().try_into().unwrap(),
            blocks[2].hash().as_slice().try_into().unwrap(),
            Limits::default(),
        );
        for block in blocks {
            replay.push(block.data().as_slice()).unwrap();
        }
        assert_eq!(replay.finish().unwrap().passed, passing);
    }
}

#[test]
fn cellscript_creates_funded_unique_proposal_with_actual_header_adapter() {
    let (context, tx) = creation(CreationMutation::None);
    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("actual CellScript creation");
    println!("funded proposal creation cycles: {cycles}");
}

#[test]
fn creation_rejects_immutable_policy_and_coordinate_substitutions() {
    for mutation in [
        CreationMutation::Genesis,
        CreationMutation::GenesisNumber,
        CreationMutation::MissingHeader,
        CreationMutation::Vote,
        CreationMutation::Dao,
        CreationMutation::DurationZero,
        CreationMutation::DurationBound,
        CreationMutation::Quorum,
        CreationMutation::Amount,
        CreationMutation::Recipient,
        CreationMutation::Refund,
        CreationMutation::TreasuryCode,
        CreationMutation::TreasuryHashType,
        CreationMutation::ShortArgs,
        CreationMutation::CapacityLow,
        CreationMutation::CapacityHigh,
        CreationMutation::FirstInputData,
        CreationMutation::FirstInputType,
        CreationMutation::WrongId,
        CreationMutation::Duplicate,
        CreationMutation::ProposalTrailing,
        CreationMutation::ContextCode,
        CreationMutation::WitnessShort,
        CreationMutation::WitnessTrailing,
    ] {
        let (context, tx) = creation(mutation);
        let error = context
            .verify_tx(&tx, 10_000_000)
            .expect_err("creation mutation must reject");
        let text = format!("{error:?}");
        let code = match mutation {
            CreationMutation::Refund => 47,
            CreationMutation::ShortArgs => 38,
            CreationMutation::Genesis
            | CreationMutation::GenesisNumber
            | CreationMutation::MissingHeader
            | CreationMutation::WrongId => 1,
            CreationMutation::ContextCode => 41,
            CreationMutation::WitnessShort | CreationMutation::WitnessTrailing => 25,
            _ => 5,
        };
        assert!(
            text.contains(&format!("error code {code} on page")),
            "{mutation:?}: {text}"
        );
    }
}
