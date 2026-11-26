// Prova 2 do ADR 0003: a hidratação das ilhas não duplica nem apaga conteúdo.
//
//   node tools/site/hydration.mjs [dist]
//
// Para cada página de `dist/`, num Chrome ou Edge sem janela:
//   1. lê o texto visível com o JavaScript desligado (a página como sai do
//      gerador, sem WebAssembly);
//   2. abre de novo com o JavaScript ligado, espera as ilhas hidratarem
//      (`<html data-islands="hydrated">`) e lê o texto visível outra vez;
//   3. exige os dois textos iguais, e também o `textContent` do conteúdo
//      (`<main>`, onde está o corpo do capítulo): um bloco duplicado e
//      escondido por CSS não aparece no texto visível, mas aparece aqui;
//   4. clica num botão do seletor de tema e exige o tema trocado (no
//      `<html data-theme>`, no `localStorage` e no botão marcado);
//   5. exige nenhuma exceção da página e nenhum erro ou aviso no console (em
//      release um descasamento de hidratação vira pânico sem mensagem, por isso
//      as exceções contam; o WebAssembly de depuração escreve o aviso).
//
// Sai com código 1 se qualquer página falhar. Só Node 22+ e um navegador
// (`BROWSER` aponta outro executável; `BROWSER_ARGS` acrescenta argumentos,
// como `--no-sandbox` no Linux do CI).

import { resolve } from 'node:path';
import { detectBase, launch, listPages, serve } from './cdp.mjs';

const dist = resolve(process.argv[2] ?? 'dist');
const base = detectBase(dist);
const pages = listPages(dist);
const THEMES = ['classico', 'escuro', 'legivel'];
// O texto inteiro do conteúdo, visível ou não.
const CONTENT = "document.querySelector('main').textContent";

const server = await serve(dist, base);
const browser = await launch();
let failures = 0;

console.log(`hidratação: ${pages.length} páginas de ${dist} (base ${base}), navegador ${browser.executable}`);

for (const rel of pages) {
  const url = `${server.origin}${base}${rel}`;
  const problems = [];

  // 1. Sem JavaScript.
  const plain = await browser.newPage();
  await plain.send('Page.enable');
  await plain.send('Runtime.enable');
  await plain.send('Emulation.setScriptExecutionDisabled', { value: true });
  await plain.goto(url);
  const before = await plain.eval('document.body.innerText');
  const contentBefore = await plain.eval(CONTENT);
  await plain.close();

  // 2. Com JavaScript e as ilhas.
  const page = await browser.newPage();
  const messages = [];
  page.on('Runtime.consoleAPICalled', (p) => {
    if (['error', 'warning', 'assert'].includes(p.type)) {
      messages.push(`console.${p.type}: ${p.args.map((a) => a.value ?? a.description ?? '').join(' ')}`);
    }
  });
  page.on('Runtime.exceptionThrown', (p) => {
    const d = p.exceptionDetails;
    messages.push(`exceção: ${d.exception?.description ?? d.text}`);
  });
  page.on('Log.entryAdded', (p) => {
    if (['error', 'warning'].includes(p.entry.level)) messages.push(`log.${p.entry.level}: ${p.entry.text} ${p.entry.url ?? ''}`);
  });
  await page.send('Page.enable');
  await page.send('Runtime.enable');
  await page.send('Log.enable');
  await page.goto(url);
  const started = Date.now();
  try {
    await page.waitFor("document.documentElement.dataset.islands === 'hydrated'", 30000);
  } catch (e) {
    problems.push(`as ilhas não hidrataram: ${e.message}`);
  }
  const hydrateMs = Date.now() - started;
  await new Promise((ok) => setTimeout(ok, 150));
  const after = await page.eval('document.body.innerText');
  const contentAfter = await page.eval(CONTENT);

  // 3. Mesmo texto.
  if (before !== after) problems.push(`o texto mudou na hidratação:\n${firstDiff(before, after)}`);
  if (contentBefore !== contentAfter) {
    problems.push(`o textContent do conteúdo mudou na hidratação:\n${firstDiff(contentBefore, contentAfter)}`);
  }
  if (before.trim().length < 50) problems.push('texto quase vazio antes da hidratação');

  // 4. O seletor de tema.
  const current = await page.eval('document.documentElement.dataset.theme');
  const target = THEMES[(THEMES.indexOf(current) + 1) % THEMES.length];
  const rect = await page.eval(`(() => {
    const b = document.querySelector('.theme-switcher button[data-theme-key="${target}"]');
    if (!b) return null;
    b.scrollIntoView({ block: 'center' });
    const r = b.getBoundingClientRect();
    return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
  })()`);
  if (!rect) {
    problems.push(`sem o botão do tema ${target}`);
  } else {
    for (const type of ['mousePressed', 'mouseReleased']) {
      await page.send('Input.dispatchMouseEvent', { type, x: rect.x, y: rect.y, button: 'left', clickCount: 1 });
    }
    const state = await page.eval(`({
      theme: document.documentElement.dataset.theme,
      stored: localStorage.getItem('tema'),
      pressed: document.querySelector('.theme-switcher button[aria-pressed="true"]')?.dataset.themeKey ?? null,
    })`);
    if (state.theme !== target || state.stored !== target || state.pressed !== target) {
      problems.push(`o clique no tema ${target} não trocou: ${JSON.stringify(state)}`);
    }
    const afterClick = await page.eval('document.body.innerText');
    if (afterClick !== before) problems.push(`o texto mudou ao trocar o tema:\n${firstDiff(before, afterClick)}`);
    const contentAfterClick = await page.eval(CONTENT);
    if (contentAfterClick !== contentBefore) {
      problems.push(`o textContent do conteúdo mudou ao trocar o tema:\n${firstDiff(contentBefore, contentAfterClick)}`);
    }
  }

  // 5. Console limpo.
  problems.push(...messages);
  await page.close();

  if (problems.length) {
    failures++;
    console.log(`FALHA ${base}${rel}\n  ${problems.join('\n  ')}`);
  } else {
    console.log(`ok    ${base}${rel} (${before.length} caracteres, ilhas em ${hydrateMs} ms, tema ${current} -> ${target})`);
  }
}

await browser.close();
await server.close();
console.log(failures ? `${failures} de ${pages.length} páginas falharam` : `todas as ${pages.length} páginas passaram`);
process.exit(failures ? 1 : 0);

function firstDiff(a, b) {
  let i = 0;
  while (i < a.length && a[i] === b[i]) i++;
  const from = Math.max(0, i - 40);
  return `    antes:  ${JSON.stringify(a.slice(from, i + 80))}\n    depois: ${JSON.stringify(b.slice(from, i + 80))}`;
}
