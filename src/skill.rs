//! External, harness-specific skill bundle installer.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::Error;

pub const CONTRACT: &str = "3.4";
const MARKER: &str = ".skz-install.json";
const MANIFEST: &str = "manifest.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Claude,
    Codex,
    Openclaw,
    Hermes,
}

impl Target {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Openclaw => "openclaw",
            Self::Hermes => "hermes",
        }
    }
    fn config_dir(self) -> &'static str {
        match self {
            Self::Claude => ".claude",
            Self::Codex => ".codex",
            Self::Openclaw => ".openclaw",
            Self::Hermes => ".hermes",
        }
    }
    pub const ALL: [Target; 4] = [Self::Claude, Self::Codex, Self::Openclaw, Self::Hermes];
    pub fn is_present(self) -> bool {
        directories::BaseDirs::new()
            .map(|b| b.home_dir().join(self.config_dir()).is_dir())
            .unwrap_or(false)
    }
}

pub fn present_targets() -> Vec<Target> {
    Target::ALL.into_iter().filter(|t| t.is_present()).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    User,
    Project,
}
impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
        }
    }
}

#[derive(Debug, Deserialize)]
struct Manifest {
    cli: String,
    contract: String,
    targets: Vec<String>,
    books: Vec<String>,
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

#[derive(Serialize, Deserialize)]
pub struct Marker {
    pub cli: String,
    pub contract: String,
    #[serde(default)]
    pub target: String,
    pub book: String,
    #[serde(default)]
    pub digest: String,
}

#[derive(Serialize)]
pub struct InstalledBook {
    pub name: String,
    pub path: String,
}
#[derive(Serialize)]
pub struct InstallReport {
    pub target: &'static str,
    pub scope: &'static str,
    pub root: String,
    pub installed: Vec<InstalledBook>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub legacy_cleanup: Vec<RemovedBook>,
    pub cli: &'static str,
    pub contract: &'static str,
}
#[derive(Serialize)]
pub struct BookStatus {
    pub name: String,
    pub path: String,
    pub installed: bool,
    pub stale: bool,
    pub foreign: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_path: Option<String>,
    pub migration_required: bool,
    pub legacy_foreign: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_cli: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_contract: Option<String>,
}
#[derive(Serialize)]
pub struct StatusReport {
    pub target: &'static str,
    pub scope: &'static str,
    pub root: String,
    pub books: Vec<BookStatus>,
    pub cli: &'static str,
    pub contract: &'static str,
    pub needs_install: bool,
}
#[derive(Serialize)]
pub struct RemovedBook {
    pub name: String,
    pub path: String,
    pub removed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped: Option<&'static str>,
}
#[derive(Serialize)]
pub struct UninstallReport {
    pub target: &'static str,
    pub scope: &'static str,
    pub root: String,
    pub books: Vec<RemovedBook>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub legacy_books: Vec<RemovedBook>,
}

fn fail(message: impl Into<String>) -> Error {
    Error::Internal(message.into())
}

pub fn bundle_root() -> Result<PathBuf, Error> {
    if let Some(path) = std::env::var_os("SKZ_SKILLS_DIR") {
        return Ok(PathBuf::from(path));
    }
    let exe =
        std::env::current_exe().map_err(|e| fail(format!("cannot locate executable: {e}")))?;
    let real = exe
        .canonicalize()
        .map_err(|e| fail(format!("cannot resolve executable {}: {e}", exe.display())))?;
    Ok(real
        .parent()
        .ok_or_else(|| fail("executable has no parent directory"))?
        .join("skills"))
}

fn safe_relative(raw: &str) -> Result<PathBuf, Error> {
    let path = Path::new(raw);
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(fail(format!("invalid skill manifest path: {raw}")));
    }
    Ok(path.to_path_buf())
}

