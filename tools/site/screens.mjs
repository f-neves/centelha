// Capturas de tela de uma página nos três temas, no celular (375 px) e no
// desktop (1280 px), e a conferência de rolagem horizontal no celular.
//
//   node tools/site/screens.mjs <dist> <página> <pasta de saída> [rótulo] [--overflow]
//
// Grava `<rótulo>-<tema>-375.png` e `<rótulo>-<tema>-1280.png` (a primeira tela
// da página). Com `--overflow`, abre todas as páginas de `dist/` a 375 px e diz
// quais rolam na horizontal (largura do documento maior que a da tela).
// Serve qualquer `dist` com `index.html` na raiz; o caminho base é lido da
// marca do topo, ou de `BASE` quando o site não é este.

import { mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { detectBase, launch, listPages, serve } from './cdp.mjs';

const args = process.argv.slice(2).filter((a) => !a.startsWith('--'));
const overflow = process.argv.includes('--overflow');
const dist = resolve(args[0] ?? 'dist');
const rel = args[1] ?? 'regras/combate/';
const out = resolve(args[2] ?? 'capturas');
const label = args[3] ?? 'novo';
const base = process.env.BASE ?? detectBase(dist);
const THEMES = ['classico', 'escuro', 'legivel'];
const SIZES = [
  { width: 375, height: 812, scale: 2, mobile: true },
  { width: 1280, height: 900, scale: 1, mobile: false },
];

mkdirSync(out, { recursive: true });
const server = await serve(dist, base);
const browser = await launch();
const page = await browser.newPage();
await page.send('Page.enable');
await page.send('Runtime.enable');
await page.send('Page.bringToFront');
const url = (r) => `${server.origin}${base}${r}`;

for (const size of SIZES) {
  await page.send('Emulation.setDeviceMetricsOverride', {
    width: size.width, height: size.height, deviceScaleFactor: size.scale, mobile: size.mobile,
  });
  for (const theme of THEMES) {
    await page.goto(url(rel));
    await page.eval(`localStorage.setItem('tema', '${theme}')`);
    await page.goto(url(rel));
    await page.eval('document.fonts.ready.then(() => true)');
    await new Promise((ok) => setTimeout(ok, 600));
    const shot = await page.send('Page.captureScreenshot', { format: 'png' });
    const file = join(out, `${label}-${theme}-${size.width}.png`);
    writeFileSync(file, Buffer.from(shot.data, 'base64'));
    console.log(`gravei ${file}`);
  }
}

if (overflow) {
  await page.send('Emulation.setDeviceMetricsOverride', { width: 375, height: 812, deviceScaleFactor: 1, mobile: true });
  let bad = 0;
  const pages = listPages(dist);
  for (const r of pages) {
    await page.goto(url(r));
    await new Promise((ok) => setTimeout(ok, 200));
    const w = await page.eval('({ doc: document.documentElement.scrollWidth, view: window.innerWidth })');
    if (w.doc > w.view) {
      bad++;
      console.log(`ROLA  ${base}${r}: documento ${w.doc} px numa tela de ${w.view} px`);
    }
  }
  console.log(bad ? `${bad} de ${pages.length} páginas rolam na horizontal a 375 px` : `nenhuma das ${pages.length} páginas rola na horizontal a 375 px`);
}

await page.close();
await browser.close();
await server.close();
