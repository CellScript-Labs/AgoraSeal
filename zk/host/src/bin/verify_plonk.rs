//! Revalidate persisted proof artifacts without starting a prover or trusting a
//! saved public statement/key. This does not resume an unfinished SP1 computation.
use agoraseal_prover::{bounded_file, replay_input};
use anyhow::{Result, ensure};
use std::path::Path;

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 1,
        "usage: agoraseal-verify-plonk INPUT_DIRECTORY"
    );
    let root = Path::new(&args[0]);
    let (_, expected) = replay_input(root)?;
    let public = bounded_file(&root.join("public-values.bin"), expected.len())?;
    ensure!(
        public == expected,
        "stored statement does not match replayed input"
    );
    let key = include_str!("../../../program-vkey.txt").trim();
    let stored_key = bounded_file(&root.join("program-vkey.txt"), 67)?;
    ensure!(
        std::str::from_utf8(&stored_key)?.trim() == key,
        "stored guest key differs from compiled pin"
    );
    let proof = bounded_file(&root.join("proof-plonk.bin"), 964)?;
    ensure!(
        proof.len() == 964,
        "PLONK proof must contain exactly 964 bytes"
    );
    sp1_verifier::PlonkVerifier::verify(&proof, &expected, key, &sp1_verifier::PLONK_VK_BYTES)
        .map_err(|e| anyhow::anyhow!("stored PLONK proof failed: {e:?}"))?;
    println!(
        "stored PLONK proof verified against replayed input and pinned guest; no proving performed"
    );
    Ok(())
}
