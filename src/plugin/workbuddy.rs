//! Windows desktop adapter. Only the bundled CLI writes WorkBuddy state.
use std::collections::BTreeMap;
use std::process::Stdio;

use super::*;

const TARGET: Target = Target::Workbuddy;
const ID: &str = "skz@skz";
const ENTRY: &str = "resources/app.asar.unpacked/cli/bin/codebuddy";

#[cfg(test)]
#[path = "workbuddy_tests.rs"]
mod tests;

fn argument(message: impl Into<String>) -> Error {
    Error::Args(message.into())
}

fn app_dir() -> Result<PathBuf, Error> {
    let path = std::env::var_os("SKZ_WORKBUDDY_APP_DIR")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("Programs/WorkBuddy"))
        })
        .ok_or_else(|| {
            argument("找不到 WorkBuddy；请设置 SKZ_WORKBUDDY_APP_DIR 为桌面安装根目录")
        })?;
    if !path.is_absolute() {
        return Err(argument("SKZ_WORKBUDDY_APP_DIR 必须是绝对路径"));
    }
    Ok(path)
}

pub(super) fn is_present() -> bool {
    cfg!(windows)
        && app_dir().is_ok_and(|p| p.join("WorkBuddy.exe").is_file() && p.join(ENTRY).is_file())
}

struct Adapter {
    entry: PathBuf,
    config: PathBuf,
    source: PathBuf,
}

#[derive(Debug, Deserialize)]
struct Installed {
    id: String,
    scope: String,
    enabled: bool,
    #[serde(rename = "installPath")]
    path: PathBuf,
}

#[derive(Deserialize)]
struct Registration {
    scope: String,
    #[serde(rename = "installPath")]
    path: PathBuf,
}

#[derive(Deserialize)]
struct Registry {
    version: u32,
    plugins: BTreeMap<String, Vec<Registration>>,
}

