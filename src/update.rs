//! 自更新：识别 Homebrew/Scoop 安装路径 → shell 出该渠道自己的升级命令 →
//! 核对本机 plugin 是否落后。CLI（`bin/skz.rs`）只负责 `current_exe()` 探测和
//! TTY 问答 glue——这里的函数要能被未来的 MCP server 复用，那种入口没有
//! "终端"这个概念（同 `plugin.rs` 的 lib/bin 分层）。
//!
//! **为什么 staleness 比对不直接用 `plugin::status()` 自带的 `stale` 字段**：
//! 那个字段硬编码比对 `env!(CARGO_PKG_VERSION)`——也就是"正在跑这次检查的进程自己的
//! 版本"。升级成功的那一刻，磁盘上的二进制换了，但这份检查仍在**旧**进程里跑，`env!()`
//! 还是旧值，跟旧标记一比"完全一致"，把真正需要刷新的场景直接漏掉。这里把比对基准做成
//! 显式参数（`find_stale`），未升级时传 `env!()` 自己，升级后传二次探测到的新版本，
//! 同一份逻辑两种场景都对。

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::error::Error;
use crate::plugin::{self, Target};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Brew,
    Scoop,
    Unknown,
}

impl Channel {
    pub fn as_str(self) -> &'static str {
        match self {
            Channel::Brew => "brew",
            Channel::Scoop => "scoop",
            Channel::Unknown => "unknown",
        }
    }
}

/// 纯函数：手动按 `/` 切分（先把 `\` 统一替换成 `/`），不用
/// `std::path::Path::components()`——后者按编译目标 OS 解析，在 macOS/Linux 主机上测
/// 一个 Windows 反斜杠路径字面量，反斜杠只是个普通文件名字符，压根不会被当分隔符切开，
/// "Windows 路径"的单测会悄悄测不出任何东西。按完整 segment 模式匹配（而非子串），
/// 避免 "Cellar-backup" 这类目录名误命中。
pub fn detect_channel(exe_path: &Path) -> Channel {
    let raw = exe_path.to_string_lossy();
    let normalized = raw.replace('\\', "/");
    let segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
    if brew_cellar_index(&segments).is_some() {
        Channel::Brew
    } else if scoop_apps_index(&segments).is_some() {
        Channel::Scoop
    } else {
        Channel::Unknown
    }
}

/// 升级后用于 `--version` 自检和 delegated plugin 刷新的稳定入口。
///
/// Homebrew 的 Cellar 版本目录和 Scoop 的版本目录会随升级变化，必须分别转到
/// `opt/skz/bin/skz` 与 `apps/skz/current/skz.exe`。
/// 检测与重建都基于字符串 segment，因此在非 Windows 主机上也能可靠测试反斜杠路径。
pub fn post_upgrade_exe(channel: Channel, exe_path: &Path) -> PathBuf {
    if channel == Channel::Unknown {
        return exe_path.to_path_buf();
    }

    let raw = exe_path.to_string_lossy();
    let normalized = raw.replace('\\', "/");
    let segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
    let stable = match channel {
        Channel::Brew => brew_cellar_index(&segments).map(|cellar| {
            let mut path = segments[..cellar].to_vec();
            path.extend(["opt", "skz", "bin", "skz"]);
            path
        }),
        Channel::Scoop => scoop_apps_index(&segments).map(|scoop| {
            let mut path = segments[..scoop + 3].to_vec();
            path.extend(["current", "skz.exe"]);
            path
        }),
        Channel::Unknown => unreachable!(),
    };

    stable
        .map(|segments| rebuilt_path(&raw, &segments))
        .unwrap_or_else(|| exe_path.to_path_buf())
}

fn brew_cellar_index(segments: &[&str]) -> Option<usize> {
    segments.windows(5).enumerate().find_map(|(index, w)| {
        (index + 5 == segments.len()
            && w[0].eq_ignore_ascii_case("Cellar")
            && w[1].eq_ignore_ascii_case("skz")
            && w[3].eq_ignore_ascii_case("bin")
            && w[4].eq_ignore_ascii_case("skz"))
        .then_some(index)
    })
}

fn scoop_apps_index(segments: &[&str]) -> Option<usize> {
    segments.windows(5).enumerate().find_map(|(index, w)| {
        (index + 5 == segments.len()
            // README 只推荐用户级 Scoop；默认全局根是 C:\ProgramData\scoop，
            // 其升级需要 `--global`，不能误走用户级 `scoop update skz`。
            && (index == 0 || !segments[index - 1].eq_ignore_ascii_case("ProgramData"))
            && w[0].eq_ignore_ascii_case("scoop")
            && w[1].eq_ignore_ascii_case("apps")
            && w[2].eq_ignore_ascii_case("skz")
            && w[4].eq_ignore_ascii_case("skz.exe"))
        .then_some(index)
    })
}

