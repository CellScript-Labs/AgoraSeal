//! Explicit synthetic VM/guest fixture export. No node consensus or signature claim.
#[path = "../tests/support/funded.rs"]
#[allow(dead_code)]
mod funded;
use agoraseal_protocol::{GuestInput, Limits, PublicStatement, Replay, hash};
use ckb_types::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 || !matches!(args[0].as_str(), "passed" | "failed") {
        return Err("usage: funded_fixture passed|failed NEW_DIRECTORY".into());
    }
    let root = std::path::Path::new(&args[1]);
    // Do not overwrite proof or evidence artifacts from an earlier run.
    std::fs::create_dir(root)?;
    let passing = args[0] == "passed";
    let (context, creation, blocks) = funded::history(passing);
    let proposal = creation.outputs().get(0).unwrap().type_().to_opt().unwrap();
    let input = GuestInput {
        proposal_script: hash(proposal.as_slice()),
        start_hash: blocks[0].hash().as_slice().try_into()?,
        end_hash: blocks.last().unwrap().hash().as_slice().try_into()?,
        blocks: u32::try_from(blocks.len())?,
    };
    let mut replay = Replay::new(
        input.proposal_script,
        input.start_hash,
        input.end_hash,
        Limits::default(),
    );
    for (index, block) in blocks.iter().enumerate() {
        replay
            .push(block.data().as_slice())
            .map_err(|error| format!("replay: {error:?}"))?;
        std::fs::write(
            root.join(format!("block-{index:06}.bin")),
            block.data().as_slice(),
        )?;
    }
    let tally = replay
        .finish()
        .map_err(|error| format!("finish: {error:?}"))?;
    assert_eq!(tally.passed, passing);
    std::fs::write(root.join("input.bin"), input.encode())?;
    std::fs::write(
        root.join("public-values.bin"),
        PublicStatement::from_tally(&tally).encode(),
    )?;
    std::fs::write(root.join("creation.bin"), creation.data().as_slice())?;
    let funding = context
        .get_cell(&creation.inputs().get(0).unwrap().previous_output())
        .ok_or("fixture funding Cell")?;
    std::fs::write(root.join("refund-script.bin"), funding.0.lock().as_slice())?;
    println!(
        "synthetic {}/{} fixture: real proposal/vote VM policies executed; not node evidence",
        tally.yes, tally.no
    );
    Ok(())
}
