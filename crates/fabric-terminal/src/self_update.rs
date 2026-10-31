use anyhow::{Result, bail};
use serde::Deserialize;

const GITHUB_REPO: &str = "KooshaPari/PhenoFabric";

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

    println!(
        "Updating {} from {} to {}...",
        binary_name, current_version, latest_version
    );

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
            exe_bytes[0],
            exe_bytes[1]
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

    println!(
        "Updated {} from {} to {}",
        binary_name, current_version, latest_version
    );
    println!("Restart {} to use the new version.", binary_name);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::GITHUB_REPO;

    /// A PII sweep replaced the owner segment of `GITHUB_REPO` with the
    /// literal `<REDACTED>` in commit 6840d4f (2026-09-16). The constant is
    /// interpolated straight into a GitHub API URL, so the redaction silently
    /// produced a URL that cannot resolve:
    ///
    ///     https://api.github.com/repos/<REDACTED>/terminal-fabric/releases/latest
    ///
    /// It only fails at runtime, on a machine trying to self-update, which is
    /// why it went unnoticed. These tests pin the constant so a redaction
    /// cannot break it again.
    #[test]
    fn github_repo_is_not_redacted() {
        assert!(
            !GITHUB_REPO.contains("REDACTED"),
            "GITHUB_REPO is redacted, so the self-update URL cannot resolve: \
             https://api.github.com/repos/{GITHUB_REPO}/releases/latest"
        );
    }

    #[test]
    fn github_repo_is_owner_slash_name() {
        let parts: Vec<&str> = GITHUB_REPO.split('/').collect();
        assert_eq!(
            parts.len(),
            2,
            "GITHUB_REPO must be exactly `owner/name`, got {GITHUB_REPO:?}"
        );
        assert!(!parts[0].is_empty(), "owner segment must not be empty");
        assert!(!parts[1].is_empty(), "name segment must not be empty");
    }

    #[test]
    fn github_repo_names_the_current_repository() {
        // terminal-fabric was absorbed into phenotype-fabric. The old path is
        // 404 and can never serve a release.
        assert_eq!(
            GITHUB_REPO, "KooshaPari/PhenoFabric",
            "self-update must read releases from the repository that publishes them"
        );
    }

    #[test]
    fn release_api_url_is_well_formed() {
        let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");
        assert!(
            url.starts_with("https://api.github.com/repos/KooshaPari/"),
            "release URL has the wrong shape: {url}"
        );
        assert!(
            !url.contains('<') && !url.contains('>'),
            "release URL contains angle brackets: {url}"
        );
    }
}
