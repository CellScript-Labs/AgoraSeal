#![no_std]
#![no_main]

// Exact cryptographic boundary only. Proposal eligibility, window admission,
// state transitions, quorum and payout policies belong to the CellScript parent.
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{packed::WitnessArgsReader, prelude::*},
    env, syscalls,
};
use sp1_verifier::PlonkVerifier;

ckb_std::entry!(entry);
ckb_std::default_alloc!(4096, 2_097_152, 64);

const PROGRAM_KEY: &str = include_str!("../../../zk/program-vkey.txt");
const PROOF_BYTES: usize = 964;
const PUBLIC_BYTES: usize = 277;

fn hex<const N: usize>(text: &[u8]) -> Result<[u8; N], i8> {
    if text.len() != N * 2 {
        return Err(10);
    }
    let digit = |byte| match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(10),
    };
    let mut bytes = [0; N];
    for (index, value) in bytes.iter_mut().enumerate() {
        *value = (digit(text[index * 2])? << 4) | digit(text[index * 2 + 1])?;
    }
    Ok(bytes)
}

fn verify() -> Result<(), i8> {
    let args = env::argv();
    if args.len() != 4 || args[1..].iter().any(|arg| !arg.to_bytes().is_empty()) {
        return Err(10);
    }
    let expected = hex::<32>(args[0].to_bytes())?;
    // Fixed witness source shared with the calling Type Script group. Check
    // the complete witness bound before Molecule parsing or heap allocation.
    let mut witness = [0; 2048];
    let size = syscalls::load_witness(&mut witness, 0, 0, Source::GroupInput).map_err(|_| 13)?;
    if size > witness.len() {
        return Err(13);
    }
    let witness = WitnessArgsReader::from_slice(&witness[..size]).map_err(|_| 13)?;
    let packet = witness.output_type().to_opt().ok_or(13)?;
    let packet = packet.raw_data();
    if packet.len() != PROOF_BYTES + PUBLIC_BYTES {
        return Err(13);
    }
    let (proof, public) = packet.split_at(PROOF_BYTES);
    if &public[..8] != b"AGZKPV01" || public[276] > 1 {
        return Err(11);
    }
    if ckb_hash::blake2b_256(public) != expected {
        return Err(14);
    }
    // The upstream verifier authenticates exit code zero and the release VK
    // tree root, in addition to the program key and all public statement bytes.
    PlonkVerifier::verify(
        proof,
        public,
        PROGRAM_KEY.trim(),
        sp1_verifier::PLONK_VK_BYTES,
    )
    .map_err(|_| 12)
}

fn entry() -> i8 {
    match verify() {
        Ok(()) => 0,
        Err(error) => error,
    }
}