fn rebuilt_path(raw: &str, segments: &[&str]) -> PathBuf {
    let separator = if raw.contains('\\') { "\\" } else { "/" };
    let joined = segments.join(separator);
    if raw.starts_with("\\\\") {
        PathBuf::from(format!("\\\\{joined}"))
    } else if raw.starts_with("//") {
        PathBuf::from(format!("//{joined}"))
    } else if raw.starts_with('/') {
        PathBuf::from(format!("/{joined}"))
    } else {
        PathBuf::from(joined)
    }
}

/// impure：shell 出该渠道自己的 upgrade 命令。调用方保证 `channel != Unknown`。
pub fn upgrade(channel: Channel) -> Result<(), Error> {
    let (tool, args, command): (&str, &[&str], &'static str) = match channel {
        Channel::Brew => ("brew", &["upgrade", "skz"], "brew upgrade skz"),
        Channel::Scoop => ("scoop", &["update", "skz"], "scoop update skz"),
        Channel::Unknown => unreachable!("caller must not attempt upgrade for Unknown channel"),
    };
    match Command::new(tool).args(args).output() {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(Error::UpgradeFailed {
            channel: channel.as_str(),
            command,
            message: format!(
                "`{tool} {}` 退出码 {}：{}",
                args.join(" "),
                out.status
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "被信号终止".to_string()),
                tail(&String::from_utf8_lossy(&out.stderr), 2000),
            ),
        }),
        Err(e) => Err(Error::UpgradeFailed {
            channel: channel.as_str(),
            command,
            message: format!("无法执行 `{tool} {}`：{e}", args.join(" ")),
        }),
    }
}

/// 重新探测到的磁盘上的真实版本（`--version` 的 JSON 输出）。
pub struct VersionProbe {
    pub cli: String,
    pub contract: String,
}

/// impure、尽力而为：重新 spawn `<exe_path> --version` 读磁盘上现在的真实版本
/// （Homebrew/Scoop 是各自的稳定 current 路径）。失败一律 `None`——这是升级
/// 成功之后的确认步骤，它自己出错不该把一次成功的升级拖成命令整体失败。
pub fn probe_version(exe_path: &Path) -> Option<VersionProbe> {
    let out = Command::new(exe_path).arg("--version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    Some(VersionProbe {
        cli: v.get("cli")?.as_str()?.to_string(),
        contract: v.get("contract")?.as_str()?.to_string(),
    })
}

/// 本机已装且带 SKZ receipt 的 plugin。
pub struct MarkedPlugin {
    pub target: &'static str,
    pub installed_cli: String,
    pub installed_contract: String,
    pub requires_refresh: bool,
}

/// impure：给定一组本机 harness，收集其中带 SKZ receipt 的 user plugin。
/// 目标列表由调用方传入（通常是 `plugin::present_targets()`），不在这里
/// 重新计算一遍——调用方往往还要拿这份列表去填报告的 `checked_targets`。
pub fn installed_plugins(targets: &[Target]) -> Result<Vec<MarkedPlugin>, Error> {
    let mut marked = Vec::new();
    for &target in targets {
        let status = plugin::status(target)?;
        if let (Some(cli), Some(contract)) = (status.installed_cli, status.installed_contract) {
            marked.push(MarkedPlugin {
                target: target.as_str(),
                installed_cli: cli,
                installed_contract: contract,
                requires_refresh: status.needs_upgrade,
            });
        }
    }
    Ok(marked)
}

#[derive(Serialize)]
pub struct StalePlugin {
    pub target: &'static str,
    pub installed_cli: String,
    pub installed_contract: String,
}

/// 纯函数：给定显式比对基准，挑出过期的 plugin（模块文档说明了为什么不能用
/// `plugin::status()` 自带的 `stale` 字段）。
pub fn find_stale(marked: &[MarkedPlugin], ref_cli: &str, ref_contract: &str) -> Vec<StalePlugin> {
    marked
        .iter()
        .filter(|b| {
            b.requires_refresh || b.installed_cli != ref_cli || b.installed_contract != ref_contract
        })
        .map(|b| StalePlugin {
            target: b.target,
            installed_cli: b.installed_cli.clone(),
            installed_contract: b.installed_contract.clone(),
        })
        .collect()
}

#[derive(Serialize)]
pub struct RefreshOutcome {
    pub target: &'static str,
    pub ok: bool,
    pub detail: String,
}

