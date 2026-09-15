use anyhow::{bail, Result};
use serde::Deserialize;

const GITHUB_REPO: &str = "kooshapari/terminal-fabric";

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

/// Run the self-update process for the given binary name.
pub fn run(binary_name: &str) -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");
    println!("Current version: {}", current_version);

    // Check latest release from GitHub
    let api_url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );
    let mut resp = ureq::get(&api_url)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", format!("{}-self-update", binary_name))
        .call()?;

    let body: String = resp.body_mut().read_to_string()?;

    let release: GitHubRelease = serde_json::from_str(&body)?;
    let latest_version = release.tag_name.trim_start_matches('v');

    println!("Latest version:  {}", latest_version);

    if latest_version == current_version {
        println!("Already up to date.");
        return Ok(());
    }

    println!("Updating {} from {} to {}...", binary_name, current_version, latest_version);

    // Download the new executable
    let download_url = format!(
        "https://github.com/{}/releases/download/v{}/{}.exe",
        GITHUB_REPO, latest_version, binary_name
    );

    let mut resp = ureq::get(&download_url)
        .header("User-Agent", format!("{}-self-update", binary_name))
        .call()?;

    let exe_bytes = resp.body_mut().read_to_vec()?;

    if exe_bytes.len() < 2 {
        bail!("Downloaded file is too small to be a valid executable");
    }

    // Verify MZ header (PE executable signature)
    if exe_bytes[0] != b'M' || exe_bytes[1] != b'Z' {
        bail!(
            "Downloaded file is not a valid PE executable (expected MZ header, got {:02X} {:02X})",
            exe_bytes[0], exe_bytes[1]
        );
    }

    println!("Downloaded {} bytes, PE header verified.", exe_bytes.len());

    // Get paths for atomic replacement
    let current_exe = std::env::current_exe()?;
    let bak_path = current_exe.with_extension("exe.bak");
    let tmp_path = current_exe.with_extension("exe.new");

    // Backup current executable
    if bak_path.exists() {
        std::fs::remove_file(&bak_path)?;
    }
    std::fs::copy(&current_exe, &bak_path)?;
    println!("Backed up current exe to {}", bak_path.display());

    // Write new executable to temp file
    std::fs::write(&tmp_path, &exe_bytes)?;

    // Atomic replacement: rename old -> bak, rename new -> current
    // On Windows, rename fails if destination exists, so remove first.
    // On Unix, rename is atomic even over existing files.
    #[cfg(windows)]
    {
        std::fs::remove_file(&current_exe)?;
    }
    std::fs::rename(&tmp_path, &current_exe)?;

    println!("Updated {} from {} to {}", binary_name, current_version, latest_version);
    println!("Restart {} to use the new version.", binary_name);

    Ok(())
}
