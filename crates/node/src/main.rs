//! Full disposable-node rehearsal. Policy execution remains in .cell/guest.
//! All signing uses an explicitly public fixture key on an isolated local chain.
use agoraseal_node::{
    devnet::{self, CkbDevnet},
    proof::prove,
    wallet,
};
use agoraseal_protocol::{
    Ballot, Choice, GuestInput, Limits, Proposal, PublicStatement, Replay, hash,
};
use anyhow::{Context, Result, ensure};
use ckb_types::{
    bytes::Bytes,
    core::{Capacity, ScriptHashType, TransactionBuilder, TransactionView},
    packed,
    prelude::*,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const CKB_PIN: &str = "f7fa4436737756f97a24e254f22c13a36316ecea";
const SH: u64 = 100_000_000;
const FEE: u64 = 100_000;
fn sha(path: &Path) -> Result<String> {
    Ok(hex::encode(Sha256::digest(fs::read(path)?)))
}
fn persist_exact(path: &Path, bytes: &[u8]) -> Result<()> {
    if path.exists() {
        ensure!(
            fs::read(path)? == bytes,
            "saved artifact differs from current canonical state: {}",
            path.display()
        );
    } else {
        fs::write(path, bytes)?;
    }
    Ok(())
}
fn chain_transactions(node: &CkbDevnet) -> Result<Vec<TransactionView>> {
    let tip = node.rpc("get_tip_header", vec![])?;
    let tip = u64::from_str_radix(
        tip["number"]
            .as_str()
            .context("tip")?
            .trim_start_matches("0x"),
        16,
    )?;
    ensure!(tip <= 10_000, "disposable recovery scan bound");
    let mut rows = Vec::new();
    for number in 0..=tip {
        let block = node.get_block_by_number(number)?;
        for tx in block["transactions"]
            .as_array()
            .context("block transactions")?
        {
            let packed: packed::Transaction =
                serde_json::from_value::<ckb_jsonrpc_types::TransactionView>(tx.clone())?
                    .inner
                    .into();
            rows.push(packed.into_view());
        }
    }
    Ok(rows)
}
fn recover_deployment(
    node: &CkbDevnet,
    transactions: &[TransactionView],
    name: &str,
    bytes: &[u8],
) -> Result<Value> {
    let expected = hash(bytes);
    for tx in transactions {
        for (index, data) in tx.outputs_data().into_iter().enumerate() {
            if hash(&data.raw_data()) == expected {
                let point = tx_point(tx, u32::try_from(index)?);
                ensure!(
                    node.rpc(
                        "get_live_cell",
                        vec![
                            serde_json::to_value(ckb_jsonrpc_types::OutPoint::from(point.clone()))?,
                            json!(true)
                        ]
                    )?["status"]
                        == "live",
                    "saved code Cell is spent"
                );
                return Ok(
                    json!({"name":name,"artifact_size_bytes":bytes.len(),"data_hash":devnet::hex0x(&expected),
                    "cell_dep":{"out_point":serde_json::to_value(ckb_jsonrpc_types::OutPoint::from(point))?,"dep_type":"code"},
                    "recovered_from_canonical_chain":true}),
                );
            }
        }
    }
    anyhow::bail!("saved deployment not found: {name}")
}
fn hex32(value: &Value) -> Result<[u8; 32]> {
    hex::decode(
        value
            .as_str()
            .context("expected hex string")?
            .trim_start_matches("0x"),
    )?
    .try_into()
    .map_err(|_| anyhow::anyhow!("expected 32 bytes"))
}
fn point(value: &Value) -> Result<packed::OutPoint> {
    Ok(serde_json::from_value::<ckb_jsonrpc_types::OutPoint>(value.clone())?.into())
}
fn dep(point: packed::OutPoint) -> packed::CellDep {
    packed::CellDep::new_builder().out_point(point).build()
}
fn input(point: packed::OutPoint) -> packed::CellInput {
    packed::CellInput::new_builder()
        .previous_output(point)
        .build()
}
fn tx_json(tx: &TransactionView) -> Result<Value> {
    Ok(serde_json::to_value(ckb_jsonrpc_types::Transaction::from(
        tx.data(),
    ))?)
}
fn tx_point(tx: &TransactionView, index: u32) -> packed::OutPoint {
    packed::OutPoint::new_builder()
        .tx_hash(tx.hash())
        .index(index)
        .build()
}
fn entry(statement: &[u8; 32], recipient: &[u8; 20], recipient_hash: &[u8; 32]) -> Bytes {
    let mut bytes = b"CSARGv1\0".to_vec();
    bytes.extend_from_slice(statement);
    bytes.extend_from_slice(recipient);
    bytes.extend_from_slice(recipient_hash);
    bytes.into()
}
fn witness(entry: Bytes) -> packed::Bytes {
    packed::WitnessArgs::new_builder()
        .input_type(Some(entry).pack())
        .build()
        .as_bytes()
        .pack()
}
fn status_number(status: &Value) -> Result<u64> {
    Ok(u64::from_str_radix(
        status["status"]["block_number"]
            .as_str()
            .context("committed block number")?
            .trim_start_matches("0x"),
        16,
    )?)
}
fn already_committed(node: &CkbDevnet, tx: &TransactionView) -> Result<bool> {
    Ok(node.rpc(
        "get_transaction",
        vec![json!(devnet::hex0x(tx.hash().as_slice()))],
    )?["tx_status"]["status"]
        == "committed")
}
fn commit(
    node: &CkbDevnet,
    tx: &TransactionView,
    label: &str,
    rows: &mut Vec<Value>,
) -> Result<u64> {
    let rpc = tx_json(tx)?;
    let known = node.rpc(
        "get_transaction",
        vec![json!(devnet::hex0x(tx.hash().as_slice()))],
    )?;
    let (dry, committed) = if known["tx_status"]["status"] == "committed" {
        let saved: packed::Transaction = serde_json::from_value::<
            ckb_jsonrpc_types::TransactionView,
        >(known["transaction"].clone())?
        .inner
        .into();
        ensure!(
            saved.as_slice() == tx.data().as_slice(),
            "committed transaction/witness differs from prepared recovery artifact"
        );
        let number = known["tx_status"]["block_number"]
            .as_str()
            .context("recovered block number")?;
        let number = u64::from_str_radix(number.trim_start_matches("0x"), 16)?;
        ensure!(
            node.get_block_by_number(number)?["header"]["hash"] == known["tx_status"]["block_hash"],
            "recovered transaction is noncanonical"
        );
        (
            json!({"recovered_committed":true}),
            json!({"tx_hash":devnet::hex0x(tx.hash().as_slice()),"status":known["tx_status"]}),
        )
    } else {
        (node.dry_run(&rpc)?, node.submit_and_commit(&rpc, label)?)
    };
    let number = status_number(&committed)?;
    rows.push(json!({"case":label,"dry_run":dry,"commit":committed,"tx_bytes":tx.data().as_slice().len()}));
    fs::write(
        node.ckb_dir
            .parent()
            .context("run directory")?
            .join("journal.json"),
        serde_json::to_vec_pretty(rows)?,
    )?;
    println!("{label}: committed at {number}");
    Ok(number)
}
struct Codes {
    points: Vec<packed::OutPoint>,
    hashes: Vec<[u8; 32]>,
    secp: packed::CellDep,
    dao: packed::CellDep,
    genesis: [u8; 32],
    lock: packed::Script,
}
impl Codes {
    fn settlement_deps(&self) -> Vec<packed::CellDep> {
        // SP1 0, state adapter 1; proposal and funding Lock follow.
        [0usize, 1, 4, 5]
            .iter()
            .map(|i| dep(self.points[*i].clone()))
            .collect()
    }
}
fn prepare_proposal(
    node: &CkbDevnet,
    codes: &Codes,
    wallet_point: packed::OutPoint,
    wallet_capacity: u64,
    rows: &mut Vec<Value>,
    label: &str,
) -> Result<(TransactionView, packed::OutPoint, u64, u64)> {
    let first = input(wallet_point);
    let mut id = first.as_slice().to_vec();
    id.extend_from_slice(&0u64.to_le_bytes());
    let ty = packed::Script::new_builder()
        .code_hash(codes.hashes[4])
        .hash_type(ScriptHashType::Data2)
        .args(Bytes::copy_from_slice(&hash(&id)).pack())
        .build();
    let lock = packed::Script::new_builder()
        .code_hash(codes.hashes[5])
        .hash_type(ScriptHashType::Data2)
        .args(codes.lock.calc_script_hash().as_bytes().pack())
        .build();
    let data = Proposal {
        genesis: codes.genesis,
        vote_code: codes.hashes[3],
        dao_script: hash(
            packed::Script::new_builder()
                .code_hash([
                    0x82, 0xd7, 0x6d, 0x1b, 0x75, 0xfe, 0x2f, 0xd9, 0xa2, 0x7d, 0xfb, 0xaa, 0x65,
                    0xa0, 0x39, 0x22, 0x1a, 0x38, 0x0d, 0x76, 0xc9, 0x26, 0xf3, 0x78, 0xd3, 0xf8,
                    0x1c, 0xf3, 0xe7, 0xe1, 0x3f, 0x2e,
                ])
                .hash_type(1u8)
                .build()
                .as_slice(),
        ),
        duration: 12,
        quorum: 500 * SH,
        amount: 100 * SH,
        recipient: hash(codes.lock.as_slice()),
        description: hash(label.as_bytes()),
    }
    .encode();
    let output = packed::CellOutput::new_builder()
        .lock(lock)
        .type_(Some(ty).pack())
        .build();
    let reserved = output.occupied_capacity(Capacity::bytes(465)?)?.as_u64() + 100 * SH + FEE;
    let remaining = wallet_capacity
        .checked_sub(reserved + FEE)
        .context("funding capacity")?;
    let mut deps = codes.settlement_deps();
    deps.push(codes.secp.clone());
    let tx = TransactionBuilder::default()
        .input(first)
        .output(output.as_builder().capacity(reserved).build())
        .output_data(Bytes::copy_from_slice(&data).pack())
        .output(
            packed::CellOutput::new_builder()
                .capacity(remaining)
                .lock(codes.lock.clone())
                .build(),
        )
        .output_data(Bytes::new().pack())
        .set_cell_deps(deps)
        .header_dep(packed::Byte32::from_slice(&codes.genesis)?)
        .witness(witness(entry(
            &[0; 32],
            &wallet::fixture_args(),
            &hash(codes.lock.as_slice()),
        )))
        .build();
    let tx = wallet::sign_fixture_group(&tx, &[0])?;
    let bad = tx
        .as_advanced_builder()
        .set_witnesses(vec![witness(entry(
            &[0; 32],
            &[0; 20],
            &hash(codes.lock.as_slice()),
        ))])
        .build();
    if !already_committed(node, &tx)? {
        rows.push(node.dry_run_rejects(
            &tx_json(&bad)?,
            "creation missing real signature",
            Some("Inputs[0].Lock"),
            None,
            None,
        )?);
    }
    let height = commit(node, &tx, label, rows)?;
    Ok((tx.clone(), tx_point(&tx, 1), remaining, height))
}

fn export_history(
    node: &CkbDevnet,
    create: &TransactionView,
    height: u64,
    dir: &Path,
) -> Result<PublicStatement> {
    if dir.exists() {
        ensure!(dir.is_dir(), "saved fixture is not a directory");
    } else {
        fs::create_dir(dir)?;
    }
    let end = height + 12;
    while u64::from_str_radix(
        node.rpc("get_tip_header", vec![])?["number"]
            .as_str()
            .context("tip")?
            .trim_start_matches("0x"),
        16,
    )? < end
    {
        node.rpc("generate_block", vec![])?;
    }
    let mut blocks = Vec::new();
    for number in height..=end {
        let block: ckb_types::core::BlockView = serde_json::from_value::<
            ckb_jsonrpc_types::BlockView,
        >(node.get_block_by_number(number)?)?
        .into();
        persist_exact(
            &dir.join(format!("block-{:06}.bin", number - height)),
            block.data().as_slice(),
        )?;
        blocks.push(block);
    }
    let guest = GuestInput {
        proposal_script: hash(
            create
                .outputs()
                .get(0)
                .context("proposal")?
                .type_()
                .to_opt()
                .context("proposal type")?
                .as_slice(),
        ),
        start_hash: blocks[0].hash().as_slice().try_into()?,
        end_hash: blocks.last().context("end")?.hash().as_slice().try_into()?,
        blocks: u32::try_from(blocks.len())?,
    };
    let mut replay = Replay::new(
        guest.proposal_script,
        guest.start_hash,
        guest.end_hash,
        Limits::default(),
    );
    for block in blocks {
        replay
            .push(block.data().as_slice())
            .map_err(|error| anyhow::anyhow!("node replay: {error:?}"))?;
    }
    let statement = PublicStatement::from_tally(
        &replay
            .finish()
            .map_err(|error| anyhow::anyhow!("node tally: {error:?}"))?,
    );
    persist_exact(&dir.join("input.bin"), &guest.encode())?;
    persist_exact(&dir.join("public-values.bin"), &statement.encode())?;
    persist_exact(&dir.join("creation.bin"), create.data().as_slice())?;
    Ok(statement)
}
fn settle(
    node: &CkbDevnet,
    codes: &Codes,
    create: &TransactionView,
    statement: &PublicStatement,
    dir: &Path,
    rows: &mut Vec<Value>,
) -> Result<TransactionView> {
    let original = create.outputs().get(0).context("proposal output")?;
    let proposal = Proposal::decode(
        &create
            .outputs_data()
            .get(0)
            .context("proposal data")?
            .raw_data(),
    )
    .map_err(|error| anyhow::anyhow!("{error:?}"))?;
    let mut data = proposal.encode().to_vec();
    data.extend_from_slice(&statement.encode());
    let proof = fs::read(dir.join("proof-plonk.bin"))?;
    ensure!(proof.len() == 964, "proof length");
    let mut packet = proof;
    packet.extend_from_slice(&statement.encode());
    let wit = packed::WitnessArgs::new_builder()
        .input_type(
            Some(entry(
                &hash(&statement.encode()),
                &wallet::fixture_args(),
                &proposal.recipient,
            ))
            .pack(),
        )
        .output_type(Some(Bytes::from(packet)).pack())
        .build();
    let kept = original
        .occupied_capacity(Capacity::bytes(data.len())?)?
        .as_u64();
    let capacity: u64 = original.capacity().unpack();
    let payment = if statement.passed {
        proposal.amount
    } else {
        capacity - kept - 10_000
    };
    let tx = TransactionBuilder::default()
        .input(input(tx_point(create, 0)))
        .output(original.as_builder().capacity(kept).build())
        .output_data(Bytes::from(data).pack())
        .output(
            packed::CellOutput::new_builder()
                .capacity(payment)
                .lock(codes.lock.clone())
                .build(),
        )
        .output_data(Bytes::new().pack())
        .set_cell_deps(codes.settlement_deps())
        .header_dep(packed::Byte32::from_slice(&statement.start_hash)?)
        .header_dep(packed::Byte32::from_slice(&statement.end_hash)?)
        .header_dep(packed::Byte32::from_slice(&codes.genesis)?)
        .witness(wit.as_bytes().pack())
        .build();
    let mut corrupted = wit
        .output_type()
        .to_opt()
        .context("packet")?
        .raw_data()
        .to_vec();
    corrupted[100] ^= 1;
    let bad_wit = wit
        .as_builder()
        .output_type(Some(Bytes::from(corrupted)).pack())
        .build();
    let bad = tx
        .as_advanced_builder()
        .set_witnesses(vec![bad_wit.as_bytes().pack()])
        .build();
    if !already_committed(node, &tx)? {
        rows.push(node.dry_run_rejects(
            &tx_json(&bad)?,
            "real proof substitution",
            None,
            None,
            None,
        )?);
    }
    // A competing, independently valid transaction may be prepared before spending.
    let concurrent = tx
        .as_advanced_builder()
        .set_outputs(vec![
            tx.outputs().get(0).unwrap(),
            tx.outputs()
                .get(1)
                .unwrap()
                .as_builder()
                .capacity(payment - 1)
                .build(),
        ])
        .build();
    // Passing amount is exact, so only the failed refund has a second valid fee.
    if !statement.passed && !already_committed(node, &tx)? {
        node.dry_run(&tx_json(&concurrent)?)?;
    }
    commit(
        node,
        &tx,
        if statement.passed {
            "passed settlement"
        } else {
            "failed refund"
        },
        rows,
    )?;
    ensure!(
        node.rpc(
            "get_live_cell",
            vec![
                serde_json::to_value(ckb_jsonrpc_types::OutPoint::from(tx_point(create, 0)))?,
                json!(false)
            ]
        )?["status"]
            != "live",
        "proposal must be spent"
    );
    let retry = tx
        .as_advanced_builder()
        .set_outputs(vec![
            tx.outputs().get(0).unwrap(),
            tx.outputs()
                .get(1)
                .unwrap()
                .as_builder()
                .capacity(payment - 1)
                .build(),
        ])
        .build();
    ensure!(
        node.rpc(
            "send_transaction",
            vec![tx_json(&retry)?, json!("passthrough")]
        )
        .is_err(),
        "spent proposal replay accepted"
    );
    rows.push(json!({"case":"spent proposal replay","rejected":true}));
    fs::write(dir.join("settlement.bin"), tx.data().as_slice())?;
    Ok(tx)
}

fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 3 || (args.len() == 4 && args[3] == "--resume"),
        "usage: agoraseal-node CKB_REPO CKB_BINARY RUN_DIRECTORY [--resume]"
    );
    let resume = args.len() == 4;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let repo = PathBuf::from(&args[0]).canonicalize()?;
    let bin = PathBuf::from(&args[1]).canonicalize()?;
    let git = Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["rev-parse", "HEAD"])
        .output()?;
    ensure!(
        git.status.success() && String::from_utf8(git.stdout)?.trim() == CKB_PIN,
        "CKB source pin mismatch"
    );
    let dirty = Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["status", "--porcelain", "--untracked-files=no"])
        .output()?;
    ensure!(
        dirty.status.success() && dirty.stdout.is_empty(),
        "CKB source is dirty"
    );
    let version = Command::new(&bin).arg("--version").output()?;
    let version = String::from_utf8(version.stdout)?;
    ensure!(
        version.contains("0.207.0")
            && version.contains(&CKB_PIN[..7])
            && !version.contains("dirty"),
        "CKB binary identity mismatch: {version}"
    );
    let run_dir = PathBuf::from(&args[2]);
    if !resume {
        fs::create_dir(&run_dir)?;
    }
    let run_dir = run_dir.canonicalize()?;
    let mut node = if resume {
        CkbDevnet::reopen(repo, bin.clone(), run_dir.clone())?
    } else {
        CkbDevnet::new(repo, bin.clone(), run_dir.clone())?
    };
    node.start()?;
    let genesis = node.get_block_by_number(0)?;
    let genesis_tx = genesis["transactions"][0]["hash"]
        .as_str()
        .context("genesis tx")?;
    let always = devnet::always_success_dep(genesis_tx);
    let recovered = if resume {
        chain_transactions(&node)?
    } else {
        Vec::new()
    };
    let paths = [
        "verifiers/sp1-plonk/target/riscv64imac-unknown-none-elf/release/agoraseal-sp1-plonk",
        "verifiers/ckb-state-context/target/riscv64imac-unknown-none-elf/release/agoraseal-ckb-state-context",
        "verifiers/ckb-context/target/riscv64imac-unknown-none-elf/release/agoraseal-ckb-context",
        "target/cellscript/vote.elf",
        "target/cellscript/funded-proposal.elf",
        "target/cellscript/funded-treasury.elf",
    ];
    let mut deployments = Vec::new();
    for path in paths {
        let bytes = fs::read(root.join(path))?;
        deployments.push(if resume {
            recover_deployment(&node, &recovered, path, &bytes)?
        } else {
            devnet::deploy_code(&mut node, path, &bytes, &always)?
        });
    }
    let secp_code: packed::Script = serde_json::from_value::<ckb_jsonrpc_types::Script>(
        genesis["transactions"][0]["outputs"][1]["type"].clone(),
    )?
    .into();
    let dao_code: packed::Script = serde_json::from_value::<ckb_jsonrpc_types::Script>(
        genesis["transactions"][0]["outputs"][2]["type"].clone(),
    )?
    .into();
    let lock = packed::Script::new_builder()
        .code_hash(secp_code.calc_script_hash())
        .hash_type(1u8)
        .args(Bytes::copy_from_slice(&wallet::fixture_args()).pack())
        .build();
    let codes = Codes {
        points: deployments
            .iter()
            .map(|d| point(&d["cell_dep"]["out_point"]))
            .collect::<Result<_>>()?,
        hashes: deployments
            .iter()
            .map(|d| hex32(&d["data_hash"]))
            .collect::<Result<_>>()?,
        secp: packed::CellDep::new_builder()
            .out_point(point(&devnet::out_point(
                genesis["transactions"][1]["hash"]
                    .as_str()
                    .context("depgroup")?,
                0,
            ))?)
            .dep_type(1u8)
            .build(),
        dao: dep(point(&devnet::out_point(genesis_tx, 2))?),
        genesis: hex32(&genesis["header"]["hash"])?,
        lock: lock.clone(),
    };
    fs::write(
        run_dir.join("deployments.json"),
        serde_json::to_vec_pretty(&deployments)?,
    )?;
    let bootstrap = if resume {
        recovered
            .iter()
            .find(|tx| {
                tx.outputs().get(0).is_some_and(|output| {
                    let capacity: u64 = output.capacity().unpack();
                    capacity >= 10_000 * SH
                        && output.lock() == lock
                        && output.type_().is_none()
                        && tx
                            .outputs_data()
                            .get(0)
                            .is_some_and(|data| data.raw_data().is_empty())
                })
            })
            .context("original public fixture wallet bootstrap")?
            .clone()
    } else {
        let funding = node.collect_spendable(10_000 * SH)?;
        let capacity = funding["total_capacity"].as_u64().context("funding")? - FEE;
        TransactionBuilder::default()
            .set_inputs(
                devnet::funding_cells(&funding)
                    .iter()
                    .map(|cell| {
                        Ok(input(point(&devnet::out_point(
                            cell["tx_hash"].as_str().context("funding hash")?,
                            cell["index"].as_u64().context("index")?,
                        ))?))
                    })
                    .collect::<Result<_>>()?,
            )
            .output(
                packed::CellOutput::new_builder()
                    .capacity(capacity)
                    .lock(lock.clone())
                    .build(),
            )
            .output_data(Bytes::new().pack())
            .cell_dep(dep(point(&always["out_point"])?))
            .build()
    };
    let capacity: u64 = bootstrap
        .outputs()
        .get(0)
        .context("bootstrap output")?
        .capacity()
        .unpack();
    let mut rows = Vec::new();
    commit(
        &node,
        &bootstrap,
        "bootstrap public fixture wallet",
        &mut rows,
    )?;
    let dao_type = packed::Script::new_builder()
        .code_hash(dao_code.calc_script_hash())
        .hash_type(1u8)
        .build();
    let deposit_tx = TransactionBuilder::default()
        .input(input(tx_point(&bootstrap, 0)))
        .output(
            packed::CellOutput::new_builder()
                .capacity(1000 * SH)
                .lock(lock.clone())
                .type_(Some(dao_type).pack())
                .build(),
        )
        .output_data(Bytes::from(vec![0; 8]).pack())
        .output(
            packed::CellOutput::new_builder()
                .capacity(capacity - 1000 * SH - FEE)
                .lock(lock.clone())
                .build(),
        )
        .output_data(Bytes::new().pack())
        .cell_dep(codes.dao.clone())
        .cell_dep(codes.secp.clone())
        .build();
    let deposit_tx = wallet::sign_fixture_group(&deposit_tx, &[0])?;
    let deposit_height = commit(&node, &deposit_tx, "real signed DAO deposit", &mut rows)?;
    let mut wallet_capacity = capacity - 1000 * SH - FEE;
    let mut wallet_point = tx_point(&deposit_tx, 1);
    for passing in [true, false] {
        let label = if passing {
            "create passing proposal"
        } else {
            "create failing proposal"
        };
        let (create, next, remaining, start) = prepare_proposal(
            &node,
            &codes,
            wallet_point,
            wallet_capacity,
            &mut rows,
            label,
        )?;
        wallet_point = next;
        wallet_capacity = remaining;
        if passing {
            let proposal = create.outputs().get(0).unwrap().type_().to_opt().unwrap();
            let ty = packed::Script::new_builder()
                .code_hash(codes.hashes[3])
                .hash_type(4u8)
                .args(proposal.calc_script_hash().as_bytes().pack())
                .build();
            let ballot = Ballot {
                deposit: tx_point(&deposit_tx, 0).as_slice().try_into()?,
                weight: 1000 * SH,
                choice: Choice::Yes,
            };
            let mut abi = b"CSARGv1\0".to_vec();
            abi.extend_from_slice(&deposit_height.to_le_bytes());
            abi.extend_from_slice(&start.to_le_bytes());
            let header_d = node.get_block_by_number(deposit_height)?;
            let header_s = node.get_block_by_number(start)?;
            let vote = TransactionBuilder::default()
                .input(input(wallet_point))
                .output(
                    packed::CellOutput::new_builder()
                        .capacity(200 * SH)
                        .lock(lock.clone())
                        .type_(Some(ty).pack())
                        .build(),
                )
                .output_data(Bytes::copy_from_slice(&ballot.encode()).pack())
                .output(
                    packed::CellOutput::new_builder()
                        .capacity(wallet_capacity - 200 * SH - FEE)
                        .lock(lock.clone())
                        .build(),
                )
                .output_data(Bytes::new().pack())
                .cell_dep(dep(tx_point(&deposit_tx, 0)))
                .cell_dep(dep(tx_point(&create, 0)))
                .cell_dep(dep(codes.points[2].clone()))
                .cell_dep(dep(codes.points[3].clone()))
                .cell_dep(codes.secp.clone())
                .header_dep(packed::Byte32::from_slice(&hex32(
                    &header_d["header"]["hash"],
                )?)?)
                .header_dep(packed::Byte32::from_slice(&hex32(
                    &header_s["header"]["hash"],
                )?)?)
                .witness(witness(abi.into()))
                .build();
            let vote = wallet::sign_fixture_group(&vote, &[0])?;
            ensure!(
                commit(&node, &vote, "real signed eligible vote", &mut rows)? <= start + 12,
                "vote missed proof window"
            );
            wallet_point = tx_point(&vote, 1);
            wallet_capacity -= 200 * SH + FEE;
        }
        let dir = run_dir.join(if passing { "passed" } else { "failed" });
        let statement = export_history(&node, &create, start, &dir)?;
        ensure!(
            statement.passed == passing,
            "unexpected authoritative result"
        );
        prove(&root, &dir)?;
        let tx = settle(&node, &codes, &create, &statement, &dir, &mut rows)?;
        let amount: u64 = tx.outputs().get(1).unwrap().capacity().unpack();
        let spend = TransactionBuilder::default()
            .input(input(tx_point(&tx, 1)))
            .output(
                packed::CellOutput::new_builder()
                    .capacity(amount - FEE)
                    .lock(lock.clone())
                    .build(),
            )
            .output_data(Bytes::new().pack())
            .cell_dep(codes.secp.clone())
            .build();
        commit(
            &node,
            &wallet::sign_fixture_group(&spend, &[0])?,
            "real signed payout/refund spend",
            &mut rows,
        )?;
    }
    node.stop();
    let report = json!({"schema":"agoraseal-node-rehearsal-v1","status":"passed","production_admitted":false,
        "ckb_revision":CKB_PIN,"ckb_version":version.trim(),"ckb_binary_sha256":sha(&bin)?,
        "genesis_hash":genesis["header"]["hash"],"admission":"normal send_transaction; disposable Dummy-PoW chain",
        "signatures":"real standard secp256k1 signatures with public fixture scalar 0x45 repeated; no user key imported",
        "rows":rows,"deployments":deployments});
    fs::write(
        run_dir.join("evidence.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!(
        "full disposable-node rehearsal passed: {}",
        run_dir.display()
    );
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
