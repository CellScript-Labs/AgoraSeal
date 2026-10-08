use agoraseal_protocol::GuestInput;
use agoraseal_prover::{bounded_file, replay_input};
use anyhow::{Result, ensure};
use sp1_sdk::{
    Elf, HashableKey, ProveRequest, Prover, ProverClient, ProvingKey, SP1Proof, SP1PublicValues,
    SP1Stdin,
};
use std::{fs, path::Path};

fn stdin(frames: &[Vec<u8>]) -> SP1Stdin {
    let mut input = SP1Stdin::new();
    for frame in frames {
        input.write_slice(frame);
    }
    input
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 3
            && matches!(
                args[0].as_str(),
                "execute" | "test-guest" | "prove-core" | "prove-plonk"
            ),
        "usage: agoraseal-prover execute|test-guest|prove-core|prove-plonk GUEST_ELF INPUT_DIRECTORY"
    );
    let plonk = args[0] == "prove-plonk";
    ensure!(
        !plonk || cfg!(feature = "native-gnark"),
        "prove-plonk requires the native-gnark build feature"
    );
    for name in ["SP1_CIRCUIT_MODE", "WITHOUT_VK_VERIFICATION", "SP1_DUMP"] {
        ensure!(std::env::var_os(name).is_none(), "{name} must be unset");
    }
    sp1_sdk::setup_logger();
    let root = Path::new(&args[2]);
    let (frames, expected) = replay_input(root)?;
    let elf = Elf::from(bounded_file(Path::new(&args[1]), 32 * 1024 * 1024)?);
    println!("initializing local SP1 CPU prover");
    let client = ProverClient::builder().cpu().build().await;
    println!("executing bounded block replay guest");
    let (public, report) = client.execute(elf.clone(), stdin(&frames)).await?;
    ensure!(
        report.exit_code == 0,
        "guest failed: exit {}",
        report.exit_code
    );
    ensure!(
        public.as_slice() == expected,
        "guest/native public statement mismatch"
    );
    println!(
        "guest executed: {} instructions, {} public bytes; ZK proof not implied",
        report.total_instruction_count(),
        public.as_slice().len()
    );
    if args[0] == "test-guest" {
        // These mutations go straight into the guest. The native precheck above
        // must not substitute for evidence that the guest itself rejects them.
        for case in 0..5 {
            let mut changed = frames.clone();
            match case {
                0 => changed[0][40] ^= 1, // wrong anchored start
                1 => {
                    changed.pop();
                } // missing final block
                2 => changed.push(vec![0]), // unconsumed extra input
                3 => changed[1][0] ^= 1,  // corrupt Molecule block
                _ => {
                    changed[0].truncate(GuestInput::LEN - 1);
                }
            }
            let (rejected_public, rejected_report) =
                client.execute(elf.clone(), stdin(&changed)).await?;
            ensure!(
                rejected_report.exit_code == 1,
                "guest mutation {case}: expected panic exit 1, got {}",
                rejected_report.exit_code
            );
            ensure!(
                rejected_public.as_slice().is_empty(),
                "failed guest committed a statement"
            );
            println!("guest mutation {case}: exit 1, no public statement");
        }
        println!("five direct guest rejection cases passed");
    }
    if args[0] == "prove-core" || plonk {
        println!("setting up guest proving key");
        let pk = client.setup(elf).await?;
        println!(
            "generating real SP1 {} proof on local CPU",
            if plonk { "PLONK" } else { "core" }
        );
        let proof = if plonk {
            client.prove(&pk, stdin(&frames)).plonk().await?
        } else {
            client.prove(&pk, stdin(&frames)).core().await?
        };
        client.verify(&proof, pk.verifying_key(), None)?;
        ensure!(
            proof.public_values.as_slice() == expected,
            "proof statement mismatch"
        );
        // Every context field is committed by the real proof, not merely
        // compared in the host's native precheck.
        for offset in [
            0, 8, 40, 72, 108, 140, 172, 204, 212, 220, 228, 236, 244, 276,
        ] {
            let mut changed = proof.clone();
            let mut public = expected;
            public[offset] ^= 1;
            changed.public_values = SP1PublicValues::from(&public);
            ensure!(
                client.verify(&changed, pk.verifying_key(), None).is_err(),
                "proof accepted changed public field at {offset}"
            );
        }
        let mut wrong_key = pk.verifying_key().clone();
        wrong_key.vk.pc_start[0] = -wrong_key.vk.pc_start[0];
        ensure!(
            wrong_key.bytes32() != pk.verifying_key().bytes32(),
            "key mutation unchanged"
        );
        ensure!(
            client.verify(&proof, &wrong_key, None).is_err(),
            "proof accepted wrong program key"
        );
        let mut empty_proof = proof.clone();
        empty_proof.proof = SP1Proof::Core(vec![]);
        ensure!(
            client
                .verify(&empty_proof, pk.verifying_key(), None)
                .is_err(),
            "empty proof accepted"
        );
        println!(
            "real proof rejected 14 public-field substitutions, wrong program key and empty proof"
        );
        if plonk {
            let raw = proof.bytes();
            let key = pk.verifying_key().bytes32();
            let verify = |bytes: &[u8], public: &[u8], program_key: &str| {
                sp1_verifier::PlonkVerifier::verify(
                    bytes,
                    public,
                    program_key,
                    &sp1_verifier::PLONK_VK_BYTES,
                )
            };
            verify(&raw, &expected, &key)
                .map_err(|e| anyhow::anyhow!("standalone PLONK verifier: {e:?}"))?;
            for offset in [0, 4, 36, 68, 100, raw.len() - 1] {
                let mut changed = raw.clone();
                changed[offset] ^= 1;
                ensure!(
                    verify(&changed, &expected, &key).is_err(),
                    "PLONK proof mutation at {offset} accepted"
                );
            }
            for length in [0, 4, 99, raw.len() - 1] {
                ensure!(
                    verify(&raw[..length], &expected, &key).is_err(),
                    "truncated PLONK proof accepted"
                );
            }
            let mut trailing = raw.clone();
            trailing.push(0);
            ensure!(
                verify(&trailing, &expected, &key).is_err(),
                "PLONK proof trailing byte accepted"
            );
            fs::write(root.join("proof-plonk.bin"), &raw)?;
            proof.save(root.join("proof-plonk-bundle.bin"))?;
            println!(
                "real PLONK proof: {} bytes; standalone verifier and 11 raw-proof rejection checks passed",
                raw.len()
            );
        } else {
            proof.save(root.join("proof-core.bin"))?;
        }
        fs::write(root.join("program-vkey.txt"), pk.verifying_key().bytes32())?;
        fs::write(root.join("public-values.bin"), expected)?;
        println!(
            "real SP1 proof generated and verified natively; CKB-VM verification remains separate"
        );
    }
    Ok(())
}
