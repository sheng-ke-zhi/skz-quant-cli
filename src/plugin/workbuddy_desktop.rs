use base64::{Engine, engine::general_purpose::STANDARD};

use super::*;

#[derive(Serialize, Deserialize)]
pub(super) struct Desktop {
    pub executable: PathBuf,
    pub source: String,
}

fn binding_file() -> Result<PathBuf, Error> {
    Ok(state_root(Target::Workbuddy)?.join("desktop.json"))
}

pub(super) fn resolve(interactive: bool) -> Result<Desktop, Error> {
    if !cfg!(windows) {
        return Err(Error::Args("WorkBuddy 首版仅支持 Windows".into()));
    }
    let script = format!(
        "& {{\n{}\n}} {}",
        include_str!("workbuddy_desktop.ps1"),
        if interactive { "-Interactive" } else { "" }
    );
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let system =
        std::env::var_os("SystemRoot").ok_or_else(|| fail("Windows SystemRoot is missing"))?;
    let mut command =
        Command::new(PathBuf::from(system).join("System32/WindowsPowerShell/v1.0/powershell.exe"));
    command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-STA",
            "-EncodedCommand",
        ])
        .arg(STANDARD.encode(bytes))
        .env("SKZ_WB_BINDING_FILE", binding_file()?)
        .stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let output = command
        .output()
        .map_err(|e| fail(format!("无法读取 WorkBuddy 系统登记：{e}")))?;
    if !output.status.success() {
        return Err(Error::Args(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    let desktop: Desktop = serde_json::from_slice(&output.stdout)
        .map_err(|e| fail(format!("invalid WorkBuddy location result: {e}")))?;
    if !desktop.executable.is_absolute() || !desktop.executable.is_file() {
        return Err(Error::Args(
            "WorkBuddy 程序位置已失效，请重新检测或选择程序。".into(),
        ));
    }
    Ok(desktop)
}

pub(super) fn remember(desktop: &Desktop) -> Result<(), Error> {
    let path = binding_file()?;
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| fail(e.to_string()))?;
    let temp = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(
        &temp,
        serde_json::to_vec(desktop).map_err(|e| fail(e.to_string()))?,
    )
    .and_then(|_| fs::rename(&temp, &path))
    .map_err(|e| fail(format!("无法保存 WorkBuddy 程序位置：{e}")))
}