fn hash_file(path: &Path) -> Result<String, Error> {
    let bytes = fs::read(path).map_err(|e| fail(format!("cannot read {}: {e}", path.display())))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn load_bundle() -> Result<Bundle, Error> {
    let root = bundle_root()?;
    let explicit = std::env::var_os("SKZ_SKILLS_DIR").is_some();
    let manifest_path = root.join(MANIFEST);
    let raw = fs::read_to_string(&manifest_path).map_err(|e| {
        fail(format!(
            "skill bundle unavailable at {}: {e}",
            manifest_path.display()
        ))
    })?;
    let manifest: Manifest = serde_json::from_str(&raw)
        .map_err(|e| fail(format!("invalid {}: {e}", manifest_path.display())))?;
    let cli_matches =
        manifest.cli == env!("CARGO_PKG_VERSION") || (explicit && manifest.cli == "development");
    if !cli_matches || manifest.contract != CONTRACT {
        return Err(fail(format!(
            "skill bundle version mismatch: bundle cli={} contract={}, expected cli={} contract={}",
            manifest.cli,
            manifest.contract,
            env!("CARGO_PKG_VERSION"),
            CONTRACT
        )));
    }
    let expected_targets: BTreeSet<_> = Target::ALL
        .into_iter()
        .map(|t| t.as_str().to_string())
        .collect();
    if manifest.targets.iter().cloned().collect::<BTreeSet<_>>() != expected_targets {
        return Err(fail(
            "skill manifest targets do not match supported targets",
        ));
    }
    let mut declared = BTreeSet::new();
    for file in &manifest.files {
        let rel = safe_relative(&file.path)?;
        if !declared.insert(rel.clone()) {
            return Err(fail(format!(
                "duplicate skill manifest path: {}",
                file.path
            )));
        }
        let full = root.join(&rel);
        let meta = fs::symlink_metadata(&full)
            .map_err(|e| fail(format!("missing bundle file {}: {e}", full.display())))?;
        if !meta.file_type().is_file() || meta.file_type().is_symlink() {
            return Err(fail(format!(
                "bundle entry is not a regular file: {}",
                full.display()
            )));
        }
        if hash_file(&full)? != file.sha256 {
            return Err(fail(format!(
                "bundle checksum mismatch: {}",
                full.display()
            )));
        }
    }
    let actual: BTreeSet<_> = walk_files(&root)?
        .into_iter()
        .filter_map(|p| p.strip_prefix(&root).ok().map(Path::to_path_buf))
        .filter(|p| p != Path::new(MANIFEST))
        .collect();
    if actual != declared {
        return Err(fail("skill bundle contains undeclared or missing files"));
    }
    Ok(Bundle { root, manifest })
}

fn walk_files(root: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(dir) = pending.pop() {
        for entry in
            fs::read_dir(&dir).map_err(|e| fail(format!("cannot read {}: {e}", dir.display())))?
        {
            let entry = entry.map_err(|e| fail(format!("cannot read bundle entry: {e}")))?;
            let ty = entry
                .file_type()
                .map_err(|e| fail(format!("cannot inspect {}: {e}", entry.path().display())))?;
            if ty.is_symlink() {
                return Err(fail(format!(
                    "symlinks are not allowed in skill bundle: {}",
                    entry.path().display()
                )));
            }
            if ty.is_dir() {
                pending.push(entry.path());
            } else if ty.is_file() {
                files.push(entry.path());
            } else {
                return Err(fail(format!(
                    "unsupported bundle entry: {}",
                    entry.path().display()
                )));
            }
        }
    }
    files.sort();
    Ok(files)
}

fn book_files<'a>(bundle: &'a Bundle, target: Target, book: &str) -> Vec<&'a ManifestFile> {
    let prefix = format!("{}/skz-{book}/", target.as_str());
    bundle
        .manifest
        .files
        .iter()
        .filter(|f| f.path.starts_with(&prefix))
        .collect()
}

fn digest(files: &[&ManifestFile]) -> String {
    let mut h = Sha256::new();
    for file in files {
        h.update(file.path.as_bytes());
        h.update([0]);
        h.update(file.sha256.as_bytes());
        h.update([0]);
        #[cfg(unix)]
        h.update(file.mode.to_le_bytes());
        #[cfg(not(unix))]
        h.update(0u32.to_le_bytes());
    }
    format!("{:x}", h.finalize())
}

pub fn show(target: Target, name: Option<&str>) -> Result<String, Error> {
    let bundle = load_bundle()?;
    let book = name.unwrap_or("guide");
    if !bundle.manifest.books.iter().any(|b| b == book) {
        return Err(Error::Args(format!(
            "unknown skill book {book}; choose {}",
            bundle.manifest.books.join(" | ")
        )));
    }
    fs::read_to_string(
        bundle
            .root
            .join(target.as_str())
            .join(format!("skz-{book}/SKILL.md")),
    )
    .map_err(|e| {
        fail(format!(
            "cannot read skill {book} for {}: {e}",
            target.as_str()
        ))
    })
}

