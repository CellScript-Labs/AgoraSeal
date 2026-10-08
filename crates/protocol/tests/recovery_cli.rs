//! Persisted-proof recovery checks use the standalone verifier process, without
//! starting SP1 proving or accepting stored public values as replay authority.
use std::{fs, path::Path, process::Command};

fn run(root: &Path) -> std::process::Output {
    Command::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../zk/target/release/agoraseal-verify-plonk"
    ))
    .arg(root)
    .output()
    .expect("build agoraseal-verify-plonk first")
}

#[test]
fn stored_proof_revalidation_rejects_stale_and_partial_artifacts() {
    let source = std::env::var_os("AGORASEAL_PROOF_FIXTURE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/sp1-plonk")
        });
    let files = [
        "input.bin",
        "block-000000.bin",
        "block-000001.bin",
        "block-000002.bin",
        "block-000003.bin",
        "public-values.bin",
        "program-vkey.txt",
        "proof-plonk.bin",
    ];
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("agoraseal-recovery-{}-{stamp}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let cases = [
        ("proof-plonk.bin", "stored PLONK proof failed"),
        ("proof-plonk.bin", "must contain exactly 964 bytes"),
        ("proof-plonk.bin", "exceeds bound"),
        ("public-values.bin", "stored statement does not match"),
        ("program-vkey.txt", "stored guest key differs"),
        ("input.bin", "block 0"),
        ("public-values.bin", "exceeds bound"),
    ];
    for (case, (file, expected)) in cases.iter().enumerate() {
        for file in files {
            fs::copy(source.join(file), root.join(file)).unwrap();
        }
        if case == 0 {
            let output = run(&root);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(String::from_utf8_lossy(&output.stdout).contains("no proving performed"));
        }
        let path = root.join(file);
        let mut bytes = fs::read(&path).unwrap();
        match case {
            0 => bytes[963] ^= 1,
            1 => {
                bytes.pop();
            }
            2 | 6 => bytes.push(0),
            3 => bytes[276] ^= 1,
            4 => bytes[65] = if bytes[65] == b'0' { b'1' } else { b'0' },
            5 => bytes[40] ^= 1,
            _ => unreachable!(),
        }
        fs::write(&path, bytes).unwrap();
        let output = run(&root);
        assert!(!output.status.success(), "recovery case {case} accepted");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(expected), "recovery case {case}: {error}");
    }
    fs::remove_dir_all(root).unwrap();
}