/// 版本没变时用当前进程定位并验证的同级 bundle 直接安装。
pub fn refresh_in_process(targets: &[Target]) -> Vec<RefreshOutcome> {
    targets
        .iter()
        .map(|&t| match plugin::upgrade(t) {
            Ok(r) => RefreshOutcome {
                target: t.as_str(),
                ok: true,
                detail: format!("installed {} plugin", r.plugin),
            },
            Err(e) => RefreshOutcome {
                target: t.as_str(),
                ok: false,
                detail: e.to_string(),
            },
        })
        .collect()
}

/// 确认版本变了时用：转手给磁盘上的新二进制执行 `plugin upgrade`。
/// 避免旧进程使用旧版本目录里的 bundle。
/// 不回读子进程 stdout 做 JSON 结构化解析：`plugin.rs` 的 `Serialize` 结构体里有
/// `&'static str` 字段，没法从运行期缓冲区反序列化出 `'static` 生命周期，这里只按
/// 退出码判定成败。
pub fn refresh_delegated(exe_path: &Path, targets: &[Target]) -> Vec<RefreshOutcome> {
    targets
        .iter()
        .map(|&t| {
            let run = Command::new(exe_path)
                .args(["plugin", "upgrade", t.as_str()])
                .output();
            match run {
                Ok(out) if out.status.success() => RefreshOutcome {
                    target: t.as_str(),
                    ok: true,
                    detail: tail(&String::from_utf8_lossy(&out.stdout), 500),
                },
                Ok(out) => RefreshOutcome {
                    target: t.as_str(),
                    ok: false,
                    detail: tail(&String::from_utf8_lossy(&out.stderr), 500),
                },
                Err(e) => RefreshOutcome {
                    target: t.as_str(),
                    ok: false,
                    detail: e.to_string(),
                },
            }
        })
        .collect()
}

/// Plugin 新鲜度小节：数据形状纯粹，是否问人、问的结果这些"只有终端场景才有意义"的字段
/// 由 `bin/skz.rs` 填——report 结构体只装数据、glue 逻辑留给调用方。
#[derive(Serialize)]
pub struct PluginsReport {
    pub checked_targets: Vec<&'static str>,
    /// 只在 `updated` 探测失败（`Option::None`）时为 `false`——那种情况下没有可信的比对
    /// 基准，宁可不评估，也不要用错误的基准假装评估过。
    pub evaluated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
    pub stale: Vec<StalePlugin>,
    pub refresh_offered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_accepted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refreshed: Option<Vec<RefreshOutcome>>,
}

#[derive(Serialize)]
pub struct UpdateReport {
    pub channel: &'static str,
    pub attempted: bool,
    /// `None` = 升级子进程退出 0，但重新探测磁盘上的新版本号失败——不是"没变"，是
    /// "不知道"，两者不能共用一个裸 `bool`（参见模块文档 `WriteNetwork` 那次教训的同类问题）。
    pub updated: Option<bool>,
    /// 本进程自己的（升级前）版本。
    pub cli: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cli_after: Option<String>,
    /// 只在 `channel == "unknown"` 时出现：指回 README 的四种安装渠道，不指向
    /// GitHub Release；原始二进制分发不是受支持的自动升级渠道。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<serde_json::Value>,
    pub plugins: PluginsReport,
}

