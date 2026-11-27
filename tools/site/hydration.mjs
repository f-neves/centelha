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
// As páginas rodam em `HYDRATION_TABS` abas ao mesmo tempo (padrão 4), cada
// aba num contexto de navegador próprio. A saída sai na ordem das páginas; uma
// página que lança erro ou passa de `HYDRATION_PAGE_TIMEOUT_MS` (padrão 120 s)
// falha com o motivo, e as outras seguem.
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
// O que a ilha das abas dos Efeitos esconde ou abre depois de montar: desfeito
// para comparar o texto com o de antes das ilhas, e refeito depois.
const UNDO_ISLAND_VIEW = `(() => {
  const saved = [];
  document.querySelectorAll('.ef-grupo, .ef-grupo-tit, .ef-grupo-sub').forEach((e) => { saved.push([e, 'hidden', e.hidden]); e.hidden = false; });
  document.querySelectorAll('details.ef-bloco').forEach((d) => { saved.push([d, 'open', d.open]); d.open = false; });
  window.__islandView = saved;
  return saved.length;
})()`;
const REDO_ISLAND_VIEW = `(() => { for (const [e, k, v] of window.__islandView ?? []) e[k] = v; return true; })()`;

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

// Quantas abas ao mesmo tempo (`HYDRATION_TABS`, padrão 4). Cada aba tem o
// seu contexto de navegador (armazenamento próprio) e pega a próxima página da
// fila; dentro de uma aba as páginas seguem em ordem, como na prova em série.
const TABS = Math.max(1, Number.parseInt(process.env.HYDRATION_TABS ?? '4', 10) || 1);
// Uma página que trava não segura as outras: passou disto, falha com o motivo.
const PAGE_TIMEOUT_MS = Number.parseInt(process.env.HYDRATION_PAGE_TIMEOUT_MS ?? '120000', 10);

const server = await serve(dist, base);
// Abas em segundo plano não podem ter o relógio nem o desenho freados.
const browser = await launch(undefined, [
  '--disable-background-timer-throttling',
  '--disable-backgrounding-occluded-windows',
  '--disable-renderer-backgrounding',
]);
let failures = 0;
// Quantas páginas tiveram termo do glossário marcado (a prova falha se
// nenhuma tiver: a conferência do cartão não pode passar calada).
let pagesWithTerms = 0;
// Os capítulos sem nenhum termo do glossário no texto (na comparação com o
// auto-link do site atual, os outros 27 têm termo marcado).
const CHAPTERS_WITHOUT_TERMS = ['regras/custo-montarias-e-viagens/'];

const tabs = Math.min(TABS, pages.length) || 1;
console.log(`hidratação: ${pages.length} páginas de ${dist} (base ${base}), navegador ${browser.executable}, ${tabs} abas`);

// As linhas saem na ordem das páginas, como na prova em série: cada resultado
// espera os de antes dele.
const results = new Array(pages.length);
let printed = 0;
const flush = () => {
  while (printed < pages.length && results[printed]) {
    const r = results[printed++];
    if (r.problems.length) {
      failures++;
      console.log(`FALHA ${base}${r.rel}\n  ${r.problems.join('\n  ')}`);
    } else {
      console.log(`ok    ${base}${r.rel} (${r.summary})`);
    }
  }
};

let next = 0;
const runStarted = Date.now();
await Promise.all(Array.from({ length: tabs }, async () => {
  const { browserContextId } = await browser.send('Target.createBrowserContext');
  try {
    while (next < pages.length) {
      const i = next++;
      results[i] = await runPage(browserContextId, pages[i]);
      flush();
    }
  } finally {
    await browser.send('Target.disposeBrowserContext', { browserContextId }).catch(() => {});
  }
}));
flush();

if (pages.some((r) => r.startsWith('regras/')) && pagesWithTerms === 0) {
  failures++;
  console.log('FALHA nenhuma página com termo do glossário marcado (o cartão não foi conferido)');
}