fn read_json(path: &Path) -> Result<Option<serde_json::Value>, Error> {
    match fs::read(path) {
        Ok(raw) => serde_json::from_slice(&raw)
            .map(Some)
            .map_err(|e| argument(format!("invalid WorkBuddy state {}: {e}", path.display()))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(argument(format!("cannot read {}: {e}", path.display()))),
    }
}

fn same_path(left: &Path, right: &Path) -> bool {
    if !left.is_absolute() || !right.is_absolute() {
        return false;
    }
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

impl Adapter {
    fn locate() -> Result<Self, Error> {
        if !cfg!(windows) {
            return Err(argument(
                "WorkBuddy 首版仅支持 Windows；不支持 WSL 跨系统管理 Windows 桌面版",
            ));
        }
        if !is_present() {
            return Err(argument(
                "找不到 WorkBuddy 桌面包内 CLI；请安装 WorkBuddy 或设置 SKZ_WORKBUDDY_APP_DIR",
            ));
        }
        let config = match std::env::var_os("WORKBUDDY_CONFIG_DIR").filter(|p| !p.is_empty()) {
            Some(p) => PathBuf::from(p),
            None => home()?.join(".workbuddy"),
        };
        if !config.is_absolute() {
            return Err(argument("WORKBUDDY_CONFIG_DIR 必须是绝对路径"));
        }
        Ok(Self {
            entry: app_dir()?.join(ENTRY),
            config,
            source: source_root(TARGET)?,
        })
    }

    fn run(&self, args: &[&str]) -> Result<String, Error> {
        if !executable_on_path("node") {
            return Err(argument(
                "WorkBuddy 插件管理需要 PATH 中的 Node.js；请安装 Node.js（最低版本由 WorkBuddy 入口检查）",
            ));
        }
        let mut command = Command::new("node");
        command
            .arg(&self.entry)
            .args(args)
            // Do not load project-scope settings from the user's current checkout.
            .current_dir(
                self.entry
                    .parent()
                    .ok_or_else(|| fail("invalid WorkBuddy entry"))?,
            )
            .env("CODEBUDDY_CONFIG_DIR", &self.config)
            .env("WORKBUDDY_CONFIG_DIR", &self.config)
            .stdin(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let output = command.output().map_err(|e| {
            argument(format!(
                "cannot start WorkBuddy bundled CLI: {e}; 请检查 Node.js 和 SKZ_WORKBUDDY_APP_DIR"
            ))
        })?;
        // Some desktop builds report command failures on stdout but exit 0.
        if !output.status.success()
            || String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(|line| line.trim_start().starts_with('✘'))
        {
            return Err(argument(format!(
                "WorkBuddy {} failed: {}\n{}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim(),
                String::from_utf8_lossy(&output.stdout).trim()
            )));
        }
        String::from_utf8(output.stdout)
            .map(|s| s.trim().to_owned())
            .map_err(|e| argument(format!("invalid WorkBuddy output: {e}")))
    }

    fn preflight(&self) -> Result<(), Error> {
        for (args, required) in [
            (
                vec!["plugin", "--help"],
                vec!["install", "uninstall", "update", "list", "disable"],
            ),
            (
                vec!["plugin", "marketplace", "--help"],
                vec!["add", "update", "remove"],
            ),
        ] {
            let help = self.run(&args)?;
            if !required.iter().all(|name| help.contains(name)) {
                return Err(argument(
                    "WorkBuddy 包内 CLI 缺少所需 plugin 命令；请升级 WorkBuddy",
                ));
            }
        }
        Ok(())
    }

    // The marketplace list has no JSON flag. Read its registry, never edit it.
    fn owned_market(&self) -> Result<bool, Error> {
        let Some(value) = read_json(&self.config.join("plugins/known_marketplaces.json"))? else {
            return Ok(false);
        };
        let markets = value
            .as_object()
            .ok_or_else(|| argument("invalid WorkBuddy marketplace registry"))?;
        let Some(market) = markets.get("skz") else {
            return Ok(false);
        };
        let path = market
            .pointer("/source/path")
            .and_then(|v| v.as_str())
            .map(Path::new);
        let location = market
            .get("installLocation")
            .and_then(|v| v.as_str())
            .map(Path::new);
        if market.get("type").and_then(|v| v.as_str()) != Some("directory")
            || market.pointer("/source/source").and_then(|v| v.as_str()) != Some("directory")
            || !path.is_some_and(|p| same_path(p, &self.source))
            || !location.is_some_and(|p| same_path(p, &self.source))
        {
            return Err(argument(
                "WorkBuddy 的 skz 市场指向其他来源；拒绝覆盖或卸载，请先在 WorkBuddy 中处理同名市场",
            ));
        }
        Ok(true)
    }

    fn registry(&self) -> Result<BTreeMap<String, Vec<Registration>>, Error> {
        let Some(value) = read_json(&self.config.join("plugins/installed_plugins.json"))? else {
            return Ok(BTreeMap::new());
        };
        let registry: Registry = serde_json::from_value(value)
            .map_err(|e| argument(format!("invalid WorkBuddy installed registry: {e}")))?;
        if registry.version != 2 {
            return Err(argument(
                "unsupported WorkBuddy installed registry version; 请更新 SKZ 适配器",
            ));
        }
        Ok(registry.plugins)
    }

    fn installed(&self) -> Result<Option<Installed>, Error> {
        let output = self.run(&["plugin", "list", "--json"])?;
        let entries: Vec<Installed> = serde_json::from_str(&output)
            .map_err(|e| argument(format!("invalid WorkBuddy plugin list JSON: {e}")))?;
        let mut found = entries
            .into_iter()
            .filter(|e| e.id == ID && e.scope == "user");
        let entry = found.next();
        if found.next().is_some() {
            return Err(argument(
                "duplicate WorkBuddy user registration for skz@skz",
            ));
        }
        let registry = self.registry()?;
        let registered: Vec<_> = registry
            .get(ID)
            .into_iter()
            .flatten()
            .filter(|e| e.scope == "user")
            .collect();
        match (&entry, registered.as_slice()) {
            (None, []) => {}
            (Some(e), [r]) if same_path(&e.path, &r.path) => {}
            _ => {
                return Err(argument(
                    "WorkBuddy plugin list and user registry disagree; 请在 WorkBuddy 中检查 skz@skz",
                ));
            }
        }
        Ok(entry)
    }

    fn live_ok(&self, bundle: &Bundle, entry: &Installed) -> bool {
        let Ok(path) = entry.path.canonicalize() else {
            return false;
        };
        let Ok(cache) = self.config.join("plugins/cache/skz/skz").canonicalize() else {
            return false;
        };
        if !path.starts_with(cache) {
            return false;
        }
        let files: Vec<_> = target_files(bundle, TARGET)
            .into_iter()
            .filter_map(|f| {
                f.path
                    .strip_prefix("workbuddy/plugins/skz/")
                    .map(|p| (f, path.join(p)))
            })
            .collect();
        !files.is_empty() && files.into_iter().all(|(f, p)| installed_file_ok(&p, f))
    }

    fn prepare(&self) -> Result<Option<Installed>, Error> {
        self.preflight()?;
        let owned = self.owned_market()?;
        let registry = self.registry()?;
        // Updating a shared cache or marketplace may affect other scopes.
        if registry
            .get(ID)
            .is_some_and(|entries| entries.iter().any(|e| e.scope != "user"))
        {
            return Err(argument(
                "skz@skz 同时存在其他 scope；拒绝升级共享缓存，请先在 WorkBuddy 中处理",
            ));
        }
        if !owned && registry.keys().any(|id| id.ends_with("@skz")) {
            return Err(argument("WorkBuddy 存在来源不明的 @skz 安装登记；拒绝覆盖"));
        }
        self.installed()
    }

    fn install(&self, previous: Option<&Installed>) -> Result<(), Error> {
        if !self.owned_market()? {
            self.run(&[
                "plugin",
                "marketplace",
                "add",
                &self.source.to_string_lossy(),
            ])?;
        } else {
            self.run(&["plugin", "marketplace", "update", "skz"])?;
        }
        if !self.owned_market()? {
            return Err(argument("WorkBuddy 未注册 SKZ 本地市场"));
        }
        self.run(&[
            "plugin",
            if previous.is_some() {
                "update"
            } else {
                "install"
            },
            ID,
            "--scope",
            "user",
        ])?;
        if previous.is_some_and(|p| !p.enabled) && self.installed()?.is_some_and(|p| p.enabled) {
            self.run(&["plugin", "disable", ID, "--scope", "user"])?;
        }
        Ok(())
    }

    // Return whether the local source must stay for another installation scope.
    fn remove(&self) -> Result<bool, Error> {
        self.preflight()?;
        if !self.owned_market()? {
            if self.registry()?.keys().any(|id| id.ends_with("@skz")) {
                return Err(argument("WorkBuddy @skz 登记没有可信市场来源；拒绝卸载"));
            }
            return Ok(false);
        }
        if self.installed()?.is_some() {
            self.run(&["plugin", "uninstall", ID, "--scope", "user"])?;
            if self.installed()?.is_some() {
                return Err(argument(
                    "WorkBuddy 未移除用户范围的 skz@skz；保留 SKZ 状态供重试",
                ));
            }
        }
        let remaining = self
            .registry()?
            .iter()
            .any(|(id, entries)| id.ends_with("@skz") && !entries.is_empty());
        if !remaining {
            self.run(&["plugin", "marketplace", "remove", "skz"])?;
            if self.owned_market()? {
                return Err(argument("WorkBuddy 未移除 SKZ 市场；保留本地来源供重试"));
            }
        }
        Ok(remaining)
    }
}

pub(super) fn reconcile(upgrade: bool) -> Result<InstallReport, Error> {
    let adapter = Adapter::locate()?;
    let bundle = load_bundle()?;
    reconcile_with(&adapter, &bundle, upgrade)
}

fn reconcile_with(
    adapter: &Adapter,
    bundle: &Bundle,
    upgrade: bool,
) -> Result<InstallReport, Error> {
    let previous = adapter.prepare()?;
    let current = previous
        .as_ref()
        .is_some_and(|p| adapter.live_ok(bundle, p));
    let root = adapter
        .source
        .parent()
        .ok_or_else(|| fail("invalid SKZ source"))?;
    let pending_path = root.join("workbuddy-pending.json");
    let pending = read_json(&pending_path)?;
    let preserve_disabled = previous.as_ref().is_some_and(|p| !p.enabled)
        || pending
            .as_ref()
            .and_then(|p| p.get("preserve_disabled"))
            .and_then(|v| v.as_bool())
            == Some(true);
    let source = copy_target_to(bundle, TARGET, root)?;
    fs::write(
        &pending_path,
        serde_json::json!({"preserve_disabled": preserve_disabled}).to_string(),
    )
    .map_err(|e| fail(e.to_string()))?;
    if upgrade || !current {
        adapter.install(previous.as_ref())?;
    }
    // Native update may reuse a version cache without validating its bytes.
    // Repair through native commands, retaining plugin data and install scope.
    if let Some(entry) = adapter.installed()?
        && !adapter.live_ok(bundle, &entry)
    {
        adapter.run(&["plugin", "uninstall", ID, "--scope", "user", "--keep-data"])?;
        adapter.run(&["plugin", "install", ID, "--scope", "user"])?;
    }
    if preserve_disabled && adapter.installed()?.is_some_and(|p| p.enabled) {
        adapter.run(&["plugin", "disable", ID, "--scope", "user"])?;
    }
    let installed = adapter
        .installed()?
        .ok_or_else(|| argument("WorkBuddy 未登记用户范围的 skz@skz；未写入成功 receipt"))?;
    if !adapter.live_ok(bundle, &installed) {
        return Err(argument(
            "WorkBuddy 实际安装内容与 bundle 不一致；未写入成功 receipt，请检查宿主缓存后重试",
        ));
    }
    if preserve_disabled && installed.enabled {
        return Err(argument("WorkBuddy 未保留禁用状态；未写入成功 receipt"));
    }
    write_receipt_to(TARGET, digest(&target_files(bundle, TARGET)), root)?;
    fs::remove_file(pending_path).map_err(|e| fail(e.to_string()))?;
    Ok(InstallReport {
        target: TARGET.as_str(),
        plugin: "skz",
        installed: true,
        cli: env!("CARGO_PKG_VERSION"),
        contract: CONTRACT,
        source: source.display().to_string(),
        migrated_legacy: vec![],
        note: Some("请重启 WorkBuddy 或开启新会话以加载更新；已禁用的 SKZ 插件保持禁用。".into()),
    })
}

pub(super) fn status() -> Result<StatusReport, Error> {
    let adapter = Adapter::locate()?;
    let bundle = load_bundle()?;
    let owned = adapter.owned_market()?;
    let native = adapter.installed()?;
    let receipt = read_receipt(TARGET);
    let native_ok = owned && native.is_some();
    let content_ok = staged_content_ok(&bundle, TARGET)
        && native.as_ref().is_some_and(|p| adapter.live_ok(&bundle, p));
    let pending = state_root(TARGET)?.join("workbuddy-pending.json").exists();
    let fresh = !pending
        && receipt.as_ref().is_some_and(|r| {
            r.plugin == "skz"
                && r.target == "workbuddy"
                && r.cli == env!("CARGO_PKG_VERSION")
                && r.contract == CONTRACT
                && r.digest == digest(&target_files(&bundle, TARGET))
        });
    Ok(StatusReport {
        target: TARGET.as_str(),
        plugin: "skz",
        installed: receipt.is_some() && native_ok && content_ok,
        installed_cli: receipt.as_ref().map(|r| r.cli.clone()),
        installed_contract: receipt.as_ref().map(|r| r.contract.clone()),
        current_cli: env!("CARGO_PKG_VERSION"),
        current_contract: CONTRACT,
        content_ok,
        native_ok,
        needs_upgrade: !fresh || !content_ok || !native_ok,
    })
}

pub(super) fn uninstall() -> Result<UninstallReport, Error> {
    let adapter = Adapter::locate()?;
    let existed = read_receipt(TARGET).is_some() || adapter.owned_market()?;
    let keep_source = adapter.remove()?;
    let root = state_root(TARGET)?;
    if keep_source {
        for name in [RECEIPT, "workbuddy-pending.json"] {
            let file = root.join(name);
            if file.exists() {
                fs::remove_file(file).map_err(|e| fail(e.to_string()))?;
            }
        }
    } else if root.exists() {
        fs::remove_dir_all(root).map_err(|e| fail(e.to_string()))?;
    }
    Ok(UninstallReport {
        target: TARGET.as_str(),
        plugin: "skz",
        removed: existed,
    })
}
