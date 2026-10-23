use super::{Phase, archive};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
const MAX_ARCHIVE: u64 = 512 * 1024 * 1024;
#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    size: u64,
}
fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(Duration::from_secs(15))
        .timeout_write(Duration::from_secs(15))
        .redirects(5)
        .https_only(true)
        .timeout(Duration::from_secs(600))
        .build()
}
fn get(agent: &ureq::Agent, url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let response = agent
        .get(url)
        .timeout(Duration::from_secs(15))
        .set("User-Agent", concat!("Ada3D/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("Update response exceeds the size limit.".into());
    }
    Ok(bytes)
}
fn newer(tag: &str, current: &str) -> Result<bool, String> {
    let version = semver::Version::parse(tag.trim_start_matches('v'))
        .map_err(|_| "The release tag is not a valid version.")?;
    let current = semver::Version::parse(current).map_err(|e| e.to_string())?;
    Ok(version.pre.is_empty() && version > current)
}
pub(super) fn check_and_download(
    repo: &str,
    automatic: bool,
    report: &impl Fn(Phase, String, Option<f32>),
) -> Result<Option<(String, PathBuf)>, String> {
    let agent = agent();
    let bytes = get(
        &agent,
        &format!("https://api.github.com/repos/{repo}/releases/latest"),
        2 * 1024 * 1024,
    )?;
    let release: Release =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid release metadata: {e}"))?;
    if release.draft || release.prerelease || !newer(&release.tag_name, env!("CARGO_PKG_VERSION"))?
    {
        return Ok(None);
    }
    let version = release.tag_name.trim_start_matches('v').to_owned();
    let kind = super::packages::installed();
    let name = format!("Ada3D-v{version}-{}{}", super::platform(), kind.suffix());
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == name)
        .ok_or("This release does not include a package for this platform.")?;
    let checksum_name = format!("{name}.sha256");
    let checksum = release
        .assets
        .iter()
        .find(|a| a.name == checksum_name)
        .ok_or("The release checksum is missing.")?;
    let prefix = format!(
        "https://github.com/{repo}/releases/download/{}/",
        release.tag_name
    );
    if asset.browser_download_url != format!("{prefix}{name}")
        || checksum.browser_download_url != format!("{prefix}{checksum_name}")
    {
        return Err("The release points outside the configured repository.".into());
    }
    if asset.size == 0 || asset.size > MAX_ARCHIVE {
        return Err("The update package size is invalid.".into());
    }
    let expected = get(&agent, &checksum.browser_download_url, 1024)?;
    let expected = std::str::from_utf8(&expected)
        .map_err(|_| "Invalid update checksum")?
        .split_whitespace()
        .next()
        .ok_or("Empty update checksum")?
        .to_ascii_lowercase();
    if expected.len() != 64 || !expected.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("Invalid update checksum.".into());
    }
    let cache = crate::settings::default_path()
        .ok_or("No writable update directory")?
        .parent()
        .ok_or("No update directory")?
        .join("updates");
    if automatic
        && let Ok(marker) = fs::read(cache.join("last-install-attempt.json"))
        && let Ok(marker) = serde_json::from_slice::<serde_json::Value>(&marker)
        && marker["version"].as_str() == Some(&version)
        && let Some(time) = marker["time"].as_u64()
        && SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs()
            .saturating_sub(time)
            < 3600
    {
        return Err(
            "The previous update did not finish. Check for updates in Settings to retry.".into(),
        );
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    let stage = cache.join(format!("update-{}-{nonce}", std::process::id()));
    fs::create_dir(&stage).map_err(|e| e.to_string())?;
    let result = (|| {
        let mut input = agent
            .get(&asset.browser_download_url)
            .set("User-Agent", "Ada3D")
            .call()
            .map_err(|e| e.to_string())?
            .into_reader();
        let mut output = File::create(stage.join(kind.staged_name())).map_err(|e| e.to_string())?;
        let mut digest = Sha256::new();
        let mut buffer = [0u8; 65536];
        let mut downloaded = 0u64;
        report(
            Phase::Downloading,
            format!("Updating to {version}..."),
            Some(0.0),
        );
        loop {
            let count = input.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            downloaded += count as u64;
            if downloaded > asset.size || downloaded > MAX_ARCHIVE {
                return Err("Update download exceeded its declared size.".into());
            }
            output
                .write_all(&buffer[..count])
                .map_err(|e| e.to_string())?;
            digest.update(&buffer[..count]);
            report(
                Phase::Downloading,
                format!(
                    "Updating - {:.1} / {:.1} MB",
                    downloaded as f64 / 1048576.0,
                    asset.size as f64 / 1048576.0
                ),
                Some(downloaded as f32 / asset.size as f32),
            );
        }
        output.sync_all().map_err(|e| e.to_string())?;
        drop(output);
        if downloaded != asset.size || format!("{:x}", digest.finalize()) != expected {
            return Err("Update checksum verification failed.".into());
        }
        report(
            Phase::Verifying,
            "Updating - verifying package...".into(),
            Some(1.0),
        );
        let package = stage.join(kind.staged_name());
        let mut header = [0; 8];
        File::open(&package)
            .and_then(|mut file| file.read_exact(&mut header))
            .map_err(|e| e.to_string())?;
        if !kind.valid_header(&header) {
            return Err("The update package format is invalid.".into());
        }
        if kind == super::packages::Kind::Portable {
            archive::unpack(&package, &stage.join("payload"))?;
        }
        Ok(Some((version, stage.clone())))
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&stage);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn comparison_uses_semantic_versions_and_never_downgrades() {
        assert!(newer("v1.10.0", "1.9.0").unwrap());
        for tag in ["v1.0.0", "v0.9.0", "v2.0.0-rc.1"] {
            assert!(!newer(tag, "1.0.0").unwrap());
        }
        assert!(newer("../../malicious", "1.0.0").is_err());
    }
}