await browser.close();
await server.close();
console.log(`(${((Date.now() - runStarted) / 1000).toFixed(1)} s com ${tabs} abas)`);
console.log(failures ? `${failures} de ${pages.length} páginas falharam` : `todas as ${pages.length} páginas passaram`);
process.exit(failures ? 1 : 0);

/**
 * Uma página, com as abas dela no contexto `contextId`. Qualquer erro (uma
 * conferência que lança, a página que passa do tempo) vira problema desta
 * página, e as abas dela fecham; as outras seguem.
 */
async function runPage(contextId, rel) {
  // As abas abertas, os problemas e as mensagens do console e as exceções da
  // página: as mensagens entram no fim mesmo se a prova parar no meio.
  const ctx = { opened: [], problems: [], messages: [], offs: [] };
  let summary = '';
  let timer;
  const timeout = new Promise((_, fail) => {
    timer = setTimeout(() => fail(new Error(`a página passou de ${PAGE_TIMEOUT_MS} ms`)), PAGE_TIMEOUT_MS);
  });
  try {
    summary = await Promise.race([checkPage(contextId, rel, ctx), timeout]);
  } catch (e) {
    ctx.problems.push(`a prova parou nesta página: ${e.message}`);
  } finally {
    clearTimeout(timer);
    for (const off of ctx.offs) off();
    for (const p of ctx.opened) await p.close().catch(() => {});
  }
  // 5. Console limpo.
  const problems = [...ctx.problems, ...ctx.messages];
  return { rel, problems, summary };
}

