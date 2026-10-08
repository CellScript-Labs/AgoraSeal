//! Identity substitution checks for the build's explicit pin/check separation.
use std::{fs, process::Command};

fn check_pin(mode: &str, bytes_marker: &str, hash_marker: &str) {
    let root = std::env::temp_dir().join(format!("agoraseal-{mode}-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/main.cell.in"),
        format!("hash={bytes_marker}\n"),
    )
    .unwrap();
    fs::write(root.join("Cell.toml.in"), format!("hash='{hash_marker}'\n")).unwrap();
    let elf = root.join("test.elf");
    fs::write(&elf, b"identity-fixture-a").unwrap();
    let command = |check: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_agoraseal"));
        command.arg(mode).arg(&elf).arg(&root);
        if check {
            command.arg("--check");
        }
        command.output().unwrap()
    };
    assert!(command(false).status.success());
    assert!(command(true).status.success());
    let committed = fs::read(root.join("Cell.toml")).unwrap();
    fs::write(&elf, b"identity-fixture-b").unwrap();
    assert!(!command(true).status.success());
    assert_eq!(fs::read(root.join("Cell.toml")).unwrap(), committed);
    fs::write(&elf, b"identity-fixture-a").unwrap();
    fs::write(root.join("Cell.toml"), b"hash='substituted'\n").unwrap();
    assert!(!command(true).status.success());
    assert_eq!(
        fs::read(root.join("Cell.toml")).unwrap(),
        b"hash='substituted'\n"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn context_check_rejects_changed_elf_and_manifest_without_rewriting() {
    check_pin("context-pin", "@CONTEXT_HASH_BYTES@", "@CONTEXT_HASH@");
}

#[test]
fn verifier_check_rejects_changed_elf_and_manifest_without_rewriting() {
    check_pin("verifier-pin", "@VERIFIER_HASH_BYTES@", "@VERIFIER_HASH@");
}
