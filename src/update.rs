//! 自更新：识别 pipx/uv 安装路径 → shell 出该渠道自己的升级命令 →
//! 核对本机技能副本是否落后。CLI（`bin/skz.rs`）只负责 `current_exe()` 探测和
//! TTY 问答 glue——这里的函数要能被未来的 MCP server 复用，那种入口没有
//! "终端"这个概念（同 `skill.rs` 的 lib/bin 分层）。
//!
//! **为什么 staleness 比对不直接用 `skill::status()` 自带的 `stale` 字段**：
//! 那个字段硬编码比对 `env!(CARGO_PKG_VERSION)`——也就是"正在跑这次检查的进程自己的
//! 版本"。升级成功的那一刻，磁盘上的二进制换了，但这份检查仍在**旧**进程里跑，`env!()`
//! 还是旧值，跟旧标记一比"完全一致"，把真正需要刷新的场景直接漏掉。这里把比对基准做成
//! 显式参数（`find_stale`），未升级时传 `env!()` 自己，升级后传二次探测到的新版本，
//! 同一份逻辑两种场景都对。

use std::path::Path;
use std::process::Command;

use serde::Serialize;

use crate::error::Error;
use crate::skill::{self, Scope, Target};

/// 发行包名（pip/pipx/uv 认这个），不是二进制名 "skz"——传错会让升级命令直接失败。
pub const PACKAGE_NAME: &str = "skz-quant-cli";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Pipx,
    Uv,
    Unknown,
}

impl Channel {
    pub fn as_str(self) -> &'static str {
        match self {
            Channel::Pipx => "pipx",
            Channel::Uv => "uv",
            Channel::Unknown => "unknown",
        }
    }
}

/// 纯函数：手动按 `/` 切分（先把 `\` 统一替换成 `/`），不用
/// `std::path::Path::components()`——后者按编译目标 OS 解析，在 macOS/Linux 主机上测
/// 一个 Windows 反斜杠路径字面量，反斜杠只是个普通文件名字符，压根不会被当分隔符切开，
/// "Windows 路径"的单测会悄悄测不出任何东西。相邻两段匹配（而非子串匹配），避免
/// "pipx-venvs-backup" 这类目录名误命中。
pub fn detect_channel(exe_path: &Path) -> Channel {
    let normalized = exe_path.to_string_lossy().replace('\\', "/");
    let segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
    let adjacent = |a: &str, b: &str| {
        segments
            .windows(2)
            .any(|w| w[0].eq_ignore_ascii_case(a) && w[1].eq_ignore_ascii_case(b))
    };
    if adjacent("pipx", "venvs") {
        Channel::Pipx
    } else if adjacent("uv", "tools") {
        Channel::Uv
    } else {
        Channel::Unknown
    }
}

