// Peças comuns das provas de navegador do site: servidor estático de `dist/`,
// navegador sem janela (Chrome ou Edge) e um cliente mínimo do protocolo de
// depuração (CDP). Só Node (22 ou mais novo: usa o WebSocket embutido), sem
// pacote do npm.

import { spawn, spawnSync } from 'node:child_process';
import { createServer } from 'node:http';
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { extname, join, normalize, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.wasm': 'application/wasm',
  '.json': 'application/json',
  '.webmanifest': 'application/manifest+json',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.ico': 'image/x-icon',
  '.webp': 'image/webp',
};

/** O caminho base gravado nas páginas (o `href` da marca no topo). */
export function detectBase(dist) {
  const html = readFileSync(join(dist, 'index.html'), 'utf8');
  const m = html.match(/<a href="([^"]*)" class="brand"/);
  return m ? m[1] : '/';
}

/** As páginas de `dist/`, como caminhos relativos ao caminho base (mais a `404.html`). */
export function listPages(dist) {
  // Toda pasta com `index.html` (menos `pkg/` e `assets/`), em ordem.
  const pages = [];
  const walk = (rel) => {
    if (existsSync(join(dist, rel, 'index.html'))) pages.push(rel);
    for (const name of readdirSorted(join(dist, rel))) {
      const child = `${rel}${name}/`;
      if (rel === '' && (name === 'pkg' || name === 'assets')) continue;
      // Sem a barra do fim: no Linux, `stat` de um arquivo com `/` no fim falha.
      if (statSync(join(dist, rel, name)).isDirectory()) walk(child);
    }
  };
  walk('');
  // A página de "não encontrada" mora num arquivo solto na raiz.
  if (existsSync(join(dist, '404.html'))) pages.push('404.html');
  return pages;
}

function readdirSorted(dir) {
  return readdirSync(dir).sort();
}

/**
 * Serve `dist/` sob o caminho base. `opts.blockScripts` responde 404 ao
 * WebAssembly e à cola (a página fica como sem WebAssembly).
 */
export async function serve(dist, base, opts = {}) {
  const server = createServer(async (req, res) => {
    const url = new URL(req.url, 'http://x');
    let path = decodeURIComponent(url.pathname);
    if (!path.startsWith(base)) {
      res.writeHead(404).end();
      return;
    }
    path = path.slice(base.length);
    if (path === '' || path.endsWith('/')) path += 'index.html';
    const file = normalize(join(dist, path));
    if (!file.startsWith(normalize(dist) + sep) && file !== normalize(dist)) {
      res.writeHead(403).end();
      return;
    }
    if (opts.blockScripts && path.startsWith('pkg/')) {
      res.writeHead(404).end();
      return;
    }
    try {
      if (!statSync(file).isFile()) throw new Error('não é arquivo');
      const body = await readFile(file);
      res.writeHead(200, { 'content-type': TYPES[extname(file)] || 'application/octet-stream', 'cache-control': 'no-store' });
      res.end(body);
    } catch {
      res.writeHead(404).end();
    }
  });
  await new Promise((ok) => server.listen(0, '127.0.0.1', ok));
  const { port } = server.address();
  return { origin: `http://127.0.0.1:${port}`, close: () => new Promise((ok) => server.close(ok)) };
}

/** O executável do navegador: `BROWSER`, ou Chrome, ou Edge. */
export function findBrowser() {
  const candidates = [
    process.env.BROWSER,
    'C:/Program Files/Google/Chrome/Application/chrome.exe',
    'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
    'C:/Program Files/Microsoft/Edge/Application/msedge.exe',
    '/usr/bin/google-chrome',
    '/usr/bin/google-chrome-stable',
    '/usr/bin/chromium',
    '/usr/bin/chromium-browser',
    '/usr/bin/microsoft-edge',
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  ].filter(Boolean);
  const found = candidates.find((c) => existsSync(c));
  if (!found) throw new Error('nenhum Chrome ou Edge encontrado (defina BROWSER)');
  return found;
}

/**
 * Argumentos extras do navegador, separados por espaço, lidos de
 * `BROWSER_ARGS` (no Linux do CI, por exemplo, `--no-sandbox`).
 */
export function browserArgs() {
  return (process.env.BROWSER_ARGS ?? '').split(/\s+/).filter(Boolean);
}

// Os navegadores abertos por este processo, para fechar em qualquer saída.
const open = new Set();

/** O processo existe. */
function alive(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch (e) {
    return e.code === 'EPERM';
  }
}

