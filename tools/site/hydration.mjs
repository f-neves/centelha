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
//      com Esc; onde há "Nesta página", abre o sumário e escolhe um título; e,
//      onde há termos do glossário marcados, abre e fecha o cartão pelo mouse
//      e pelo teclado (foco e Esc) e abre e fecha o diálogo do termo;
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
// `PAGES` (caminhos separados por vírgula) limita a prova a essas páginas.
const pages = process.env.PAGES ? process.env.PAGES.split(',') : listPages(dist);
const THEMES = ['classico', 'escuro', 'legivel'];
// O texto inteiro do conteúdo, visível ou não.
const CONTENT = "document.querySelector('main').textContent";
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
  const vazio = (s) => s === '' || s === '\u2014' || s === '\u2013';
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
// As duas leituras do texto usam a mesma tela (a de desktop, com a barra
// lateral à vista); a gaveta do celular é conferida à parte.
// O corpo do capítulo usa `content-visibility: auto` (o navegador só faz o
// layout do que está na tela), e o `innerText` deixa de fora o que não teve
// layout. Antes de cada leitura, esta folha desliga o layout preguiçoso, e a
// comparação fica a mesma.
const ALL_VISIBLE = `(() => {
  if (!document.getElementById('test-all-visible')) {
    const s = document.createElement('style');
    s.id = 'test-all-visible';
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
// Quantas páginas tiveram termo do glossário marcado (a prova falha se
// nenhuma tiver: a conferência do cartão não pode passar calada).
let pagesWithTerms = 0;
// Os capítulos sem nenhum termo do glossário no texto (na comparação com o
// auto-link do site atual, os outros 27 têm termo marcado).
const CHAPTERS_WITHOUT_TERMS = ['regras/custo-montarias-e-viagens/'];

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

  // 4c. O cartão do glossário (antes do "Nesta página": escolher um título
  // muda a âncora, e a página rola de novo até ela por um segundo e meio, o
  // que fecha o cartão).
  problems.push(...(await checkCards(page, rel)));

  // 4d. O "Nesta página".
  problems.push(...(await checkToc(page)));

  // 4e. As tabelas que ordenam.
  problems.push(...(await checkTables(page)));

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

if (pages.some((r) => r.startsWith('regras/')) && pagesWithTerms === 0) {
  failures++;
  console.log('FALHA nenhuma página com termo do glossário marcado (o cartão não foi conferido)');
}

await browser.close();
await server.close();
console.log(failures ? `${failures} de ${pages.length} páginas falharam` : `todas as ${pages.length} páginas passaram`);
process.exit(failures ? 1 : 0);

/** Clica no meio do elemento, como um dedo (sem `element.click()`). */
/**
 * O centro de um elemento depois que ele parou de mexer (com a página em
 * duas colunas, ou logo depois de trocar a tela, o layout assenta aos
 * poucos e um clique medido cedo cairia noutro ponto). Rola até ele antes.
 */
async function stableCenter(page, selector) {
  let last = null;
  for (let i = 0; i < 20; i++) {
    const r = await page.eval(`(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) return null;
      if (${i} === 0) el.scrollIntoView({ block: 'center', behavior: 'instant' });
      // O primeiro pedaço: um link que quebra de linha tem o meio da caixa
      // entre as duas linhas, fora dele.
      const r = el.getClientRects()[0] ?? el.getBoundingClientRect();
      return r.width && r.height ? { x: r.x + r.width / 2, y: r.y + r.height / 2 } : null;
    })()`);
    if (!r) return null;
    if (last && Math.abs(last.x - r.x) < 0.5 && Math.abs(last.y - r.y) < 0.5) return r;
    last = r;
    await new Promise((ok) => setTimeout(ok, 100));
  }
  return last;
}

/** Clica no meio do elemento, como um dedo (sem `element.click()`). */
async function click(page, selector) {
  const rect = await stableCenter(page, selector);
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


async function checkTables(page) {
  const problems = [];
  // O passo anterior escolhe um título no "Nesta página", e a página rola de
  // novo até a âncora por um segundo e meio (a ilha do sumário segura a
  // âncora no lugar); um clique nesse meio-tempo cairia noutro ponto.
  await new Promise((r) => setTimeout(r, 1700));
  const count = await page.eval("[...document.querySelectorAll('main table')].filter((t) => t.querySelector('th.ord')).length");
  for (let t = 0; t < Math.min(2, count); t++) {
    const sel = `[...document.querySelectorAll('main table')].filter((t) => t.querySelector('th.ord'))[${t}]`;
    const label = `tabela ${t + 1}`;
    // A ordem que a regra do site atual daria a partir da ordem de agora
    // (a ordenação é estável, então o ponto de partida conta).
    // `NEGATIVE_SORT_ORACLE=1` troca a direção do oráculo: o teste negativo
    // da própria prova (ela tem de falhar).
    const flip = (dir) => (process.env.NEGATIVE_SORT_ORACLE ? (dir === 'asc' ? 'desc' : 'asc') : dir);
    const expected = (col, dir) => page.eval(`(${ORACLE})(${sel}, ${col}, '${flip(dir)}')`);
    const actual = () => page.eval(`Array.from(${sel}.tBodies[0].rows).map((r) => r.textContent)`);
    const sortState = () => page.eval(`[...${sel}.tHead.rows[0].cells].map((c) => c.getAttribute('aria-sort'))`);
    const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
    const press = async (key, code, keyCode, text) => {
      for (const type of ['keyDown', 'keyUp']) {
        await page.send('Input.dispatchKeyEvent', { type, key, code, windowsVirtualKeyCode: keyCode, ...(type === 'keyDown' ? { text } : {}) });
      }
      await new Promise((r) => setTimeout(r, 50));
    };

    // Clique no primeiro cabeçalho que ordena: crescente.
    const first = await page.eval(`(() => {
      const ths = [...${sel}.tHead.rows[0].cells];
      const k = ths.findIndex((c) => c.classList.contains('ord'));
      const b = k >= 0 ? ths[k].querySelector('.ord-btn') : null;
      if (!b) return null;
      b.scrollIntoView({ block: 'center', behavior: 'instant' });
      return k;
    })()`);
    if (first === null) {
      problems.push(`${label}: cabeçalho que ordena sem o botão`);
      continue;
    }
    await new Promise((r) => setTimeout(r, 100));
    const rect = await page.eval(`(() => { const r = ${sel}.tHead.rows[0].cells[${first}].getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`);
    let want = await expected(first, 'asc');
    for (const type of ['mousePressed', 'mouseReleased']) {
      await page.send('Input.dispatchMouseEvent', { type, x: rect.x, y: rect.y, button: 'left', clickCount: 1 });
    }
    if (!same(await actual(), want)) problems.push(`${label}: o clique não ordenou como o site atual`);
    if ((await sortState())[first] !== 'ascending') problems.push(`${label}: aria-sort não ficou ascending depois do clique`);

    // Teclado: no segundo cabeçalho que ordena, se houver, Enter dá crescente
    // e espaço inverte; se só um ordena, Enter inverte e espaço volta.
    const second = await page.eval(`(() => { const ths = [...${sel}.tHead.rows[0].cells]; return ths.findIndex((c, i) => i > ${first} && c.classList.contains('ord')); })()`);
    const col = second > 0 ? second : first;
    await page.eval(`${sel}.tHead.rows[0].cells[${col}].querySelector('.ord-btn').focus({ preventScroll: true })`);
    const afterEnterDir = col === first ? 'desc' : 'asc';
    want = await expected(col, afterEnterDir);
    await press('Enter', 'Enter', 13, '\r');
    if (!same(await actual(), want)) problems.push(`${label}: Enter na coluna ${col + 1} não ordenou como o site atual`);
    let state = await sortState();
    const enterAria = afterEnterDir === 'asc' ? 'ascending' : 'descending';
    if (state[col] !== enterAria || (col !== first && state[first] !== 'none')) {
      problems.push(`${label}: aria-sort depois do Enter: ${JSON.stringify(state)}`);
    }
    const afterSpaceDir = afterEnterDir === 'asc' ? 'desc' : 'asc';
    want = await expected(col, afterSpaceDir);
    await press(' ', 'Space', 32, ' ');
    if (!same(await actual(), want)) problems.push(`${label}: espaço na coluna ${col + 1} não ordenou como o site atual`);
    state = await sortState();
    if (state[col] !== (afterSpaceDir === 'asc' ? 'ascending' : 'descending')) {
      problems.push(`${label}: aria-sort depois do espaço: ${JSON.stringify(state)}`);
    }
  }
  return problems;
}

async function checkCards(page, rel) {
  const problems = [];
  if (!(await page.eval("!!document.querySelector('main .ref')"))) {
    if (rel.startsWith('regras/') && !CHAPTERS_WITHOUT_TERMS.includes(rel)) {
      problems.push('capítulo sem nenhum termo do glossário marcado');
    }
    return problems;
  }
  pagesWithTerms++;
  await page.eval("window.scrollTo({ top: 0, behavior: 'instant' })");
  const pop = () => page.eval("document.querySelector('.ref-pop b')?.textContent ?? null");
  const name = await page.eval(`(() => {
    const a = document.querySelector('main .ref');
    const data = JSON.parse(document.getElementById('ref-data').textContent);
    return data.find((d) => d.id === a.dataset.ref)?.name ?? null;
  })()`);
  // Mouse: passar por cima abre; sair fecha.
  const rect = await stableCenter(page, 'main .ref');
  // A rolagem fecha o cartão (como no site atual): espera o evento de
  // rolagem do scrollIntoView passar antes de mover o mouse.
  await new Promise((ok) => setTimeout(ok, 200));
  await page.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: rect.x, y: rect.y });
  await new Promise((ok) => setTimeout(ok, 100));
  if ((await pop()) !== name) problems.push(`o cartão não abriu pelo mouse (esperava ${name}, veio ${await pop()})`);
  await page.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 2, y: 2 });
  await new Promise((ok) => setTimeout(ok, 400));
  if ((await pop()) !== null) problems.push('o cartão não fechou ao tirar o mouse');
  // Teclado: o foco abre; Esc fecha.
  await page.eval("document.querySelector('main .ref').focus({ preventScroll: true })");
  await new Promise((ok) => setTimeout(ok, 100));
  if ((await pop()) !== name) problems.push('o cartão não abriu pelo foco');
  for (const type of ['keyDown', 'keyUp']) {
    await page.send('Input.dispatchKeyEvent', { type, key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 });
  }
  await new Promise((ok) => setTimeout(ok, 100));
  if ((await pop()) !== null) problems.push('Esc não fechou o cartão');
  // Clique: abre o diálogo do termo; Esc fecha.
  if (!(await click(page, 'main .ref'))) {
    problems.push('sem termo para clicar');
    return problems;
  }
  await new Promise((ok) => setTimeout(ok, 150));
  const modal = await page.eval("({ open: document.querySelector('.ref-modal').open, name: document.querySelector('.ref-modal .rm-head b')?.textContent ?? null, url: location.href })");
  if (!modal.open || modal.name !== name) problems.push(`o clique não abriu o diálogo do termo: ${JSON.stringify(modal)}`);
  for (const type of ['keyDown', 'keyUp']) {
    await page.send('Input.dispatchKeyEvent', { type, key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 });
  }
  await new Promise((ok) => setTimeout(ok, 150));
  if (await page.eval("document.querySelector('.ref-modal').open")) problems.push('Esc não fechou o diálogo');
  // Fechar o diálogo devolve o foco ao termo, e o foco abre o cartão de novo
  // (como no site atual); mais um Esc fecha, para o passo seguinte achar a
  // tela limpa.
  for (const type of ['keyDown', 'keyUp']) {
    await page.send('Input.dispatchKeyEvent', { type, key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 });
  }
  await new Promise((ok) => setTimeout(ok, 100));
  if ((await pop()) !== null) problems.push('Esc não fechou o cartão depois do diálogo');
  return problems;
}

async function checkToc(page) {
  const problems = [];
  const has = await page.eval("!!document.getElementById('otp')");
  if (!has) return problems;
  await page.eval("window.scrollTo({ top: 0, behavior: 'instant' })");
  // Ao descer a página o botão some (otp-hidden, sem receber clique); no alto
  // ele volta quando a ilha trata a rolagem, o que pode demorar numa máquina
  // lenta. Espera ele voltar antes de clicar.
  try {
    await page.waitFor("!document.getElementById('otp-fab').classList.contains('otp-hidden')", 5000);
  } catch { /* segue; a conferência abaixo diz o que houve */ }
  // O ponteiro sai de cima do texto (o passo anterior o deixou sobre um termo).
  await page.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 2, y: 2 });
  await new Promise((ok) => setTimeout(ok, 300));
  const fabState = await page.eval(`(() => {
    const f = document.getElementById('otp-fab');
    const r = f.getBoundingClientRect();
    const top = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
    return { cls: f.className, top: top ? top.outerHTML.slice(0, 80) : null, pop: !!document.querySelector('.ref-pop'), dialog: document.querySelector('.ref-modal')?.open ?? null };
  })()`);
  if (!(await click(page, '#otp-fab'))) return ['sem o botão do "Nesta página"'];
  if (!(await page.eval("document.getElementById('otp').open"))) {
    problems.push(`o "Nesta página" não abriu (antes do clique: ${JSON.stringify(fabState)})`);
  }
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
