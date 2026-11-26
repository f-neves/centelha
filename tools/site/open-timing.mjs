// Medida de abertura: do começo da navegação até a ilha hidratar.
//
//   node tools/site/open-timing.mjs [dist] [página] [rodadas]
//   (padrão: dist, regras/combate/, 7 rodadas)
//
// Edge (ou o navegador de `BROWSER`) sem janela, CPU 4x mais lenta
// (`Emulation.setCPUThrottlingRate`), cache desligado, servidor local. Para
// cada rodada, numa aba nova: primeira pintura com conteúdo (FCP), `load` e a
// marca `island-hydrated` que a ilha grava ao hidratar. Mostra cada rodada e a
// mediana. Não há atraso de rede: mede o custo de CPU da página.

import { resolve } from 'node:path';
import { existsSync } from 'node:fs';
import { detectBase, findBrowser, launch, serve } from './cdp.mjs';

const dist = resolve(process.argv[2] ?? 'dist');
const rel = process.argv[3] ?? 'regras/combate/';
const rounds = Number(process.argv[4] ?? 7);
const base = detectBase(dist);

const edge = [
  'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
  'C:/Program Files/Microsoft/Edge/Application/msedge.exe',
  '/usr/bin/microsoft-edge',
].find((p) => existsSync(p));
const executable = process.env.BROWSER ?? edge ?? findBrowser();

const server = await serve(dist, base);
const browser = await launch(executable);
const url = `${server.origin}${base}${rel}`;
const rows = [];

// Uma aba só, na frente, reaproveitada (aba de fundo adia pintura e o
// `requestIdleCallback` do script das ilhas).
const page = await browser.newPage();
await page.send('Page.enable');
await page.send('Network.enable');
await page.send('Network.setCacheDisabled', { cacheDisabled: true });
await page.send('Emulation.setCPUThrottlingRate', { rate: 4 });
await page.send('Page.bringToFront');

for (let i = 0; i < rounds + 1; i++) {
  await page.goto('about:blank');
  await page.goto(url, 60000);
  await page.waitFor("performance.getEntriesByName('island-hydrated').length > 0", 60000);
  // Folga para a entrada da primeira pintura ser registrada.
  await new Promise((ok) => setTimeout(ok, 500));
  const m = await page.eval(`(() => {
    const nav = performance.getEntriesByType('navigation')[0];
    const fcp = performance.getEntriesByName('first-contentful-paint')[0];
    const isl = performance.getEntriesByName('island-hydrated')[0];
    return { fcp: fcp ? fcp.startTime : null, load: nav.loadEventEnd, island: isl.startTime };
  })()`);
  // A primeira rodada aquece o navegador e não conta.
  if (i > 0) rows.push(m);
}
await page.close();

await browser.close();
await server.close();

const median = (xs) => {
  const s = [...xs].sort((a, b) => a - b);
  return s[Math.floor(s.length / 2)];
};
const fmt = (x) => (x == null ? '-' : `${Math.round(x)} ms`);
console.log(`abertura de ${base}${rel}, ${executable}, CPU 4x, cache desligado, ${rounds} rodadas`);
rows.forEach((r, i) => console.log(`  rodada ${i + 1}: FCP ${fmt(r.fcp)} · load ${fmt(r.load)} · ilha hidratada ${fmt(r.island)}`));
console.log(`mediana: FCP ${fmt(median(rows.map((r) => r.fcp)))} · load ${fmt(median(rows.map((r) => r.load)))} · ilha hidratada ${fmt(median(rows.map((r) => r.island)))}`);
