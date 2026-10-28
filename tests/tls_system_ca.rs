//! Linux 系统 CA 回归；子进程隔离证书环境变量，不修改宿主信任库。

#[test]
#[cfg(target_os = "linux")]
fn system_ca_validates_business_and_oauth_connections() {
    let output = std::process::Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/system_ca.py"
        ))
        .arg(env!("CARGO_BIN_EXE_skz"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
