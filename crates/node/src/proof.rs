//! Shared bounded local proving and completed-artifact recovery.
use anyhow::{Result, ensure};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use wait_timeout::ChildExt;

pub fn prove(root: &Path, dir: &Path) -> Result<()> {
    let artifacts = [
        "proof-plonk.bin",
        "proof-plonk-bundle.bin",
        "program-vkey.txt",
    ];
    if artifacts.iter().any(|name| dir.join(name).exists()) {
        ensure!(
            artifacts.iter().all(|name| dir.join(name).is_file()),
            "partial proof has no computation checkpoint; preserve artifacts and start an explicit fresh attempt"
        );
        let status = Command::new(root.join("zk/target/release/agoraseal-verify-plonk"))
            .arg(dir)
            .status()?;
        ensure!(
            status.success(),
            "completed proof recovery verification rejected"
        );
        println!(
            "reused completed proof after independent input/key verification: {}",
            dir.display()
        );
        return Ok(());
    }
    ensure!(
        ["SP1_CIRCUIT_MODE", "WITHOUT_VK_VERIFICATION", "SP1_DUMP"]
            .iter()
            .all(|name| std::env::var_os(name).is_none()),
        "unsafe proof mode is set"
    );
    let checked = Command::new(root.join("scripts/check-plonk-circuits.sh"))
        .current_dir(root)
        .env(
            "SP1_PLONK_CIRCUIT_PATH",
            root.join(".local/sp1/circuits/plonk"),
        )
        .status()?;
    ensure!(checked.success(), "release circuit preflight");
    let start = Instant::now();
    let mut child = Command::new(root.join("zk/target/release/agoraseal-prover"))
        .arg("prove-plonk")
        .arg(root.join("target/sp1/agoraseal-guest"))
        .arg(dir)
        .env(
            "SP1_PLONK_CIRCUIT_PATH",
            root.join(".local/sp1/circuits/plonk"),
        )
        .env("SP1_WORKER_NUM_CORE_WORKERS", "1")
        .env("SP1_WORKER_CORE_BUFFER_SIZE", "1")
        .env("SP1_WORKER_NUM_PREPARE_REDUCE_WORKERS", "1")
        .env("SP1_WORKER_PREPARE_REDUCE_BUFFER_SIZE", "1")
        .env("SP1_WORKER_NUM_RECURSION_EXECUTOR_WORKERS", "1")
        .env("SP1_WORKER_RECURSION_EXECUTOR_BUFFER_SIZE", "1")
        .env("SP1_WORKER_NUM_RECURSION_PROVER_WORKERS", "1")
        .env("SP1_WORKER_RECURSION_PROVER_BUFFER_SIZE", "1")
        .env("RAYON_NUM_THREADS", "8")
        .env("GOMAXPROCS", "4")
        .env("GOMEMLIMIT", "32GiB")
        .env("GOGC", "50")
        .stdout(Stdio::from(fs::File::create_new(dir.join("proof.log"))?))
        .stderr(Stdio::from(fs::File::create_new(
            dir.join("proof.stderr.log"),
        )?))
        .spawn()?;
    let status = match child.wait_timeout(Duration::from_secs(3600))? {
        Some(status) => status,
        None => {
            child.kill()?;
            child.wait()?;
            anyhow::bail!("prover exceeded one-hour rehearsal bound");
        }
    };
    ensure!(
        status.success(),
        "real prover failed; inspect {}",
        dir.display()
    );
    fs::write(
        dir.join("proving-wall-seconds.txt"),
        start.elapsed().as_secs_f64().to_string(),
    )?;
    let status = Command::new(root.join("zk/target/release/agoraseal-verify-plonk"))
        .arg(dir)
        .status()?;
    ensure!(status.success(), "persisted real proof verification");
    Ok(())
}