/**
 * Mata o navegador (e os processos filhos dele) e apaga o perfil, sem esperar.
 * São dois números de processo: o que o `spawn` abriu e o do navegador de
 * fato, que o próprio navegador informa pelo CDP. No Windows o Chrome sem
 * janela abre o navegador num processo novo e o primeiro sai logo; matar só o
 * primeiro deixava o navegador vivo (foi assim que sobraram os navegadores
 * esquecidos).
 */
function killNow(entry) {
  open.delete(entry);
  // O vigia só serve se este processo morrer à força: num fim que passa por
  // aqui, ele sai antes, para não agir sobre um número de processo reusado.
  if (entry.watchdog && entry.watchdog.exitCode === null) {
    try { entry.watchdog.kill(); } catch { /* já saiu */ }
  }
  for (const pid of livePids(entry)) {
    if (process.platform === 'win32') {
      spawnSync('taskkill', ['/PID', String(pid), '/T', '/F'], { stdio: 'ignore' });
    } else {
      try { process.kill(pid, 'SIGKILL'); } catch { /* já saiu */ }
    }
  }
  // Espera (síncrono: pode estar num `process.on('exit')`) o navegador soltar
  // os arquivos do perfil antes de apagar.
  const deadline = Date.now() + 3000;
  const pause = new Int32Array(new SharedArrayBuffer(4));
  while (livePids(entry).length && Date.now() < deadline) {
    Atomics.wait(pause, 0, 0, 50);
  }
  try { rmSync(entry.profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 }); } catch { /* arquivo preso */ }
  // Se algum arquivo ainda estava preso (os filhos do navegador soltam com
  // atraso), um vigia novo, sem navegador para matar, apaga o perfil depois
  // que este processo sair.
  if (existsSync(entry.profile)) watchdog('-', entry.profile);
}

/**
 * Os processos do navegador ainda vivos. O do `spawn` só conta enquanto o
 * Node não o viu sair: depois disso o número pode já ser de outro processo.
 */
function livePids(entry) {
  const pids = [];
  if (entry.browserPid && alive(entry.browserPid)) pids.push(entry.browserPid);
  const { proc } = entry;
  if (proc.pid && proc.exitCode === null && proc.signalCode === null && alive(proc.pid)) pids.push(proc.pid);
  return pids;
}

let handlersInstalled = false;
/**
 * Fecha os navegadores ao sair, com Ctrl+C (`SIGINT`), `SIGTERM` ou exceção
 * não tratada. Um `kill` forçado do Node não passa por aqui: para isso há o
 * vigia de cada navegador (`watchdog`).
 */
function installHandlers() {
  if (handlersInstalled) return;
  handlersInstalled = true;
  const closeAll = () => { for (const entry of [...open]) killNow(entry); };
  process.on('exit', closeAll);
  for (const signal of ['SIGINT', 'SIGTERM']) {
    process.on(signal, () => { closeAll(); process.exit(130); });
  }
  for (const event of ['uncaughtException', 'unhandledRejection']) {
    process.on(event, (err) => { console.error(err); closeAll(); process.exit(1); });
  }
}

/**
 * Um processo Node à parte (`watchdog.cjs`) que vigia este: se ele morrer à
 * força, sem passar por `killNow` (que encerra o vigia antes), o vigia mata o
 * navegador e apaga o perfil. Devolve o processo do vigia.
 */
function watchdog(browserPid, profile) {
  const script = fileURLToPath(new URL('./watchdog.cjs', import.meta.url));
  const child = spawn(process.execPath, [script, String(process.pid), String(browserPid), profile], {
    detached: true,
    stdio: 'ignore',
    windowsHide: true,
  });
  child.unref();
  return child;
}

/**
 * Abre o navegador sem janela num perfil temporário e devolve o cliente CDP do
 * navegador. `close()` fecha só este processo (nunca outro navegador aberto).
 * Os argumentos de `BROWSER_ARGS` entram sempre, depois de `extraArgs`.
 */
