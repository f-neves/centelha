// Vigia de um navegador de teste aberto pelo `launch()` de `cdp.mjs`.
//
//   node watchdog.cjs <pid do Node que abriu> <pid do navegador ou -> <pasta do perfil>
//
// Roda à parte (processo solto) enquanto o Node que abriu o navegador viver.
// No fim normal o próprio Node encerra o vigia (`killNow` em `cdp.mjs`); o
// vigia só age se o Node morrer à força: mata o navegador com os filhos, se
// ainda estiver vivo, e apaga o perfil, tentando por até 10 s (os processos
// filhos do navegador soltam os arquivos com atraso). Depois sai. Com `-` no
// lugar do navegador, só apaga o perfil.
//
// Só apaga uma pasta `centelha-cdp-*` dentro da pasta temporária do sistema.

const { spawnSync } = require('node:child_process');
const { rmSync } = require('node:fs');
const { tmpdir } = require('node:os');
const { basename, dirname, resolve } = require('node:path');

const [parent, browser, profile] = process.argv.slice(2);

/** A pasta é um perfil do `launch()`: `centelha-cdp-*`, direto na pasta temporária. */
function isOurProfile(dir) {
  const full = resolve(dir);
  return basename(full).startsWith('centelha-cdp-') && dirname(full).toLowerCase() === resolve(tmpdir()).toLowerCase();
}

function alive(pid) {
  if (!/^\d+$/.test(String(pid))) return false;
  try {
    process.kill(Number(pid), 0);
    return true;
  } catch (e) {
    return e.code === 'EPERM';
  }
}

function removeProfile(triesLeft) {
  if (!isOurProfile(profile)) return;
  try {
    rmSync(profile, { recursive: true, force: true });
  } catch {
    // Arquivo ainda preso pelo navegador que acabou de morrer.
    if (triesLeft > 0) setTimeout(() => removeProfile(triesLeft - 1), 500);
  }
}

// Depois que o navegador some uma vez, o número dele pode ser reusado por
// outro processo: daí em diante o vigia não mata mais nada, só apaga o perfil.
let browserGone = false;

const timer = setInterval(() => {
  if (!alive(browser)) browserGone = true;
  if (alive(parent)) return;
  clearInterval(timer);
  if (!browserGone) {
    if (process.platform === 'win32') {
      spawnSync('taskkill', ['/PID', browser, '/T', '/F'], { stdio: 'ignore' });
    } else {
      try {
        process.kill(Number(browser), 'SIGKILL');
      } catch {
        // Já saiu.
      }
    }
  }
  setTimeout(() => removeProfile(20), 500);
}, 1000);
