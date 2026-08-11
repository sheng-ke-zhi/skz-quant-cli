//! 命名身份凭据仓库 + auth 管理。
//!
//! 路径仍为 Unix `~/.config/skz/credentials`、Windows LocalAppData 下的同名文件。
//! 旧版纯文本 token 会在内存中映射为 active 的 `default` 身份；只有下一次 auth 写
//! 操作才把文件原子迁移为版本化 JSON。token 始终只来自该受限权限文件。

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::token::Token;

const STORE_VERSION: u8 = 1;
const LEGACY_IDENTITY: &str = "default";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WritePolicy {
    Deny,
    Allow,
}

impl WritePolicy {
    pub fn is_read_only(self) -> bool {
        self == Self::Deny
    }
}

#[derive(Serialize, Deserialize)]
struct StoredIdentity {
    account: String,
    token: String,
    #[serde(rename = "writePolicy")]
    write_policy: WritePolicy,
}

#[derive(Serialize, Deserialize)]
struct CredentialStore {
    version: u8,
    active: Option<String>,
    identities: BTreeMap<String, StoredIdentity>,
}

impl CredentialStore {
    fn empty() -> Self {
        Self {
            version: STORE_VERSION,
            active: None,
            identities: BTreeMap::new(),
        }
    }
}

pub struct SelectedCredential {
    pub token: Token,
    pub write_policy: WritePolicy,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityInfo {
    pub name: String,
    pub account: String,
    pub write_policy: WritePolicy,
    pub active: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthList {
    pub active: Option<String>,
    pub identities: Vec<IdentityInfo>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    pub present: bool,
    pub active: Option<String>,
    pub account: Option<String>,
    pub write_policy: Option<WritePolicy>,
    pub read_only: bool,
    pub global_read_only: bool,
}

pub fn credentials_path() -> Result<PathBuf, Error> {
    let base = directories::BaseDirs::new()
        .ok_or_else(|| Error::Internal("无法定位配置目录".to_string()))?;
    #[cfg(target_os = "macos")]
    let config_dir = base.home_dir().join(".config");
    #[cfg(not(target_os = "macos"))]
    let config_dir = base.config_local_dir().to_path_buf();
    Ok(config_dir.join("skz").join("credentials"))
}

/// 加载当前默认身份。存在身份但尚未选择时，返回可供 agent 分支的 `IdentityRequired`。
pub fn load_selected() -> Result<SelectedCredential, Error> {
    let store = read_store()?;
    if store.identities.is_empty() {
        return Err(Error::MissingCredentials);
    }
    let Some(active) = store.active.as_deref() else {
        return Err(Error::IdentityRequired {
            identities: store.identities.keys().cloned().collect(),
        });
    };
    let identity = store.identities.get(active).ok_or_else(|| {
        Error::Internal(format!("credentials 中的 active 身份 {active:?} 不存在"))
    })?;
    Ok(SelectedCredential {
        token: Token::new(identity.token.clone()),
        write_policy: identity.write_policy,
    })
}

/// 旧版入口：覆盖 `default` 身份，并立即把它设为默认，保持单 key 用户行为不变。
pub fn set_from_stdin() -> Result<IdentityInfo, Error> {
    let token = read_token_from_stdin()?;
    let mut store = read_store()?;
    store.identities.insert(
        LEGACY_IDENTITY.to_string(),
        StoredIdentity {
            account: LEGACY_IDENTITY.to_string(),
            token,
            write_policy: WritePolicy::Allow,
        },
    );
    store.active = Some(LEGACY_IDENTITY.to_string());
    write_store(&store)?;
    Ok(identity_info(&store, LEGACY_IDENTITY).expect("刚插入的 default 身份必须存在"))
}

pub fn add_from_stdin(
    name: &str,
    account: Option<&str>,
    write_policy: WritePolicy,
    replace: bool,
) -> Result<IdentityInfo, Error> {
    validate_name(name, "identity")?;
    let account = account.unwrap_or(name);
    validate_name(account, "account")?;
    let mut store = read_store()?;
    if store.identities.contains_key(name) && !replace {
        return Err(Error::Args(format!(
            "身份 {name:?} 已存在；确认要覆盖时显式加 --replace"
        )));
    }
    let token = read_token_from_stdin()?;
    store.identities.insert(
        name.to_string(),
        StoredIdentity {
            account: account.to_string(),
            token,
            write_policy,
        },
    );
    write_store(&store)?;
    Ok(identity_info(&store, name).expect("刚插入的身份必须存在"))
}

pub fn list() -> Result<AuthList, Error> {
    let store = read_store()?;
    let identities = store
        .identities
        .keys()
        .filter_map(|name| identity_info(&store, name))
        .collect();
    Ok(AuthList {
        active: store.active,
        identities,
    })
}

pub fn use_identity(name: &str) -> Result<IdentityInfo, Error> {
    validate_name(name, "identity")?;
    let mut store = read_store()?;
    if !store.identities.contains_key(name) {
        return Err(Error::Args(format!(
            "身份 {name:?} 不存在；先运行 `skz auth list` 查看可用身份"
        )));
    }
    store.active = Some(name.to_string());
    write_store(&store)?;
    Ok(identity_info(&store, name).expect("已校验的身份必须存在"))
}

pub fn remove(name: &str) -> Result<serde_json::Value, Error> {
    validate_name(name, "identity")?;
    let mut store = read_store()?;
    if store.identities.remove(name).is_none() {
        return Err(Error::Args(format!("身份 {name:?} 不存在")));
    }
    if store.active.as_deref() == Some(name) {
        store.active = None;
    }
    write_store(&store)?;
    Ok(serde_json::json!({
        "removed": name,
        "active": store.active,
    }))
}

pub fn status(global_read_only: bool) -> Result<AuthStatus, Error> {
    let store = read_store()?;
    let selected = store
        .active
        .as_deref()
        .and_then(|name| store.identities.get(name));
    if store.active.is_some() && selected.is_none() {
        return Err(Error::Internal(
            "credentials 中的 active 身份不存在".to_string(),
        ));
    }
    Ok(AuthStatus {
        present: selected.is_some(),
        active: store.active,
        account: selected.map(|item| item.account.clone()),
        write_policy: selected.map(|item| item.write_policy),
        read_only: global_read_only
            || selected.is_some_and(|item| item.write_policy.is_read_only()),
        global_read_only,
    })
}

/// 旧版入口：只删除 `default`，不影响后来添加的命名身份。
pub fn unset() -> Result<serde_json::Value, Error> {
    let mut store = read_store()?;
    let removed = store.identities.remove(LEGACY_IDENTITY).is_some();
    if store.active.as_deref() == Some(LEGACY_IDENTITY) {
        store.active = None;
    }
    if removed || credentials_path()?.exists() {
        write_store(&store)?;
    }
    Ok(serde_json::json!({
        "removed": if removed { Some(LEGACY_IDENTITY) } else { None::<&str> },
        "active": store.active,
    }))
}

fn identity_info(store: &CredentialStore, name: &str) -> Option<IdentityInfo> {
    let identity = store.identities.get(name)?;
    Some(IdentityInfo {
        name: name.to_string(),
        account: identity.account.clone(),
        write_policy: identity.write_policy,
        active: store.active.as_deref() == Some(name),
    })
}

fn validate_name(value: &str, field: &str) -> Result<(), Error> {
    let valid = !value.is_empty()
        && value.len() <= 64
        && value.bytes().enumerate().all(|(index, byte)| match byte {
            b'a'..=b'z' => true,
            b'0'..=b'9' => index > 0,
            b'.' | b'_' | b'-' => index > 0,
            _ => false,
        });
    if !valid {
        return Err(Error::Args(format!(
            "{field} 只接受小写字母开头、其后为小写字母/数字/._- 的 1～64 字符名称"
        )));
    }
    Ok(())
}

fn read_token_from_stdin() -> Result<String, Error> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| Error::Internal(format!("读取 stdin 失败: {e}")))?;
    let token = buf.trim();
    if token.is_empty() {
        return Err(Error::Args("stdin 为空，未提供 token".to_string()));
    }
    Ok(token.to_string())
}

