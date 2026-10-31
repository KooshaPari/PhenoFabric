//! Tests for fabric CLI version subcommand.

use std::process::Command;

#[test]
fn version_output_uses_crate_version() {
    let bin = env!("CARGO_BIN_EXE_fabric");
    let output = Command::new(bin)
        .arg("version")
        .output()
        .expect("failed to run fabric version");
    assert!(
        output.status.success(),
        "fabric version failed: {:?}",
        output
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let crate_version = env!("CARGO_PKG_VERSION");

    assert!(
        stdout.contains(&format!("fabric-cli {}", crate_version)),
        "missing fabric-cli version line: {}",
        stdout
    );
    assert!(
        stdout.contains(&format!("Fabric workspace v{}", crate_version)),
        "missing Fabric workspace version line: {}",
        stdout
    );
    // Regression guard: a hardcoded stale literal must never reappear.
    assert!(
        !stdout.contains("Fabric workspace v0.1.0"),
        "hardcoded stale version literal leaked: {}",
        stdout
    );
}
