//! credentials 文件（token 唯一来源）+ auth set/status/unset。
//!
//! 路径：Unix（Linux/macOS）统一 `~/.config/skz/credentials`；
//! Windows `%LOCALAPPDATA%\skz\credentials`（`directories` 解析）。
//! macOS 故意不用 `directories` 默认给的 Apple Application Support——这是终端
//! 工具，用户更可能人肉记「~/.config」、也更可能跨 mac/linux 机器要一致心智。
//! Unix 以 `0600` 原子创建（temp + rename）；Windows 依赖 LocalAppData 默认 ACL。

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::Error;
use crate::token::Token;

pub fn credentials_path() -> Result<PathBuf, Error> {
    let base = directories::BaseDirs::new()
        .ok_or_else(|| Error::Internal("无法定位配置目录".to_string()))?;
    // macOS 上 `directories` 默认解析到 Apple 的 Application Support；这里手动
    // 覆盖成 Unix 通用的 ~/.config，和 Linux 分支保持同一套心智（见模块头注释）。
    #[cfg(target_os = "macos")]
    let config_dir = base.home_dir().join(".config");
    #[cfg(not(target_os = "macos"))]
    let config_dir = base.config_local_dir().to_path_buf();
    Ok(config_dir.join("skz").join("credentials"))
}

/// 读取 token。文件不存在或为空 → `MissingCredentials`（→ exit 3 + remediation）。
pub fn load_token() -> Result<Token, Error> {
    let path = credentials_path()?;
    let content = fs::read_to_string(&path).map_err(|_| Error::MissingCredentials)?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(Error::MissingCredentials);
    }
    Ok(Token::new(trimmed.to_string()))
}

/// 从 stdin 读 token，裁掉首尾空白/换行后写入受限权限文件。
pub fn set_from_stdin() -> Result<(), Error> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| Error::Internal(format!("读取 stdin 失败: {e}")))?;
    let token = buf.trim();
    if token.is_empty() {
        return Err(Error::Args("stdin 为空，未提供 token".to_string()));
    }
    let path = credentials_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| Error::Internal(format!("创建配置目录失败: {e}")))?;
        restrict_dir(parent);
    }
    write_restricted(&path, token)
}

/// 离线就绪自检：文件里是否有可用 token（不发网络、不返回 token 内容）。
pub fn is_present() -> Result<bool, Error> {
    match load_token() {
        Ok(_) => Ok(true),
        Err(Error::MissingCredentials) => Ok(false),
        Err(e) => Err(e),
    }
}

/// 删除 credentials 文件（幂等）。
pub fn unset() -> Result<(), Error> {
    let path = credentials_path()?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error::Internal(format!("删除 credentials 失败: {e}"))),
    }
}

#[cfg(unix)]
fn write_restricted(path: &Path, content: &str) -> Result<(), Error> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let tmp = path.with_extension("tmp");
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&tmp)
        .map_err(|e| Error::Internal(format!("写入 credentials 失败: {e}")))?;
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
