//! Bundled, harness-native SKZ plugin lifecycle management.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::Error;

pub const CONTRACT: &str = "4.1";
const MANIFEST: &str = "manifest.json";
const RECEIPT: &str = ".skz-plugin-install.json";
const LEGACY_MARKER: &str = ".skz-install.json";
const SKILLS: [&str; 5] = ["factor", "candidate", "strategy", "guide", "portfolio"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Claude,
    Codex,
    Openclaw,
    Hermes,
}

impl Target {
    pub const ALL: [Target; 4] = [Self::Claude, Self::Codex, Self::Openclaw, Self::Hermes];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Openclaw => "openclaw",
            Self::Hermes => "hermes",
        }
    }

    fn executable(self) -> &'static str {
        self.as_str()
    }

    pub fn is_present(self) -> bool {
        executable_on_path(self.executable())
    }
}

pub fn present_targets() -> Vec<Target> {
    Target::ALL
        .into_iter()
        .filter(|target| target.is_present())
        .collect()
}

#[derive(Debug, Deserialize)]
struct Manifest {
    cli: String,
    contract: String,
    plugin: String,
    targets: Vec<String>,
    files: Vec<ManifestFile>,
}

#[derive(Debug, Deserialize)]
struct ManifestFile {
    path: String,
    sha256: String,
    mode: u32,
}

struct Bundle {
    root: PathBuf,
    manifest: Manifest,
}

#[derive(Debug, Serialize, Deserialize)]
struct Receipt {
    plugin: String,
    target: String,
    cli: String,
    contract: String,
    digest: String,
}

#[derive(Debug, Deserialize)]
struct LegacyMarker {
    target: String,
    book: String,
    digest: String,
}

#[derive(Serialize)]
pub struct InstallReport {
    pub target: &'static str,
    pub plugin: &'static str,
    pub installed: bool,
    pub cli: &'static str,
    pub contract: &'static str,
    pub source: String,
    pub migrated_legacy: Vec<String>,
}

#[derive(Serialize)]
pub struct StatusReport {
    pub target: &'static str,
    pub plugin: &'static str,
    pub installed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_cli: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_contract: Option<String>,
    pub current_cli: &'static str,
    pub current_contract: &'static str,
    pub content_ok: bool,
    pub native_ok: bool,
    pub needs_upgrade: bool,
}

#[derive(Serialize)]
pub struct UninstallReport {
    pub target: &'static str,
    pub plugin: &'static str,
    pub removed: bool,
}

fn fail(message: impl Into<String>) -> Error {
    Error::Internal(message.into())
}

fn home() -> Result<PathBuf, Error> {
    directories::BaseDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .ok_or_else(|| fail("cannot locate home directory"))
}

fn state_root(target: Target) -> Result<PathBuf, Error> {
    Ok(home()?.join(".skz").join("plugins").join(target.as_str()))
}

fn source_root(target: Target) -> Result<PathBuf, Error> {
    Ok(state_root(target)?.join("source"))
}

pub fn bundle_root() -> Result<PathBuf, Error> {
    if let Some(path) = std::env::var_os("SKZ_PLUGINS_DIR") {
        return Ok(PathBuf::from(path));
    }
    let exe = std::env::current_exe()
        .and_then(|path| path.canonicalize())
        .map_err(|e| fail(format!("cannot locate executable: {e}")))?;
    Ok(exe
        .parent()
        .ok_or_else(|| fail("executable has no parent directory"))?
        .join("plugins"))
}

fn safe_relative(raw: &str) -> Result<PathBuf, Error> {
    let path = Path::new(raw);
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(fail(format!("invalid plugin manifest path: {raw}")));
    }
    Ok(path.to_path_buf())
}

