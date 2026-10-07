#![no_main]

use agoraseal_protocol::{GuestInput, Limits, PublicStatement, Replay};
use sp1_zkvm::{io, syscalls::syscall_hint_len};

// Succinct Rust 1.96's prebuilt std references the GCC __atomic ABI. This
// imports ckb-std's existing single-thread polyfills, not its CKB entrypoint.
#[cfg(target_os = "zkvm")]
use ckb_std as _;

sp1_zkvm::entrypoint!(main);

fn bounded_frame(max: usize) -> Vec<u8> {
    // Check SP1's next frame length before read_vec allocates/reserves its input.
    // The syscall is read-only; read_vec then consumes exactly that same frame.
    let size = syscall_hint_len();
    assert!(size <= max, "missing or oversized frame");
    let bytes = io::read_vec();
    assert_eq!(bytes.len(), size, "input frame changed");
    bytes
}

pub fn main() {
    let input = GuestInput::decode(&bounded_frame(GuestInput::LEN)).expect("guest input");
    let limits = Limits::default();
    assert!(
        input.blocks >= 2 && input.blocks <= limits.blocks,
        "block count"
    );
    let mut replay = Replay::new(
        input.proposal_script,
        input.start_hash,
        input.end_hash,
        limits,
    );
    let mut total = 0u64;
    for _ in 0..input.blocks {
        let next = syscall_hint_len();
        assert!(next <= limits.block_bytes, "block size");
        total = total.checked_add(next as u64).expect("input size overflow");
        assert!(total <= limits.total_bytes, "total input size");
        replay
            .push(&bounded_frame(limits.block_bytes))
            .expect("invalid committed history");
    }
    assert_eq!(syscall_hint_len(), usize::MAX, "extra input frames");
    let tally = replay.finish().expect("incomplete voting window");
    io::commit_slice(&PublicStatement::from_tally(&tally).encode());
}
