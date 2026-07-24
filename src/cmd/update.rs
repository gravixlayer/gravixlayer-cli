// src/cmd/update.rs — Self-update via GitHub Releases (no self_update crate).
//
// Asset contract matches scripts/install.sh and the release workflow:
//   gravixlayer-<tag>-<rust-triple>.tar.gz|.zip + matching .sha256 side-cars.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::cli::{OutputFormat, UpdateArgs};
use crate::output;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const REPO_OWNER: &str = "gravixlayer";
const REPO_NAME: &str = "gravixlayer-cli";
const BIN_NAME: &str = "gravixlayer";
const USER_AGENT: &str = concat!("gravixlayer-cli/", env!("CARGO_PKG_VERSION"));

pub async fn handle(output_fmt: OutputFormat, args: UpdateArgs) -> Result<()> {
    output::info(output_fmt, format!("Current version: {CURRENT_VERSION}"));

    let check_only = args.check;
    let target_version = args
        .version
        .as_deref()
        .map(|v| v.trim().trim_start_matches('v').to_string());

    if check_only && target_version.is_some() {
        bail!("--check and --version cannot be used together");
    }

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .context("build HTTP client")?;

    let tag = match target_version {
        Some(v) => format!("v{v}"),
        None => fetch_latest_tag(&client).await?,
    };
    let remote = tag.trim_start_matches('v').to_string();

    if remote == CURRENT_VERSION {
        output::success(
            output_fmt,
            format!("Already up to date ({CURRENT_VERSION})."),
        );
        return Ok(());
    }

    if check_only {
        output::info(output_fmt, format!("New version available: {remote}"));
        output::info(output_fmt, "Run `gravixlayer update` to upgrade.");
        return Ok(());
    }

    let triple = release_target_triple()?;
    let archive_name = archive_name(&tag, triple);
    let archive_url = format!(
        "https://github.com/{REPO_OWNER}/{REPO_NAME}/releases/download/{tag}/{archive_name}"
    );
    let checksum_url = format!("{archive_url}.sha256");

    output::info(output_fmt, format!("Downloading {archive_url}"));
    let archive_bytes = download_bytes(&client, &archive_url)
        .await
        .with_context(|| format!("download {archive_url}"))?;
    let checksum_text = download_text(&client, &checksum_url)
        .await
        .with_context(|| format!("download {checksum_url}"))?;
    let expected = parse_checksum_line(&checksum_text)
        .context("parse SHA-256 sidecar (expected `<hash>  <filename>`)")?;
    let actual = hex_sha256(&archive_bytes);
    if !actual.eq_ignore_ascii_case(&expected) {
        bail!("checksum mismatch for {archive_name}: expected {expected}, got {actual}");
    }

    let tmp = tempfile::tempdir().context("create temp dir for update")?;
    let extracted =
        extract_binary(&archive_bytes, tmp.path()).context("extract release archive")?;

    self_replace::self_replace(&extracted)
        .context("replace running binary (may need write permission on the install directory)")?;

    output::success(
        output_fmt,
        format!("Updated to {remote}. Run `gravixlayer --version` to confirm."),
    );
    Ok(())
}

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
}

async fn fetch_latest_tag(client: &reqwest::Client) -> Result<String> {
    let url = format!("https://api.github.com/repos/{REPO_OWNER}/{REPO_NAME}/releases/latest");
    let release: GhRelease = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .with_context(|| format!("GET {url}"))?
        .error_for_status()
        .with_context(|| format!("GET {url} returned error"))?
        .json()
        .await
        .context("decode GitHub release JSON")?;
    if release.tag_name.trim().is_empty() {
        bail!("GitHub latest release has empty tag_name");
    }
    Ok(release.tag_name)
}

async fn download_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    let resp = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("GET {url}"))?
        .error_for_status()
        .with_context(|| format!("GET {url} returned error"))?;
    Ok(resp.bytes().await?.to_vec())
}

