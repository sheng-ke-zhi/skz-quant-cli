use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::error::Error;

const REPOSITORY: &str = "sheng-ke-zhi/skz-quant-cli";
const MAX_INSTALLER: u64 = 128 * 1024 * 1024;

pub(super) fn is_installation(exe: &Path) -> bool {
    exe.parent().is_some_and(|root| {
        fs::read_to_string(root.join(".skz-desktop-install"))
            .is_ok_and(|v| v.trim() == "SKZ WorkBuddy desktop installer")
            && root.join("update.ps1").is_file()
    })
}

fn version(raw: &str) -> Result<[u64; 3], Error> {
    let parts: Vec<_> = raw.split('.').collect();
    if parts.len() != 3
        || parts.iter().any(|p| {
            p.is_empty()
                || !p.bytes().all(|b| b.is_ascii_digit())
                || p.len() > 1 && p.starts_with('0')
        })
    {
        return Err(Error::Args(
            "升级版本号必须是正式的 major.minor.patch".into(),
        ));
    }
    let numbers = parts
        .into_iter()
        .map(str::parse)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| Error::Args("升级版本号无效".into()))?;
    Ok(numbers.try_into().unwrap())
}

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

impl Release {
    fn stable(&self) -> Result<&str, Error> {
        let value = self.tag_name.strip_prefix('v').unwrap_or("");
        version(value)?;
        if self.draft || self.prerelease {
            return Err(Error::Args("升级入口只接受正式发布版本".into()));
        }
        Ok(value)
    }

    fn asset(&self, name: &str) -> Result<&Asset, Error> {
        let found: Vec<_> = self.assets.iter().filter(|a| a.name == name).collect();
        let expected = format!(
            "https://github.com/{REPOSITORY}/releases/download/{}/{name}",
            self.tag_name
        );
        match found.as_slice() {
            [a] if a.browser_download_url == expected && a.size > 0 && a.size <= MAX_INSTALLER => {
                Ok(a)
            }
            _ => Err(Error::Args(format!(
                "正式发布缺少有效的 {name}，原版本未修改"
            ))),
        }
    }
}

fn checksum(text: &str, filename: &str) -> Result<String, Error> {
    let found: Vec<_> = text
        .lines()
        .filter_map(|line| line.split_once("  "))
        .filter(|(_, name)| *name == filename)
        .map(|(hash, _)| hash)
        .collect();
    match found.as_slice() {
        [hash] if hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()) => {
            Ok(hash.to_ascii_lowercase())
        }
        _ => Err(Error::Args(
            "安装包 SHA256 清单缺失、重复或无效，未启动升级".into(),
        )),
    }
}

fn save_download(
    agent: &ureq::Agent,
    asset: &Asset,
    hash: &str,
    directory: &Path,
) -> Result<PathBuf, Error> {
    fs::create_dir_all(directory).map_err(|e| Error::Internal(e.to_string()))?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(directory.join("download.lock"))
        .map_err(|e| Error::Internal(e.to_string()))?;
    fs2::FileExt::try_lock_exclusive(&lock)
        .map_err(|_| Error::Args("另一个 SKZ 下载正在进行，请稍后重试".into()))?;
    let destination = directory.join(&asset.name);
    let partial = directory.join(format!("{}.partial-{}", asset.name, std::process::id()));
    let outcome = (|| {
        let mut response = agent
            .get(&asset.browser_download_url)
            .call()
            .map_err(|e| Error::Network(e.to_string()))?;
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&partial)
            .map_err(|e| Error::Internal(e.to_string()))?;
        let mut reader = response.body_mut().as_reader().take(asset.size + 1);
        let mut digest = Sha256::new();
        let mut total = 0;
        let mut buffer = [0; 64 * 1024];
        loop {
            let count = reader
                .read(&mut buffer)
                .map_err(|e| Error::Network(e.to_string()))?;
            if count == 0 {
                break;
            }
            total += count as u64;
            output
                .write_all(&buffer[..count])
                .map_err(|e| Error::Internal(e.to_string()))?;
            digest.update(&buffer[..count]);
        }
        if total != asset.size || format!("{:x}", digest.finalize()) != hash {
            return Err(Error::Args(
                "安装包大小或 SHA256 校验失败，未启动升级".into(),
            ));
        }
        output
            .sync_all()
            .map_err(|e| Error::Internal(e.to_string()))?;
        drop(output);
        fs::rename(&partial, &destination).map_err(|e| Error::Internal(e.to_string()))?;
        Ok(destination)
    })();
    if outcome.is_err() {
        let _ = fs::remove_file(partial);
    }
    outcome
}

