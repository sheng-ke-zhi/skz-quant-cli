# Skill authoring source

Edit `books/` and `common/`, then run:

```bash
python3 scripts/release/build_plugins.py --sync-only
```

The command renders one shared payload at `plugins/shared/skills/`, the native
configuration for each harness, and `plugins/manifest.json`. Do not add skill
copies or overrides under individual harnesses.

`skz plugin install|upgrade` combines the shared payload with the selected
harness configuration in its managed installation cache. Those caches are
self-contained so native plugin managers can copy and update them without
external paths or symlinks. Edit only `books/` and `common/`; all five harnesses
receive the same skills, scripts, and references on their next upgrade.