fn hash_file(path: &Path) -> Result<String, Error> {
    let bytes = fs::read(path).map_err(|e| fail(format!("cannot read {}: {e}", path.display())))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn target_files(bundle: &Bundle, target: Target) -> Vec<&ManifestFile> {
    let prefix = format!("{}/", target.as_str());
    bundle
        .manifest
        .files
        .iter()
        .filter(|file| file.path.starts_with(&prefix))
        .collect()
}

fn digest(files: &[&ManifestFile]) -> String {
    let mut hash = Sha256::new();
    for file in files {
        hash.update(file.path.as_bytes());
        hash.update([0]);
        hash.update(file.sha256.as_bytes());
        hash.update([0]);
        hash.update(file.mode.to_le_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn load_bundle() -> Result<Bundle, Error> {
    let root = bundle_root()?;
    let raw = fs::read_to_string(root.join(MANIFEST)).map_err(|e| {
        fail(format!(
            "plugin bundle unavailable at {}: {e}",
            root.display()
        ))
    })?;
    let manifest: Manifest =
        serde_json::from_str(&raw).map_err(|e| fail(format!("invalid plugin manifest: {e}")))?;
    let explicit = std::env::var_os("SKZ_PLUGINS_DIR").is_some();
    if manifest.plugin != "skz"
        || manifest.contract != CONTRACT
        || !(manifest.cli == env!("CARGO_PKG_VERSION")
            || (explicit && manifest.cli == "development"))
    {
        return Err(fail(format!(
            "plugin bundle version mismatch: cli={} contract={}",
            manifest.cli, manifest.contract
        )));
    }
    let expected: BTreeSet<_> = Target::ALL
        .into_iter()
        .map(|target| target.as_str().to_string())
        .collect();
    if manifest.targets.iter().cloned().collect::<BTreeSet<_>>() != expected {
        return Err(fail(
            "plugin manifest targets do not match supported targets",
        ));
    }
    let mut declared = BTreeSet::new();
    for file in &manifest.files {
        let relative = safe_relative(&file.path)?;
        if !declared.insert(relative.clone()) {
            return Err(fail(format!("duplicate plugin path: {}", file.path)));
        }
        let full = root.join(relative);
        let metadata = fs::symlink_metadata(&full)
            .map_err(|e| fail(format!("missing bundle file {}: {e}", full.display())))?;
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return Err(fail(format!(
                "bundle entry is not a regular file: {}",
                file.path
            )));
        }
        if hash_file(&full)? != file.sha256 {
            return Err(fail(format!("bundle checksum mismatch: {}", file.path)));
        }
    }
    Ok(Bundle { root, manifest })
}

fn executable_on_path(name: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| {
        let plain = dir.join(name);
        plain.is_file() || cfg!(windows) && dir.join(format!("{name}.exe")).is_file()
    })
}

fn require_harness(target: Target) -> Result<(), Error> {
    if target.is_present() {
        Ok(())
    } else {
        Err(Error::Args(format!(
            "找不到 `{}`；请先安装对应 harness",
            target.executable()
        )))
    }
}

fn run_native(target: Target, args: &[&str]) -> Result<String, Error> {
    let output = Command::new(target.executable())
        .args(args)
        .output()
        .map_err(|e| fail(format!("cannot run {}: {e}", target.executable())))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(Error::Args(format!(
            "{} plugin command failed: {}",
            target.as_str(),
            if detail.is_empty() {
                "unknown error"
            } else {
                &detail
            }
        )))
    }
}

fn copy_target(bundle: &Bundle, target: Target) -> Result<PathBuf, Error> {
    let root = state_root(target)?;
    fs::create_dir_all(&root)
        .map_err(|e| fail(format!("cannot create {}: {e}", root.display())))?;
    let source = root.join("source");
    let tmp = root.join(format!("source.tmp-{}", std::process::id()));
    let backup = root.join(format!("source.bak-{}", std::process::id()));
    if tmp.exists() {
        fs::remove_dir_all(&tmp).map_err(|e| fail(e.to_string()))?;
    }
    fs::create_dir_all(&tmp).map_err(|e| fail(e.to_string()))?;
    let prefix = PathBuf::from(target.as_str());
    for file in target_files(bundle, target) {
        let relative = safe_relative(&file.path)?
            .strip_prefix(&prefix)
            .map_err(|_| fail("invalid target plugin path"))?
            .to_path_buf();
        let output = tmp.join(relative);
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|e| fail(e.to_string()))?;
        }
        fs::copy(bundle.root.join(&file.path), &output).map_err(|e| fail(e.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&output, fs::Permissions::from_mode(file.mode))
                .map_err(|e| fail(e.to_string()))?;
        }
    }
    if source.exists() {
        fs::rename(&source, &backup).map_err(|e| fail(e.to_string()))?;
    }
    if let Err(error) = fs::rename(&tmp, &source) {
        if backup.exists() {
            let _ = fs::rename(&backup, &source);
        }
        return Err(fail(format!("cannot activate plugin source: {error}")));
    }
    if backup.exists() {
        fs::remove_dir_all(backup).map_err(|e| fail(e.to_string()))?;
    }
    Ok(source)
}

