//! Verifier: each cmd/checker testdata JSON round-trips through the real
//! `fabric_capability::CapabilityDescriptor` type.
//!
//! Run with:
//!     cargo run --example verify_fixtures -p fabric-capability -- <path>
use fabric_capability::CapabilityDescriptor;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn main() {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./cmd/checker/testdata".to_string());
    let dir = PathBuf::from(dir);

    let entries = match std::fs::read_dir(&dir) {
        Ok(it) => it,
        Err(e) => {
            eprintln!("cannot read testdata dir {dir:?}: {e}");
            std::process::exit(2);
        }
    };

    let mut descriptor_files: Vec<PathBuf> = Vec::new();
    let mut manifest_files: Vec<PathBuf> = Vec::new();
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().map(|x| x == "json").unwrap_or(false) {
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            if name.contains("descriptor") {
                descriptor_files.push(p);
            } else if name.contains("manifest") {
                manifest_files.push(p);
            }
        }
    }
    descriptor_files.sort();
    manifest_files.sort();

    println!("=== descriptor fixtures ({}) ===", descriptor_files.len());
    let mut desc_results: BTreeMap<String, String> = BTreeMap::new();
    for desc in &descriptor_files {
        let raw = match std::fs::read_to_string(desc) {
            Ok(s) => s,
            Err(e) => {
                desc_results.insert(
                    desc.file_name().unwrap().to_string_lossy().to_string(),
                    format!("READ-ERR {e}"),
                );
                continue;
            }
        };
        match serde_json::from_str::<CapabilityDescriptor>(&raw) {
            Ok(d) => {
                let mem = d
                    .capabilities
                    .compute
                    .as_ref()
                    .map(|c| c.memory_bytes);
                let cores = d
                    .capabilities
                    .compute
                    .as_ref()
                    .map(|c| (c.cores_physical, c.cores_logical));
                let proc = d
                    .capabilities
                    .compute
                    .as_ref()
                    .map(|c| c.processor.clone());
                desc_results.insert(
                    desc.file_name().unwrap().to_string_lossy().to_string(),
                    format!(
                        "OK  epoch={} mem={:?} cores={:?} proc={:?}",
                        d.epoch, mem, cores, proc
                    ),
                );
            }
            Err(e) => {
                desc_results.insert(
                    desc.file_name().unwrap().to_string_lossy().to_string(),
                    format!("PARSE-ERR {e}"),
                );
            }
        }
    }
    for (k, v) in &desc_results {
        println!("  {k:50}  {v}");
    }
    let d_ok = desc_results.values().filter(|v| v.starts_with("OK")).count();
    let d_err = desc_results.len() - d_ok;

    println!("\n=== manifest fixtures ({}) ===", manifest_files.len());
    let mut man_results: BTreeMap<String, String> = BTreeMap::new();
    for mp in &manifest_files {
        let raw = match std::fs::read_to_string(mp) {
            Ok(s) => s,
            Err(e) => {
                man_results.insert(
                    mp.file_name().unwrap().to_string_lossy().to_string(),
                    format!("READ-ERR {e}"),
                );
                continue;
            }
        };
        match serde_json::from_str::<Value>(&raw) {
            Ok(v) => {
                let n = v.as_object().map(|o| o.len()).unwrap_or(0);
                man_results.insert(
                    mp.file_name().unwrap().to_string_lossy().to_string(),
                    format!("OK  fields={n}"),
                );
            }
            Err(e) => {
                man_results.insert(
                    mp.file_name().unwrap().to_string_lossy().to_string(),
                    format!("PARSE-ERR {e}"),
                );
            }
        }
    }
    for (k, v) in &man_results {
        println!("  {k:50}  {v}");
    }
    let m_ok = man_results.values().filter(|v| v.starts_with("OK")).count();
    let m_err = man_results.len() - m_ok;

    println!(
        "\n=== summary ===\n  descriptors: ok={d_ok}  err={d_err}\n  manifests:   ok={m_ok}  err={m_err}"
    );
    if d_err > 0 || m_err > 0 {
        std::process::exit(1);
    }
}
