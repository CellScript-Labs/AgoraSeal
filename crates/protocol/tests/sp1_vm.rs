//! Exact crypto ABI and real CellScript parent/child statement binding.
//! The parent is a binding component, not proposal or canonical-chain admission.
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use ckb_types::{
    core::{ScriptHashType, TransactionBuilder, TransactionView},
    packed,
    prelude::*,
};
use ckb_vm::{DefaultMachineRunner, SupportMachine, bytes::Bytes};

const MAX_CYCLES: u64 = 3_000_000_000;

fn verifier() -> Bytes {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../verifiers/sp1-plonk/target/riscv64imac-unknown-none-elf/release/agoraseal-sp1-plonk"
    ))
    .expect("build the pinned SP1 CKB verifier first")
    .into()
}

fn hex(bytes: &[u8]) -> Bytes {
    let digits = b"0123456789abcdef";
    let mut out = Vec::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(digits[(byte >> 4) as usize]);
        out.push(digits[(byte & 15) as usize]);
    }
    out.into()
}

fn run_bad_args(args: Vec<Bytes>) -> i8 {
    let core = <Box<ckb_vm::machine::asm::AsmCoreMachine> as SupportMachine>::new(
        ckb_vm::ISA_IMC | ckb_vm::ISA_B | ckb_vm::ISA_MOP,
        ckb_vm::machine::VERSION2,
        MAX_CYCLES,
    );
    let core = ckb_vm::DefaultMachineBuilder::new(core)
        .instruction_cycle_func(Box::new(ckb_vm::cost_model::estimate_cycles))
        .build();
    let mut machine = ckb_vm::machine::asm::AsmMachine::new(core);
    machine
        .load_program(&verifier(), args.into_iter().map(Ok))
        .unwrap();
    // No witness syscall is installed: malformed arguments must reject before it.
    machine
        .run()
        .expect("malformed argv must exit before reading witness")
}

#[derive(Clone, Copy, Debug)]
enum Mutation {
    None,
    ClaimedHash,
    ClaimedHashTail,
    MissingPacket,
    OversizeWitness,
    VerifierCode,
    ExtraOutput,
}

fn fixture(
    proof: &[u8],
    public: &[u8],
    packet_public: &[u8],
    mutation: Mutation,
) -> (Context, TransactionView) {
    let mut context = Context::new_with_deterministic_rng();
    let parent = context.deploy_cell(
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/cellscript/proof-binding.elf"
        ))
        .expect("build CellScript proof-binding parent first")
        .into(),
    );
    let success = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let child = if matches!(mutation, Mutation::VerifierCode) {
        success.clone()
    } else {
        context.deploy_cell(verifier())
    };
    let lock = context.build_script(&success, Bytes::new()).unwrap();
    let type_script = context
        .build_script_with_hash_type(&parent, ScriptHashType::Data2, Bytes::new())
        .unwrap();
    let cell = packed::CellOutput::new_builder()
        .capacity(100_000_000_000u64)
        .lock(lock)
        .type_(Some(type_script).pack())
        .build();
    let input = context.create_cell(cell.clone(), Bytes::new());
    let mut claim = agoraseal_protocol::hash(public);
    if matches!(mutation, Mutation::ClaimedHash) {
        claim[0] ^= 1;
    }
    if matches!(mutation, Mutation::ClaimedHashTail) {
        claim[31] ^= 1;
    }
    let mut entry = b"CSARGv1\0".to_vec();
    entry.extend_from_slice(&claim);
    let mut packet = proof.to_vec();
    packet.extend_from_slice(packet_public);
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
        .lock(
            if matches!(mutation, Mutation::OversizeWitness) {
                Some(Bytes::from(vec![0; 2048]))
            } else {
                None
            }
            .pack(),
        )
        .build();
    let mut tx = TransactionBuilder::default()
        .input(
            packed::CellInput::new_builder()
                .previous_output(input)
                .build(),
        )
        .output(cell.clone())
        .output_data(Bytes::copy_from_slice(public).pack())
        .cell_dep(packed::CellDep::new_builder().out_point(child).build())
        .witness(witness.as_bytes().pack());
    if matches!(mutation, Mutation::ExtraOutput) {
        tx = tx
            .output(cell)
            .output_data(Bytes::copy_from_slice(public).pack());
    }
    (context.clone(), context.complete_tx(tx.build()))
}

fn reject(proof: &[u8], public: &[u8], packet_public: &[u8], mutation: Mutation, code: i8) {
    let (context, tx) = fixture(proof, public, packet_public, mutation);
    let error = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect_err("substitution must reject");
    let text = format!("{error:?}");
    assert!(
        text.contains(&format!("error code {code} on page")),
        "{mutation:?}: {text}"
    );
}

// The transaction verifier returns cycles only for success. Find the minimum
// total transaction budget that reaches the expected explicit rejection, using
// the same uninstrumented ELF. A resource/infrastructure error is not rejection.
fn minimum_rejection_budget(context: &Context, tx: &TransactionView) -> u64 {
    let mut low = 0;
    let mut high = MAX_CYCLES;
    let rejects = |budget| {
        let error = context
            .verify_tx(tx, budget)
            .expect_err("invalid proof accepted");
        let text = format!("{error:?}");
        if text.contains("error code 1 on page") {
            true
        } else {
            assert!(
                text.contains("ExceededMaximumCycles") || text.contains("CyclesExceeded"),
                "unexpected benchmark failure at {budget}: {text}"
            );
            false
        }
    };
    assert!(rejects(high));
    assert!(!rejects(low));
    while high - low > 1 {
        let midpoint = low + (high - low) / 2;
        if rejects(midpoint) {
            high = midpoint;
        } else {
            low = midpoint;
        }
    }
    high
}