fn legacy_roots(target: Target) -> Result<Vec<PathBuf>, Error> {
    let home = home()?;
    Ok(match target {
        Target::Claude => vec![home.join(".claude/skills")],
        Target::Codex => vec![home.join(".agents/skills"), home.join(".codex/skills")],
        Target::Openclaw => vec![home.join(".openclaw/skills")],
        Target::Hermes => vec![home.join(".hermes/skills")],
    })
}

fn walk_files(root: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut files = Vec::new();
    for entry in fs::read_dir(root).map_err(|e| fail(e.to_string()))? {
        let entry = entry.map_err(|e| fail(e.to_string()))?;
        let kind = entry.file_type().map_err(|e| fail(e.to_string()))?;
        if kind.is_symlink() {
            return Err(Error::Args(format!(
                "旧安装包含符号链接，拒绝迁移：{}",
                entry.path().display()
            )));
        }
        if kind.is_dir() {
            files.extend(walk_files(&entry.path())?);
        } else if kind.is_file() {
            files.push(entry.path());
        }
    }
    Ok(files)
}

fn legacy_digest(dir: &Path, marker: &LegacyMarker) -> Result<String, Error> {
    let mut entries = Vec::new();
    for path in walk_files(dir)? {
        if path.file_name().is_some_and(|name| name == LEGACY_MARKER)
            || path
                .components()
                .any(|part| part.as_os_str() == "__pycache__")
            || path.extension().is_some_and(|extension| extension == "pyc")
        {
            continue;
        }
        let relative = path
            .strip_prefix(dir)
            .map_err(|e| fail(e.to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            path.metadata()
                .map_err(|e| fail(e.to_string()))?
                .permissions()
                .mode()
                & 0o777
        };
        #[cfg(not(unix))]
        let mode = 0u32;
        entries.push((relative, hash_file(&path)?, mode));
    }
    entries.sort();
    let mut hash = Sha256::new();
    for (relative, sha, mode) in entries {
        hash.update(format!("{}/skz-{}/{relative}", marker.target, marker.book).as_bytes());
        hash.update([0]);
        hash.update(sha.as_bytes());
        hash.update([0]);
        hash.update(mode.to_le_bytes());
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn legacy_dirs(target: Target) -> Result<Vec<PathBuf>, Error> {
    let mut managed = Vec::new();
    for root in legacy_roots(target)? {
        for book in SKILLS {
            let dir = root.join(format!("skz-{book}"));
            if !dir.exists() {
                continue;
            }
            let marker: LegacyMarker = fs::read_to_string(dir.join(LEGACY_MARKER))
                .ok()
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .ok_or_else(|| {
                    Error::Args(format!(
                        "{} 不是可安全迁移的 SKZ skills 目录；请先手工处理",
                        dir.display()
                    ))
                })?;
            if marker.target != target.as_str() || marker.book != book || marker.digest.is_empty() {
                return Err(Error::Args(format!(
                    "{} 的旧安装标记不匹配；拒绝覆盖",
                    dir.display()
                )));
            }
            if legacy_digest(&dir, &marker)? != marker.digest {
                return Err(Error::Args(format!(
                    "{} 已被修改；拒绝自动迁移",
                    dir.display()
                )));
            }
            managed.push(dir);
        }
    }
    Ok(managed)
}

fn native_install(target: Target, source: &Path, upgrade: bool) -> Result<(), Error> {
    let source_text = source.to_string_lossy();
    match target {
        Target::Claude => {
            if !upgrade {
                run_native(target, &["plugin", "marketplace", "add", &source_text])?;
                run_native(target, &["plugin", "install", "skz@skz", "--scope", "user"])?;
            } else {
                run_native(target, &["plugin", "marketplace", "update", "skz"])?;
                run_native(target, &["plugin", "update", "skz@skz", "--scope", "user"])?;
            }
        }
        Target::Codex => {
            if !upgrade {
                run_native(target, &["plugin", "marketplace", "add", &source_text])?;
            }
            run_native(target, &["plugin", "add", "skz@skz"])?;
        }
        Target::Openclaw => {
            let mut args = vec!["plugins", "install"];
            if upgrade {
                args.push("--force");
            }
            args.extend(["--marketplace", &source_text, "skz"]);
            run_native(target, &args)?;
        }
        Target::Hermes => {
            let destination = home()?.join(".hermes/plugins/skz");
            if destination.exists() {
                fs::remove_dir_all(&destination).map_err(|e| fail(e.to_string()))?;
            }
            copy_tree(&source.join("plugins/skz"), &destination)?;
            run_native(target, &["plugins", "enable", "skz"])?;
        }
    }
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), Error> {
    fs::create_dir_all(destination).map_err(|e| fail(e.to_string()))?;
    for entry in fs::read_dir(source).map_err(|e| fail(e.to_string()))? {
        let entry = entry.map_err(|e| fail(e.to_string()))?;
        let output = destination.join(entry.file_name());
        if entry.file_type().map_err(|e| fail(e.to_string()))?.is_dir() {
            copy_tree(&entry.path(), &output)?;
        } else {
            fs::copy(entry.path(), output).map_err(|e| fail(e.to_string()))?;
        }
    }
    Ok(())
}

fn write_receipt(target: Target, digest: String) -> Result<(), Error> {
    let receipt = Receipt {
        plugin: "skz".into(),
        target: target.as_str().into(),
        cli: env!("CARGO_PKG_VERSION").into(),
        contract: CONTRACT.into(),
        digest,
    };
    fs::write(
        state_root(target)?.join(RECEIPT),
        serde_json::to_vec(&receipt).map_err(|e| fail(e.to_string()))?,
    )
    .map_err(|e| fail(e.to_string()))
}

fn reconcile(target: Target, upgrade: bool) -> Result<InstallReport, Error> {
    require_harness(target)?;
    let bundle = load_bundle()?;
    let legacy = legacy_dirs(target)?;
    if target == Target::Hermes
        && read_receipt(target).is_none()
        && home()?.join(".hermes/plugins/skz").exists()
    {
        return Err(Error::Args(
            "~/.hermes/plugins/skz 不是由 SKZ 管理；拒绝覆盖".to_string(),
        ));
    }
    let source = copy_target(&bundle, target)?;
    native_install(target, &source, upgrade && read_receipt(target).is_some())?;
    write_receipt(target, digest(&target_files(&bundle, target)))?;
    let migrated_legacy = legacy
        .into_iter()
        .map(|dir| {
            fs::remove_dir_all(&dir).map_err(|e| fail(e.to_string()))?;
            Ok(dir.display().to_string())
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(InstallReport {
        target: target.as_str(),
        plugin: "skz",
        installed: true,
        cli: env!("CARGO_PKG_VERSION"),
        contract: CONTRACT,
        source: source.display().to_string(),
        migrated_legacy,
    })
}

pub fn install(target: Target) -> Result<InstallReport, Error> {
    reconcile(target, false)
}

pub fn upgrade(target: Target) -> Result<InstallReport, Error> {
    reconcile(target, true)
}

fn read_receipt(target: Target) -> Option<Receipt> {
    serde_json::from_str(&fs::read_to_string(state_root(target).ok()?.join(RECEIPT)).ok()?).ok()
}

fn staged_content_ok(bundle: &Bundle, target: Target) -> bool {
    let Ok(source) = source_root(target) else {
        return false;
    };
    let prefix = PathBuf::from(target.as_str());
    target_files(bundle, target).into_iter().all(|file| {
        safe_relative(&file.path)
            .ok()
            .and_then(|path| path.strip_prefix(&prefix).ok().map(Path::to_path_buf))
            .and_then(|path| hash_file(&source.join(path)).ok())
            .is_some_and(|sha| sha == file.sha256)
    })
}

fn native_status(target: Target) -> bool {
    if !target.is_present() {
        return false;
    }
    if target == Target::Hermes {
        return home().is_ok_and(|home| home.join(".hermes/plugins/skz/plugin.yaml").is_file());
    }
    let args: &[&str] = match target {
        Target::Claude => &["plugin", "list", "--json"],
        Target::Codex => &["plugin", "list", "--json"],
        Target::Openclaw => &["plugins", "list", "--json"],
        Target::Hermes => unreachable!(),
    };
    run_native(target, args).is_ok_and(|output| {
        serde_json::from_str(&output).is_ok_and(|value| json_contains_exact_string(&value, "skz"))
    })
}

fn json_contains_exact_string(value: &serde_json::Value, expected: &str) -> bool {
    match value {
        serde_json::Value::String(value) => value == expected,
        serde_json::Value::Array(values) => values
            .iter()
            .any(|value| json_contains_exact_string(value, expected)),
        serde_json::Value::Object(values) => values
            .values()
            .any(|value| json_contains_exact_string(value, expected)),
        _ => false,
    }
}

pub fn status(target: Target) -> Result<StatusReport, Error> {
    let bundle = load_bundle()?;
    let receipt = read_receipt(target);
    let content_ok = staged_content_ok(&bundle, target);
    let native_ok = native_status(target);
    let installed = receipt.is_some() && content_ok && native_ok;
    let needs_upgrade = receipt.as_ref().is_none_or(|receipt| {
        receipt.plugin != "skz"
            || receipt.target != target.as_str()
            || receipt.cli != env!("CARGO_PKG_VERSION")
            || receipt.contract != CONTRACT
            || receipt.digest != digest(&target_files(&bundle, target))
    }) || !content_ok
        || !native_ok;
    Ok(StatusReport {
        target: target.as_str(),
        plugin: "skz",
        installed,
        installed_cli: receipt.as_ref().map(|receipt| receipt.cli.clone()),
        installed_contract: receipt.as_ref().map(|receipt| receipt.contract.clone()),
        current_cli: env!("CARGO_PKG_VERSION"),
        current_contract: CONTRACT,
        content_ok,
        native_ok,
        needs_upgrade,
    })
}

pub fn uninstall(target: Target) -> Result<UninstallReport, Error> {
    require_harness(target)?;
    if read_receipt(target).is_none() {
        return Ok(UninstallReport {
            target: target.as_str(),
            plugin: "skz",
            removed: false,
        });
    }
    match target {
        Target::Claude => {
            run_native(
                target,
                &["plugin", "uninstall", "skz@skz", "--scope", "user"],
            )?;
        }
        Target::Codex => {
            run_native(target, &["plugin", "remove", "skz"])?;
        }
        Target::Openclaw => {
            run_native(target, &["plugins", "uninstall", "skz", "--force"])?;
        }
        Target::Hermes => {
            run_native(target, &["plugins", "remove", "skz"])?;
        }
    }
    fs::remove_dir_all(state_root(target)?).map_err(|e| fail(e.to_string()))?;
    Ok(UninstallReport {
        target: target.as_str(),
        plugin: "skz",
        removed: true,
    })
}

#[cfg(test)]
mod tests {
    use super::json_contains_exact_string;

    #[test]
    fn native_status_finds_exact_plugin_name_in_json() {
        let value = serde_json::json!({"plugins": [{"name": "skz"}]});
        assert!(json_contains_exact_string(&value, "skz"));
    }

    #[test]
    fn native_status_rejects_plugin_name_substrings() {
        let value = serde_json::json!({"plugins": [{"name": "not-skz"}]});
        assert!(!json_contains_exact_string(&value, "skz"));
    }
}
