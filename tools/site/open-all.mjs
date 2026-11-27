// Abertura de todas as páginas do site, numa tabela: a mesma receita do
// `open-timing.mjs` (ADR 0003: navegador sem janela, CPU 4x mais lenta, cache
// desligado, uma rodada de aquecimento e a mediana de 7), página por página.
//
//   node tools/site/open-all.mjs <dist> [--url <base>] [--rodadas 7]
//        [--md <tabela.md>] [--json <medida.json>] [--ref <outra.json>]
//
// - As páginas são as do `dist/` (as mesmas da prova de hidratação), menos as
//   que só redirecionam (`<meta http-equiv="refresh">`). Com `--url`, mede o
//   site publicado nesse endereço em vez de servir o `dist/`.
// - `COMPILANDO`: um comando que sai 0 quando a máquina está quieta (ninguém
//   compilando). Antes de cada página ele roda; se a máquina estiver ocupada,
//   a medida espera (30 s por vez, até `COMPILANDO_ESPERA_S`, padrão 600 s).
//   Depois de cada página roda de novo. A tabela marca a página que não ficou
//   quieta do começo ao fim. Sem `COMPILANDO`, a coluna diz "não conferido".
// - `--ref`: a medida de outro site (o JSON que `--json` grava), para a
//   diferença por página.
//
// Ferramenta da máquina: o runner do CI é barulhento demais para medir tempo.
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { join, resolve } from 'node:path';
import { detectBase, findBrowser, launch, listPages, serve } from './cdp.mjs';

const args = process.argv.slice(2);
const opcao = (nome, padrao = null) => {
  const i = args.indexOf(nome);
  return i >= 0 ? args[i + 1] : padrao;
};
const dist = resolve(args.find((a, i) => !a.startsWith('--') && !args[i - 1]?.startsWith('--')) ?? 'dist');
const rodadas = Number(opcao('--rodadas', '7'));
const mdPath = opcao('--md');
const jsonPath = opcao('--json');
const refPath = opcao('--ref');
const urlBase = opcao('--url');

