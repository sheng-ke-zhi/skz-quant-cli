# Native CLI Output

`claude-list-2.1.241.json` was captured from Claude Code 2.1.241 using
`claude plugin list --json` after a real SKZ installation in an isolated HOME.
Only the temporary HOME path was replaced with `<HOME>`. Claude 2.1.220 was
also checked and returned the same schema.

To run the real-client lifecycle check after building SKZ:

```sh
python3 tests/plugins/check_claude_native.py --skz target/debug/skz \
  --claude "$(command -v claude)" --output-dir target/claude-native-check
```

The check isolates HOME and Claude configuration, verifies native cached
payloads after install and a 4.3 receipt upgrade, uninstalls, and records JSON
status evidence. It is opt-in because CI uses the captured fixture and does
not install the third-party client.
