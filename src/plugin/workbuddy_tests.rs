use super::*;

#[test]
fn managed_runtime_skips_incomplete_versions_and_sorts_numerically() {
    let temp = tempfile::tempdir().unwrap();
    for version in ["22.9.0-1", "22.22.2-6", "24.0.0-1", "unpacked.tmp"] {
        fs::create_dir(temp.path().join(version)).unwrap();
    }
    let binary = if cfg!(windows) { "node.exe" } else { "node" };
    for version in ["22.9.0-1", "22.22.2-6", "unpacked.tmp"] {
        fs::write(temp.path().join(version).join(binary), "runtime").unwrap();
    }
    assert_eq!(
        managed_node(temp.path()).unwrap(),
        temp.path().join("22.22.2-6").join(binary)
    );
    assert!(managed_node(&temp.path().join("missing")).is_none());
}

#[test]
fn exact_absolute_path_ownership() {
    let temp = tempfile::tempdir().unwrap();
    assert!(same_path(temp.path(), temp.path()));
    assert!(!same_path(Path::new("relative"), Path::new("relative")));
    assert!(!same_path(temp.path(), &temp.path().join("other")));
}

#[test]
fn status_registration_reads_only_plugin_registry_without_starting_host() {
    let temp = tempfile::tempdir().unwrap();
    let adapter = Adapter {
        desktop: workbuddy_desktop::Desktop {
            executable: temp.path().join("WorkBuddy.exe"),
            source: "test".into(),
        },
        node: temp.path().join("missing-node"),
        entry: temp.path().join("missing-host-cli"),
        config: temp.path().join("profile"),
        source: temp.path().join("source"),
    };
    fs::create_dir_all(adapter.config.join("plugins")).unwrap();
    // 认证和会话放不可解析的诱饵，状态检查不能加载它们。
    for name in ["settings.json", "sessions", "local_storage", "logs"] {
        fs::write(adapter.config.join(name), "must not read").unwrap();
    }
    assert!(adapter.registered_user_path().unwrap().is_none());
    let path = adapter.config.join("plugins/cache/skz/skz/version");
    let registry = adapter.config.join("plugins/installed_plugins.json");
    fs::write(
        &registry,
        serde_json::json!({"version": 2, "plugins": {
            ID: [{"scope": "project", "installPath": "ignored"},
                 {"scope": "user", "installPath": path}]
        }})
        .to_string(),
    )
    .unwrap();
    assert_eq!(adapter.registered_user_path().unwrap(), Some(path.clone()));
    fs::write(
        &registry,
        serde_json::json!({"version": 2, "plugins": {
            ID: [{"scope": "user", "installPath": path},
                 {"scope": "user", "installPath": path}]
        }})
        .to_string(),
    )
    .unwrap();
    assert!(adapter.registered_user_path().is_err());
    fs::write(&registry, "{broken").unwrap();
    assert!(adapter.registered_user_path().is_err());
}

#[cfg(not(windows))]
#[test]
fn unsupported_platform_is_explicit() {
    assert!(!is_present());
    assert!(
        Adapter::locate()
            .err()
            .unwrap()
            .to_string()
            .contains("Windows")
    );
}

// These tests exercise Windows process arguments and child-only environment.
// CI installs Node explicitly; the tests never touch the real WorkBuddy profile.
#[cfg(windows)]
mod desktop {
    use super::*;

    struct Fixture {
        _temp: tempfile::TempDir,
        adapter: Adapter,
        bundle: Bundle,
    }

    impl Fixture {
        fn new() -> Self {
            assert!(
                executable_on_path("node"),
                "WorkBuddy tests require Node.js on PATH"
            );
            let temp = tempfile::tempdir().unwrap();
            let app = temp.path().join("应用 space/app");
            fs::create_dir_all(&app).unwrap();
            let entry = app.join("codebuddy.cjs");
            fs::write(&entry, include_str!("../../tests/fixtures/workbuddy.cjs")).unwrap();
            let adapter = Adapter {
                desktop: workbuddy_desktop::Desktop {
                    executable: app.join("WorkBuddy.exe"),
                    source: "test".into(),
                },
                node: PathBuf::from("node"),
                entry,
                config: temp.path().join("配置 profile"),
                source: temp.path().join("skz state/source"),
            };
            let bundle_root = temp.path().join("bundle");
            let relative = "workbuddy/plugins/skz/skills/skz-guide/SKILL.md";
            let skill = bundle_root.join(relative);
            fs::create_dir_all(skill.parent().unwrap()).unwrap();
            fs::write(&skill, "original skill").unwrap();
            let bundle = Bundle {
                root: bundle_root,
                manifest: Manifest {
                    cli: "development".into(),
                    contract: CONTRACT.into(),
                    plugin: "skz".into(),
                    targets: vec!["workbuddy".into()],
                    skills: vec!["skz-guide".into()],
                    files: vec![ManifestFile {
                        path: relative.into(),
                        sha256: hash_file(&skill).unwrap(),
                        mode: 0o644,
                    }],
                },
            };
            Self {
                _temp: temp,
                adapter,
                bundle,
            }
        }