const redireciona = (rel) => {
  const arq = rel.endsWith('.html') ? join(dist, rel) : join(dist, rel, 'index.html');
  return /<meta[^>]+http-equiv=["']?refresh/i.test(readFileSync(arq, 'utf8'));
};
const todas = process.env.PAGES ? process.env.PAGES.split(',') : listPages(dist);
const puladas = todas.filter(redireciona);
const paginas = todas.filter((r) => !puladas.includes(r));

const comando = process.env.COMPILANDO;
const esperaMax = Number(process.env.COMPILANDO_ESPERA_S ?? '600') * 1000;
const quieta = () => (comando ? spawnSync(comando, { shell: true, stdio: 'ignore' }).status === 0 : null);
const dormir = (ms) => new Promise((ok) => setTimeout(ok, ms));

// O Edge primeiro, como no `open-timing.mjs` (as medidas do ADR 0003).
const edge = [
  'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
  'C:/Program Files/Microsoft/Edge/Application/msedge.exe',
  '/usr/bin/microsoft-edge',
].find((p) => existsSync(p));
const executable = process.env.BROWSER ?? edge ?? findBrowser();

let origin;
let base;
let server = null;
if (urlBase) {
  const u = new URL(urlBase);
  origin = u.origin;
  base = u.pathname.endsWith('/') ? u.pathname : `${u.pathname}/`;
} else {
  base = detectBase(dist);
  server = await serve(dist, base);
  origin = server.origin;
}

const browser = await launch(executable);
// Uma aba só, na frente, reaproveitada (aba de fundo adia pintura e o
// `requestIdleCallback` do script das ilhas).
const page = await browser.newPage();
await page.send('Page.enable');
await page.send('Network.enable');
await page.send('Network.setCacheDisabled', { cacheDisabled: true });
await page.send('Emulation.setCPUThrottlingRate', { rate: 4 });
await page.send('Page.bringToFront');

const mediana = (xs) => {
  const s = xs.filter((x) => x != null).sort((a, b) => a - b);
  return s.length ? s[Math.floor(s.length / 2)] : null;
};

console.log(`abertura de ${paginas.length} páginas em ${origin}${base}, ${executable}, CPU 4x, cache desligado, ${rodadas} rodadas`
  + `${puladas.length ? ` (puladas, só redirecionam: ${puladas.join(', ')})` : ''}`);

const linhas = [];
for (const rel of paginas) {
  // Espera a máquina quieta antes de começar a página.
  let antes = quieta();
  const limite = Date.now() + esperaMax;
  while (antes === false && Date.now() < limite) {
    await dormir(30000);
    antes = quieta();
  }
  const url = `${origin}${base}${rel}`;
  const medidas = [];
  let erro = null;
  try {
    for (let i = 0; i < rodadas + 1; i++) {
      await page.goto('about:blank');
      await page.goto(url, 60000);
      await page.waitFor("performance.getEntriesByName('island-hydrated').length > 0", 60000);
      // Folga para a entrada da primeira pintura ser registrada.
      await dormir(500);
      const m = await page.eval(`(() => {
        const nav = performance.getEntriesByType('navigation')[0];
        const fcp = performance.getEntriesByName('first-contentful-paint')[0];
        const isl = performance.getEntriesByName('island-hydrated')[0];
        return { fcp: fcp ? fcp.startTime : null, load: nav.loadEventEnd, island: isl.startTime };
      })()`);
      // A primeira rodada aquece o navegador e não conta.
      if (i > 0) medidas.push(m);
    }
  } catch (e) {
    erro = e.message;
  }
  const depois = quieta();
  const maquina = antes === null ? 'não conferido' : antes && depois ? 'quieta' : antes ? 'alguém começou a compilar no meio' : 'ocupada desde o começo';
  const linha = {
    pagina: rel,
    fcp: mediana(medidas.map((m) => m.fcp)),
    load: mediana(medidas.map((m) => m.load)),
    ilha: mediana(medidas.map((m) => m.island)),
    rodadas: medidas.length,
    maquina,
    erro,
  };
  linhas.push(linha);
  const ms = (x) => (x == null ? '-' : `${Math.round(x)} ms`);
  console.log(`  ${rel}: FCP ${ms(linha.fcp)} · load ${ms(linha.load)} · ilha ${ms(linha.ilha)} · ${maquina}${erro ? ` · ERRO: ${erro}` : ''}`);
}
await page.close();
await browser.close();
if (server) await server.close();

const ref = refPath && existsSync(refPath) ? JSON.parse(readFileSync(refPath, 'utf8')) : null;
const n = (x) => (x == null ? '-' : String(Math.round(x)));
const dif = (a, b) => (a == null || b == null ? '-' : `${a - b >= 0 ? '+' : ''}${Math.round(a - b)}`);
const tabela = [];
tabela.push(`Abertura com CPU 4x mais lenta, cache desligado, mediana de ${rodadas} (depois de uma de aquecimento), em ms.`);
tabela.push(`Navegador: \`${executable}\`. Site: ${urlBase ?? `\`${dist}\` servido em ${base}`}.`);
if (ref) tabela.push(`Diferença contra \`${ref.origem}\` (positivo: o site medido aqui é mais lento).`);
tabela.push('');
tabela.push(ref
  ? '| Página | FCP | load | ilha | FCP dif. | load dif. | ilha dif. | máquina |'
  : '| Página | FCP | load | ilha | máquina |');
tabela.push(ref ? '|---|---:|---:|---:|---:|---:|---:|---|' : '|---|---:|---:|---:|---|');
for (const l of linhas) {
  const maq = l.erro ? `${l.maquina}; erro: ${l.erro.replace(/\|/g, '/')}` : l.maquina;
  if (ref) {
    const r = ref.linhas.find((x) => x.pagina === l.pagina);
    tabela.push(`| \`${l.pagina}\` | ${n(l.fcp)} | ${n(l.load)} | ${n(l.ilha)} | ${dif(l.fcp, r?.fcp)} | ${dif(l.load, r?.load)} | ${dif(l.ilha, r?.ilha)} | ${maq} |`);
  } else {
    tabela.push(`| \`${l.pagina}\` | ${n(l.fcp)} | ${n(l.load)} | ${n(l.ilha)} | ${maq} |`);
  }
}
const quietas = linhas.filter((l) => l.maquina === 'quieta').length;
tabela.push('');
tabela.push(`Páginas medidas com a máquina quieta do começo ao fim: ${comando ? `${quietas} de ${linhas.length}` : 'não conferido (sem COMPILANDO)'}.`);
if (puladas.length) tabela.push(`Puladas (só redirecionam): ${puladas.map((p) => `\`${p}\``).join(', ')}.`);
const md = `${tabela.join('\n')}\n`;
console.log(`\n${md}`);
if (mdPath) writeFileSync(mdPath, md);
if (jsonPath) writeFileSync(jsonPath, `${JSON.stringify({ origem: urlBase ?? dist, rodadas, linhas }, null, 2)}\n`);
process.exit(linhas.some((l) => l.erro) ? 1 : 0);
