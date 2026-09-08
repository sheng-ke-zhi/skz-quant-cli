# Skill authoring source

Edit `books/` and `common/`, then run:

```bash
python3 scripts/release/build_plugins.py --sync-only
```

The command renders all harness-specific plugin trees and rebuilds `plugins/manifest.json`.