async function checkPage(contextId, rel, ctx) {
  const { problems, opened, messages } = ctx;
  const url = `${server.origin}${base}${rel}`;
  const newPage = async () => {
    const p = await browser.newPage({ browserContextId: contextId, newWindow: true });
    opened.push(p);
    // A aba se comporta como a da frente (foco, eventos de foco e teclado),
    // mesmo com outras abertas ao mesmo tempo.
    await p.send('Emulation.setFocusEmulationEnabled', { enabled: true });
    return p;
  };

  // 1. Sem JavaScript.
  const plain = await newPage();
  await plain.send('Page.enable');
  await plain.send('Runtime.enable');
  await plain.send('Emulation.setScriptExecutionDisabled', { value: true });
  await plain.send('Emulation.setDeviceMetricsOverride', DESKTOP);
  await plain.goto(url);
  await plain.eval(ALL_VISIBLE);
  const before = await plain.eval('document.body.innerText');
  const contentBefore = await plain.eval(CONTENT);
  await plain.close();
  opened.splice(opened.indexOf(plain), 1);

  // 2. Com JavaScript e as ilhas.
  const page = await newPage();
  const on = (method, fn) => ctx.offs.push(page.on(method, fn));
  on('Runtime.consoleAPICalled', (p) => {
    if (['error', 'warning', 'assert'].includes(p.type)) {
      messages.push(`console.${p.type}: ${p.args.map((a) => a.value ?? a.description ?? '').join(' ')}`);
    }
  });
  on('Runtime.exceptionThrown', (p) => {
    const d = p.exceptionDetails;
    messages.push(`exceção: ${d.exception?.description ?? d.text}`);
  });
  on('Log.entryAdded', (p) => {
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
  // As abas dos Efeitos mudam o que se vê depois de montar (de propósito, como
  // no site atual): confere esse estado e o desfaz só para comparar o texto.
  problems.push(...(await checkTabsMounted(page)));
  await page.eval(UNDO_ISLAND_VIEW);
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

  await page.eval(REDO_ISLAND_VIEW);

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

  // 4f. O filtro de Equipamentos.
  problems.push(...(await checkFilter(page)));

  // 4g. O cartão das formas, o voltar ao topo e o painel dos fixados.
  problems.push(...(await checkShapes(page)));
  problems.push(...(await checkBackToTop(page)));
  problems.push(...(await checkBookmarks(page, rel)));

  // 4h. As Artes: o filtro e o marcador do catálogo; as abas, a âncora e a
  // troca de largura dos Efeitos.
  problems.push(...(await checkArtFilter(page)));
  problems.push(...(await checkEffectTabs(page, url, contentBefore, newPage, opened)));

  // 5. Console limpo: as mensagens entram em `runPage`.
  return `${before.length} caracteres, ilhas em ${hydrateMs} ms, tema ${current} -> ${target}`;
}

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
    // No canto direito do cabeçalho, onde fica a seta (o `::after` do `<th>`),
    // fora do botão: prova que o ouvinte está no `<th>` inteiro.
    const rect = await page.eval(`(() => {
      const th = ${sel}.tHead.rows[0].cells[${first}];
      const r = th.getBoundingClientRect();
      const x = r.right - 4, y = r.y + r.height / 2;
      return { x, y, onTh: document.elementFromPoint(x, y) === th };
    })()`);
    if (!rect.onTh) problems.push(`${label}: o canto direito do cabeçalho não é o próprio <th>`);
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
  // Espera o cartão mostrar `want` (o nome, ou null para fechado), até 2 s.
  const popIs = async (want) => {
    for (let i = 0; i < 20; i++) {
      if ((await pop()) === want) return true;
      await new Promise((ok) => setTimeout(ok, 100));
    }
    return false;
  };
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
  if (!(await popIs(name))) problems.push(`o cartão não abriu pelo mouse (esperava ${name}, veio ${await pop()})`);
  await page.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 2, y: 2 });
  if (!(await popIs(null))) problems.push('o cartão não fechou ao tirar o mouse');
  // Teclado: o foco abre; Esc fecha.
  await page.eval("document.querySelector('main .ref').focus({ preventScroll: true })");
  if (!(await popIs(name))) problems.push('o cartão não abriu pelo foco');
  for (const type of ['keyDown', 'keyUp']) {
    await page.send('Input.dispatchKeyEvent', { type, key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 });
  }
  if (!(await popIs(null))) problems.push('Esc não fechou o cartão');
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
  // tela limpa. Antes, espera o foco voltar.
  for (let i = 0; i < 20; i++) {
    if (await page.eval("document.activeElement?.classList.contains('ref') ?? false")) break;
    await new Promise((ok) => setTimeout(ok, 100));
  }
  await new Promise((ok) => setTimeout(ok, 100));
  for (const type of ['keyDown', 'keyUp']) {
    await page.send('Input.dispatchKeyEvent', { type, key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 });
  }
  if (!(await popIs(null))) problems.push('Esc não fechou o cartão depois do diálogo');
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
  // Tirar o `otp-hidden` devolve o botão com uma transição (ele sobe de baixo
  // da tela). Com a máquina ocupada (várias abas ao mesmo tempo no CI), a
  // transição pode não ter andado ainda: espera o botão à mostra e recebendo
  // o clique no meio dele (até 5 s), como o botão do menu em `checkShell`.
  try {
    await page.waitFor(`(() => {
      const f = document.getElementById('otp-fab');
      const r = f.getBoundingClientRect();
      return document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)?.closest('#otp-fab') === f;
    })()`, 5000);
  } catch { /* segue; a conferência abaixo diz o que houve */ }
  const fabState = await page.eval(`(() => {
    const f = document.getElementById('otp-fab');
    const r = f.getBoundingClientRect();
    const top = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
    return {
      cls: f.className, top: top ? top.outerHTML.slice(0, 80) : null, pop: !!document.querySelector('.ref-pop'), dialog: document.querySelector('.ref-modal')?.open ?? null,
      rect: [r.x, r.y, r.width, r.height].map(Math.round), tela: [innerWidth, innerHeight], foco: document.hasFocus(), visivel: document.visibilityState,
    };
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

async function checkFilter(page) {
  const problems = [];
  if (!(await page.eval("!!document.querySelector('[data-ef]')"))) return problems;
  const state = () => page.eval(`({
    visible: [...document.querySelectorAll('tr[data-arma]')].filter((r) => r.style.display !== 'none').length,
    total: document.querySelectorAll('tr[data-arma]').length,
    count: document.querySelector('.ef-count')?.textContent ?? null,
    hiddenBlocks: [...document.querySelectorAll('[data-bloco]')].filter((b) => b.hidden).length,
  })`);
  let s = await state();
  if (s.visible !== s.total || s.count !== `${s.total} armas`) problems.push(`filtro, ao abrir: ${JSON.stringify(s)}`);
  // Busca pelo teclado: digitar "adaga" no campo.
  await page.eval("document.querySelector('[data-ef=\"q\"]').focus({ preventScroll: true })");
  await page.send('Input.insertText', { text: 'adaga' });
  await new Promise((ok) => setTimeout(ok, 100));
  s = await state();
  const expected = await page.eval("[...document.querySelectorAll('tr[data-arma]')].filter((r) => r.dataset.txt.includes('adaga')).length");
  if (s.visible !== expected || s.count !== `${expected} arma${expected === 1 ? '' : 's'}`) problems.push(`filtro pela busca: ${JSON.stringify(s)} (esperava ${expected})`);
  // Limpa a busca e marca a classe "distancia" pelo mouse: a tabela de corpo a corpo some.
  await page.eval("(() => { const q = document.querySelector('[data-ef=\"q\"]'); q.value = ''; q.dispatchEvent(new Event('input')); return true; })()");
  if (!(await click(page, '[data-ef="classe"][value="distancia"]'))) problems.push('sem a caixa da classe Distância');
  await new Promise((ok) => setTimeout(ok, 100));
  s = await state();
  const ranged = await page.eval("document.querySelectorAll('tr[data-arma][data-classe=\"distancia\"]').length");
  if (s.visible !== ranged || s.hiddenBlocks !== 1) problems.push(`filtro pela classe: ${JSON.stringify(s)} (esperava ${ranged} e um bloco escondido)`);
  await click(page, '[data-ef="classe"][value="distancia"]');
  return problems;
}

async function checkShapes(page) {
  const problems = [];
  if (!(await page.eval("!!document.querySelector('[data-formas]')"))) return problems;
  const title = () => page.eval("document.querySelector('.formas-pop .formas-tit')?.textContent ?? null");
  const until = async (want) => {
    for (let i = 0; i < 20; i++) {
      if ((await title()) === want) return true;
      await new Promise((ok) => setTimeout(ok, 100));
    }
    return false;
  };
  const rect = await stableCenter(page, '[data-formas="area"]');
  await new Promise((ok) => setTimeout(ok, 200));
  await page.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: rect.x, y: rect.y });
  if (!(await until('Área'))) problems.push('o cartão das formas não abriu pelo mouse');
  await page.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 2, y: 2 });
  if (!(await until(null))) problems.push('o cartão das formas não fechou ao tirar o mouse');
  // Teclado: um botão de prova logo antes do primeiro rótulo de Volume; o Tab
  // a partir dele chega ao rótulo e abre o cartão; Esc fecha.
  await page.eval(`(() => {
    const label = [...document.querySelectorAll('[data-formas="volume"]')].find((e) => e.offsetParent) ?? document.querySelector('[data-formas="volume"]');
    const probe = document.createElement('button');
    probe.id = 'tab-probe';
    label.before(probe);
    probe.focus({ preventScroll: true });
    return true;
  })()`);
  // Uma rolagem atrasada (do passo do mouse) fecharia o cartão logo depois
  // de abrir: espera a página ficar 400 ms sem rolar antes do Tab.
  await settleScroll(page);
  for (const type of ['keyDown', 'keyUp']) {
    await page.send('Input.dispatchKeyEvent', { type, key: 'Tab', code: 'Tab', windowsVirtualKeyCode: 9 });
  }
  await page.eval("document.getElementById('tab-probe').remove()");
  if (!(await until('Volume'))) {
    const state = await page.eval("({ active: (document.activeElement?.className || document.activeElement?.tagName) + ': ' + (document.activeElement?.textContent ?? '').slice(0, 30), pop: !!document.querySelector('.formas-pop'), title: document.querySelector('.formas-pop .formas-tit')?.textContent ?? null, scrolls: window.__scrolls })");
    problems.push(`o Tab não abriu o cartão das formas no rótulo de Volume: ${JSON.stringify(state)}`);
  }
  for (const type of ['keyDown', 'keyUp']) {
    await page.send('Input.dispatchKeyEvent', { type, key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 });
  }
  if (!(await until(null))) problems.push('Esc não fechou o cartão das formas');
  return problems;
}

/** Recarrega e espera a página nova hidratar (e não a velha, ainda na tela). */
async function reloadAndWait(page) {
  await page.eval('(() => { window.__old = true; location.reload(); return true; })()');
  const ready = "!window.__old && document.readyState === 'complete' && document.documentElement.dataset.islands === 'hydrated'";
  for (let i = 0; i < 300; i++) {
    // No meio da troca de documento, a avaliação pode falhar: tenta de novo.
    if (await page.eval(ready).catch(() => false)) return;
    await new Promise((ok) => setTimeout(ok, 100));
  }
  throw new Error('a página recarregada não hidratou em 30 s');
}

/** Espera a página ficar 400 ms sem evento de rolagem (até 3 s). */
async function settleScroll(page) {
  await page.eval("(() => { if (!window.__scrolls) { window.__scrolls = { n: 0 }; addEventListener('scroll', () => { window.__scrolls.n++; window.__scrolls.at = performance.now(); }, true); } return true; })()");
  for (let i = 0; i < 30; i++) {
    const quiet = await page.eval("performance.now() - (window.__scrolls.at ?? 0) > 400");
    if (quiet) return;
    await new Promise((ok) => setTimeout(ok, 100));
  }
}

async function checkBackToTop(page) {
  const problems = [];
  if (!(await page.eval("!!document.getElementById('topo-btn')"))) return problems;
  const hidden = () => page.eval("document.getElementById('topo-btn').hidden");
  await page.eval("window.scrollTo({ top: 0, behavior: 'instant' })");
  await new Promise((ok) => setTimeout(ok, 200));
  if (!(await hidden())) problems.push('o voltar ao topo aparece no alto da página');
  await page.eval("window.scrollTo({ top: innerHeight * 2.5, behavior: 'instant' })");
  // O evento de rolagem chega no quadro seguinte, que demora com as outras
  // abas trabalhando: espera até 3 s.
  for (let i = 0; i < 30 && (await hidden()); i++) await new Promise((ok) => setTimeout(ok, 100));
  if (await hidden()) problems.push('o voltar ao topo não apareceu depois de duas telas');
  await page.eval("history.replaceState(null, '', location.pathname + '#algum-lugar')");
  if (!(await click(page, '#topo-btn'))) problems.push('sem o botão de voltar ao topo');
  for (let i = 0; i < 30 && (await page.eval('scrollY')) > 0; i++) await new Promise((ok) => setTimeout(ok, 100));
  const after = await page.eval("({ y: scrollY, hash: location.hash })");
  if (after.y > 0 || after.hash !== '') problems.push(`o voltar ao topo não levou ao alto sem a âncora: ${JSON.stringify(after)}`);
  return problems;
}

async function checkBookmarks(page, rel) {
  const problems = [];
  if (!(await page.eval("!!document.querySelector('main .ref')"))) return problems;
  const id = await page.eval("document.querySelector('main .ref').dataset.ref");
  const here = await page.eval('location.pathname');
  await page.eval("window.scrollTo({ top: 0, behavior: 'instant' })");
  // A rolagem fecha o cartão, e o evento dela chega um quadro depois: espera
  // passar antes de abrir.
  await new Promise((ok) => setTimeout(ok, 300));
  // O termo pode já estar com o foco (o passo do cartão o deixou assim), e
  // focar de novo não dispara o evento: tira o foco antes.
  await page.eval("(() => { document.activeElement?.blur(); document.querySelector('main .ref').focus({ preventScroll: true }); return true; })()");
  // Sem fixados, o índice dos fixados não é baixado.
  const fetched = "performance.getEntriesByType('resource').some((e) => e.name.endsWith('/ref-index.json'))";
  if (await page.eval(fetched)) problems.push('o ref-index.json foi baixado sem nenhum termo fixado');
  let name = null;
  for (let i = 0; i < 20 && !name; i++) {
    name = await page.eval("document.querySelector('.ref-pop b')?.textContent ?? null");
    if (!name) await new Promise((ok) => setTimeout(ok, 100));
  }
  for (const type of ['keyDown', 'keyUp']) {
    await page.send('Input.dispatchKeyEvent', { type, key: 'f', code: 'KeyF', windowsVirtualKeyCode: 70, ...(type === 'keyDown' ? { text: 'f' } : {}) });
  }
  // O painel baixa o índice (`ref-index.json`) no primeiro fixado: espera.
  for (let i = 0; i < 30 && !(await page.eval("!!document.querySelector('#consulta .cs-card')")); i++) {
    await new Promise((ok) => setTimeout(ok, 100));
  }
  if (!(await page.eval(fetched))) problems.push('o ref-index.json não foi baixado ao fixar o primeiro termo');
  const panel = await page.eval(`({
    names: [...document.querySelectorAll('#consulta .cs-card b')].map((b) => b.textContent),
    pinned: !!document.querySelector('main .ref.ref-pinned'),
    stored: sessionStorage.getItem('centelha:consulta'),
  })`);
  if (!name || !panel.names.includes(name) || !panel.pinned) problems.push(`a tecla f não fixou o termo no painel: ${JSON.stringify({ name, ...panel })}`);
  if (!(await click(page, '#consulta .cs-x'))) problems.push('sem o ✕ no painel dos fixados');
  await new Promise((ok) => setTimeout(ok, 200));
  const after = await page.eval("({ panel: !!document.getElementById('consulta'), stored: sessionStorage.getItem('centelha:consulta') })");
  if (after.panel || after.stored !== '[]') problems.push(`o ✕ não tirou o termo do painel: ${JSON.stringify(after)}`);
  // Num capítulo só (é lento): o download que falha tenta de novo no próximo
  // fixado, e a página aberta já com um fixado na sessão mostra o painel.
  if (rel !== 'regras/combate/') return problems;
  if ((await page.eval('location.pathname')) !== here) {
    problems.push(`o teste dos fixados saiu da página: ${await page.eval('location.href')}`);
    return problems;
  }
  const pin = (list) => page.eval(`(() => { sessionStorage.setItem('centelha:consulta', '${JSON.stringify(list)}'); dispatchEvent(new Event('rs:consulta')); return true; })()`);
  const cards = () => page.eval("document.querySelectorAll('#consulta .cs-card').length");
  const waitCards = async (n) => {
    for (let i = 0; i < 30 && (await cards()) !== n; i++) await new Promise((ok) => setTimeout(ok, 100));
    return cards();
  };
  await page.send('Network.enable');
  await page.send('Network.setBlockedURLs', { urls: ['*ref-index.json'] });
  await reloadAndWait(page);
  await pin([`termo:${id}`]);
  await new Promise((ok) => setTimeout(ok, 500));
  if ((await cards()) !== 0) problems.push('o painel apareceu sem o índice');
  await page.send('Network.setBlockedURLs', { urls: [] });
  await pin([`termo:${id}`]);
  if ((await waitCards(1)) !== 1) problems.push('depois de uma falha, o painel não baixou o índice de novo');
  // Abrir a página já com o fixado na sessão.
  await reloadAndWait(page);
  if ((await waitCards(1)) !== 1) problems.push('a página aberta com um fixado na sessão não mostrou o painel');
  await pin([]);
  if ((await waitCards(0)) !== 0) problems.push('o painel não sumiu ao esvaziar a lista');
  return problems;
}

async function checkTabsMounted(page) {
  const problems = [];
  if (!(await page.eval("!!document.querySelector('.ef-aba')"))) return problems;
  const s = await page.eval(`({
    titles: [...document.querySelectorAll('.ef-grupo-tit, .ef-grupo-sub')].every((e) => e.hidden),
    groups: [...document.querySelectorAll('.ef-grupo')].map((g) => g.dataset.cat + ':' + (g.hidden ? 'oculto' : 'visivel')).join(' '),
    open: [...document.querySelectorAll('details.ef-bloco')].every((d) => d.open),
    selected: document.querySelector('.ef-aba[aria-selected="true"]')?.dataset.aba ?? null,
  })`);
  if (!s.titles || s.groups !== 'elemental:visivel universal:oculto' || !s.open || s.selected !== 'elemental') {
    problems.push(`abas dos Efeitos, ao montar na tela larga: ${JSON.stringify(s)}`);
  }
  return problems;
}

async function checkEffectTabs(page, url, contentBefore, newPage, opened) {
  const problems = [];
  if (!(await page.eval("!!document.querySelector('.ef-aba')"))) return problems;
  const group = () => page.eval("[...document.querySelectorAll('.ef-grupo')].filter((g) => !g.hidden).map((g) => g.dataset.cat).join(' ')");
  await page.eval("window.scrollTo({ top: 0, behavior: 'instant' })");
  if (!(await click(page, '.ef-aba[data-aba="universal"]'))) problems.push('sem a aba das universais');
  if ((await group()) !== 'universal') problems.push(`a aba das universais não trocou o grupo: ${await group()}`);
  await click(page, '.ef-aba[data-aba="elemental"]');
  if ((await group()) !== 'elemental') problems.push(`a aba das elementais não voltou: ${await group()}`);
  // A âncora de um Efeito do outro grupo troca de aba, abre o bloco e rola até ele.
  const target = await page.eval("document.querySelector('.ef-grupo[data-cat=\"universal\"] .efeito')?.id ?? null");
  if (!target) {
    problems.push('sem cartão de Efeito nas universais');
    return problems;
  }
  await page.eval(`(() => { document.querySelector('details.ef-bloco:has(#${target})').open = false; location.hash = '${target}'; return true; })()`);
  let st = null;
  for (let i = 0; i < 20; i++) {
    st = await page.eval(`(() => {
      const t = document.getElementById('${target}');
      const r = t.getBoundingClientRect();
      return { group: [...document.querySelectorAll('.ef-grupo')].filter((g) => !g.hidden).map((g) => g.dataset.cat).join(' '),
        open: t.closest('details').open, top: Math.round(r.top), inView: r.top >= -2 && r.top < innerHeight };
    })()`);
    if (st.group === 'universal' && st.open && st.inView) break;
    await new Promise((ok) => setTimeout(ok, 100));
  }
  if (st.group !== 'universal' || !st.open || !st.inView) problems.push(`a âncora ${target} não abriu o Efeito: ${JSON.stringify(st)}`);
  // Troca de largura: no celular os blocos abrem fechados, sem descasamento.
  // A aba do celular sai do contexto da página (armazenamento isolado, foco
  // emulado) e fecha junto se a prova parar.
  const phone = await newPage();
  const errors = [];
  phone.on('Runtime.consoleAPICalled', (p) => { if (['error', 'warning'].includes(p.type)) errors.push(p.args.map((a) => a.value ?? a.description ?? '').join(' ')); });
  phone.on('Runtime.exceptionThrown', (p) => errors.push(p.exceptionDetails?.exception?.description ?? p.exceptionDetails?.text));
  await phone.send('Page.enable');
  await phone.send('Runtime.enable');
  await phone.send('Emulation.setDeviceMetricsOverride', PHONE);
  await phone.goto(url);
  try {
    await phone.waitFor("document.documentElement.dataset.islands === 'hydrated'", 30000);
    await new Promise((ok) => setTimeout(ok, 150));
    const m = await phone.eval(`({
      closed: [...document.querySelectorAll('details.ef-bloco')].every((d) => !d.open),
      content: document.querySelector('main').textContent,
    })`);
    if (!m.closed) problems.push('no celular, os blocos dos Efeitos não abriram fechados');
    if (m.content !== contentBefore) problems.push(`no celular, o textContent mudou na hidratação:\n${firstDiff(contentBefore, m.content)}`);
    if (errors.length) problems.push(`no celular, o console falou: ${errors.join(' | ')}`);
  } catch (e) {
    problems.push(`no celular, as ilhas não hidrataram: ${e.message}`);
  }
  await phone.close();
  opened.splice(opened.indexOf(phone), 1);
  return problems;
}

async function checkArtFilter(page) {
  const problems = [];
  if (!(await page.eval("!!document.querySelector('[data-fa]')"))) return problems;
  const state = () => page.eval(`({
    shown: [...document.querySelectorAll('.arte[data-cat]')].filter((c) => c.style.display !== 'none').map((c) => c.id),
    sections: [...document.querySelectorAll('.arte-sec')].filter((s) => s.style.display !== 'none').length,
    on: document.querySelectorAll('.f-chip.on').length,
  })`);
  const total = await page.eval("document.querySelectorAll('.arte[data-cat]').length");
  await page.eval("window.scrollTo({ top: 0, behavior: 'instant' })");
  await page.eval("document.querySelector('[data-fa=\"q\"]').focus({ preventScroll: true })");
  await page.send('Input.insertText', { text: 'cura' });
  await new Promise((ok) => setTimeout(ok, 100));
  let s = await state();
  const expected = await page.eval("[...document.querySelectorAll('.arte[data-cat]')].filter((c) => c.dataset.txt.includes('cura')).map((c) => c.id)");
  if (JSON.stringify(s.shown) !== JSON.stringify(expected) || !expected.length) problems.push(`filtro das Artes pela busca: ${JSON.stringify(s)} (esperava ${expected})`);
  if (!(await click(page, '#limpar-filtros'))) problems.push('sem o Limpar');
  s = await state();
  if (s.shown.length !== total) problems.push(`o Limpar não voltou tudo: ${s.shown.length} de ${total}`);
  // O título do grupo marca o grupo inteiro: só as universais, e a seção das elementais some.
  if (!(await click(page, '.f-tit[data-grupo="Universais"]'))) problems.push('sem o título das Universais');
  s = await state();
  const universal = await page.eval("[...document.querySelectorAll('.arte[data-cat=\"universal\"]')].map((c) => c.id)");
  if (JSON.stringify(s.shown) !== JSON.stringify(universal) || s.sections !== 1 || s.on !== universal.length) {
    problems.push(`o título das Universais não filtrou o grupo: ${JSON.stringify(s)}`);
  }
  await click(page, '#limpar-filtros');
  // O marcador: ☆ vira ★ e a lista gravada ganha a Arte; outro clique desfaz.
  const id = await page.eval("document.querySelector('.bm').dataset.bm");
  await click(page, '.bm');
  let m = await page.eval("({ text: document.querySelector('.bm').textContent, pressed: document.querySelector('.bm').getAttribute('aria-pressed'), stored: localStorage.getItem('centelha:marcadores') })");
  if (m.text !== '★' || m.pressed !== 'true' || m.stored !== JSON.stringify([id])) problems.push(`o marcador não marcou: ${JSON.stringify(m)}`);
  await click(page, '.bm');
  m = await page.eval("({ text: document.querySelector('.bm').textContent, stored: localStorage.getItem('centelha:marcadores') })");
  if (m.text !== '☆' || m.stored !== '[]') problems.push(`o marcador não desmarcou: ${JSON.stringify(m)}`);
  return problems;
}

function firstDiff(a, b) {
  let i = 0;
  while (i < a.length && a[i] === b[i]) i++;
  const from = Math.max(0, i - 40);
  return `    antes:  ${JSON.stringify(a.slice(from, i + 80))}\n    depois: ${JSON.stringify(b.slice(from, i + 80))}`;
}
