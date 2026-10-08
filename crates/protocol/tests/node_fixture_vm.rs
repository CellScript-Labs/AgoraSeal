//! Re-execute node-produced public golden settlements in CKB-VM.
//! This regression does not rerun node consensus or the original signatures.
use agoraseal_protocol::{PublicStatement, hash};
use ckb_testtool::context::Context;
use ckb_types::{
    bytes::Bytes,
    core::{Capacity, TransactionView},
    packed,
    prelude::*,
};
use std::path::Path;

fn read(path: &Path) -> Bytes {
    std::fs::read(path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
        .into()
}
fn fixture(name: &str) -> (Context, TransactionView, PublicStatement) {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture = repo.join("tests/fixtures/node-lifecycle");
    let root = fixture.join(name);
    let create = packed::Transaction::from_slice(&read(&root.join("creation.bin")))
        .unwrap()
        .into_view();
    let settle = packed::Transaction::from_slice(&read(&root.join("settlement.bin")))
        .unwrap()
        .into_view();
    let statement = PublicStatement::decode(&read(&root.join("public-values.bin"))).unwrap();
    assert_eq!(
        statement.proposal_outpoint,
        settle.inputs().get(0).unwrap().previous_output().as_slice()
    );
    assert_eq!(
        settle.inputs().get(0).unwrap().previous_output().tx_hash(),
        create.hash()
    );
    let mut context = Context::new_with_deterministic_rng();
    for (index, file) in [
        "verifiers/sp1-plonk/target/riscv64imac-unknown-none-elf/release/agoraseal-sp1-plonk",
        "verifiers/ckb-state-context/target/riscv64imac-unknown-none-elf/release/agoraseal-ckb-state-context",
        "target/cellscript/funded-proposal.elf", "target/cellscript/funded-treasury.elf",
    ].iter().enumerate() {
        let data = read(&repo.join(file));
        let output = packed::CellOutput::new_builder().lock(settle.outputs().get(1).unwrap().lock()).build();
        let capacity = output.occupied_capacity(Capacity::bytes(data.len()).unwrap()).unwrap().as_u64();
        let output = output.as_builder().capacity(capacity).build();
        context.create_cell_with_out_point(settle.cell_deps().get(index).unwrap().out_point(), output, data);
    }
    let genesis = packed::Header::from_slice(&read(&fixture.join("genesis-header.bin")))
        .unwrap()
        .into_view();
    assert_eq!(genesis.hash().as_slice(), statement.genesis);
    context.insert_header(genesis);
    let mut creation_index = None;
    for index in 0..=statement.end_number - statement.start_number {
        let bytes = read(&root.join(format!("block-{index:06}.bin")));
        let block = packed::BlockReader::from_compatible_slice(&bytes).unwrap();
        let header = block.header().to_entity().into_view();
        if index == 0 {
            creation_index = block
                .transactions()
                .iter()
                .position(|tx| hash(tx.raw().as_slice()) == create.hash().as_slice());
        }
        context.insert_header(header);
    }
    let input = settle.inputs().get(0).unwrap().previous_output();
    context.create_cell_with_out_point(
        input.clone(),
        create.outputs().get(0).unwrap(),
        create.outputs_data().get(0).unwrap().raw_data(),
    );
    context.link_cell_with_block(
        input,
        packed::Byte32::new(statement.start_hash),
        creation_index.unwrap(),
    );
    (context, settle, statement)
}
fn run_case(name: &str, passed: bool) {
    let (context, settle, statement) = fixture(name);
    assert_eq!(statement.passed, passed);
    let cycles = context.verify_tx(&settle, 70_000_000).unwrap();
    assert!(
        cycles > 60_000_000,
        "real crypto execution must not disappear"
    );
    println!(
        "node golden {name}: {cycles} cycles; {} tx bytes",
        settle.data().as_slice().len()
    );
}
#[test]
fn node_golden_real_pass_and_refund_execute_complete_policies() {
    run_case("passed", true);
    run_case("failed", false);
}
#[test]
fn ccc_golden_reproof_settles_and_old_proof_rejects_replacement_context() {
    run_case("ccc", true);
    // The old proof is cryptographically valid for its own historical frames.
    // Canonical admission was tested on the node; this VM test isolates the
    // policy's explicit error when that proof is paired with the new end header.
    run_case("ccc/stale", true);
    let (mut context, stale, old) = fixture("ccc/stale");
    let (_, _, new) = fixture("ccc");
    assert_eq!(old.proposal_outpoint, new.proposal_outpoint);
    assert_eq!(old.start_hash, new.start_hash);
    assert_eq!(old.end_number, new.end_number);
    assert_ne!(old.end_hash, new.end_hash);
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bytes = read(&repo.join("tests/fixtures/node-lifecycle/ccc/block-000016.bin"));
    let block = packed::BlockReader::from_compatible_slice(&bytes).unwrap();
    let header = block.header().to_entity().into_view();
    assert_eq!(header.hash().as_slice(), new.end_hash);
    context.insert_header(header);
    let mut headers: Vec<_> = stale.header_deps().into_iter().collect();
    headers[1] = packed::Byte32::new(new.end_hash);
    let changed = stale.as_advanced_builder().set_header_deps(headers).build();
    let error = context
        .verify_tx(&changed, 140_000_000)
        .expect_err("old proof must reject new context");
    assert!(
        format!("{error:?}").contains("error code 1 on page"),
        "{error:?}"
    );
}
