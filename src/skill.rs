//! 技能套件安装器：把内嵌的四册技能装成各 harness 的原生技能包。
//!
//! 设计边界（重要）：**只往我们自己的技能目录写，绝不碰用户的任何配置文件**
//! （settings.json / CLAUDE.md 一概不动）。这样卸载 = 删自己那几个目录，完全可逆，
//! 不需要去 unmerge 别人 JSON 里的条目。想要权限规则兜底的用户，`permissions`
//! 只打印文本，贴不贴由他自己决定。
//!
//! 内容的唯一真源是二进制（`include_str!`）：安装器既是真源又是写文件的手，
//! 所以装出来的内容不可能描述一个二进制没有的命令。唯一的漂移窗口是
//! 「升级了二进制但忘了重装」——用 `.skz-install.json` 里的版本戳 + `status` 比对关掉。

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Error;

/// 契约版本：与 `--version` 输出保持一致。
pub const CONTRACT: &str = "2.15";

/// 安装标记文件名。它同时是**归属证明**：没有它的同名目录不是我们装的，
/// install 不覆盖、uninstall 不删——否则 uninstall 就是对用户 home 下路径的无保护 rm -rf。
const MARKER: &str = ".skz-install.json";

/// 目标 harness。四家的技能约定**实测/查证一致**：`<root>/skills/<name>/SKILL.md`
/// 加 `name`/`description` frontmatter，所以 adapter 只是换一个根目录，内容不必改写。
/// claude 与 codex 在本机实证；openclaw / hermes 依官方文档。
/// 后两者另有 `~/.agents/skills` 跨 agent 共享目录，我们只装各自主目录、不碰共享区。
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
            Target::Claude => "claude",
            Target::Codex => "codex",
            Target::Openclaw => "openclaw",
            Target::Hermes => "hermes",
        }
    }

    /// home / cwd 下的配置目录名（技能根 = `<这里>/skills`）。
    fn config_dir(self) -> &'static str {
        match self {
            Target::Claude => ".claude",
            Target::Codex => ".codex",
            Target::Openclaw => ".openclaw",
            Target::Hermes => ".hermes",
        }
    }

    pub const ALL: [Target; 4] = [
        Target::Claude,
        Target::Codex,
        Target::Openclaw,
        Target::Hermes,
    ];

    /// 该 harness 是否在本机出现过（装了 CLI 或有配置目录）。
    /// 只用于 `install --target all` 的自动选择——**不做探测式安装**：
    /// 给不存在的 harness 造目录既没用又是噪音。
    pub fn is_present(self) -> bool {
        directories::BaseDirs::new()
            .map(|b| b.home_dir().join(self.config_dir()).is_dir())
            .unwrap_or(false)
    }
}

/// 本机出现过的 harness 全集（`Scope::User` 下才有意义，探测看的是 home 下的配置目录）。
/// `bin/skz.rs` 的 `--target all` 与 `update` 模块的技能新鲜度核对共用这一份过滤逻辑，
/// 不各写一份容易悄悄漂移。
pub fn present_targets() -> Vec<Target> {
    Target::ALL.into_iter().filter(|t| t.is_present()).collect()
}

/// 安装范围：user = 跨项目的个人能力（默认，量化研究不属于某个 repo）；
/// project = 签进仓库、随项目走。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    User,
    Project,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::User => "user",
            Scope::Project => "project",
        }
    }
}

/// 一册技能：装出来是一个独立技能目录。
/// 拆开而不是一个塞四册，是因为**触发语义由各自的 description 决定**：
/// guide 要被显式召唤，factor/strategy/portfolio 要在聊到因子/实盘/组合时被自动想起——
/// 一个技能只有一个 description，装不下两种。
pub struct Book {
    pub name: &'static str,
    pub dir: &'static str,
    body: &'static str,
}

const COMMON: &str = include_str!("../skill/_common.md");

