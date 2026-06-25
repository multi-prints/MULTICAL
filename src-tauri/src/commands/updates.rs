use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResult {
    pub available: bool,
    pub version: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
struct UpdateManifest {
    version: String,
    #[serde(rename = "notes")]
    _notes: Option<String>,
    #[serde(rename = "pub_date")]
    _pub_date: Option<String>,
    platforms: HashMap<String, UpdateAsset>,
}

#[derive(Debug, Clone, Deserialize)]
struct UpdateAsset {
    url: String,
    sha256: String,
}

#[allow(unreachable_code)]
#[tauri::command]
pub async fn check_and_install_update(app: tauri::AppHandle) -> Result<UpdateResult, String> {
    let manifest = fetch_manifest().await?;
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    if manifest.version == current_version {
        return Ok(UpdateResult {
            available: false,
            version: None,
            message: "Already on the latest version.".into(),
        });
    }

    let platform_key = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    let asset = manifest.platforms.get(&platform_key).ok_or_else(|| {
        format!(
            "No update asset is published for platform '{}'",
            platform_key
        )
    })?;

    let installer_path = download_installer(&asset.url)?;
    let bytes = std::fs::read(&installer_path).map_err(|e| e.to_string())?;
    verify_sha256(&bytes, &asset.sha256)?;
    run_installer(&installer_path)?;
    app.restart();

    Ok(UpdateResult {
        available: true,
        version: Some(manifest.version),
        message: "Update installed. Restarting now.".into(),
    })
}

async fn fetch_manifest() -> Result<UpdateManifest, String> {
    let manifest_url =
        "https://github.com/multi-prints/MULTICAL/releases/latest/download/latest.json";
    let manifest_path = std::env::temp_dir().join("multiprints-update-manifest.json");
    download_file(manifest_url, &manifest_path)?;
    let manifest_json = std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    serde_json::from_str(&manifest_json).map_err(|e| e.to_string())
}

fn verify_sha256(bytes: &[u8], expected: &str) -> Result<(), String> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let actual = hex::encode(hasher.finalize());
    if actual == expected.to_lowercase() {
        Ok(())
    } else {
        Err("Downloaded update failed checksum validation".into())
    }
}

fn download_installer(url: &str) -> Result<PathBuf, String> {
    let ext = if url.ends_with(".exe") {
        "exe"
    } else if url.ends_with(".deb") {
        "deb"
    } else {
        "bin"
    };
    let path = std::env::temp_dir().join(format!("multiprints-update.{ext}"));
    download_file(url, &path)?;
    Ok(path)
}

fn download_file(url: &str, path: &Path) -> Result<(), String> {
    if cfg!(target_os = "windows") {
        let path_str = powershell_quote(&path.to_string_lossy());
        let url_str = powershell_quote(url);
        let status = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(format!(
                "Invoke-WebRequest -Uri '{url_str}' -OutFile '{path_str}'"
            ))
            .status()
            .map_err(|e| e.to_string())?;
        if status.success() {
            return Ok(());
        }
        return Err("Failed to download update payload".into());
    }

    let curl_status = Command::new("curl")
        .arg("-LfsS")
        .arg(url)
        .arg("-o")
        .arg(path)
        .status();
    if let Ok(status) = curl_status {
        if status.success() {
            return Ok(());
        }
    }

    let wget_status = Command::new("wget")
        .arg("-q")
        .arg("-O")
        .arg(path)
        .arg(url)
        .status();
    if let Ok(status) = wget_status {
        if status.success() {
            return Ok(());
        }
    }

    Err("Failed to download update payload".into())
}

fn powershell_quote(value: &str) -> String {
    value.replace('\'', "''")
}

fn run_installer(path: &Path) -> Result<(), String> {
    if cfg!(target_os = "windows") {
        let status = Command::new(path)
            .arg("/S")
            .status()
            .map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err("Update installer failed".into())
        }
    } else if cfg!(target_os = "linux") {
        let status = Command::new("pkexec")
            .arg("dpkg")
            .arg("-i")
            .arg(path)
            .status()
            .map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err("Update installer failed".into())
        }
    } else {
        Err("Updates are not supported on this platform".into())
    }
}
