use agoraseal_protocol::{GuestInput, Limits, PublicStatement, Replay};
use anyhow::{Context, Result, ensure};
use std::{fs, io::Read, path::Path};

pub fn bounded_file(path: &Path, max: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= max, "{} exceeds bound", path.display());
    Ok(bytes)
}

/// Recompute the authoritative expected public bytes from complete block frames.
/// Persisted proof/public/key files do not supply votes or statement authority.
pub fn replay_input(root: &Path) -> Result<(Vec<Vec<u8>>, [u8; PublicStatement::LEN])> {
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
    Ok((frames, PublicStatement::from_tally(&tally).encode()))
}