pub fn skills_root(target: Target, scope: Scope) -> Result<PathBuf, Error> {
    if target == Target::Codex {
        return match scope {
            Scope::User => directories::BaseDirs::new()
                .map(|b| b.home_dir().join(".agents").join("skills"))
                .ok_or_else(|| fail("cannot locate home directory")),
            Scope::Project => std::env::current_dir()
                .map(|p| p.join(".agents").join("skills"))
                .map_err(|e| fail(format!("cannot locate current directory: {e}"))),
        };
    }
    match scope {
        Scope::User => directories::BaseDirs::new()
            .map(|b| b.home_dir().join(target.config_dir()).join("skills"))
            .ok_or_else(|| fail("cannot locate home directory")),
        Scope::Project => std::env::current_dir()
            .map(|p| p.join(target.config_dir()).join("skills"))
            .map_err(|e| fail(format!("cannot locate current directory: {e}"))),
    }
}

fn legacy_skills_root(target: Target, scope: Scope) -> Result<Option<PathBuf>, Error> {
    if target != Target::Codex {
        return Ok(None);
    }
    match scope {
        Scope::User => directories::BaseDirs::new()
            .map(|b| Some(b.home_dir().join(".codex").join("skills")))
            .ok_or_else(|| fail("cannot locate home directory")),
        Scope::Project => std::env::current_dir()
            .map(|p| Some(p.join(".codex").join("skills")))
            .map_err(|e| fail(format!("cannot locate current directory: {e}"))),
    }
}

fn read_marker(dir: &Path) -> Option<Marker> {
    serde_json::from_str(&fs::read_to_string(dir.join(MARKER)).ok()?).ok()
}

fn is_managed_book(dir: &Path, target: Target, book: &str) -> bool {
    read_marker(dir).is_some_and(|marker| marker.target == target.as_str() && marker.book == book)
}

fn remove_managed_books(
    root: &Path,
    target: Target,
    names: &[String],
    include_absent: bool,
) -> Result<Vec<RemovedBook>, Error> {
    let mut books = Vec::new();
    for book in names {
        let dir = root.join(format!("skz-{book}"));
        let (removed, skipped) = if !dir.exists() {
            if !include_absent {
                continue;
            }
            (false, Some("absent"))
        } else if !is_managed_book(&dir, target, book) {
            (false, Some("foreign"))
        } else {
            fs::remove_dir_all(&dir)
                .map_err(|e| fail(format!("cannot remove {}: {e}", dir.display())))?;
            (true, None)
        };
        books.push(RemovedBook {
            name: book.clone(),
            path: dir.display().to_string(),
            removed,
            skipped,
        });
    }
    Ok(books)
}

fn installed_digest(dir: &Path) -> Option<String> {
    let marker = read_marker(dir)?;
    let mut entries = Vec::new();
    for path in walk_files(dir).ok()? {
        if path.file_name()?.to_string_lossy() == MARKER {
            continue;
        }
        if path
            .components()
            .any(|component| component.as_os_str() == "__pycache__")
            || path.extension().is_some_and(|extension| extension == "pyc")
        {
            continue;
        }
        let rel = path
            .strip_prefix(dir)
            .ok()?
            .to_string_lossy()
            .replace('\\', "/");
        let sha = hash_file(&path).ok()?;
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            path.metadata().ok()?.permissions().mode() & 0o777
        };
        #[cfg(not(unix))]
        let mode = 0u32;
        entries.push((rel, sha, mode));
    }
    entries.sort();
    let mut h = Sha256::new();
    for (rel, sha, mode) in entries {
        h.update(format!("{}/skz-{}/{rel}", marker.target, marker.book).as_bytes());
        h.update([0]);
        h.update(sha.as_bytes());
        h.update([0]);
        h.update(mode.to_le_bytes());
    }
    Some(format!("{:x}", h.finalize()))
}

