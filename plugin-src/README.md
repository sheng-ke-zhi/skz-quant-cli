# Skill authoring source

Edit `books/` and `common/`, then run:

```bash
python3 scripts/release/build_plugins.py --sync-only
```

The command renders all harness-specific plugin trees and rebuilds `plugins/manifest.json`.

`skills.txt` is the ordered skill catalog shared by the renderer, the Rust installer,
and bundle tests. Add a skill there together with its `books/<name>/` source and
routing fixtures; tests reject missing or unlisted books.
