#![no_std]
#![no_main]

// Generic unique-state/header-coordinate adapter. No DAO, vote program,
// duration, amount, quorum, result or payout decision belongs here.
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{packed, prelude::*},
    env, syscalls,
};

ckb_std::entry!(entry);
ckb_std::default_alloc!(4096, 4096, 64);

fn hash(bytes: &[u8]) -> [u8; 32] {
    let mut result = [0; 32];
    let mut state = ckb_hash::new_blake2b();
    state.update(bytes);
    state.finalize(&mut result);
    result
}

fn hex(text: &[u8]) -> Result<[u8; 113], i8> {
    if text.len() != 226 {
        return Err(10);
    }
    let digit = |byte| match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(10),
    };
    let mut result = [0; 113];
    for (index, byte) in result.iter_mut().enumerate() {
        *byte = (digit(text[index * 2])? << 4) | digit(text[index * 2 + 1])?;
    }
    Ok(result)
}

fn header(index: usize, source: Source) -> Result<([u8; 32], u64), i8> {
    let mut bytes = [0; packed::Header::TOTAL_SIZE];
    let size = syscalls::load_header(&mut bytes, 0, index, source).map_err(|_| 13)?;
    if size != bytes.len() {
        return Err(13);
    }
    let value = packed::HeaderReader::from_slice(&bytes).map_err(|_| 13)?;
    let number = u64::from_le_bytes(value.raw().number().as_slice().try_into().map_err(|_| 13)?);
    Ok((hash(&bytes), number))
}

fn run() -> Result<(), i8> {
    let args = env::argv();
    if args.len() != 4 || args.iter().skip(1).any(|arg| !arg.to_bytes().is_empty()) {
        return Err(10);
    }
    let tuple = hex(args[0].to_bytes())?;
    // Script header (53) plus an exact 32-byte unique-state identifier. Bound
    // the syscall before parsing/allocation; no caller can select another ID.
    let mut bytes = [0; 85];
    let size = syscalls::load_script(&mut bytes, 0).map_err(|_| 11)?;
    if size != bytes.len() {
        return Err(11);
    }
    let script = packed::ScriptReader::from_slice(&bytes).map_err(|_| 11)?;
    let id = script.args();
    if id.raw_data().len() != 32 {
        return Err(11);
    }
    ckb_std::type_id::validate_type_id(id.raw_data()).map_err(|_| 12)?;
    let genesis: [u8; 32] = tuple[1..33].try_into().map_err(|_| 10)?;
    match tuple[0] {
        0 => {
            if tuple[33..].iter().any(|byte| *byte != 0)
                || header(0, Source::HeaderDep)? != (genesis, 0)
            {
                return Err(14);
            }
        }
        1 => {
            let start_hash = tuple[33..65].try_into().map_err(|_| 10)?;
            let end_hash = tuple[65..97].try_into().map_err(|_| 10)?;
            let start_number = u64::from_le_bytes(tuple[97..105].try_into().map_err(|_| 10)?);
            let end_number = u64::from_le_bytes(tuple[105..113].try_into().map_err(|_| 10)?);
            let start = (start_hash, start_number);
            if header(0, Source::HeaderDep)? != start
                || header(0, Source::GroupInput)? != start
                || header(1, Source::HeaderDep)? != (end_hash, end_number)
                || header(2, Source::HeaderDep)? != (genesis, 0)
            {
                return Err(14);
            }
        }
        _ => return Err(10),
    }
    Ok(())
}

fn entry() -> i8 {
    match run() {
        Ok(()) => 0,
        Err(error) => error,
    }
}