fn copy_book(bundle: &Bundle, target: Target, book: &str, dst: &Path) -> Result<String, Error> {
    let files = book_files(bundle, target, book);
    if files.is_empty() {
        return Err(fail(format!(
            "bundle has no files for {}/{book}",
            target.as_str()
        )));
    }
    fs::create_dir_all(dst).map_err(|e| fail(format!("cannot create {}: {e}", dst.display())))?;
    let prefix = PathBuf::from(target.as_str()).join(format!("skz-{book}"));
    for file in &files {
        let rel = safe_relative(&file.path)?
            .strip_prefix(&prefix)
            .map_err(|_| fail("invalid book path"))?
            .to_path_buf();
        let out = dst.join(rel);
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| fail(format!("cannot create {}: {e}", parent.display())))?;
        }
        fs::copy(bundle.root.join(&file.path), &out)
            .map_err(|e| fail(format!("cannot copy {}: {e}", file.path)))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&out, fs::Permissions::from_mode(file.mode))
                .map_err(|e| fail(format!("cannot set permissions on {}: {e}", out.display())))?;
        }
    }
    Ok(digest(&files))
}

pub fn install(target: Target, scope: Scope) -> Result<InstallReport, Error> {
    let bundle = load_bundle()?;
    let root = skills_root(target, scope)?;
    for book in &bundle.manifest.books {
        let dir = root.join(format!("skz-{book}"));
        if dir.exists() && !is_managed_book(&dir, target, book) {
            return Err(Error::Args(format!(
                "{} is not owned by skz for {}/{}; refusing to overwrite",
                dir.display(),
                target.as_str(),
                book
            )));
        }
    }
    fs::create_dir_all(&root)
        .map_err(|e| fail(format!("cannot create {}: {e}", root.display())))?;
    let mut installed = Vec::new();
    for book in &bundle.manifest.books {
        let dir = root.join(format!("skz-{book}"));
        let tmp = root.join(format!(".skz-{book}.tmp-{}", std::process::id()));
        let backup = root.join(format!(".skz-{book}.bak-{}", std::process::id()));
        if tmp.exists() {
            fs::remove_dir_all(&tmp).map_err(|e| fail(e.to_string()))?;
        }
        let digest = copy_book(&bundle, target, book, &tmp)?;
        let marker = Marker {
            cli: env!("CARGO_PKG_VERSION").into(),
            contract: CONTRACT.into(),
            target: target.as_str().into(),
            book: book.clone(),
            digest,
        };
        fs::write(
            tmp.join(MARKER),
            serde_json::to_vec(&marker).map_err(|e| fail(e.to_string()))?,
        )
        .map_err(|e| fail(format!("cannot write marker: {e}")))?;
        if dir.exists() {
            fs::rename(&dir, &backup)
                .map_err(|e| fail(format!("cannot back up {}: {e}", dir.display())))?;
        }
        if let Err(e) = fs::rename(&tmp, &dir) {
            if backup.exists() {
                let _ = fs::rename(&backup, &dir);
            }
            return Err(fail(format!("cannot activate {}: {e}", dir.display())));
        }
        if backup.exists() {
            fs::remove_dir_all(&backup).map_err(|e| fail(format!("cannot remove backup: {e}")))?;
        }
        installed.push(InstalledBook {
            name: book.clone(),
            path: dir.display().to_string(),
        });
    }
    let legacy_cleanup = match legacy_skills_root(target, scope)? {
        Some(legacy_root) => {
            remove_managed_books(&legacy_root, target, &bundle.manifest.books, false)?
        }
        None => Vec::new(),
    };
    Ok(InstallReport {
        target: target.as_str(),
        scope: scope.as_str(),
        root: root.display().to_string(),
        installed,
        legacy_cleanup,
        cli: env!("CARGO_PKG_VERSION"),
        contract: CONTRACT,
    })
}

