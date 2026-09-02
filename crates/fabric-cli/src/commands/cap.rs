//! `fabric cap` — capability inventory commands.
//!
//! PF-WP-020.06 / spec 015. Surfaces the `fabric-capability` capability
//! inventory to humans. Wraps probe, sign, verify, and inspect.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use clap::Subcommand;
use ed25519_dalek::{SigningKey, VerifyingKey, SECRET_KEY_LENGTH};
use fabric_capability::{probe, schema, signing, CapabilityDescriptor};
use rand::rngs::OsRng;
use serde::Serialize;

use crate::output::{self, OutputFormat};

#[derive(Subcommand, Debug)]
pub enum CapCommand {
    /// Probe the current host and emit a CapabilityDescriptor JSON
    Probe {
        /// Optional override for the topology epoch (default: 1)
        #[arg(long)]
        epoch: Option<u64>,
    },

    /// Inspect a CapabilityDescriptor JSON file
    Inspect {
        /// Path to the descriptor JSON
        path: PathBuf,
    },

    /// Sign a CapabilityDescriptor with a private key
    Sign {
        /// Path to the descriptor JSON
        descriptor: PathBuf,
        /// Path to the signing key (32 raw bytes)
        #[arg(long = "key")]
        key_path: PathBuf,
        /// Output path for the signed descriptor (default: overwrite input)
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Verify the signature on a CapabilityDescriptor
    Verify {
        /// Path to the descriptor JSON
        descriptor: PathBuf,
        /// Path to the verification key (32 raw bytes)
        #[arg(long = "key")]
        key_path: PathBuf,
    },

    /// Generate a new Ed25519 keypair (raw 32-byte format)
    Keygen {
        /// Output path for the signing key
        #[arg(long = "signing-key")]
        signing_key: PathBuf,
        /// Output path for the verification key
        #[arg(long = "verification-key")]
        verification_key: PathBuf,
    },
}

impl CapCommand {
    pub fn run(self, format: OutputFormat) -> Result<()> {
        match self {
            Self::Probe { epoch } => run_probe(epoch, format),
            Self::Inspect { path } => run_inspect(&path, format),
            Self::Sign {
                descriptor,
                key_path,
                out,
            } => run_sign(&descriptor, &key_path, out.as_deref(), format),
            Self::Verify {
                descriptor,
                key_path,
            } => run_verify(&descriptor, &key_path, format),
            Self::Keygen {
                signing_key,
                verification_key,
            } => run_keygen(&signing_key, &verification_key),
        }
    }
}

fn run_probe(epoch_override: Option<u64>, format: OutputFormat) -> Result<()> {
    let p = probe::default_probe();
    let mut descriptor = p.probe().context("probe failed")?;
    if let Some(epoch) = epoch_override {
        descriptor.topology_epoch = epoch;
    }
    if format.is_json() {
        println!("{}", serde_json::to_string_pretty(&descriptor)?);
    } else {
        output::print_descriptor_table(&descriptor);
    }
    Ok(())
}

fn run_inspect(path: &Path, format: OutputFormat) -> Result<()> {
    let d = read_descriptor(path)?;
    if format.is_json() {
        println!("{}", serde_json::to_string_pretty(&d)?);
    } else {
        output::print_descriptor_table(&d);
    }
    Ok(())
}

fn run_sign(
    descriptor_path: &Path,
    key_path: &Path,
    out_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    let mut d = read_descriptor(descriptor_path)?;
    let key = read_signing_key(key_path)?;
    let vk = key.verification_key();
    signing::sign(&mut d, &key).context("signing failed")?;

    let dest = out_path.unwrap_or(descriptor_path);
    write_descriptor(dest, &d)?;

    if format.is_json() {
        let r = SignReport {
            signed: true,
            signer: B64.encode(vk.inner.to_bytes()),
            node_id: d.node_id.to_string(),
            signatures: d.signatures.len(),
        };
        println!("{}", serde_json::to_string_pretty(&r)?);
    } else {
        println!("Signed descriptor: {}", d.node_id);
        println!("Signer: {}", B64.encode(vk.inner.to_bytes()));
        println!("Signatures: {}", d.signatures.len());
        println!("Wrote: {}", dest.display());
    }
    Ok(())
}

fn run_verify(descriptor: &Path, key_path: &Path, format: OutputFormat) -> Result<()> {
    let d = read_descriptor(descriptor)?;
    let vk = read_verification_key(key_path)?;
    match signing::verify(&d, &vk) {
        Ok(()) => {
            if format.is_json() {
                let r = VerifyReport {
                    valid: true,
                    signatures: d.signatures.len(),
                };
                println!("{}", serde_json::to_string_pretty(&r)?);
            } else {
                println!("Signature VALID");
                println!("Signatures: {}", d.signatures.len());
            }
            Ok(())
        }
        Err(e) => {
            if format.is_json() {
                let r = VerifyReport {
                    valid: false,
                    signatures: d.signatures.len(),
                };
                println!("{}", serde_json::to_string_pretty(&r)?);
            } else {
                println!("Signature INVALID: {e}");
            }
            bail!("signature invalid")
        }
    }
}

fn run_keygen(signing: &Path, verification: &Path) -> Result<()> {
    let key = SigningKey::generate(&mut OsRng);
    let vk = key.verification_key();
    std::fs::write(signing, key.to_bytes()).context("write signing key")?;
    std::fs::write(verification, vk.to_bytes()).context("write verification key")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(signing)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(signing, perms)?;
    }

    println!("Generated Ed25519 keypair:");
    println!("  signing:      {} (32 bytes, mode 0600)", signing.display());
    println!("  verification: {} (32 bytes)", verification.display());
    Ok(())
}

fn read_descriptor(p: &Path) -> Result<CapabilityDescriptor> {
    let mut s = String::new();
    std::fs::File::open(p)?.read_to_string(&mut s)?;
    let d: CapabilityDescriptor = serde_json::from_str(&s).context("parse descriptor")?;
    schema::validate_descriptor(&d).context("schema validation failed")?;
    Ok(d)
}

fn write_descriptor(p: &Path, d: &CapabilityDescriptor) -> Result<()> {
    let s = serde_json::to_string_pretty(d)?;
    let mut f = std::fs::File::create(p)?;
    f.write_all(s.as_bytes())?;
    Ok(())
}

fn read_signing_key(p: &Path) -> Result<SigningKey> {
    let bytes = std::fs::read(p).with_context(|| format!("read {}", p.display()))?;
    if bytes.len() != SECRET_KEY_LENGTH {
        bail!(
            "signing key must be {} raw bytes; got {}",
            SECRET_KEY_LENGTH,
            bytes.len()
        );
    }
    let arr: [u8; SECRET_KEY_LENGTH] = bytes.as_slice().try_into()?;
    Ok(SigningKey::from_bytes(&arr))
}

fn read_verification_key(p: &Path) -> Result<VerifyingKey> {
    let bytes = std::fs::read(p).with_context(|| format!("read {}", p.display()))?;
    if bytes.len() != SECRET_KEY_LENGTH {
        bail!(
            "verification key must be {} raw bytes; got {}",
            SECRET_KEY_LENGTH,
            bytes.len()
        );
    }
    let arr: [u8; SECRET_KEY_LENGTH] = bytes.as_slice().try_into()?;
    Ok(VerifyingKey::from_bytes(&arr)?)
}

#[derive(Serialize)]
struct SignReport {
    signed: bool,
    signer: String,
    node_id: String,
    signatures: usize,
}

#[derive(Serialize)]
struct VerifyReport {
    valid: bool,
    signatures: usize,
}
