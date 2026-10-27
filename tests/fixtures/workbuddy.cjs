// Isolated desktop CLI double; launched through Node just like the bundled CLI.
const fs = require('node:fs');
const path = require('node:path');
const root = process.env.WORKBUDDY_CONFIG_DIR;
if (root !== process.env.CODEBUDDY_CONFIG_DIR) throw Error('config isolation failed');
if (process.env.CODEBUDDY_FORCE_LITE_WB_BUNDLE !== '1') throw Error('desktop bundle mode missing');
fs.mkdirSync(root, {recursive: true});
const read = (file, fallback) => fs.existsSync(path.join(root, file)) ? JSON.parse(fs.readFileSync(path.join(root, file), 'utf8')) : fallback;
const write = (file, value) => {
  fs.mkdirSync(path.dirname(path.join(root, file)), {recursive: true});
  fs.writeFileSync(path.join(root, file), JSON.stringify(value));
};
// Explicit file copies keep the fixture deterministic on Windows short-name temp paths.
const copyTree = (source, target) => {
  fs.mkdirSync(target, {recursive: true});
  for (const item of fs.readdirSync(source, {withFileTypes: true})) {
    const from = path.join(source, item.name);
    const to = path.join(target, item.name);
    if (item.isDirectory()) copyTree(from, to);
    else fs.copyFileSync(from, to);
  }
};
const args = process.argv.slice(2);
fs.appendFileSync(path.join(root, 'calls.jsonl'), JSON.stringify({args, cwd: process.cwd()}) + '\n');
const control = read('control.json', {});
if (args.includes('--help')) {
  console.log(control.missingCommands ? 'old CLI' : 'install uninstall update list disable enable add remove');
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
if (args.includes('--serve')) {
  if (control.failGateway) process.exit(1);
  if (process.env.CODEBUDDY_GATEWAY_AUTH !== 'password') throw Error('native maintenance auth missing');
  const http = require('node:http');
  const password = process.env.CODEBUDDY_GATEWAY_PASSWORD;
  if (!password || password.length < 32) throw Error('native maintenance credential missing');
  http.createServer((request, response) => {
    const authenticated = request.headers.authorization === `Bearer ${password}`;
    if (request.url === '/api/v1/auth/status') {
      response.setHeader('content-type', 'application/json');
      response.end(JSON.stringify({authEnabled: true, authenticated}));
    } else if (authenticated && request.url === '/api/v1/plugins' && request.method === 'POST') {
      let body = '';
      request.on('data', data => { body += data; });
      request.on('end', () => {
        const value = JSON.parse(body);
        if (request.headers['x-codebuddy-request'] !== '1' || value.plugin !== id || !value.options.force ||
            value.options.scope !== 'user' || value.options.autoEnable !== false || value.options.persistEnabledState !== false) {
          response.statusCode = 400; response.end(); return;
        }
        const location = registry.plugins[id][0].installPath;
        fs.rmSync(location, {recursive: true, force: true});
        copyTree(path.join(markets.skz.installLocation, 'plugins/skz'), location);
        response.statusCode = 204; response.end();
      });
    } else { response.statusCode = 401; response.end(); }
  }).listen(Number(args[args.indexOf('--port') + 1]), '127.0.0.1');
  return;
}
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
  // The real host reuses a metadata-valid cache even after uninstall/reinstall.
  if (!fs.existsSync(installPath)) copyTree(path.join(markets.skz.installLocation, 'plugins/skz'), installPath);
  registry.plugins[id] = [{scope: 'user', installPath}];
  settings.enabledPlugins[id] = true; // Exercise adapter preservation even on a regressing host.
  write('plugins/installed_plugins.json', registry);
  write('settings.json', settings);
} else if (action === 'disable' || action === 'enable') {
  settings.enabledPlugins[id] = action === 'enable';
  write('settings.json', settings);
} else if (action === 'uninstall') {
  registry.plugins[id] = (registry.plugins[id] || []).filter(entry => entry.scope !== 'user');
  if (!registry.plugins[id].length) delete registry.plugins[id];
  write('plugins/installed_plugins.json', registry);
}