pub fn status(target: Target, scope: Scope) -> Result<StatusReport, Error> {
    let bundle = load_bundle()?;
    let root = skills_root(target, scope)?;
    let legacy_root = legacy_skills_root(target, scope)?;
    let mut books = Vec::new();
    let mut needs_install = false;
    for book in &bundle.manifest.books {
        let dir = root.join(format!("skz-{book}"));
        let legacy_dir = legacy_root
            .as_ref()
            .map(|legacy_root| legacy_root.join(format!("skz-{book}")));
        let legacy_marker = legacy_dir.as_ref().and_then(|dir| read_marker(dir));
        let migration_required = legacy_marker
            .as_ref()
            .is_some_and(|marker| marker.target == target.as_str() && marker.book == *book);
        let legacy_foreign = legacy_dir.as_ref().is_some_and(|dir| dir.exists())
            && legacy_marker
                .as_ref()
                .is_none_or(|marker| marker.target != target.as_str() || marker.book != *book);
        let legacy_path = legacy_dir
            .as_ref()
            .filter(|dir| dir.exists())
            .map(|dir| dir.display().to_string());
        let expected = digest(&book_files(&bundle, target, book));
        let (installed, stale, foreign, mut cli, mut contract) = match read_marker(&dir) {
            Some(m) => {
                let same = m.cli == env!("CARGO_PKG_VERSION")
                    && m.contract == CONTRACT
                    && m.target == target.as_str()
                    && m.book == *book
                    && m.digest == expected
                    && installed_digest(&dir).as_deref() == Some(expected.as_str());
                (same, !same, false, Some(m.cli), Some(m.contract))
            }
            None if dir.exists() => (false, false, true, None, None),
            None => (false, false, false, None, None),
        };
        if cli.is_none() && migration_required {
            cli = legacy_marker.as_ref().map(|marker| marker.cli.clone());
            contract = legacy_marker.as_ref().map(|marker| marker.contract.clone());
        }
        needs_install |= !installed || migration_required;
        books.push(BookStatus {
            name: book.clone(),
            path: dir.display().to_string(),
            installed,
            stale,
            foreign,
            legacy_path,
            migration_required,
            legacy_foreign,
            installed_cli: cli,
            installed_contract: contract,
        });
    }
    Ok(StatusReport {
        target: target.as_str(),
        scope: scope.as_str(),
        root: root.display().to_string(),
        books,
        cli: env!("CARGO_PKG_VERSION"),
        contract: CONTRACT,
        needs_install,
    })
}

pub fn uninstall(target: Target, scope: Scope) -> Result<UninstallReport, Error> {
    let root = skills_root(target, scope)?;
    let names = ["factor", "candidate", "strategy", "guide", "portfolio"]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let books = remove_managed_books(&root, target, &names, true)?;
    let legacy_books = match legacy_skills_root(target, scope)? {
        Some(legacy_root) => remove_managed_books(&legacy_root, target, &names, true)?,
        None => Vec::new(),
    };
    Ok(UninstallReport {
        target: target.as_str(),
        scope: scope.as_str(),
        root: root.display().to_string(),
        books,
        legacy_books,
    })
}

pub fn permissions() -> serde_json::Value {
    serde_json::json!({
        "note": "把 ask 规则贴进你的 harness 权限配置；skz 不会替你修改任何配置文件。\
    注意 `skz strategy status:*` 是前缀匹配，会一并拦下按底表可自主的 `--status 暂停`——\
    前缀规则切不开三个状态值，宁可多问一次也不漏掉 实盘/废弃。\
    同理 `skz factor-routes delete:*` 会一并拦下零修改的 `--dry-run` 预演，\
    `skz experiment delete:*` 会一并拦下 `delete-run`（那条本来就要问人）。",
        "rationale": "命中 HITL 底表（花钱 / 不可逆 / 对已有资产下处置）的写命令，在调用发生前需人确认。",
        "strongerOption": "这份规则按**命令字符串前缀**匹配，`cd x && skz ...`、绝对路径调用、\
    `env skz ...` 都能让它落空——而这些是 agent 的日常写法，不是刻意规避。要真的兜住，\
    设环境变量 `SKZ_READ_ONLY=1`：闸在二进制内部，跟命令怎么拼写无关，所有写直接 exit 8 \
    且请求不发出。代价是那台机器上你自己也写不了（要写就在另一个没设变量的终端里跑）。",
        "ask": [
            "Bash(skz mine start:*)",
            "Bash(skz explore start:*)",
            "Bash(skz promote start:*)",
            "Bash(skz factor delete:*)",
            "Bash(skz mining delete-run:*)",
            "Bash(skz experiment delete:*)",
            "Bash(skz factor-routes delete:*)",
            "Bash(skz gift create:*)",
            "Bash(skz gift claim:*)",
            "Bash(skz strategy status:*)",
            "Bash(skz strategy register:*)",
            "Bash(skz portfolio create:*)"
        ]
    })
}