/// 截断到最后 `max` 字节（对齐到字符边界），供子进程输出摘要用——不在成功 JSON 里
/// 回显整段包管理器输出，这里只用于失败诊断信息和 delegated 刷新的简短摘要。
fn tail(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.len() <= max {
        return s.to_string();
    }
    let cut = s.len() - max;
    let boundary = (cut..=s.len())
        .find(|&i| s.is_char_boundary(i))
        .unwrap_or(s.len());
    format!("…{}", &s[boundary..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn refresh_delegated_passes_target_as_positional_argument() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::TempDir::new().unwrap();
        let exe = dir.path().join("skz-record-args");
        let args = dir.path().join("args");
        std::fs::write(
            &exe,
            format!("#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\n", args.display()),
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&exe).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&exe, permissions).unwrap();

        let outcomes = refresh_delegated(&exe, &[Target::Codex]);

        assert!(outcomes[0].ok);
        assert_eq!(
            std::fs::read_to_string(args).unwrap(),
            "plugin\nupgrade\ncodex\n"
        );
    }

    #[test]
    fn detect_channel_brew_linux_and_macos_paths() {
        for path in [
            "/home/linuxbrew/.linuxbrew/Cellar/skz/0.1.9/bin/skz",
            "/opt/homebrew/Cellar/skz/0.1.9/bin/skz",
            "/usr/local/Cellar/skz/0.1.9/bin/skz",
        ] {
            assert_eq!(detect_channel(Path::new(path)), Channel::Brew, "{path}");
        }
    }

    #[test]
    fn detect_channel_scoop_slash_and_backslash_paths() {
        for path in [
            r"C:/Users/x/scoop/apps/skz/0.1.9/skz.exe",
            r"C:\Users\x\scoop\apps\skz\current\skz.exe",
        ] {
            assert_eq!(detect_channel(Path::new(path)), Channel::Scoop, "{path}");
        }
    }

    #[test]
    fn detect_channel_unrelated_path_is_unknown() {
        assert_eq!(
            detect_channel(Path::new("/usr/local/bin/skz")),
            Channel::Unknown
        );
    }

    #[test]
    fn detect_channel_rejects_similar_brew_and_scoop_paths() {
        for path in [
            "/opt/Cellar-backup/skz/0.1.9/bin/skz",
            "/opt/Cellar/skz-cli/0.1.9/bin/skz",
            "/opt/Cellar/skz/0.1.9/bin/skz-helper",
            "/opt/Cellar/skz/0.1.9/bin/skz/extra",
            r"C:\Users\x\scoop-backup\apps\skz\0.1.9\skz.exe",
            r"C:\Users\x\scoop\apps\skz-cli\0.1.9\skz.exe",
            r"C:\Users\x\scoop\apps\skz\0.1.9\helper.exe",
            r"C:\Users\x\scoop\apps\skz\0.1.9\skz.exe\extra",
            r"C:\ProgramData\scoop\apps\skz\0.1.9\skz.exe",
        ] {
            assert_eq!(detect_channel(Path::new(path)), Channel::Unknown, "{path}");
        }
    }

    #[test]
    fn post_upgrade_exe_uses_brew_opt_path() {
        assert_eq!(
            post_upgrade_exe(
                Channel::Brew,
                Path::new("/home/linuxbrew/.linuxbrew/Cellar/skz/0.1.9/bin/skz")
            ),
            PathBuf::from("/home/linuxbrew/.linuxbrew/opt/skz/bin/skz")
        );
    }

    #[test]
    fn post_upgrade_exe_uses_scoop_current_path() {
        assert_eq!(
            post_upgrade_exe(
                Channel::Scoop,
                Path::new(r"C:\Users\x\scoop\apps\skz\0.1.9\skz.exe")
            ),
            PathBuf::from(r"C:\Users\x\scoop\apps\skz\current\skz.exe")
        );
    }

    #[test]
    fn post_upgrade_exe_preserves_windows_verbatim_prefix() {
        assert_eq!(
            post_upgrade_exe(
                Channel::Scoop,
                Path::new(r"\\?\C:\Users\x\scoop\apps\skz\0.1.9\skz.exe")
            ),
            PathBuf::from(r"\\?\C:\Users\x\scoop\apps\skz\current\skz.exe")
        );
    }

    #[test]
    fn post_upgrade_exe_keeps_unknown_channel_unchanged() {
        let path = Path::new("/tmp/skz");
        assert_eq!(post_upgrade_exe(Channel::Unknown, path), path);
    }

    fn book(cli: &str, contract: &str) -> MarkedPlugin {
        MarkedPlugin {
            target: "claude",
            installed_cli: cli.to_string(),
            installed_contract: contract.to_string(),
            requires_refresh: false,
        }
    }

    #[test]
    fn find_stale_includes_matching_legacy_install() {
        let mut marked = book("0.2.0", "2.1");
        marked.requires_refresh = true;
        assert_eq!(find_stale(&[marked], "0.2.0", "2.1").len(), 1);
    }

    #[test]
    fn find_stale_excludes_matching_version() {
        let marked = vec![book("0.2.0", "2.1")];
        assert!(find_stale(&marked, "0.2.0", "2.1").is_empty());
    }

    #[test]
    fn find_stale_includes_different_cli() {
        let marked = vec![book("0.1.1", "2.1")];
        let stale = find_stale(&marked, "0.2.0", "2.1");
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].installed_cli, "0.1.1");
    }

    #[test]
    fn find_stale_includes_different_contract_only() {
        let marked = vec![book("0.2.0", "2.0")];
        let stale = find_stale(&marked, "0.2.0", "2.1");
        assert_eq!(stale.len(), 1);
    }

    #[test]
    fn find_stale_empty_input_is_empty_output() {
        assert!(find_stale(&[], "0.2.0", "2.1").is_empty());
    }
}