fn read_store() -> Result<CredentialStore, Error> {
    let path = credentials_path()?;
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(CredentialStore::empty()),
        Err(e) => return Err(Error::Internal(format!("读取 credentials 失败: {e}"))),
    };
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(CredentialStore::empty());
    }
    if !trimmed.starts_with('{') {
        let mut identities = BTreeMap::new();
        identities.insert(
            LEGACY_IDENTITY.to_string(),
            StoredIdentity {
                account: LEGACY_IDENTITY.to_string(),
                token: trimmed.to_string(),
                write_policy: WritePolicy::Allow,
            },
        );
        return Ok(CredentialStore {
            version: STORE_VERSION,
            active: Some(LEGACY_IDENTITY.to_string()),
            identities,
        });
    }
    let store: CredentialStore = serde_json::from_str(trimmed)
        .map_err(|e| Error::Internal(format!("credentials JSON 无法解析: {e}")))?;
    if store.version != STORE_VERSION {
        return Err(Error::Internal(format!(
            "不支持的 credentials 版本 {}",
            store.version
        )));
    }
    for (name, identity) in &store.identities {
        validate_name(name, "identity")
            .map_err(|e| Error::Internal(format!("credentials identity 非法: {e}")))?;
        validate_name(&identity.account, "account")
            .map_err(|e| Error::Internal(format!("credentials account 非法: {e}")))?;
        if identity.token.trim().is_empty() {
            return Err(Error::Internal(format!(
                "credentials 身份 {name:?} 的 token 为空"
            )));
        }
    }
    Ok(store)
}

fn write_store(store: &CredentialStore) -> Result<(), Error> {
    let path = credentials_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| Error::Internal(format!("创建配置目录失败: {e}")))?;
        restrict_dir(parent);
    }
    let content = serde_json::to_string(store)
        .map_err(|e| Error::Internal(format!("序列化 credentials 失败: {e}")))?;
    write_restricted(&path, &content)
}

#[cfg(unix)]
fn write_restricted(path: &Path, content: &str) -> Result<(), Error> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let tmp = path.with_extension("tmp");
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&tmp)
        .map_err(|e| Error::Internal(format!("写入 credentials 失败: {e}")))?;
    f.set_permissions(fs::Permissions::from_mode(0o600))
        .map_err(|e| Error::Internal(format!("限制 credentials 权限失败: {e}")))?;
    f.write_all(content.as_bytes())
        .map_err(|e| Error::Internal(format!("写入 credentials 失败: {e}")))?;
    let _ = f.sync_all();
    fs::rename(&tmp, path).map_err(|e| Error::Internal(format!("提交 credentials 失败: {e}")))?;
    Ok(())
}

#[cfg(not(unix))]
fn write_restricted(path: &Path, content: &str) -> Result<(), Error> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, content).map_err(|e| Error::Internal(format!("写入 credentials 失败: {e}")))?;
    fs::rename(&tmp, path).map_err(|e| Error::Internal(format!("提交 credentials 失败: {e}")))?;
    Ok(())
}

#[cfg(unix)]
fn restrict_dir(dir: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
}

#[cfg(not(unix))]
fn restrict_dir(_dir: &Path) {}
