//! Evidence preflight must fail before building or invoking any prover.
use std::{path::Path, process::Command};

fn script() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/zk.sh")
}

fn preflight() -> Command {
    let mut command = Command::new("bash");
    command.arg(script()).arg("verify-circuits");
    for name in ["SP1_CIRCUIT_MODE", "WITHOUT_VK_VERIFICATION", "SP1_DUMP"] {
        command.env_remove(name);
    }
    command
}

#[test]
fn unsafe_proving_modes_reject_even_when_set_to_false() {
    for name in ["SP1_CIRCUIT_MODE", "WITHOUT_VK_VERIFICATION", "SP1_DUMP"] {
        let output = preflight().env(name, "false").output().unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(&format!("{name} must be unset")));
    }
}

#[test]
fn existing_circuit_directory_does_not_admit_incomplete_artifacts() {
    let root = std::env::temp_dir().join(format!(
        "agoraseal-circuit-preflight-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(root.join("v6.1.0")).unwrap();
    let check = || {
        let output = preflight()
            .env("SP1_PLONK_CIRCUIT_PATH", &root)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(
            "Circuit artifact must be a regular file of exactly 111997113 bytes: constraints.json"
        ));
    };
    check();
    std::fs::write(root.join("v6.1.0/constraints.json"), b"{}").unwrap();
    check();
    // Even a correctly sized symlink must not become a reviewed circuit file.
    #[cfg(unix)]
    {
        std::fs::remove_file(root.join("v6.1.0/constraints.json")).unwrap();
        let target = root.join("target.json");
        std::fs::File::create(&target)
            .unwrap()
            .set_len(111_997_113)
            .unwrap();
        std::os::unix::fs::symlink(&target, root.join("v6.1.0/constraints.json")).unwrap();
        check();
    }
    std::fs::remove_dir_all(root).unwrap();
}
