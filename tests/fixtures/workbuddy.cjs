// Isolated desktop CLI double; launched through Node just like the bundled CLI.
const fs = require('node:fs');
const path = require('node:path');
const root = process.env.WORKBUDDY_CONFIG_DIR;
if (root !== process.env.CODEBUDDY_CONFIG_DIR) throw Error('config isolation failed');
fs.mkdirSync(root, {recursive: true});
const read = (file, fallback) => fs.existsSync(path.join(root, file)) ? JSON.parse(fs.readFileSync(path.join(root, file), 'utf8')) : fallback;
const write = (file, value) => {
  fs.mkdirSync(path.dirname(path.join(root, file)), {recursive: true});
  fs.writeFileSync(path.join(root, file), JSON.stringify(value));
};
const args = process.argv.slice(2);
fs.appendFileSync(path.join(root, 'calls.jsonl'), JSON.stringify({args, cwd: process.cwd()}) + '\n');
const control = read('control.json', {});
if (args.includes('--help')) {
  console.log(control.missingCommands ? 'old CLI' : 'install uninstall update list disable add remove');
  process.exit(0);
}
const action = args[1] === 'marketplace' ? args[2] : args[1];
if (control.fail === action) {
  console.log('✘ simulated native failure'); // Desktop CLI can fail with exit 0.
  process.exit(0);
}
const markets = read('plugins/known_marketplaces.json', {});
const registry = read('plugins/installed_plugins.json', {version: 2, plugins: {}});
const settings = read('settings.json', {enabledPlugins: {}});
const id = 'skz@skz';
if (args[1] === 'marketplace') {
  if (action === 'add') {
    markets.skz = {type: 'directory', source: {source: 'directory', path: args[3]}, installLocation: args[3]};
  } else if (action === 'remove') {
    delete markets.skz;
  }
  write('plugins/known_marketplaces.json', markets);
} else if (action === 'list') {
  if (control.invalidJson) { console.log('{bad json'); process.exit(0); }
  const entries = Object.entries(registry.plugins).flatMap(([id, items]) => items.map(item => ({id, ...item, enabled: settings.enabledPlugins[id] !== false})));
  if (control.duplicate && entries.length) entries.push(entries[0]);
  console.log(JSON.stringify(entries));
} else if (action === 'install' || action === 'update') {
  if (action === 'update' && control.noopUpdate) process.exit(0);
  const installPath = path.join(root, 'plugins/cache/skz/skz/test-version');
  fs.rmSync(installPath, {recursive: true, force: true});
  fs.cpSync(path.join(markets.skz.installLocation, 'plugins/skz'), installPath, {recursive: true});
  registry.plugins[id] = [{scope: 'user', installPath}];
  settings.enabledPlugins[id] = true; // Exercise adapter preservation even on a regressing host.
  write('plugins/installed_plugins.json', registry);
  write('settings.json', settings);
} else if (action === 'disable') {
  settings.enabledPlugins[id] = false;
  write('settings.json', settings);
} else if (action === 'uninstall') {
  registry.plugins[id] = (registry.plugins[id] || []).filter(entry => entry.scope !== 'user');
  if (!registry.plugins[id].length) delete registry.plugins[id];
  write('plugins/installed_plugins.json', registry);
}
