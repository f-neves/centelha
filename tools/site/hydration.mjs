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
//      `<html data-theme>`, no `localStorage` e no botão marcado); recolhe e
//      expande a barra lateral; no celular (375 px), abre a gaveta e fecha
//      com Esc; e, onde há "Nesta página", abre o sumário e escolhe um título;
//   5. exige nenhuma exceção da página e nenhum erro ou aviso no console (em
//      release um descasamento de hidratação vira pânico sem mensagem, por isso
//      as exceções contam; o WebAssembly de depuração escreve o aviso).
//
// Com `CENTELHA_CONFIG` (um JSON), grava essa preferência de diagramação no
// `localStorage` antes de cada página com JavaScript, como faria quem ajustou
// o site atual: o texto tem de continuar igual e sem exceção.
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
// As duas leituras do texto usam a mesma tela (a de desktop, com a barra
// lateral à vista); a gaveta do celular é conferida à parte.
// O corpo do capítulo usa `content-visibility: auto` (o navegador só faz o
// layout do que está na tela), e o `innerText` deixa de fora o que não teve
// layout. Antes de cada leitura, esta folha desliga o layout preguiçoso, e a
// comparação fica a mesma.
const ALL_VISIBLE = `(() => {
  if (!document.getElementById('prova-visivel')) {
    const s = document.createElement('style');
    s.id = 'prova-visivel';
    s.textContent = '* { content-visibility: visible !important; }';
    document.head.appendChild(s);
  }
  return true;
})()`;
const DESKTOP = { width: 1280, height: 900, deviceScaleFactor: 1, mobile: false };
const PHONE = { width: 375, height: 812, deviceScaleFactor: 1, mobile: true };

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
  await plain.send('Emulation.setDeviceMetricsOverride', DESKTOP);
  await plain.goto(url);
  await plain.eval(ALL_VISIBLE);
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
  if (process.env.CENTELHA_CONFIG) {
    await page.send('Page.addScriptToEvaluateOnNewDocument', {
      source: `localStorage.setItem('centelha:config', ${JSON.stringify(process.env.CENTELHA_CONFIG)});`,
    });
  }
  await page.send('Emulation.setDeviceMetricsOverride', DESKTOP);
  await page.goto(url);
  const started = Date.now();
  try {
    await page.waitFor("document.documentElement.dataset.islands === 'hydrated'", 30000);
  } catch (e) {
    problems.push(`as ilhas não hidrataram: ${e.message}`);
  }
  const hydrateMs = Date.now() - started;
  await new Promise((ok) => setTimeout(ok, 150));
  await page.eval(ALL_VISIBLE);
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
    b.scrollIntoView({ block: 'center', behavior: 'instant' });
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

  // 4b. A barra lateral (desktop) e a gaveta (celular).
  problems.push(...(await checkShell(page)));

  // 4c. O "Nesta página".
  problems.push(...(await checkToc(page)));

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

/** Clica no meio do elemento, como um dedo (sem `element.click()`). */
async function click(page, selector) {
  const rect = await page.eval(`(() => {
    const el = document.querySelector(${JSON.stringify(selector)});
    if (!el) return null;
    el.scrollIntoView({ block: 'center', behavior: 'instant' });
    const r = el.getBoundingClientRect();
    return r.width && r.height ? { x: r.x + r.width / 2, y: r.y + r.height / 2 } : null;
  })()`);
  if (!rect) return false;
  for (const type of ['mousePressed', 'mouseReleased']) {
    await page.send('Input.dispatchMouseEvent', { type, x: rect.x, y: rect.y, button: 'left', clickCount: 1 });
  }
  return true;
}