async fn download_text(client: &reqwest::Client, url: &str) -> Result<String> {
    let resp = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("GET {url}"))?
        .error_for_status()
        .with_context(|| format!("GET {url} returned error"))?;
    Ok(resp.text().await?)
}

fn parse_checksum_line(text: &str) -> Result<String> {
    let line = text
        .lines()
        .map(|l| l.trim().trim_start_matches('\u{feff}'))
        .find(|l| !l.is_empty())
        .context("checksum file is empty")?;
    let hash = line
        .split_whitespace()
        .next()
        .context("checksum line missing hash")?;
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("invalid SHA-256 hash in checksum file: {hash}");
    }
    Ok(hash.to_ascii_lowercase())
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn archive_name(tag: &str, triple: &str) -> String {
    if cfg!(windows) {
        format!("{BIN_NAME}-{tag}-{triple}.zip")
    } else {
        format!("{BIN_NAME}-{tag}-{triple}.tar.gz")
    }
}

fn release_target_triple() -> Result<&'static str> {
    // Must match the release workflow / install.sh platform matrix.
    Ok(match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "x86_64-unknown-linux-musl",
        ("linux", "aarch64") => "aarch64-unknown-linux-musl",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        ("windows", "aarch64") => "aarch64-pc-windows-msvc",
        (os, arch) => bail!("unsupported platform for self-update: {os}/{arch}"),
    })
}

fn extract_binary(archive: &[u8], dest_dir: &Path) -> Result<PathBuf> {
    #[cfg(windows)]
    {
        extract_zip(archive, dest_dir)
    }
    #[cfg(not(windows))]
    {
        extract_tar_gz(archive, dest_dir)
    }
}

#[cfg(not(windows))]
fn extract_tar_gz(archive: &[u8], dest_dir: &Path) -> Result<PathBuf> {
    let dec = flate2::read::GzDecoder::new(Cursor::new(archive));
    let mut tar = tar::Archive::new(dec);
    tar.unpack(dest_dir)
        .context("unpack tar.gz release archive")?;

    let candidate = dest_dir.join(BIN_NAME);
    if candidate.is_file() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&candidate)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&candidate, perms)?;
        }
        return Ok(candidate);
    }
    bail!("binary `{BIN_NAME}` not found in archive");
}

#[cfg(windows)]
fn extract_zip(archive: &[u8], dest_dir: &Path) -> Result<PathBuf> {
    let mut zip = zip::ZipArchive::new(Cursor::new(archive)).context("open zip archive")?;
    let exe_name = format!("{BIN_NAME}.exe");
    let mut found: Option<PathBuf> = None;

    for i in 0..zip.len() {
        let mut file = zip.by_index(i).context("read zip entry")?;
        let name = file
            .enclosed_name()
            .map(|p| p.to_path_buf())
            .context("zip entry has unsafe path")?;
        let out_path = dest_dir.join(&name);
        if file.is_dir() {
            std::fs::create_dir_all(&out_path)?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = std::fs::File::create(&out_path)?;
        std::io::copy(&mut file, &mut out)?;
        if name.file_name().and_then(|s| s.to_str()) == Some(BIN_NAME)
            || name.file_name().and_then(|s| s.to_str()) == Some(exe_name.as_str())
        {
            found = Some(out_path);
        }
    }

    found.context(format!("binary `{BIN_NAME}` not found in zip archive"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_checksum_sidecar() {
        let hash = "a".repeat(64);
        assert_eq!(
            parse_checksum_line(&format!(
                "{hash}  gravixlayer-v0.1.0-x86_64-apple-darwin.tar.gz"
            ))
            .unwrap(),
            hash
        );
        assert_eq!(parse_checksum_line(&format!("{hash}\r\n")).unwrap(), hash);
    }

    #[test]
    fn rejects_bad_checksum() {
        assert!(parse_checksum_line("not-a-hash file.tar.gz").is_err());
        assert!(parse_checksum_line("").is_err());
    }
}