/// 正文里的这一行会被替换成共享前言（auth / HITL 底表 / I/O 契约）。
/// 四册各写一份副本是有意的：**安装器是装出来那些文件的唯一写者**，没人手工维护副本，
/// 所以重复不是债；而源文件 `skill/*.md` 里只留一行占位，仍是单处维护。
const COMMON_MARK: &str = "<!-- COMMON -->";

pub const BOOKS: &[Book] = &[
    Book {
        name: "factor",
        dir: "skz-factor",
        body: include_str!("../skill/factor.md"),
    },
    Book {
        name: "strategy",
        dir: "skz-strategy",
        body: include_str!("../skill/strategy.md"),
    },
    Book {
        name: "guide",
        dir: "skz-guide",
        body: include_str!("../skill/guide.md"),
    },
    Book {
        name: "portfolio",
        dir: "skz-portfolio",
        body: include_str!("../skill/portfolio.md"),
    },
];

/// 渲染一册的最终 SKILL.md（正文 + 就地展开的共享前言）。
pub fn render(book: &Book) -> String {
    book.body.replace(COMMON_MARK, COMMON.trim_end())
}

/// `show` 用：按名字取正文；无名 → 共享前言（它同时就是索引/总则）。
pub fn show(name: Option<&str>) -> Result<String, Error> {
    match name {
        None | Some("index") | Some("common") => Ok(COMMON.to_string()),
        Some(n) => BOOKS
            .iter()
            .find(|b| b.name == n)
            .map(render)
            .ok_or_else(|| {
                // 可选值从 BOOKS 现算，别手写第二份清单——改册名时手写的那份会悄悄过期
                // （`playbook` → `guide` 改名时就差点漏掉它）。
                let names: Vec<&str> = BOOKS.iter().map(|b| b.name).collect();
                Error::Args(format!(
                    "未知技能册 {n}；可选 index | {}",
                    names.join(" | ")
                ))
            }),
    }
}

#[derive(Serialize, Deserialize)]
pub struct Marker {
    pub cli: String,
    pub contract: String,
    pub book: String,
}

#[derive(Serialize)]
pub struct InstalledBook {
    pub name: &'static str,
    pub path: String,
}

#[derive(Serialize)]
pub struct InstallReport {
    pub target: &'static str,
    pub scope: &'static str,
    pub root: String,
    pub installed: Vec<InstalledBook>,
    pub cli: &'static str,
    pub contract: &'static str,
}

#[derive(Serialize)]
pub struct BookStatus {
    pub name: &'static str,
    pub path: String,
    /// 我们装的、版本一致
    pub installed: bool,
    /// 装过但版本落后于当前二进制 → 重装
    pub stale: bool,
    /// 目录被占用但不是我们装的（无归属标记）→ 不碰
    pub foreign: bool,
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
    /// 任一册 stale/缺失 → 提示重装（agent 只看这个布尔）
    pub needs_install: bool,
}

#[derive(Serialize)]
pub struct RemovedBook {
    pub name: &'static str,
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
}

/// 技能根目录。注意：**不是** credentials 的路径解析——
/// `~/.claude/skills/` 在所有平台（含 Windows）都是固定 home 相对路径；
/// credentials 在 Windows 上仍走 `LocalAppData`，两者不能划等号。
pub fn skills_root(target: Target, scope: Scope) -> Result<PathBuf, Error> {
    let sub = target.config_dir();
    match scope {
        Scope::User => {
            let base = directories::BaseDirs::new()
                .ok_or_else(|| Error::Internal("无法定位 home 目录".to_string()))?;
            Ok(base.home_dir().join(sub).join("skills"))
        }
        Scope::Project => {
            let cwd = std::env::current_dir()
                .map_err(|e| Error::Internal(format!("无法定位当前目录: {e}")))?;
            Ok(cwd.join(sub).join("skills"))
        }
    }
}