async function checkShell(page) {
  const problems = [];
  const state = () => page.eval(`({
    collapsed: document.documentElement.classList.contains('sidebar-collapsed'),
    stored: localStorage.getItem('sidebar'),
    open: document.body.classList.contains('drawer-open'),
    expanded: document.querySelector('.drawer-toggle')?.getAttribute('aria-expanded'),
  })`);
  if (!(await click(page, '.sidebar-collapse'))) problems.push('sem o botão de recolher a barra lateral');
  let s = await state();
  if (!s.collapsed || s.stored !== 'collapsed') problems.push(`recolher não recolheu: ${JSON.stringify(s)}`);
  if (!(await click(page, '.sidebar-expand'))) problems.push('sem o botão de expandir a barra lateral');
  s = await state();
  if (s.collapsed || s.stored !== 'open') problems.push(`expandir não expandiu: ${JSON.stringify(s)}`);

  await page.send('Emulation.setDeviceMetricsOverride', PHONE);
  // A barra lateral desliza para fora da tela em 0,22 s ao virar gaveta;
  // antes disso ela ainda cobre o botão do menu. Num navegador sem janela e
  // sem placa de vídeo, com o fundo animado, a transição pode demorar mais:
  // espera o botão ficar à mostra (até 5 s).
  try {
    await page.waitFor(`(() => {
      const b = document.querySelector('.drawer-toggle');
      if (!b) return true;
      const r = b.getBoundingClientRect();
      return document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)?.closest('.drawer-toggle') === b;
    })()`, 5000);
  } catch { /* segue, e a conferência abaixo diz o que houve */ }
  if (!(await click(page, '.drawer-toggle'))) problems.push('sem o botão do menu no celular');
  s = await state();
  if (!s.open || s.expanded !== 'true') problems.push(`o menu não abriu a gaveta: ${JSON.stringify(s)}`);
  for (const type of ['keyDown', 'keyUp']) {
    await page.send('Input.dispatchKeyEvent', { type, key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 });
  }
  s = await state();
  if (s.open || s.expanded !== 'false') problems.push(`Esc não fechou a gaveta: ${JSON.stringify(s)}`);
  await page.send('Emulation.setDeviceMetricsOverride', DESKTOP);
  await new Promise((ok) => setTimeout(ok, 100));
  return problems;
}

// A regra de ordenação do site atual (src/pages/regras/[slug].astro), como
// oráculo: a ordem esperada sai desta cópia, e não do código em Rust.
const ORACLE = `(table, col, dir) => {
  const FRAC = { '½': 0.5, '¼': 0.25, '¾': 0.75, '⅓': 1 / 3, '⅔': 2 / 3 };
  const CUR = { pc: 1, pp: 10, po: 100, pl: 1000 };
  const numOf = (raw) => {
    const s = raw.replace(/[−–]/g, '-').trim();
    const cm = s.match(/(-?)\\s*([½¼¾⅓⅔]|\\d{1,3}(?:\\.\\d{3})+(?:,\\d+)?|\\d+(?:[.,]\\d+)?)\\s*(pc|pp|po|pl)\\b/i);
    if (cm) {
      const base = FRAC[cm[2]] ?? parseFloat((/^\\d{1,3}(?:\\.\\d{3})+/.test(cm[2]) ? cm[2].replace(/\\./g, '') : cm[2]).replace(',', '.'));
      return (cm[1] === '-' ? -1 : 1) * base * CUR[cm[3].toLowerCase()];
    }
    if (FRAC[s] != null) return FRAC[s];
    const m = s.match(/-?\\d+(?:[.,]\\d+)?/);
    return m ? parseFloat(m[0].replace(',', '.')) : null;
  };
  const rows = Array.from(table.tBodies[0].rows);
  const val = (r) => (r.cells[col]?.textContent || '').trim();
  const vazio = (s) => s === '' || s === '—' || s === '–';
  const numerico = rows.every((r) => { const v = val(r); return vazio(v) || numOf(v) !== null; });
  rows.sort((a, b) => {
    const A = val(a), B = val(b);
    let c;
    if (numerico) c = (vazio(A) ? -Infinity : numOf(A)) - (vazio(B) ? -Infinity : numOf(B));
    else c = A.localeCompare(B, 'pt', { numeric: true, sensitivity: 'base' });
    return dir === 'asc' ? c : -c;
  });
  return rows.map((r) => r.textContent);
}`;