        fn set(&self, name: &str, value: serde_json::Value) {
            let path = self.adapter.config.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, value.to_string()).unwrap();
        }

        fn receipt(&self) -> PathBuf {
            self.adapter.source.parent().unwrap().join(RECEIPT)
        }
        fn install(&self) {
            reconcile_with(&self.adapter, &self.bundle, false).unwrap();
        }
    }

    #[test]
    fn installer_reset_keeps_source_and_data_and_disable_is_verified() {
        let f = Fixture::new();
        f.install();
        let snapshot = setup_with(&f.adapter, &f.bundle, "check").unwrap();
        assert_eq!(snapshot["registered"], true);
        assert_eq!(snapshot["enabled"], true);
        setup_with(&f.adapter, &f.bundle, "disable").unwrap();
        assert!(!f.adapter.installed().unwrap().unwrap().enabled);
        setup_with(&f.adapter, &f.bundle, "reset").unwrap();
        assert!(f.adapter.installed().unwrap().is_none());
        assert!(f.adapter.source.exists());
        assert!(f.receipt().is_file());
        assert!(
            fs::read_to_string(f.adapter.config.join("calls.jsonl"))
                .unwrap()
                .contains("--keep-data")
        );
        assert!(setup_with(&f.adapter, &f.bundle, "invalid").is_err());
    }

    #[test]
    fn installer_restores_staged_skills_from_another_distribution() {
        let f = Fixture::new();
        f.install();
        let source = f
            .adapter
            .source
            .join("plugins/skz/skills/skz-guide/SKILL.md");
        fs::write(&source, "old distribution skill").unwrap();
        let receipt = fs::read(f.receipt()).unwrap();
        setup_with(&f.adapter, &f.bundle, "reset").unwrap();
        setup_with(&f.adapter, &f.bundle, "restore").unwrap();
        let installed = f.adapter.installed().unwrap().unwrap();
        assert_eq!(
            fs::read_to_string(installed.path.join("skills/skz-guide/SKILL.md")).unwrap(),
            "old distribution skill"
        );
        assert_eq!(fs::read(f.receipt()).unwrap(), receipt);
        setup_with(&f.adapter, &f.bundle, "disable").unwrap();
        assert!(!f.adapter.installed().unwrap().unwrap().enabled);
        f.set("control.json", serde_json::json!({"failGateway": true}));
        assert!(setup_with(&f.adapter, &f.bundle, "restore").is_err());
    }

    #[test]
    fn lifecycle_idempotency_disabled_upgrade_and_damage_repair() {
        let f = Fixture::new();
        let parent_env = std::env::var_os("CODEBUDDY_CONFIG_DIR");
        f.install();
        assert!(f.receipt().is_file());
        f.install();
        let calls = fs::read_to_string(f.adapter.config.join("calls.jsonl")).unwrap();
        assert_eq!(calls.lines().filter(|l| l.contains("\"add\"")).count(), 1);
        assert_eq!(
            calls.lines().filter(|l| l.contains("\"install\"")).count(),
            1
        );
        assert_eq!(std::env::var_os("CODEBUDDY_CONFIG_DIR"), parent_env);
        f.set(
            "settings.json",
            serde_json::json!({"enabledPlugins": {ID: false, "other@foreign": true}}),
        );
        reconcile_with(&f.adapter, &f.bundle, true).unwrap();
        let entry = f.adapter.installed().unwrap().unwrap();
        assert!(!entry.enabled);
        assert!(f.adapter.live_ok(&f.bundle, &entry.path));
        fs::write(entry.path.join("skills/skz-guide/SKILL.md"), "damaged").unwrap();
        assert!(!f.adapter.live_ok(&f.bundle, &entry.path));
        f.set("control.json", serde_json::json!({"noopUpdate": true}));
        reconcile_with(&f.adapter, &f.bundle, true).unwrap();
        let repaired = f.adapter.installed().unwrap().unwrap();
        assert!(!repaired.enabled);
        assert!(f.adapter.live_ok(&f.bundle, &repaired.path));
        assert!(!f.adapter.remove().unwrap());
        assert!(f.adapter.installed().unwrap().is_none());
        assert!(!f.adapter.owned_market().unwrap());
        assert_eq!(
            read_json(&f.adapter.config.join("settings.json"))
                .unwrap()
                .unwrap()["enabledPlugins"]["other@foreign"],
            true
        );
        assert!(!f.adapter.remove().unwrap());
    }

    #[test]
    fn failed_install_is_retryable_without_receipt_or_duplicate_market() {
        let f = Fixture::new();
        f.set("control.json", serde_json::json!({"fail": "install"}));
        assert!(reconcile_with(&f.adapter, &f.bundle, false).is_err());
        assert!(!f.receipt().exists());
        assert!(f.adapter.owned_market().unwrap());
        f.set("control.json", serde_json::json!({}));
        f.install();
        assert!(f.receipt().exists());
        let before = fs::read(f.receipt()).unwrap();
        f.set("control.json", serde_json::json!({"fail": "update"}));
        assert!(reconcile_with(&f.adapter, &f.bundle, true).is_err());
        assert_eq!(fs::read(f.receipt()).unwrap(), before);
        f.set("control.json", serde_json::json!({}));
        reconcile_with(&f.adapter, &f.bundle, true).unwrap();
    }

    #[test]
    fn foreign_market_and_missing_commands_fail_before_staging() {
        let f = Fixture::new();
        f.set("plugins/known_marketplaces.json", serde_json::json!({"skz": {"type": "directory", "source": {"source": "directory", "path": f._temp.path()}, "installLocation": f._temp.path()}}));
        assert!(reconcile_with(&f.adapter, &f.bundle, false).is_err());
        assert!(!f.adapter.source.exists());
        assert!(f.adapter.remove().is_err());
        f.set("plugins/known_marketplaces.json", serde_json::json!({}));
        f.set("control.json", serde_json::json!({"missingCommands": true}));
        assert!(reconcile_with(&f.adapter, &f.bundle, false).is_err());
        assert!(!f.adapter.source.exists());
    }

    #[test]
    fn malformed_output_and_scope_conflicts_are_not_success() {
        let f = Fixture::new();
        f.set("control.json", serde_json::json!({"invalidJson": true}));
        assert!(reconcile_with(&f.adapter, &f.bundle, false).is_err());
        assert!(!f.adapter.source.exists());
        f.set("control.json", serde_json::json!({}));
        f.install();
        f.set("control.json", serde_json::json!({"duplicate": true}));
        assert!(f.adapter.installed().is_err());
        f.set("control.json", serde_json::json!({}));
        let cache = f.adapter.installed().unwrap().unwrap().path;
        f.set("plugins/installed_plugins.json", serde_json::json!({"version": 2, "plugins": {ID: [{"scope": "user", "installPath": cache}, {"scope": "project", "installPath": cache}]}}));
        assert!(f.adapter.prepare().is_err());
        assert!(f.adapter.remove().unwrap()); // Preserve the market/source for project scope.
        assert!(f.adapter.owned_market().unwrap());
        assert_eq!(f.adapter.registry().unwrap()[ID][0].scope, "project");
    }

    #[test]
    fn enabled_intent_survives_failed_native_cache_maintenance() {
        let f = Fixture::new();
        f.install();
        let entry = f.adapter.installed().unwrap().unwrap();
        fs::write(entry.path.join("skills/skz-guide/SKILL.md"), "damaged").unwrap();
        f.set(
            "control.json",
            serde_json::json!({"noopUpdate":true,"failGateway":true}),
        );
        assert!(reconcile_with(&f.adapter, &f.bundle, true).is_err());
        assert!(f.adapter.installed().unwrap().unwrap().enabled);
        let pending = f
            .adapter
            .source
            .parent()
            .unwrap()
            .join("workbuddy-pending.json");
        assert_eq!(
            read_json(&pending).unwrap().unwrap()["repair_enabled"],
            true
        );
        f.set("control.json", serde_json::json!({}));
        f.install();
        let restored = f.adapter.installed().unwrap().unwrap();
        assert!(restored.enabled);
        assert!(f.adapter.live_ok(&f.bundle, &restored.path));
        assert!(!pending.exists());
    }

    #[test]
    fn disabled_intent_survives_failed_repair() {
        let f = Fixture::new();
        f.install();
        f.set(
            "settings.json",
            serde_json::json!({"enabledPlugins": {ID: false}}),
        );
        f.set("control.json", serde_json::json!({"fail": "disable"}));
        assert!(reconcile_with(&f.adapter, &f.bundle, true).is_err());
        f.set("control.json", serde_json::json!({}));
        f.install();
        assert!(!f.adapter.installed().unwrap().unwrap().enabled);
    }
}
