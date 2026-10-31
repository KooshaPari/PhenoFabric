//! Print Fabric workspace version information.

pub fn execute() -> anyhow::Result<()> {
    println!("fabric-cli {}", env!("CARGO_PKG_VERSION"));
    println!("Fabric workspace v{}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