async function checkTables(page) {
  const problems = [];
  const count = await page.eval("[...document.querySelectorAll('main table')].filter((t) => t.querySelector('th.ord')).length");
  for (let t = 0; t < Math.min(2, count); t++) {
    const sel = `[...document.querySelectorAll('main table')].filter((t) => t.querySelector('th.ord'))[${t}]`;
    // O que a regra do site atual daria, para a coluna e a direção pedidas.
    const expected = (col, dir) => page.eval(`(${ORACLE})(${sel}, ${col}, '${dir}')`);
    const actual = () => page.eval(`Array.from(${sel}.tBodies[0].rows).map((r) => r.textContent)`);
    const sortState = () => page.eval(`[...${sel}.tHead.rows[0].cells].map((c) => c.getAttribute('aria-sort'))`);
    // Clique no primeiro cabeçalho: crescente.
    const want = await expected(0, 'asc');
    const ok = await page.eval(`(() => { const b = ${sel}.querySelector('th.ord .ord-btn'); b.scrollIntoView({ block: 'center', behavior: 'instant' }); return true; })()`);
    if (!ok) continue;
    await new Promise((r) => setTimeout(r, 100));
    const rect = await page.eval(`(() => { const r = ${sel}.querySelector('th.ord .ord-btn').getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`);
    for (const type of ['mousePressed', 'mouseReleased']) {
      await page.send('Input.dispatchMouseEvent', { type, x: rect.x, y: rect.y, button: 'left', clickCount: 1 });
    }
    if (JSON.stringify(await actual()) !== JSON.stringify(want)) problems.push(`tabela ${t + 1}: o clique não ordenou como o site atual`);
    if ((await sortState())[0] !== 'ascending') problems.push(`tabela ${t + 1}: aria-sort não ficou ascending`);
    // Teclado no segundo cabeçalho que ordena (ou no mesmo): Enter crescente, espaço decrescente.
    const col = await page.eval(`(() => { const ths = [...${sel}.tHead.rows[0].cells]; const k = ths.findIndex((c, i) => i > 0 && c.classList.contains('ord')); return k > 0 ? k : 0; })()`);
    await page.eval(`${sel}.tHead.rows[0].cells[${col}].querySelector('.ord-btn').focus({ preventScroll: true })`);
    const wantAsc = await expected(col, 'asc');
    for (const type of ['keyDown', 'keyUp']) {
      await page.send('Input.dispatchKeyEvent', { type, key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, ...(type === 'keyDown' ? { text: '\r' } : {}) });
    }
    await new Promise((r) => setTimeout(r, 50));
    const afterEnter = await actual();
    const stateEnter = await sortState();
    const wantDesc = await expected(col, col === 0 ? 'asc' : 'desc');
    for (const type of ['keyDown', 'keyUp']) {
      await page.send('Input.dispatchKeyEvent', { type, key: ' ', code: 'Space', windowsVirtualKeyCode: 32, ...(type === 'keyDown' ? { text: ' ' } : {}) });
    }
    await new Promise((r) => setTimeout(r, 50));
    const afterSpace = await actual();
    const stateSpace = await sortState();
    if (col !== 0) {
      if (JSON.stringify(afterEnter) !== JSON.stringify(wantAsc)) problems.push(`tabela ${t + 1}: Enter não ordenou a coluna ${col + 1} como o site atual`);
      if (stateEnter[col] !== 'ascending' || stateEnter[0] !== 'none') problems.push(`tabela ${t + 1}: aria-sort depois do Enter: ${JSON.stringify(stateEnter)}`);
      if (JSON.stringify(afterSpace) !== JSON.stringify(wantDesc)) problems.push(`tabela ${t + 1}: espaço não inverteu a coluna ${col + 1} como o site atual`);
      if (stateSpace[col] !== 'descending') problems.push(`tabela ${t + 1}: aria-sort depois do espaço: ${JSON.stringify(stateSpace)}`);
    }
  }
  return problems;
}

async function checkToc(page) {
  const problems = [];
  const has = await page.eval("!!document.getElementById('otp')");
  if (!has) return problems;
  await page.eval("window.scrollTo({ top: 0, behavior: 'instant' })");
  await new Promise((ok) => setTimeout(ok, 100));
  if (!(await click(page, '#otp-fab'))) return ['sem o botão do "Nesta página"'];
  if (!(await page.eval("document.getElementById('otp').open"))) problems.push('o "Nesta página" não abriu');
  const id = await page.eval("document.querySelector('#otp-panel a')?.dataset.id ?? null");
  if (!(await click(page, '#otp-panel a'))) {
    problems.push('sem título no "Nesta página"');
  } else {
    await new Promise((ok) => setTimeout(ok, 150));
    const after = await page.eval("({ open: document.getElementById('otp').open, hash: decodeURIComponent(location.hash) })");
    if (after.open || after.hash !== `#${id}`) problems.push(`escolher um título não levou a ele: ${JSON.stringify(after)}`);
  }
  return problems;
}

function firstDiff(a, b) {
  let i = 0;
  while (i < a.length && a[i] === b[i]) i++;
  const from = Math.max(0, i - 40);
  return `    antes:  ${JSON.stringify(a.slice(from, i + 80))}\n    depois: ${JSON.stringify(b.slice(from, i + 80))}`;
}