/// impure：shell 出该渠道自己的 upgrade 命令。调用方保证 `channel != Unknown`。
/// pipx/uv 会自动复用各自安装时记下的 `--index-url`（`pipx_metadata.json` /
/// `uv-receipt.toml`），这里不用也不该自己再传一次；认证同理是它们自己的事
/// （netrc 之类），`skz` 全程不摸凭据。
pub fn upgrade(channel: Channel) -> Result<(), Error> {
    let (tool, args, command): (&str, &[&str], &'static str) = match channel {
        Channel::Pipx => (
            "pipx",
            &["upgrade", PACKAGE_NAME],
            "pipx upgrade skz-quant-cli",
        ),
        Channel::Uv => (
            "uv",
            &["tool", "upgrade", PACKAGE_NAME],
            "uv tool upgrade skz-quant-cli",
        ),
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
/// （此时这个路径大概率已经是 pip/uv 写完的新文件）。失败一律 `None`——这是升级
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

/// 本机（`Scope::User`）已装且带归属标记的技能册——installed 或 stale 都算，
/// foreign/从未装过的不算（`skill::status()` 里没有归属标记就没有 `installed_cli`）。
pub struct MarkedBook {
    pub target: &'static str,
    pub book: &'static str,
    pub path: String,
    pub installed_cli: String,
    pub installed_contract: String,
}

/// impure：给定一组本机 harness，收集其中已装册子（只查 `Scope::User`——
/// `Scope::Project` 挂在 cwd 下，`update` 跑在哪个目录跟"技能是否过期"这件事没有稳定
/// 关系，不查）。目标列表由调用方传入（通常是 `skill::present_targets()`），不在这里
/// 重新计算一遍——调用方往往还要拿这份列表去填报告的 `checked_targets`。
pub fn installed_books(targets: &[Target]) -> Result<Vec<MarkedBook>, Error> {
    let mut marked = Vec::new();
    for &target in targets {
        let status = skill::status(target, Scope::User)?;
        for book in status.books {
            if let (Some(cli), Some(contract)) = (book.installed_cli, book.installed_contract) {
                marked.push(MarkedBook {
                    target: target.as_str(),
                    book: book.name,
                    path: book.path,
                    installed_cli: cli,
                    installed_contract: contract,
                });
            }
        }
    }
    Ok(marked)
}

#[derive(Serialize)]
pub struct StaleSkill {
    pub target: &'static str,
    pub book: &'static str,
    pub path: String,
    pub installed_cli: String,
    pub installed_contract: String,
}

/// 纯函数：给定显式比对基准，挑出过期的册子（模块文档说明了为什么不能用
/// `skill::status()` 自带的 `stale` 字段）。
pub fn find_stale(marked: &[MarkedBook], ref_cli: &str, ref_contract: &str) -> Vec<StaleSkill> {
    marked
        .iter()
        .filter(|b| b.installed_cli != ref_cli || b.installed_contract != ref_contract)
        .map(|b| StaleSkill {
            target: b.target,
            book: b.book,
            path: b.path.clone(),
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

/// 版本没变时用：当前进程自己内嵌的 `include_str!` 内容就是权威内容，直接调
/// `skill::install`。
pub fn refresh_in_process(targets: &[Target]) -> Vec<RefreshOutcome> {
    targets
        .iter()
        .map(|&t| match skill::install(t, Scope::User) {
            Ok(r) => RefreshOutcome {
                target: t.as_str(),
                ok: true,
                detail: format!("installed {} books", r.installed.len()),
            },
            Err(e) => RefreshOutcome {
                target: t.as_str(),
                ok: false,
                detail: e.to_string(),
            },
        })
        .collect()
}

/// 确认版本变了时用：转手给磁盘上的新二进制自己执行 `skills install`，不在旧进程里
/// 直接调 `skill::install()`——旧进程手上的 `include_str!` 内容本来就是要被换掉的那份。
/// 不回读子进程 stdout 做 JSON 结构化解析：`skill.rs` 的 `Serialize` 结构体里有
/// `&'static str` 字段，没法从运行期缓冲区反序列化出 `'static` 生命周期，这里只按
/// 退出码判定成败。
pub fn refresh_delegated(exe_path: &Path, targets: &[Target]) -> Vec<RefreshOutcome> {
    targets
        .iter()
        .map(|&t| {
            let run = Command::new(exe_path)
                .args(["skills", "install", "--target", t.as_str()])
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

/// 技能新鲜度小节：数据形状纯粹，是否问人、问的结果这些"只有终端场景才有意义"的字段
/// 由 `bin/skz.rs` 填——同 `skill.rs` 里 report 结构体只装数据、glue 逻辑留给调用方的分层。
#[derive(Serialize)]
pub struct SkillsReport {
    pub checked_targets: Vec<&'static str>,
    /// 只在 `updated` 探测失败（`Option::None`）时为 `false`——那种情况下没有可信的比对
    /// 基准，宁可不评估，也不要用错误的基准假装评估过。
    pub evaluated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
    pub stale: Vec<StaleSkill>,
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
    /// 只在 `channel == "unknown"` 时出现：指回 README 的两条安装命令，不指向
    /// GitHub Release；原始二进制分发不是受支持的自动升级渠道。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<serde_json::Value>,
    pub skills: SkillsReport,
}

/// 截断到最后 `max` 字节（对齐到字符边界），供子进程输出摘要用——不在成功 JSON 里
/// 回显整段 pipx/uv 输出，这里只用于失败诊断信息和 delegated 刷新的简短摘要。
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

    #[test]
    fn detect_channel_pipx_path() {
        assert_eq!(
            detect_channel(Path::new(
                "/Users/x/.local/pipx/venvs/skz-quant-cli/bin/skz"
            )),
            Channel::Pipx
        );
    }

    #[test]
    fn detect_channel_uv_path() {
        assert_eq!(
            detect_channel(Path::new(
                "/Users/x/.local/share/uv/tools/skz-quant-cli/bin/skz"
            )),
            Channel::Uv
        );
    }

    #[test]
    fn detect_channel_unrelated_path_is_unknown() {
        assert_eq!(
            detect_channel(Path::new("/usr/local/bin/skz")),
            Channel::Unknown
        );
    }

    #[test]
    fn detect_channel_requires_adjacent_segments_not_substring() {
        // "pipx" 和 "venvs" 都出现在路径字符串里，但不是相邻两段——不该误判。
        assert_eq!(
            detect_channel(Path::new("/opt/pipx-venvs-backup/skz")),
            Channel::Unknown
        );
    }

    #[test]
    fn detect_channel_case_insensitive() {
        assert_eq!(
            detect_channel(Path::new(
                r"C:\Users\x\Pipx\Venvs\skz-quant-cli\Scripts\skz.exe"
            )),
            Channel::Pipx
        );
    }

    #[test]
    fn detect_channel_windows_backslash_path() {
        // Path::components() 会按宿主机 OS 解析分隔符——这条在 macOS/Linux 开发机上跑
        // 也必须正确识别反斜杠，才能证明是手动切分在起作用，不是摆设。
        assert_eq!(
            detect_channel(Path::new(
                r"C:\Users\x\.local\share\uv\tools\skz-quant-cli\Scripts\skz.exe"
            )),
            Channel::Uv
        );
    }

    fn book(cli: &str, contract: &str) -> MarkedBook {
        MarkedBook {
            target: "claude",
            book: "factor",
            path: "/tmp/skz-factor".to_string(),
            installed_cli: cli.to_string(),
            installed_contract: contract.to_string(),
        }
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
