// sheet-editor.mjs · as contas da ficha interativa na referência de paridade
// (src/lib/ficha-engine.ts).
//
// Usado por gen-references.mjs. O motor da ficha da fonte é uma função só,
// `montarFicha`, que monta a tela e guarda as contas dentro dela (nada é
// exportado). Para rodar as contas como estão, o gerador:
//
//   1. carrega o ficha-engine.ts pelo esbuild com dois remendos de texto, cada
//      um conferido (se o trecho não for achado, o gerador para):
//      - logo depois de `export function montarFicha(opts: FichaOpts) {`, um
//        gancho em `globalThis.__ficha` que expõe as funções internas
//        (fresh, recompute, normalize, capFor, pisoAttr, clampSpecs,
//        renderCaminhos, efeitosAoAlcance) e o estado `S`;
//      - antes de `const total = ...` do `recompute`, a cópia das parcelas do
//        XP em `globalThis.__xp`;
//   2. roda `montarFicha` num DOM falso (um Proxy que aceita tudo, guarda o
//      `innerHTML` de cada elemento e os ouvintes de evento);
//   3. chama as funções pelo gancho. A troca de raça é o ouvinte `change` do
//      `#raca-sel`, chamado como o navegador chamaria. O estado das Técnicas sai
//      do HTML que o `renderCaminhos` escreve em `#tecnicas`.
//
// Cada caso é { fn, args, out }. As fichas de entrada ficam à parte, em
// `inputs` (por id, com o `massaEscolhida` que o normalize deixou), e o caso
// cita o id. São as de tests/parity/sheets.json (a `ficha` de cada uma)
// passadas pelo `normalize` da fonte, só com os campos que as contas leem, mais
// a ficha nova em cada raça e quatro fichas com compras de propósito.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const HOOK_AT = 'export function montarFicha(opts: FichaOpts) {';
const HOOK = `
  globalThis.__ficha = {
    fresh: () => fresh(), recompute: () => recompute(), normalize: () => normalize(),
    getS: () => S, setS: (v) => { S = v; }, massaEscolhida: () => massaEscolhida,
    capFor: (k, key) => capFor(k, key), pisoAttr: (k) => pisoAttr(k),
    clampSpecs: (a, s) => clampSpecs(a, s), renderCaminhos: () => renderCaminhos(),
    efeitosAoAlcance: () => efeitosAoAlcance(),
  };`;
const TOTAL_AT = 'const total = xa + xs + xsp + xv + xw + xap + xc + x2 + xt + xar + xef + xan + xr;';
const TOTAL = `${TOTAL_AT}\n    globalThis.__xp = { xa, xs, xsp, xv, xw, xap, xc, x2, xt, xar, xef, xan, xr, total };`;

function patch(text, at, by) {
  if (text.split(at).length !== 2) throw new Error(`ficha-engine.ts: o trecho do remendo não está lá uma vez só: ${at}`);
  return text.replace(at, by);
}

// O DOM falso. Todo acesso devolve um objeto que aceita tudo; o que se grava
// fica guardado. `getElementById` devolve o mesmo elemento para o mesmo id.
function fakeDom() {
  const elements = {};
  const listeners = [];
  const quiet = new Set(['firstChild', 'lastChild', 'firstElementChild', 'lastElementChild', 'nextSibling',
    'previousSibling', 'nextElementSibling', 'previousElementSibling', 'parentNode', 'parentElement', 'then']);
  function anything(name) {
    const store = {};
    const target = function fake() {};
    return new Proxy(target, {
      get(_, k) {
        if (k in store) return store[k];
        if (k === Symbol.toPrimitive) return () => '';
        if (k === Symbol.iterator) return function* empty() {};
        if (typeof k === 'symbol' || quiet.has(k)) return undefined;
        if (k === 'length') return 0;
        if (k === 'addEventListener') return (type, fn) => listeners.push({ name, type, fn });
        if (k === 'querySelectorAll' || k === 'getElementsByClassName' || k === 'getElementsByTagName') return () => [];
        if (k === 'getElementById') return (id) => (elements[id] ??= anything('#' + id));
        if (k === 'classList') return { add() {}, remove() {}, toggle() {}, contains: () => false };
        if (k === 'dataset' || k === 'style') return (store[k] = {});
        return anything(`${name}.${k}`);
      },
      set(_, k, v) { store[k] = v; return true; },
      apply() { return anything(`${name}()`); },
      construct() { return anything(`new ${name}`); },
    });
  }
  return { anything, elements, listeners };
}