export async function launch(executable = findBrowser(), extraArgs = []) {
  const profile = mkdtempSync(join(tmpdir(), 'centelha-cdp-'));
  const proc = spawn(executable, [
    '--headless=new',
    '--remote-debugging-port=0',
    `--user-data-dir=${profile}`,
    '--no-first-run',
    '--no-default-browser-check',
    '--disable-extensions',
    '--disable-background-networking',
    '--disable-sync',
    '--mute-audio',
    ...extraArgs,
    ...browserArgs(),
    'about:blank',
  ], { stdio: ['ignore', 'ignore', 'pipe'] });
  const entry = { proc, profile, browserPid: null };
  open.add(entry);
  installHandlers();
  // O navegador grava a porta em `DevToolsActivePort` no perfil (no Windows a
  // linha "DevTools listening" não chega pelo stderr redirecionado).
  let exitCode = null;
  proc.on('exit', (code) => { exitCode = code; });
  proc.stderr.resume();
  const portFile = join(profile, 'DevToolsActivePort');
  const deadline = Date.now() + 30000;
  let wsUrl = null;
  while (!wsUrl) {
    if (existsSync(portFile)) {
      const [port, path] = readFileSync(portFile, 'utf8').split(/\s+/);
      if (port && path) wsUrl = `ws://127.0.0.1:${port}${path}`;
    }
    if (!wsUrl) {
      if (Date.now() > deadline) {
        killNow(entry);
        throw new Error(`o navegador não abriu a porta de depuração (saída ${exitCode})`);
      }
      await new Promise((ok) => setTimeout(ok, 100));
    }
  }
  const browser = await connect(wsUrl);
  // O número do processo do navegador de fato (veja `killNow`).
  try {
    const info = await browser.send('SystemInfo.getProcessInfo');
    entry.browserPid = info.processInfo.find((p) => p.type === 'browser')?.id ?? null;
  } catch { /* sem a informação, fica o do spawn */ }
  entry.watchdog = watchdog(entry.browserPid ?? proc.pid, profile);
  browser.close = async () => {
    try { await browser.send('Browser.close'); } catch { /* já fechou */ }
    const pid = entry.browserPid ?? proc.pid;
    const deadline = Date.now() + 5000;
    while (alive(pid) && Date.now() < deadline) await new Promise((ok) => setTimeout(ok, 100));
    killNow(entry);
  };
  browser.executable = executable;
  return browser;
}

/** Cliente CDP sobre um WebSocket: `send`, `on`, `once`, sessões por alvo. */
export async function connect(wsUrl) {
  const ws = new WebSocket(wsUrl);
  await new Promise((ok, fail) => {
    ws.addEventListener('open', ok, { once: true });
    ws.addEventListener('error', () => fail(new Error('CDP: não conectou')), { once: true });
  });
  let nextId = 1;
  const pending = new Map();
  const listeners = new Set();
  ws.addEventListener('message', (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) {
      const { ok, fail } = pending.get(msg.id);
      pending.delete(msg.id);
      if (msg.error) fail(new Error(`${msg.error.message} (${msg.error.code})`));
      else ok(msg.result);
    } else if (msg.method) {
      for (const l of listeners) l(msg);
    }
  });
  const send = (method, params = {}, sessionId) => new Promise((ok, fail) => {
    const id = nextId++;
    pending.set(id, { ok, fail });
    ws.send(JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) }));
  });
  return {
    send,
    listen: (fn) => { listeners.add(fn); return () => listeners.delete(fn); },
    /** Abre uma aba e devolve uma sessão dela. */
    async newPage() {
      const { targetId } = await send('Target.createTarget', { url: 'about:blank' });
      const { sessionId } = await send('Target.attachToTarget', { targetId, flatten: true });
      const page = {
        targetId,
        sessionId,
        send: (method, params) => send(method, params, sessionId),
        on: (method, fn) => {
          const l = (msg) => { if (msg.sessionId === sessionId && msg.method === method) fn(msg.params); };
          listeners.add(l);
          return () => listeners.delete(l);
        },
        async eval(expression) {
          const r = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }, sessionId);
          if (r.exceptionDetails) throw new Error(`eval: ${r.exceptionDetails.text} ${r.exceptionDetails.exception?.description ?? ''}`);
          return r.result.value;
        },
        /** Navega e espera o evento `load`. */
        async goto(url, timeout = 30000) {
          const loaded = new Promise((ok, fail) => {
            const t = setTimeout(() => { off(); fail(new Error(`sem load em ${url}`)); }, timeout);
            const off = page.on('Page.loadEventFired', () => { clearTimeout(t); off(); ok(); });
          });
          const r = await send('Page.navigate', { url }, sessionId);
          if (r.errorText) throw new Error(`${url}: ${r.errorText}`);
          await loaded;
        },
        /** Espera até a expressão dar verdadeiro. */
        async waitFor(expression, timeout = 30000) {
          const end = Date.now() + timeout;
          while (Date.now() < end) {
            if (await page.eval(expression)) return true;
            await new Promise((ok) => setTimeout(ok, 25));
          }
          throw new Error(`esperei ${timeout} ms por: ${expression}`);
        },
        close: () => send('Target.closeTarget', { targetId }),
      };
      return page;
    },
  };
}