/// 读归属标记；不存在或读不动 → None（视作非我们所有）。
fn read_marker(dir: &Path) -> Option<Marker> {
    let raw = fs::read_to_string(dir.join(MARKER)).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn install(target: Target, scope: Scope) -> Result<InstallReport, Error> {
    let root = skills_root(target, scope)?;
    // 先全量检查归属，任一册被外来同名目录占用就整体拒绝：
    // 宁可什么都不装，也不要装一半让用户处在半新半旧的状态。
    for book in BOOKS {
        let dir = root.join(book.dir);
        if dir.exists() && read_marker(&dir).is_none() {
            return Err(Error::Args(format!(
                "{} 已存在且不是 skz 装的（无 {MARKER}），拒绝覆盖；请先自行处理该目录",
                dir.display()
            )));
        }
    }
    let mut installed = Vec::new();
    for book in BOOKS {
        let dir = root.join(book.dir);
        fs::create_dir_all(&dir).map_err(|e| Error::Internal(format!("创建技能目录失败: {e}")))?;
        fs::write(dir.join("SKILL.md"), render(book))
            .map_err(|e| Error::Internal(format!("写入 SKILL.md 失败: {e}")))?;
        let marker = Marker {
            cli: env!("CARGO_PKG_VERSION").to_string(),
            contract: CONTRACT.to_string(),
            book: book.name.to_string(),
        };
        let raw = serde_json::to_string(&marker)
            .map_err(|e| Error::Internal(format!("序列化安装标记失败: {e}")))?;
        fs::write(dir.join(MARKER), raw)
            .map_err(|e| Error::Internal(format!("写入安装标记失败: {e}")))?;
        installed.push(InstalledBook {
            name: book.name,
            path: dir.display().to_string(),
        });
    }
    Ok(InstallReport {
        target: target.as_str(),
        scope: scope.as_str(),
        root: root.display().to_string(),
        installed,
        cli: env!("CARGO_PKG_VERSION"),
        contract: CONTRACT,
    })
}

pub fn status(target: Target, scope: Scope) -> Result<StatusReport, Error> {
    let root = skills_root(target, scope)?;
    let mut books = Vec::new();
    let mut needs_install = false;
    for book in BOOKS {
        let dir = root.join(book.dir);
        let (installed, stale, foreign, cli, contract) = match read_marker(&dir) {
            Some(m) => {
                // 版本戳比对：装的内容是旧二进制吐的 → 提示重装。
                let same = m.cli == env!("CARGO_PKG_VERSION") && m.contract == CONTRACT;
                (same, !same, false, Some(m.cli), Some(m.contract))
            }
            None if dir.exists() => (false, false, true, None, None),
            None => (false, false, false, None, None),
        };
        if !installed {
            needs_install = true;
        }
        books.push(BookStatus {
            name: book.name,
            path: dir.display().to_string(),
            installed,
            stale,
            foreign,
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
    let mut books = Vec::new();
    for book in BOOKS {
        let dir = root.join(book.dir);
        // 只删带归属标记的目录：没有标记就不是我们的东西，绝不递归删用户 home 下的路径。
        let (removed, skipped) = if !dir.exists() {
            (false, Some("absent"))
        } else if read_marker(&dir).is_none() {
            (false, Some("foreign"))
        } else {
            fs::remove_dir_all(&dir)
                .map_err(|e| Error::Internal(format!("删除技能目录失败: {e}")))?;
            (true, None)
        };
        books.push(RemovedBook {
            name: book.name,
            path: dir.display().to_string(),
            removed,
            skipped,
        });
    }
    Ok(UninstallReport {
        target: target.as_str(),
        scope: scope.as_str(),
        root: root.display().to_string(),
        books,
    })
}

/// 建议的权限规则（**只输出文本，不写任何配置文件**）。
/// 命中 HITL 底表的写命令 → `ask`，让 harness 在调用发生前拦一道；
/// CLI 自身全程不知道有「确认」这回事，thin-CLI 分层不破。
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