export async function loadSheetEditor({ build, SRC }) {
  const file = path.join(SRC, 'src', 'lib', 'ficha-engine.ts');
  const text = patch(patch(fs.readFileSync(file, 'utf8'), HOOK_AT, HOOK_AT + HOOK), TOTAL_AT, TOTAL);
  const outfile = path.join(os.tmpdir(), `parity-ficha-engine-${process.pid}.mjs`);
  await build({
    entryPoints: [file], outfile, bundle: true, format: 'esm', platform: 'node', logLevel: 'error',
    loader: { '.json': 'json' }, define: { 'import.meta.env': 'globalThis.__ENV__' },
    plugins: [{
      name: 'ficha-remendada',
      setup(b) {
        b.onLoad({ filter: /ficha-engine\.ts$/ }, () => ({ contents: text, loader: 'ts' }));
      },
    }],
  });
  globalThis.__ENV__ = globalThis.__ENV__ || { BASE_URL: '/', MODE: 'test' };
  const dom = fakeDom();
  const saved = {};
  for (const k of ['document', 'window', 'navigator', 'location', 'localStorage', 'requestAnimationFrame',
    'MutationObserver', 'HTMLElement', 'getComputedStyle', 'matchMedia', 'FileReader']) {
    saved[k] = Object.getOwnPropertyDescriptor(globalThis, k);
    Object.defineProperty(globalThis, k, { value: dom.anything(k), configurable: true, writable: true });
  }
  let mod;
  try {
    mod = await import(pathToFileURL(outfile).href);
  } finally {
    try { fs.unlinkSync(outfile); } catch { /* o sistema limpa */ }
  }
  mod.montarFicha({ carregar: () => null, salvar: () => {} });
  await new Promise((r) => setTimeout(r, 0));
  if (!globalThis.__ficha) throw new Error('o gancho da ficha não rodou');
  return { hook: globalThis.__ficha, dom, restore: () => {
    for (const [k, d] of Object.entries(saved)) {
      if (d) Object.defineProperty(globalThis, k, d); else delete globalThis[k];
    }
  } };
}

// Os campos que as contas daqui leem da ficha.
const READS = ['raca', 'attrs', 'skills', 'spec', 'defSpec', 'virtues', 'willpower', 'aparencia', 'centelha',
  'skills2', 'spec2', 'tech', 'arte', 'efeito', 'ante', 'anteNom', 'budget', 'corpo', 'id'];
const pick = (s) => Object.fromEntries(READS.filter((k) => s[k] !== undefined).map((k) => [k, structuredClone(s[k])]));

// O estado das Técnicas no HTML do `renderCaminhos`: as classes de cada pílula e
// o motivo do bloqueio.
function techniquesFromHtml(html) {
  const out = {};
  const re = /<span class="(tpill[^"]*)" data-tech="([^"]+)"(?: title="Requer ([^"]*)")?>/g;
  let m;
  while ((m = re.exec(html))) {
    const cls = m[1].split(' ').slice(1);
    if (!cls.length) continue; // a pílula comum, nem comprada nem bloqueada
    out[m[2]] = { classes: cls, ...(m[3] ? { requer: m[3] } : {}) };
  }
  return out;
}

