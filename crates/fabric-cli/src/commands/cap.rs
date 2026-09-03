//! `fabric cap` subcommand.

use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;

use fabric_capability::{
    descriptor::CapabilityDescriptor,
    probe::default_probe,
    schema, signing,
};

use crate::output;

#[derive(Args, Debug)]
pub struct ProbeArgs {
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub pretty: bool,
}

#[derive(Args, Debug)]
pub struct SignArgs {
    #[arg(short, long)]
    pub descriptor: PathBuf,
    #[arg(short, long)]
    pub key: PathBuf,
}

#[derive(Args, Debug)]
pub struct VerifyArgs {
    #[arg(short, long)]
    pub descriptor: PathBuf,
    #[arg(short, long)]
    pub key: PathBuf,
}

#[derive(Args, Debug)]
pub struct ExportArgs {
    #[arg(short, long)]
    pub descriptor: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
}

#[derive(Args, Debug)]
pub struct ImportNvmsArgs {
    #[arg(short, long)]
    pub manifest: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
}

#[derive(Args, Debug)]
pub struct ValidateArgs {
    #[arg(short, long)]
    pub descriptor: PathBuf,
}

pub fn dispatch(sub: &super::CapCommand, _workspace: &std::path::Path) -> Result<()> {
    match sub {
        super::CapCommand::Probe(a) => probe(a),
        super::CapCommand::Sign(a) => sign(a),
        super::CapCommand::Verify(a) => verify(a),
        super::CapCommand::Export(a) => export(a),
        super::CapCommand::ImportNvms(a) => import_nvms(a),
        super::CapCommand::Validate(a) => validate(a),
    }
}

fn probe(args: &ProbeArgs) -> Result<()> {
    let probe = default_probe().context("no probe implementation available")?;
    let mut descriptor = probe.probe().context("capability probe failed on this host")?;
    let key = signing::SigningKey::generate();
    signing::sign(&mut descriptor, &key).context("auto-sign failed")?;
    let format = output::resolve_format(args.json, args.pretty);
    if let Some(out_path) = &args.output {
        let json = serde_json::to_string_pretty(&descriptor)?;
        std::fs::write(out_path, json).context("write descriptor")?;
        eprintln!("wrote {}", out_path.display());
    } else {
        output::emit(format, &descriptor, || format!("{:#?}\n", descriptor))?;
    }
    Ok(())
}

fn sign(args: &SignArgs) -> Result<()> {
    let json = std::fs::read_to_string(&args.descriptor)
        .with_context(|| format!("read {}", args.descriptor.display()))?;
    let mut descriptor: CapabilityDescriptor = serde_json::from_str(&json).context("parse descriptor JSON")?;
    let key_bytes = std::fs::read(&args.key).with_context(|| format!("read {}", args.key.display()))?;
    let key_array: [u8; 32] = key_bytes.as_slice().try_into()
        .map_err(|_| anyhow::anyhow!("signing key must be exactly 32 bytes"))?;
    let key = signing::SigningKey::from_bytes(&key_array).context("invalid signing key bytes")?;
    signing::sign(&mut descriptor, &key).context("sign")?;
    println!("signed with key id {}", key.key_id());
    println!("topology_hash: {}", descriptor.topology_hash());
    Ok(())
}

fn verify(args: &VerifyArgs) -> Result<()> {
    let json = std::fs::read_to_string(&args.descriptor)
        .with_context(|| format!("read {}", args.descriptor.display()))?;
    let descriptor: CapabilityDescriptor = serde_json::from_str(&json).context("parse descriptor JSON")?;
    let key_bytes = std::fs::read(&args.key).with_context(|| format!("read {}", args.key.display()))?;
    let key_array: [u8; 32] = key_bytes.as_slice().try_into()
        .map_err(|_| anyhow::anyhow!("verification key must be exactly 32 bytes"))?;
    let vk = signing::VerificationKey::from_bytes(&key_array).context("invalid verification key bytes")?;
    let trusted_ids: Vec<String> = vec![vk.key_id().to_string()];
    let valid = signing::verify(&descriptor, &vk).is_ok();
    let trusted = descriptor.has_trusted_signature(&trusted_ids);
    if valid && trusted {
        println!("OK: descriptor signature valid and trusted ({})", vk.key_id());
        Ok(())
    } else {
        anyhow::bail!("verification failed (valid={}, trusted={})", valid, trusted)
    }
}

fn export(args: &ExportArgs) -> Result<()> {
    let json = std::fs::read_to_string(&args.descriptor)
        .with_context(|| format!("read {}", args.descriptor.display()))?;
    let descriptor: CapabilityDescriptor = serde_json::from_str(&json).context("parse descriptor JSON")?;
    let pretty = serde_json::to_string_pretty(&descriptor)?;
    std::fs::write(&args.output, pretty).with_context(|| format!("write {}", args.output.display()))?;
    println!("exported {} bytes to {}", pretty.len(), args.output.display());
    Ok(())
}

fn import_nvms(args: &ImportNvmsArgs) -> Result<()> {
    use phenotype_nvms_adapter::{nvms_to_capability_descriptor, NVMS_VERSION};
    let yaml = std::fs::read_to_string(&args.manifest)
        .with_context(|| format!("read {}", args.manifest.display()))?;
    let descriptor = nvms_to_capability_descriptor(&yaml, None)
        .context("convert NVMS manifest to CapabilityDescriptor")?;
    let json = serde_json::to_string_pretty(&descriptor)?;
    std::fs::write(&args.output, json).with_context(|| format!("write {}", args.output.display()))?;
    eprintln!("imported from NVMS v{} manifest", NVMS_VERSION);
    eprintln!("wrote {}", args.output.display());
    Ok(())
}

fn validate(args: &ValidateArgs) -> Result<()> {
    let json = std::fs::read_to_string(&args.descriptor)
        .with_context(|| format!("read {}", args.descriptor.display()))?;
    schema::validate_descriptor(&json).context("schema validation failed")?;
    println!("OK: {} matches capability.schema.json", args.descriptor.display());
    Ok(())
}
