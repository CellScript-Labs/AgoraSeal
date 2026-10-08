//! Bounded canonical-history export. Native replay is never a proof authority.
use agoraseal_protocol::{GuestInput, Limits, Proposal, PublicStatement, Replay, hash};
use anyhow::{Context, Result, ensure};
use ckb_types::{packed, prelude::*};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    time::Duration,
};

fn rpc(
    client: &reqwest::blocking::Client,
    url: &str,
    method: &str,
    params: Vec<Value>,
) -> Result<Value> {
    let response = client
        .post(url)
        .json(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))
        .send()?
        .error_for_status()?;
    let mut bytes = Vec::new();
    response
        .take(32 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 32 * 1024 * 1024,
        "RPC response bound exceeded"
    );
    let value: Value = serde_json::from_slice(&bytes)?;
    ensure!(
        value.get("error").is_none(),
        "RPC {method}: {}",
        value["error"]
    );
    Ok(value["result"].clone())
}
fn number(value: &Value) -> Result<u64> {
    Ok(u64::from_str_radix(
        value
            .as_str()
            .context("hex number")?
            .trim_start_matches("0x"),
        16,
    )?)
}
fn hex32(value: &Value) -> Result<[u8; 32]> {
    hex::decode(value.as_str().context("hex hash")?.trim_start_matches("0x"))?
        .try_into()
        .map_err(|_| anyhow::anyhow!("hash length"))
}
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() == 4 && args[0] == "--header" {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        let height = args[2].parse::<u64>()?;
        let value = rpc(
            &client,
            &args[1],
            "get_header_by_number",
            vec![json!(format!("0x{height:x}"))],
        )?;
        let header: ckb_types::core::HeaderView =
            serde_json::from_value::<ckb_jsonrpc_types::HeaderView>(value)?.into();
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&args[3])?
            .write_all(header.data().as_slice())?;
        return Ok(());
    }
    ensure!(
        args.len() == 4,
        "usage: agoraseal-export RPC_URL PROPOSAL_TX_HASH OUTPUT_INDEX NEW_DIRECTORY"
    );
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    let point: packed::OutPoint = serde_json::from_value::<ckb_jsonrpc_types::OutPoint>(
        json!({"tx_hash":args[1],"index":format!("0x{:x}", args[2].parse::<u32>()?)}),
    )?
    .into();
    let cell = rpc(
        &client,
        &args[0],
        "get_live_cell",
        vec![
            serde_json::to_value(ckb_jsonrpc_types::OutPoint::from(point.clone()))?,
            json!(true),
        ],
    )?;
    ensure!(cell["status"] == "live", "proposal must still be live");
    let tx = rpc(&client, &args[0], "get_transaction", vec![json!(args[1])])?;
    ensure!(
        tx["tx_status"]["status"] == "committed",
        "proposal creation must be committed"
    );
    let create: packed::Transaction =
        serde_json::from_value::<ckb_jsonrpc_types::TransactionView>(tx["transaction"].clone())?
            .inner
            .into();
    let index = args[2].parse::<usize>()?;
    let output = create
        .raw()
        .outputs()
        .get(index)
        .context("proposal output")?;
    let proposal_type = output.type_().to_opt().context("proposal Type")?;
    let parent_hash = hash(&fs::read("target/cellscript/funded-proposal.elf")?);
    ensure!(
        proposal_type.code_hash().as_slice() == parent_hash
            && proposal_type.hash_type().as_slice() == [4]
            && proposal_type.args().raw_data().len() == 32,
        "proposal Type differs from the compiled fixed lifecycle"
    );
    let data = create
        .raw()
        .outputs_data()
        .get(index)
        .context("proposal data")?
        .raw_data();
    let proposal =
        Proposal::decode(&data).map_err(|error| anyhow::anyhow!("proposal codec: {error:?}"))?;
    ensure!((1..=16).contains(&proposal.duration), "funded window bound");
    let start = number(&tx["tx_status"]["block_number"])?;
    let end = start
        .checked_add(u64::from(proposal.duration))
        .context("window overflow")?;
    let genesis = rpc(
        &client,
        &args[0],
        "get_header_by_number",
        vec![json!("0x0")],
    )?;
    ensure!(hex32(&genesis["hash"])? == proposal.genesis, "wrong chain");
    let first = rpc(
        &client,
        &args[0],
        "get_header_by_number",
        vec![json!(format!("0x{start:x}"))],
    )?;
    ensure!(
        first["hash"] == tx["tx_status"]["block_hash"],
        "proposal creation became noncanonical"
    );
    let last = rpc(
        &client,
        &args[0],
        "get_header_by_number",
        vec![json!(format!("0x{end:x}"))],
    )?;
    ensure!(!last.is_null(), "voting window has not ended");
    let input = GuestInput {
        proposal_script: hash(proposal_type.as_slice()),
        start_hash: hex32(&first["hash"])?,
        end_hash: hex32(&last["hash"])?,
        blocks: u32::try_from(end - start + 1)?,
    };
    let directory = PathBuf::from(&args[3]);
    fs::create_dir(&directory).context("destination must be new; preserve old proof attempts")?;
    let mut replay = Replay::new(
        input.proposal_script,
        input.start_hash,
        input.end_hash,
        Limits::default(),
    );
    let mut anchors = Vec::new();
    for height in start..=end {
        let value = rpc(
            &client,
            &args[0],
            "get_block_by_number",
            vec![json!(format!("0x{height:x}"))],
        )?;
        let block: ckb_types::core::BlockView =
            serde_json::from_value::<ckb_jsonrpc_types::BlockView>(value)?.into();
        replay
            .push(block.data().as_slice())
            .map_err(|error| anyhow::anyhow!("authenticated replay: {error:?}"))?;
        anchors.push(block.hash());
        fs::write(
            directory.join(format!("block-{:06}.bin", height - start)),
            block.data().as_slice(),
        )?;
    }
    let statement = PublicStatement::from_tally(
        &replay
            .finish()
            .map_err(|error| anyhow::anyhow!("tally: {error:?}"))?,
    );
    ensure!(
        statement.proposal_outpoint == point.as_slice(),
        "replay selected a different proposal"
    );
    for (offset, expected) in anchors.iter().enumerate() {
        let now = rpc(
            &client,
            &args[0],
            "get_header_by_number",
            vec![json!(format!("0x{:x}", start + offset as u64))],
        )?;
        ensure!(
            hex32(&now["hash"])? == expected.as_slice(),
            "reorg during export; discard partial attempt"
        );
    }
    fs::write(directory.join("creation.bin"), create.as_slice())?;
    fs::write(directory.join("public-values.bin"), statement.encode())?;
    // Write input last; the prover separately replays and checks all frames.
    fs::write(directory.join("input.bin"), input.encode())?;
    println!(
        "exported {} canonical blocks; native tally passed={}; proof not implied",
        input.blocks, statement.passed
    );
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