export function buildSheetEditorCases({ editor, sheets, races, attributes, techniques, arts, effects }) {
  const { hook, dom } = editor;
  const cases = [];
  const add = (fn, args, out) => cases.push({ fn, args, out: out === undefined ? null : out });
  const load = (ficha) => { hook.setS(structuredClone(ficha)); hook.normalize(); return hook.getS(); };

  // a ficha nova
  hook.fresh();
  const fresh = structuredClone(hook.getS());
  add('fresh', [], fresh);

  // as fichas lidas: a de cada referência, mais a nova em cada raça. `prepare`
  // deixa o estado da ficha (e o `massaEscolhida` do normalize) como ao abrir.
  const inputs = sheets.map((sh) => {
    const prepare = () => load(sh.ficha);
    prepare();
    return { id: sh.id, s: pick(hook.getS()), chosen: hook.massaEscolhida(), prepare };
  });
  for (const r of races) {
    const prepare = () => { hook.fresh(); hook.getS().raca = r.id; };
    prepare();
    inputs.push({ id: `nova-${r.id}`, s: pick(hook.getS()), chosen: hook.massaEscolhida(), prepare });
  }
  // Técnicas compradas de propósito (uma em cada cinco), com Centelha de 0 a 6, e
  // Artes em níveis variados: o bloqueio por Centelha, o pré-requisito que falta
  // e os Efeitos ao alcance
  for (const c of [0, 1, 3, 6]) {
    const prepare = () => {
      hook.fresh();
      const st = hook.getS();
      st.centelha = c;
      techniques.forEach((t, i) => { if (i % 5 === c % 5) st.tech[t.id] = true; });
      arts.forEach((a, i) => { st.arte[a.id] = (i + c) % 7; });
      effects.forEach((e, i) => { if (i % 9 === 0) st.efeito[e.id] = true; });
    };
    prepare();
    inputs.push({ id: `compras-${c}`, s: pick(hook.getS()), chosen: hook.massaEscolhida(), prepare });
  }

  // o XP gasto
  for (const x of inputs) {
    hook.setS(structuredClone(x.s));
    globalThis.__xp = null;
    hook.recompute();
    add('xpGasto', [x.id], globalThis.__xp);
  }

  // os tetos e o piso, por raça
  const kinds = ['attr', 'skill', 'skill2', 'virtue', 'centelha', 'willpower', 'aparencia', 'ante', 'anten', 'outro'];
  for (const r of races) {
    hook.fresh();
    hook.getS().raca = r.id;
    for (const k of kinds) {
      if (k === 'attr') for (const a of attributes) add('capFor', [r.id, 0, k, a.id], hook.capFor(k, a.id));
      else add('capFor', [r.id, 0, k, null], hook.capFor(k, undefined));
    }
    for (const a of attributes) add('pisoAttr', [r.id, a.id], hook.pisoAttr(a.id));
  }
  for (const c of [-1, 0, 1, 2, 3, 4, 5, 6, 7, 9, 2.5]) {
    hook.fresh();
    hook.getS().centelha = c;
    add('capFor', ['humano', c, 'arte2', null], hook.capFor('arte2', undefined));
  }

  // a troca de raça: o ouvinte do `#raca-sel`
  const change = dom.listeners.find((l) => l.name === '#raca-sel' && l.type === 'change');
  if (!change) throw new Error('não achei o ouvinte de troca de raça');
  const someSheets = inputs.filter((x, i) => i % 7 === 0 || x.id.startsWith('nova-'));
  for (const x of someSheets) {
    for (const r of races) {
      x.prepare();
      change.fn({ target: { value: r.id } });
      const after = hook.getS();
      add('trocaDeRaca', [x.id, r.id], { attrs: after.attrs, corpo: after.corpo ?? null });
    }
  }

  // as especialidades: o recorte do `clampSpecs`
  const specs = [[], 3, 0, ['a', 'a', 'b'], [{ s: 'x', v: 5 }, { s: 'y', v: 1 }, { s: 'z', v: 0 }], [null, 'q', { v: 2 }]];
  for (const s of sheets) for (const v of Object.values(s.ficha.spec || {})) if (Array.isArray(v) && v.length) specs.push(v);
  for (const a of specs.slice(0, 80)) for (const lvl of [0, 1, 2, 3, 4, 5, 6, 7]) add('clampSpecs', [a, lvl], hook.clampSpecs(structuredClone(a), lvl));

  // o estado das Técnicas e os Efeitos ao alcance
  for (const x of inputs) {
    hook.setS(structuredClone(x.s));
    if (Object.values(x.s.tech || {}).some(Boolean) || x.id === 'nova-humano') {
      hook.renderCaminhos();
      add('tecnicas', [x.id], techniquesFromHtml(String(dom.elements.tecnicas?.innerHTML ?? '')));
    }
    add('efeitosAoAlcance', [x.id], hook.efeitosAoAlcance().map((e) => e.id));
  }
  return { inputs: Object.fromEntries(inputs.map((x) => [x.id, { ficha: x.s, massaEscolhida: x.chosen }])), cases };
}