#[test]
fn exact_abi_rejects_malformed_encoding_before_cryptography() {
    let args = |hash: &[u8]| vec![hex(hash), Bytes::new(), Bytes::new(), Bytes::new()];
    assert_eq!(run_bad_args(vec![]), 10);
    assert_eq!(run_bad_args(args(&[0; 31])), 10);
    assert_eq!(run_bad_args(args(&[0; 33])), 10);
    let mut uppercase = args(&[0; 32]);
    uppercase[0] = Bytes::from(vec![b'A'; 64]);
    assert_eq!(run_bad_args(uppercase), 10);
    let mut override_key = args(&[0; 32]);
    override_key[2] = hex(&[0; 32]);
    assert_eq!(run_bad_args(override_key), 10);
}

#[test]
fn cellscript_binding_rejects_wrong_verifier_hash_packet_and_statement() {
    let mut public = [0; 277];
    public[..8].copy_from_slice(b"AGZKPV01");
    let proof = [0; 964];
    for (mutation, code) in [
        (Mutation::None, 1),
        (Mutation::ClaimedHash, 5),
        (Mutation::ClaimedHashTail, 5),
        (Mutation::MissingPacket, 1),
        (Mutation::OversizeWitness, 1),
        (Mutation::VerifierCode, 41),
        (Mutation::ExtraOutput, 5),
    ] {
        reject(&proof, &public, &public, mutation, code);
    }
    reject(&proof[..963], &public, &public, Mutation::None, 1);
    let mut changed = public;
    changed[8] ^= 1;
    reject(&proof, &public, &changed, Mutation::None, 1);
    changed = public;
    changed[276] = 2;
    reject(&proof, &changed, &changed, Mutation::None, 1);
}

#[test]
#[ignore = "requires real locally generated PLONK proof; scripts/zk.sh verify-ckb"]
fn real_plonk_proof_and_context_substitutions_execute_in_ckb_vm() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/guest-fixture");
    let report = root.join("ckb-proof-binding.toml");
    match std::fs::remove_file(&report) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!("cannot invalidate old component report: {error}"),
    }
    let proof =
        std::fs::read(root.join("proof-plonk.bin")).expect("generate real PLONK proof first");
    let public = std::fs::read(root.join("public-values.bin")).unwrap();
    assert_eq!(proof.len(), 964);
    assert_eq!(public.len(), 277);
    let (context, tx) = fixture(&proof, &public, &public, Mutation::None);
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("real PLONK proof through CellScript parent");
    println!("CellScript + SP1 PLONK valid transaction CKB-VM cycles: {cycles}");
    for offset in [8, 40, 72, 108, 140, 172, 204, 212, 220, 228, 236, 244, 276] {
        let mut changed = public.clone();
        changed[offset] ^= 1;
        // Change both packet and claimed output so the parent hash still
        // matches: this must reach cryptographic rejection in the child.
        reject(&proof, &changed, &changed, Mutation::None, 1);
    }
    for offset in [0, 4, 36, 68, 100, 963] {
        let mut changed = proof.clone();
        changed[offset] ^= 1;
        reject(&changed, &public, &public, Mutation::None, 1);
    }
    let mut trailing = proof.clone();
    trailing.push(0);
    reject(&trailing, &public, &public, Mutation::None, 1);
    println!("20 real-proof substitution/length cases rejected within {MAX_CYCLES} cycles each");
    let mut late_invalid = public.clone();
    late_invalid[220] ^= 1;
    let (invalid_context, invalid_tx) =
        fixture(&proof, &late_invalid, &late_invalid, Mutation::None);
    let late_invalid_budget = minimum_rejection_budget(&invalid_context, &invalid_tx);
    println!(
        "Changed YES total: minimum transaction budget for explicit rejection = {late_invalid_budget} cycles"
    );
    let digest =
        |bytes: &[u8]| String::from_utf8(hex(&agoraseal_protocol::hash(bytes)).to_vec()).unwrap();
    let parent = std::fs::read(root.join("../cellscript/proof-binding.elf")).unwrap();
    std::fs::write(
        report,
        format!(
            "schema = \"agoraseal-ckb-proof-binding-v1\"\n\
             evidence = \"ckb-vm-component\"\n\
             production_admission = false\n\
             valid_cycles = {cycles}\n\
             changed_yes_minimum_rejection_budget = {late_invalid_budget}\n\
             negative_cases = 20\n\
             per_case_cycle_ceiling = {MAX_CYCLES}\n\
             transaction_bytes = {}\n\
             proof_bytes = {}\n\
             public_bytes = {}\n\
             parent_elf_bytes = {}\n\
             verifier_elf_bytes = {}\n\
             proof_ckb_hash = \"{}\"\n\
             public_ckb_hash = \"{}\"\n\
             parent_elf_ckb_hash = \"{}\"\n\
             verifier_elf_ckb_hash = \"{}\"\n",
            tx.data().as_slice().len(),
            proof.len(),
            public.len(),
            parent.len(),
            verifier().len(),
            digest(&proof),
            digest(&public),
            digest(&parent),
            digest(&verifier()),
        ),
    )
    .unwrap();
}
