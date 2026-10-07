use agoraseal_protocol::{GuestInput, Limits, PublicStatement, Replay};
use anyhow::{Context, Result, ensure};
use sp1_sdk::{
    Elf, HashableKey, ProveRequest, Prover, ProverClient, ProvingKey, SP1Proof, SP1PublicValues,
    SP1Stdin,
};
use std::{fs, io::Read, path::Path};

fn bounded_file(path: &Path, max: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= max, "{} exceeds bound", path.display());
    Ok(bytes)
}

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
        args.len() == 3 && matches!(args[0].as_str(), "execute" | "test-guest" | "prove-core"),
        "usage: agoraseal-prover execute|test-guest|prove-core GUEST_ELF INPUT_DIRECTORY"
    );
    let root = Path::new(&args[2]);
    let limits = Limits::default();
    let header_bytes = bounded_file(&root.join("input.bin"), GuestInput::LEN)?;
    let header = GuestInput::decode(&header_bytes).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    ensure!((2..=limits.blocks).contains(&header.blocks), "block count");
    let mut frames = vec![header_bytes];
    let mut replay = Replay::new(
        header.proposal_script,
        header.start_hash,
        header.end_hash,
        limits,
    );
    let mut total = 0u64;
    for i in 0..header.blocks {
        let bytes = bounded_file(&root.join(format!("block-{i:06}.bin")), limits.block_bytes)?;
        total = total
            .checked_add(bytes.len() as u64)
            .context("input size overflow")?;
        ensure!(total <= limits.total_bytes, "total bytes");
        replay
            .push(&bytes)
            .map_err(|e| anyhow::anyhow!("block {i}: {e:?}"))?;
        frames.push(bytes);
    }
    let tally = replay.finish().map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let expected = PublicStatement::from_tally(&tally).encode();
    let elf = Elf::from(bounded_file(Path::new(&args[1]), 32 * 1024 * 1024)?);
    ensure!(
        std::env::var_os("WITHOUT_VK_VERIFICATION").is_none(),
        "WITHOUT_VK_VERIFICATION must be unset"
    );
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
    if args[0] == "prove-core" {
        println!("setting up guest proving key");
        let pk = client.setup(elf).await?;
        println!("generating real SP1 core proof on local CPU");
        let proof = client.prove(&pk, stdin(&frames)).core().await?;
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
        proof.save(root.join("proof-core.bin"))?;
        fs::write(root.join("program-vkey.txt"), pk.verifying_key().bytes32())?;
        fs::write(root.join("public-values.bin"), expected)?;
        println!(
            "real SP1 core proof generated and verified natively; CKB SNARK verification remains separate"
        );
    }
    Ok(())
}
