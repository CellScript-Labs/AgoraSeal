//! Owned local-node lifetime for CCC rehearsal. No public-network operation.
use agoraseal_node::devnet::CkbDevnet;
use anyhow::{Context, Result, ensure};
use std::{io, path::PathBuf, process::Command};

fn run() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    ensure!(
        args.len() == 3,
        "usage: agoraseal-node-serve CKB_REPO CKB_BINARY EXISTING_RUN_DIRECTORY"
    );
    let repo = PathBuf::from(&args[0]).canonicalize()?;
    let bin = PathBuf::from(&args[1]).canonicalize()?;
    let root = PathBuf::from(&args[2]).canonicalize()?;
    let rev = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&repo)
        .output()?;
    ensure!(
        rev.status.success()
            && String::from_utf8(rev.stdout)?.trim() == "f7fa4436737756f97a24e254f22c13a36316ecea",
        "CKB revision differs"
    );
    let clean = Command::new("git")
        .args(["diff", "--quiet", "HEAD"])
        .current_dir(&repo)
        .status()?;
    ensure!(clean.success(), "CKB sources are dirty");
    let version = Command::new(&bin).arg("--version").output()?;
    ensure!(
        version.status.success()
            && String::from_utf8(version.stdout)?.contains("ckb 0.207.0 (f7fa443"),
        "CKB binary identity differs"
    );
    let mut node = CkbDevnet::reopen(repo, bin, root)?;
    node.start()?;
    println!("owned disposable RPC ready: {}", node.rpc_url);
    // The caller keeps stdin open, then closes it for an orderly stop. Drop
    // cleans up this child on errors; it never finds/kills an unrelated node.
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .context("owned lifetime control")?;
    node.stop();
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
