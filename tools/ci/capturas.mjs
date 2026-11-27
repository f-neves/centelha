// Capturas de página inteira e a conferência da rolagem horizontal a 375 px.
//
//   node tools/ci/capturas.mjs <dist> <pasta de saída> [--ref <main.json>] [--marcas <atual.json>]
//
// 1. Rolagem: todas as páginas do `dist/`, a 375 px, nos três temas. Largura do
//    documento maior que a da tela falha (código 1), com a lista.
// 2. Capturas: cada página escolhida sai inteira, a 375 e 1280 px, nos três
//    temas, em PNG, com o nome `<página>-<largura>-<tema>.png` (a inicial é
//    `inicio`; as barras do caminho viram `-`). Página mais alta que
//    `CAPTURAS_FATIA` px (8000) sai em partes seguidas, `-1`, `-2`...: juntas,
//    são a página inteira (o navegador trava numa captura única de dezenas de
//    milhares de pixels).
// 3. Quais páginas: as 6 capturas de todas as páginas passam de 400 MB, então
//    o artefato tem um teto (`CAPTURAS_MAX_MB`, padrão 40). Com `--ref` (as
//    marcas do último main), entram as páginas cujo HTML mudou; se a folha de
//    estilo ou o WebAssembly mudaram, ou sem `--ref`, todas contam como
//    mudadas. Entre elas, das menores para as maiores, até o teto. Página que
//    não cabe inteira fica de fora, toda, e entra na lista do resumo: nunca
//    sai uma página pela metade.
// 4. `--marcas`: grava o hash de cada página e dos arquivos comuns, para o
//    main guardar e o próximo PR comparar.
//
// Antes de cada captura, o `content-visibility: auto` do corpo é desligado
// (como na prova de hidratação): sem isso, o que está fora da tela não tem
// layout e sairia em branco. `CAPTURAS_ABAS` (padrão 4) páginas ao mesmo
// tempo, cada aba no seu contexto. O resumo vai para `$GITHUB_STEP_SUMMARY`.
import { appendFileSync, existsSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, join, resolve } from 'node:path';
import { detectBase, launch, listPages, serve } from '../site/cdp.mjs';

const args = process.argv.slice(2);
const opcao = (n) => (args.includes(n) ? args[args.indexOf(n) + 1] : null);
const posicionais = args.filter((a, i) => !a.startsWith('--') && !args[i - 1]?.startsWith('--'));
const dist = resolve(posicionais[0] ?? 'dist');
const out = resolve(posicionais[1] ?? 'capturas');
const refPath = opcao('--ref');
const marcasPath = opcao('--marcas');
const base = detectBase(dist);
const pages = process.env.PAGES ? process.env.PAGES.split(',') : listPages(dist);
const THEMES = ['classico', 'escuro', 'legivel'];
const SIZES = [
  { width: 375, height: 812, deviceScaleFactor: 1, mobile: true },
  { width: 1280, height: 900, deviceScaleFactor: 1, mobile: false },
];
const ABAS = Math.max(1, Number.parseInt(process.env.CAPTURAS_ABAS ?? '4', 10) || 1);
const FATIA = Number.parseInt(process.env.CAPTURAS_FATIA ?? '8000', 10);
const TETO = Number(process.env.CAPTURAS_MAX_MB ?? '40') * 1e6;
const ALL_VISIBLE = `(() => {
  if (!document.getElementById('test-all-visible')) {
    const s = document.createElement('style');
    s.id = 'test-all-visible';
    s.textContent = '* { content-visibility: visible !important; }';
    document.head.appendChild(s);
  }
  return true;
})()`;

