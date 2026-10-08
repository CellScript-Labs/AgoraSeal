//! Local proof attempt with one-hour bound and independent persisted validation.
use anyhow::{Result, ensure};
use std::path::PathBuf;
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    ensure!(
        args.len() == 1,
        "usage: agoraseal-prove-local EXPORTED_DIRECTORY (run from repository root)"
    );
    agoraseal_node::proof::prove(
        &std::env::current_dir()?,
        &PathBuf::from(&args[0]).canonicalize()?,
    )
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
