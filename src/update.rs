use std::{env, fs, path::Path};

#[cfg(windows)]
use std::process::Command;

use reqwest::blocking::Client;
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::Error;

const RELEASES_API: &str = "https://api.github.com/repos/suiflex/kurir/releases/latest";
const RELEASE_PAGE: &str = "https://github.com/suiflex/kurir/releases/latest";

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, serde::Serialize)]
struct UpdateReport<'a> {
    status: &'a str,
    current: &'a str,
    latest: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    changed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

/// Check for the latest release and optionally replace the running binary.
///
/// # Errors
///
/// Returns an error when release metadata, assets, checksum verification, or
/// executable replacement fails.
pub fn run(check_only: bool, json: bool) -> Result<(), Error> {
    let current = env!("CARGO_PKG_VERSION");
    let release = fetch_latest()?;
    let latest = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name);
    let current_version =
        Version::parse(current).map_err(|error| Error::Update(error.to_string()))?;
    let latest_version =
        Version::parse(latest).map_err(|error| Error::Update(error.to_string()))?;

    if latest_version <= current_version {
        return report(
            json,
            UpdateReport {
                status: "up_to_date",
                current,
                latest,
                changed: Some(false),
                message: Some("already on the latest release".to_owned()),
            },
        );
    }
    if check_only {
        return report(
            json,
            UpdateReport {
                status: "update_available",
                current,
                latest,
                changed: Some(false),
                message: Some(format!("run `kurir update` or visit {RELEASE_PAGE}")),
            },
        );
    }

    let target = target_name()?;
    let binary_name = if cfg!(windows) { "kurir.exe" } else { "kurir" };
    let suffix = binary_suffix();
    let asset_name = format!("kurir-{latest}-{target}{suffix}");
    let checksum_name = format!("{asset_name}.sha256");
    let binary_asset = find_asset(&release, &asset_name)?;
    let checksum_asset = find_asset(&release, &checksum_name)?;
    let client = http_client()?;
    let binary = client
        .get(&binary_asset.browser_download_url)
        .send()
        .map_err(|error| Error::Update(error.to_string()))?
        .error_for_status()
        .map_err(|error| Error::Update(error.to_string()))?
        .bytes()
        .map_err(|error| Error::Update(error.to_string()))?;
    let checksum_text = client
        .get(&checksum_asset.browser_download_url)
        .send()
        .map_err(|error| Error::Update(error.to_string()))?
        .error_for_status()
        .map_err(|error| Error::Update(error.to_string()))?
        .text()
        .map_err(|error| Error::Update(error.to_string()))?;
    verify_checksum(&binary, &checksum_text)?;

    let executable = env::current_exe()
        .map_err(|error| Error::Update(error.to_string()))?
        .canonicalize()
        .map_err(|error| Error::Update(error.to_string()))?;
    replace_executable(&executable, &binary, binary_name)?;
    report(
        json,
        UpdateReport {
            status: "updated",
            current,
            latest,
            changed: Some(true),
            message: Some("updated successfully; restart Kurir".to_owned()),
        },
    )
}

fn fetch_latest() -> Result<Release, Error> {
    http_client()?
        .get(RELEASES_API)
        .send()
        .map_err(|error| Error::Update(error.to_string()))?
        .error_for_status()
        .map_err(|error| Error::Update(error.to_string()))?
        .json()
        .map_err(|error| Error::Update(error.to_string()))
}

fn http_client() -> Result<Client, Error> {
    Client::builder()
        .user_agent(format!("kurir/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| Error::Update(error.to_string()))
}

fn find_asset<'a>(release: &'a Release, name: &str) -> Result<&'a Asset, Error> {
    release
        .assets
        .iter()
        .find(|asset| asset.name == name)
        .ok_or_else(|| Error::Update(format!("release asset {name} is missing")))
}

fn verify_checksum(bytes: &[u8], checksum_text: &str) -> Result<(), Error> {
    let expected = checksum_text
        .split_whitespace()
        .next()
        .ok_or_else(|| Error::Update("checksum asset is empty".to_owned()))?;
    let actual = format!("{:x}", Sha256::digest(bytes));
    if expected != actual {
        return Err(Error::Update(format!(
            "checksum mismatch: expected {expected}, got {actual}"
        )));
    }
    Ok(())
}

fn replace_executable(executable: &Path, bytes: &[u8], binary_name: &str) -> Result<(), Error> {
    let temporary =
        executable.with_file_name(format!(".{binary_name}.update-{}", std::process::id()));
    fs::write(&temporary, bytes).map_err(|error| Error::Update(error.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o755))
            .map_err(|error| Error::Update(error.to_string()))?;
        fs::rename(&temporary, executable).map_err(|error| Error::Update(error.to_string()))?;
    }
    #[cfg(windows)]
    {
        let source = temporary.display().to_string();
        let destination = executable.display().to_string();
        let script = format!(
            "Start-Sleep -Milliseconds 500; Move-Item -Force -LiteralPath '{}' -Destination '{}'",
            source.replace('\'', "''"),
            destination.replace('\'', "''")
        );
        Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .spawn()
            .map_err(|error| Error::Update(error.to_string()))?;
    }
    Ok(())
}

fn report(json: bool, report: UpdateReport<'_>) -> Result<(), Error> {
    if json {
        println!(
            "{}",
            serde_json::to_string(&report).map_err(|error| Error::Update(error.to_string()))?
        );
    } else {
        println!("{}: {} → {}", report.status, report.current, report.latest);
        if let Some(message) = report.message {
            println!("{message}");
        }
    }
    Ok(())
}

fn target_name() -> Result<&'static str, Error> {
    match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => Ok("linux-x86_64"),
        ("linux", "aarch64") => Ok("linux-aarch64"),
        ("macos", "x86_64") => Ok("darwin-x86_64"),
        ("macos", "aarch64") => Ok("darwin-aarch64"),
        ("windows", "x86_64") => Ok("windows-x86_64"),
        ("windows", "aarch64") => Ok("windows-aarch64"),
        _ => Err(Error::Update(format!(
            "unsupported update platform {}-{}",
            env::consts::OS,
            env::consts::ARCH
        ))),
    }
}

fn binary_suffix() -> &'static str {
    if cfg!(windows) { ".exe" } else { "" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_verification_accepts_sha256sum_format() {
        let bytes = b"kurir";
        let digest = format!("{:x}", Sha256::digest(bytes));
        let checksum = format!("{digest}  kurir\n");
        verify_checksum(bytes, &checksum).expect("checksum");
    }

    #[test]
    fn checksum_verification_rejects_wrong_content() {
        assert!(verify_checksum(b"kurir", "deadbeef  kurir").is_err());
    }
}
