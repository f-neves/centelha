// Vigia de um navegador de teste aberto pelo `launch()` de `cdp.mjs`.
//
//   node watchdog.cjs <pid do Node que abriu> <pid do navegador> <pasta do perfil>
//
// Roda à parte (processo solto) enquanto o Node que abriu o navegador viver.
// Quando ele morre, de qualquer jeito (fim normal, exceção, Ctrl+C ou `kill`
// forçado), o vigia mata o navegador com os filhos, se ainda estiver vivo, e
// apaga o perfil, tentando por até 10 s (os processos filhos do navegador
// soltam os arquivos com atraso). Depois sai.

const { spawnSync } = require('node:child_process');
const { rmSync } = require('node:fs');

const [parent, browser, profile] = process.argv.slice(2);

function alive(pid) {
  try {
    process.kill(Number(pid), 0);
    return true;
  } catch (e) {
    return e.code === 'EPERM';
  }
}

function removeProfile(triesLeft) {
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
