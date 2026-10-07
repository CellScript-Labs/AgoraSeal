use agoraseal_protocol::{Hash, Limits, Replay};
use std::{fs::File, io::Read};

fn parse_hash(value: &str) -> Result<Hash, String> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    if value.len() != 64 || !value.is_ascii() {
        return Err("expected a 32-byte hexadecimal hash".into());
    }
    let mut result = [0; 32];
    for (index, byte) in result.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| "invalid hex hash")?;
    }
    Ok(result)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 6 || args[0] != "replay" {
        return Err("usage: agoraseal replay PROPOSAL_SCRIPT_HASH START_HASH END_HASH block0.bin block1.bin ...".into());
    }
    let limits = Limits::default();
    if args.len() - 4 > limits.blocks as usize {
        return Err("block count exceeds replay bound".into());
    }
    let mut replay = Replay::new(
        parse_hash(&args[1])?,
        parse_hash(&args[2])?,
        parse_hash(&args[3])?,
        limits,
    );
    for path in &args[4..] {
        let file = File::open(path).map_err(|error| format!("{path}: {error}"))?;
        let mut bytes = Vec::new();
        file.take(limits.block_bytes as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("{path}: {error}"))?;
        replay
            .push(&bytes)
            .map_err(|error| format!("{path}: {error:?}"))?;
    }
    let result = replay
        .finish()
        .map_err(|error| format!("incomplete or invalid replay: {error:?}"))?;
    println!(
        "{{\"evidence\":\"native-replay-only\",\"proposal\":\"{}\",\"start\":\"{}\",\"end\":\"{}\",\"yes\":\"{}\",\"no\":\"{}\",\"counted\":{},\"counted_digest\":\"{}\",\"passed\":{}}}",
        hex(&result.proposal_script),
        hex(&result.start_hash),
        hex(&result.end_hash),
        result.yes,
        result.no,
        result.counted,
        hex(&result.counted_digest),
        result.passed
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