pub fn run(exe: &Path, check: bool, download: bool) -> Result<serde_json::Value, Error> {
    let current = env!("CARGO_PKG_VERSION");
    if !check && !download {
        let shortcut = exe.parent().unwrap().join("升级 SKZ.lnk");
        if !shortcut.is_file() {
            return Err(Error::Args(
                "升级入口缺失，请重新运行 SKZ 安装包修复".into(),
            ));
        }
        let system = std::env::var_os("SystemRoot")
            .ok_or_else(|| Error::Internal("SystemRoot missing".into()))?;
        // 经桌面 Shell 启动，升级器不随 WorkBuddy 的进程树一起退出。
        Command::new(PathBuf::from(system).join("explorer.exe"))
            .arg(shortcut)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Error::Internal(format!("无法打开升级窗口：{e}")))?;
        return Ok(
            serde_json::json!({"channel":"windows-installer", "cli":current,
            "launch_requested":true, "updated":null, "note":"升级窗口将显示最终结果；此处尚未完成升级"}),
        );
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .https_only(true)
        .timeout_global(Some(Duration::from_secs(90)))
        .build()
        .into();
    let mut response = agent
        .get(format!(
            "https://api.github.com/repos/{REPOSITORY}/releases/latest"
        ))
        .header("User-Agent", "skz-quant-cli-updater")
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| Error::Network(format!("检查新版失败：{e}；原版本未修改")))?;
    let raw = response
        .body_mut()
        .with_config()
        .limit(1024 * 1024)
        .read_to_string()
        .map_err(|e| Error::Network(e.to_string()))?;
    let release: Release =
        serde_json::from_str(&raw).map_err(|e| Error::Args(format!("发布信息无效：{e}")))?;
    let latest = release.stable()?;
    let available = version(latest)? > version(current)?;
    let mut report = serde_json::json!({"channel":"windows-installer", "cli":current,
        "latest_cli":latest, "available":available, "updated":false});
    if !available {
        return Ok(report);
    }
    let filename = format!("SKZ-WorkBuddy-Setup-{latest}.exe");
    let installer = release.asset(&filename)?;
    let sums = release.asset("SHA256SUMS")?;
    let text = agent
        .get(&sums.browser_download_url)
        .call()
        .map_err(|e| Error::Network(e.to_string()))?
        .body_mut()
        .with_config()
        .limit(1024 * 1024)
        .read_to_string()
        .map_err(|e| Error::Network(e.to_string()))?;
    let hash = checksum(&text, &filename)?;
    report["sha256"] = hash.clone().into();
    if download {
        let local = std::env::var_os("LOCALAPPDATA")
            .ok_or_else(|| Error::Internal("LOCALAPPDATA missing".into()))?;
        let path = save_download(
            &agent,
            installer,
            &hash,
            &PathBuf::from(local).join("SKZ/Installer/downloads"),
        )?;
        report["installer"] = path.to_string_lossy().as_ref().into();
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_versions_and_release_assets_are_strict() {
        assert!(version("0.1.9").unwrap() < version("0.1.10").unwrap());
        for v in ["../bad", "1.2.3-test", "01.2.3", "1.2", "1.2.3.4"] {
            assert!(version(v).is_err());
        }
        let mut release: Release = serde_json::from_value(serde_json::json!({"tag_name":"v1.2.3", "draft":false,
            "prerelease":false, "assets":[{"name":"test.exe", "size":1,
            "browser_download_url":"https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v1.2.3/test.exe"}]})).unwrap();
        assert_eq!(release.stable().unwrap(), "1.2.3");
        assert!(release.asset("test.exe").is_ok());
        release.assets[0].browser_download_url = "https://example.com/test.exe".into();
        assert!(release.asset("test.exe").is_err());
        release.prerelease = true;
        assert!(release.stable().is_err());
    }

    #[test]
    fn download_checks_bytes_and_never_leaves_a_bad_installer() {
        let server = httpmock::MockServer::start();
        let mock = server.mock(|when, then| {
            when.path("/setup");
            then.status(200).body("installer");
        });
        let dir = tempfile::tempdir().unwrap();
        let asset = Asset {
            name: "setup.exe".into(),
            browser_download_url: server.url("/setup"),
            size: 9,
        };
        let agent = ureq::Agent::new_with_defaults();
        let hash = format!("{:x}", Sha256::digest(b"installer"));
        assert_eq!(
            checksum(&format!("{hash}  setup.exe\n"), "setup.exe").unwrap(),
            hash
        );
        assert!(
            checksum(
                &format!("{hash}  setup.exe\n{hash}  setup.exe\n"),
                "setup.exe"
            )
            .is_err()
        );
        assert!(save_download(&agent, &asset, &"0".repeat(64), dir.path()).is_err());
        assert!(!dir.path().join("setup.exe").exists());
        assert!(
            !fs::read_dir(dir.path()).unwrap().any(|e| e
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("partial"))
        );
        assert_eq!(
            fs::read(save_download(&agent, &asset, &hash, dir.path()).unwrap()).unwrap(),
            b"installer"
        );
        let oversized = Asset { size: 8, ..asset };
        assert!(save_download(&agent, &oversized, &hash, dir.path()).is_err());
        assert_eq!(
            fs::read(dir.path().join("setup.exe")).unwrap(),
            b"installer"
        );
        mock.assert_calls(3);
    }
}