const nome = (rel) => (rel === '' ? 'inicio' : rel.replace(/\.html$/, '').replace(/\/$/, '').replace(/\//g, '-'));
const arquivoHtml = (rel) => join(dist, rel.endsWith('.html') ? rel : `${rel}index.html`);
const hash = (p) => createHash('sha256').update(readFileSync(p)).digest('hex').slice(0, 16);

// Marcas: o HTML de cada página e os arquivos que todas usam.
const COMUNS = ['assets/site.css', 'pkg/centelha_web_bg.wasm', 'pkg/centelha_web.js'];
const marcas = {
  commit: process.env.GITHUB_SHA ?? null,
  comuns: Object.fromEntries(COMUNS.map((c) => [c, existsSync(join(dist, c)) ? hash(join(dist, c)) : null])),
  paginas: Object.fromEntries(pages.map((r) => [r, hash(arquivoHtml(r))])),
};
if (marcasPath) {
  mkdirSync(dirname(resolve(marcasPath)), { recursive: true });
  writeFileSync(marcasPath, `${JSON.stringify(marcas, null, 2)}\n`);
}
const ref = refPath && existsSync(refPath) ? JSON.parse(readFileSync(refPath, 'utf8')) : null;
const comunsMudaram = ref ? COMUNS.filter((c) => ref.comuns?.[c] !== marcas.comuns[c]) : [];
const mudou = (r) => !ref || comunsMudaram.length > 0 || ref.paginas?.[r] !== marcas.paginas[r];
const tamanhoHtml = (r) => statSync(arquivoHtml(r)).size;
const candidatas = pages.filter(mudou).sort((a, b) => tamanhoHtml(a) - tamanhoHtml(b));

mkdirSync(out, { recursive: true });
const server = await serve(dist, base);
const browser = await launch(undefined, [
  '--disable-background-timer-throttling',
  '--disable-backgrounding-occluded-windows',
  '--disable-renderer-backgrounding',
  '--hide-scrollbars',
]);

async function abrir(page, rel, tema) {
  await page.goto(`${server.origin}${base}${rel}`);
  await page.eval(`localStorage.setItem('tema', ${JSON.stringify(tema)})`);
  await page.goto(`${server.origin}${base}${rel}`);
  try {
    await page.waitFor("document.documentElement.dataset.islands === 'hydrated'", 15000);
  } catch { /* página sem ilha: segue */ }
  await page.eval('document.fonts.ready.then(() => true)');
  await page.eval(ALL_VISIBLE);
  await new Promise((ok) => setTimeout(ok, 300));
  const atual = await page.eval('document.documentElement.dataset.theme');
  if (atual !== tema) throw new Error(`pedi o tema ${tema}, a página ficou em ${atual}`);
}

/** As capturas de uma página numa largura, nos três temas. Devolve os arquivos. */
async function capturar(page, rel, size) {
  const feitos = [];
  for (const tema of THEMES) {
    await abrir(page, rel, tema);
    const m = await page.send('Page.getLayoutMetrics');
    const alto = Math.ceil(m.cssContentSize.height);
    const largo = Math.ceil(m.cssContentSize.width);
    const partes = Math.ceil(alto / FATIA);
    for (let k = 0; k < partes; k++) {
      const y = k * FATIA;
      const shot = await page.send('Page.captureScreenshot', {
        format: 'png',
        captureBeyondViewport: true,
        clip: { x: 0, y, width: largo, height: Math.min(FATIA, alto - y), scale: 1 },
      });
      const arq = join(out, `${nome(rel)}-${size.width}-${tema}${partes > 1 ? `-${k + 1}` : ''}.png`);
      const dados = Buffer.from(shot.data, 'base64');
      writeFileSync(arq, dados);
      feitos.push({ arq, bytes: dados.length });
    }
  }
  return feitos;
}

// Abas, cada uma no seu contexto, puxando trabalho de `proximo()` até acabar.
async function emAbas(n, proximo, fazer) {
  await Promise.all(Array.from({ length: n }, async () => {
    const { browserContextId } = await browser.send('Target.createBrowserContext');
    const page = await browser.newPage({ browserContextId, newWindow: true });
    await page.send('Page.enable');
    await page.send('Runtime.enable');
    await page.send('Emulation.setFocusEmulationEnabled', { enabled: true });
    try {
      for (let t = proximo(); t !== undefined; t = proximo()) await fazer(page, t);
    } finally {
      await page.close().catch(() => {});
      await browser.send('Target.disposeBrowserContext', { browserContextId }).catch(() => {});
    }
  }));
}

const inicio = Date.now();
const erros = [];

// 1. Rolagem horizontal: todas as páginas, a 375 px, nos três temas.
const rolam = [];
const filaRolagem = [...pages];
await emAbas(Math.min(ABAS, pages.length), () => filaRolagem.shift(), async (page, rel) => {
  await page.send('Emulation.setDeviceMetricsOverride', SIZES[0]);
  for (const tema of THEMES) {
    try {
      await abrir(page, rel, tema);
      // Compara com a largura emulada, não com `innerWidth`: no modo celular, o
      // navegador afasta o zoom para caber um conteúdo largo, e o `innerWidth`
      // cresce junto com o documento.
      const doc = await page.eval('document.documentElement.scrollWidth');
      if (doc > SIZES[0].width) rolam.push(`${base}${rel} (${tema}): documento ${doc} px numa tela de ${SIZES[0].width} px`);
    } catch (e) {
      erros.push(`${rel} (375, ${tema}): ${e.message}`);
    }
  }
});
const tempoRolagem = (Date.now() - inicio) / 1000;

// 2. Capturas das páginas escolhidas, até o teto: página inteira ou nada.
const incluidas = [];
const deFora = [];
let total = 0;
const fila = [...candidatas];
await emAbas(Math.min(ABAS, fila.length), () => fila.shift(), async (page, rel) => {
  const feitos = [];
  try {
    for (const size of SIZES) {
      await page.send('Emulation.setDeviceMetricsOverride', size);
      feitos.push(...(await capturar(page, rel, size)));
    }
  } catch (e) {
    erros.push(`${rel}: ${e.message}`);
    for (const f of feitos) rmSync(f.arq, { force: true });
    return;
  }
  const bytes = feitos.reduce((s, f) => s + f.bytes, 0);
  if (total + bytes > TETO) {
    // Não cabe inteira: sai toda. As que faltam são maiores; nenhuma entra.
    for (const f of feitos) rmSync(f.arq, { force: true });
    deFora.push(rel, ...fila.splice(0));
  } else {
    total += bytes;
    incluidas.push({ rel, bytes });
  }
});
await browser.close();
await server.close();

const segundos = ((Date.now() - inicio) / 1000).toFixed(1);
const mb = (b) => (b / 1e6).toFixed(1).replace('.', ',');
const linhas = ['### Capturas e rolagem horizontal', ''];
if (rolam.length) {
  linhas.push(`**${rolam.length} capturas a 375 px rolam na horizontal** (o passo falha):`, '');
  for (const r of rolam) linhas.push(`- ${r}`);
} else {
  linhas.push(`Nenhuma das ${pages.length} páginas rola na horizontal a 375 px, nos 3 temas.`);
}
for (const e of erros) linhas.push(`- erro: ${e}`);
linhas.push('');
const motivo = !ref
  ? 'sem marcas do main para comparar: todas as páginas contam como mudadas'
  : comunsMudaram.length
    ? `mudou ${comunsMudaram.map((c) => `\`${c}\``).join(', ')}: todas as páginas contam como mudadas`
    : `${candidatas.length} páginas com o HTML diferente do main (\`${ref.commit?.slice(0, 7) ?? '?'}\`)`;
linhas.push(`Capturas de página inteira, 375 e 1280 px, 3 temas, no artefato \`capturas\` (${motivo}).`);
linhas.push(`Teto do artefato: ${mb(TETO)} MB. Entraram ${incluidas.length} páginas, ${mb(total)} MB, das menores para as maiores.`);
if (incluidas.length) {
  incluidas.sort((a, b) => a.bytes - b.bytes);
  linhas.push('', incluidas.map((p) => `\`${p.rel || '/'}\` (${mb(p.bytes)} MB)`).join(', '));
}
if (deFora.length) linhas.push('', `Fora do artefato por não caberem inteiras no teto: ${deFora.map((r) => `\`${r || '/'}\``).join(', ')}.`);
linhas.push('', `Tempo: ${segundos} s (rolagem ${tempoRolagem.toFixed(1)} s), ${ABAS} abas.`, '');
const texto = `${linhas.join('\n')}\n`;
console.log(texto);
if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, texto);
process.exit(rolam.length || erros.length ? 1 : 0);
