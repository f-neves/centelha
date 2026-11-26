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
    efeitosAoAlcance: () => efeitosAoAlcance(), renderDerived: () => renderDerived(),
    renderForca: () => renderForca(), cargaCalculada: () => cargaCalculada(), cargaAtual: () => cargaAtual(),
    numPeso: (x) => numPeso(x), r1n: (x) => r1n(x),
  };`;
const TOTAL_AT = 'const total = xa + xs + xsp + xv + xw + xap + xc + x2 + xt + xar + xef + xan + xr;';
const TOTAL = `${TOTAL_AT}\n    globalThis.__xp = { xa, xs, xsp, xv, xw, xap, xc, x2, xt, xar, xef, xan, xr, total };`;
// No renderDerived, depois das duas contas do deslocamento com a penalidade: a
// penalidade física, o deslocamento sem ela e cada medida que o card escreve,
// pelas próprias `mp` e `cmp` da fonte (as mesmas linhas da tabela).
const MOVE_AT = 'const cmp = (v: number) => Math.max(0, v - penMov * 10);';
const MOVE = `${MOVE_AT}
        globalThis.__mov = { penFisica, dz: { ...dz }, out: { batalha: mp(dz.normal), arranque: mp(dz.arranque),
          corrida: mp(dz.corrida), natacao: dz.natacao, saltoVertical: cmp(dz.saltoVertical),
          saltoHorizontalParado: mp(dz.saltoHorizontalParado), saltoHorizontalCorrendo: mp(dz.saltoHorizontalCorrendo) } };`;

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

// A semente do sorteio e o relógio parado da ficha (ver `loadSheetEditor`).
const UID_SEED = 20261126;
const FIXED_NOW = Date.UTC(2026, 10, 26);

export async function loadSheetEditor({ build, SRC }) {
  const file = path.join(SRC, 'src', 'lib', 'ficha-engine.ts');
  // No renderForca, antes da tabela de arremesso: as contas da seção de força,
  // pelas próprias funções locais da fonte (`dist`, `vel`, `velNatacao`, `velFrac`,
  // a escada `tw`). A segunda linha do deslocamento (a Vel. de Corrida com a
  // penalidade de armadura e das mãos) sai com a penalidade inteira e a corrida
  // sem ela, pelas mesmas chamadas da fonte.
  const FORCE_AT = 'const rows = tw.map((w) => {';
  const FORCE = `globalThis.__forca = (() => {
      const pen = (empilharArmaduras(pecasArmadura()).penalidade || 0) + (calcConj(conjAtivo()).penSum || 0);
      const dzC = deslocamento({ forca, destreza: A('destreza'), atletismo: atl, centelha: S.centelha }, dzFrac);
      const dzN = deslocamento({ vigor: A('vigor'), atletismo: atl, natacao: S.skills2['natacao'] || 0 }, dzFrac);
      const limites = [cMin, cLeve, cMedia, cPesada, maxKg];
      const amostra = [...limites.flatMap((w) => [w * (1 - 1e-9), w, w * (1 + 1e-9)]),
        ...Array.from({ length: 15 }, (_, i) => maxKg * (i + 1) / 20), maxKg * 0.3333, maxKg * 0.7, maxKg * 0.749];
      return { forca, atl, arr, faa, maxKg, acimaKg, tetoKg, pen, penMov, corridaBruta: dzC.corrida, vBase,
        natacaoBruta: dzN.natacao, vBaseNatacao, bandas: bandas.map((b) => ({ de: b.de, ate: b.ate, nome: b.nome })),
        tw, dist: tw.map(dist), amostra, velFrac: amostra.map(velFrac), velFracAgua: amostra.map(velFracAgua),
        vel: amostra.map(vel), velNatacao: amostra.map(velNatacao) };
    })();
    ${FORCE_AT}`;
  const text = patch(patch(patch(patch(fs.readFileSync(file, 'utf8'), HOOK_AT, HOOK_AT + HOOK), TOTAL_AT, TOTAL),
    MOVE_AT, MOVE), FORCE_AT, FORCE);
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
  // O `novoUid` da fonte sorteia com `Math.random` e o relógio: com os dois
  // fixos (o Mulberry32 de semente fixa e um `Date.now` parado), os `uid` das
  // peças saem iguais a cada regeneração e a referência se repete byte a byte.
  const random = Math.random, now = Date.now;
  let seed = UID_SEED;
  Math.random = () => {
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  Date.now = () => FIXED_NOW;
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
    Math.random = random;
    Date.now = now;
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

export function buildSheetEditorCases({ editor, sheets, races, attributes, techniques, arts, effects, pgr, tempo, forca, weapons, regras }) {
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
  // Uma ficha por Proeza, com Centelha 3 e as Técnicas dela compradas uma sim,
  // uma não (pela ordem de nível): o bloqueio pela Centelha, pelo
  // pré-requisito, pelos dois, e a comprada sem o pré-requisito
  const paths = [...new Set(techniques.map((t) => t.caminho))];
  for (const p of paths) {
    const prepare = () => {
      hook.fresh();
      const st = hook.getS();
      st.centelha = 3;
      techniques.filter((t) => t.caminho === p).sort((a, b) => a.nivel - b.nivel)
        .forEach((t, i) => { if (i % 2 === 0) st.tech[t.id] = true; });
    };
    prepare();
    inputs.push({ id: `proeza-${p}`, s: pick(hook.getS()), chosen: hook.massaEscolhida(), prepare });
  }

  // a massa escrita pelo jogador, como o normalize a lê ao abrir
  for (const x of inputs) add('massaEscolhida', [x.id], x.chosen);

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
      const pills = techniquesFromHtml(String(dom.elements.tecnicas?.innerHTML ?? ''));
      // a ficha de uma Proeza grava só as pílulas daquela Proeza (o resto é
      // a ficha nova com Centelha 3, que as outras fichas já cobrem)
      const only = x.id.startsWith('proeza-') ? new Set(techniques.filter((t) => `proeza-${t.caminho}` === x.id).map((t) => t.id)) : null;
      add('tecnicas', [x.id], only ? Object.fromEntries(Object.entries(pills).filter(([k]) => only.has(k))) : pills);
    }
    add('efeitosAoAlcance', [x.id], hook.efeitosAoAlcance().map((e) => e.id));
  }
  // o deslocamento com a penalidade (o renderDerived de cada ficha, inteira:
  // a penalidade vem da armadura e do escudo, que a entrada recortada não leva)
  for (const x of inputs) {
    x.prepare();
    globalThis.__mov = null;
    hook.renderDerived();
    const m = globalThis.__mov;
    if (m) add('deslocamento', [m.penFisica, m.dz], m.out);
  }
  // a seção de força e a carga (o renderForca de cada ficha inteira)
  // (a força em uma ficha a cada duas; a carga nas que levam peso, até 60, e em
  // cinco sem peso, com as partes da ficha que ela lê no próprio caso)
  let comPeso = 0, semPeso = 0;
  inputs.forEach((x, i) => {
    x.prepare();
    globalThis.__forca = null;
    hook.renderForca();
    const fz = globalThis.__forca;
    if (fz && i % 2 === 0) {
      // a entrada da conta (o que a ficha leu) vai nos argumentos
      const entrada = Object.fromEntries(['forca', 'atl', 'arr', 'maxKg', 'pen', 'corridaBruta', 'natacaoBruta', 'amostra']
        .map((k) => [k, fz[k]]));
      add('forca', [x.id, entrada], fz);
    }
    const calculada = hook.cargaCalculada();
    if ((calculada > 0 && comPeso < 60) || (calculada === 0 && semPeso < 5)) {
      if (calculada > 0) comPeso++; else semPeso++;
      const st = hook.getS();
      const partes = Object.fromEntries(['bolsas', 'arsenal', 'equip', 'cargaAtual']
        .filter((k) => st[k] !== undefined).map((k) => [k, structuredClone(st[k])]));
      add('carga', [x.id, partes], { calculada, atual: hook.cargaAtual() });
    }
  });
  // a carga escrita à mão por cima da calculada
  for (const w of ['', '  ', '12', '1,5 kg', '≈3', '3.2', 'x', '7,25,1']) {
    hook.fresh();
    hook.getS().cargaAtual = w;
    add('cargaEscrita', [w], hook.cargaAtual());
  }
  for (const s of [null, '', '1,5', '2 kg', '≈3', '3.2', '1,2,3', '.5', '5.', 'abc', '12,5 e 3', 7, 0.25, true])
    add('numPeso', [s], hook.numPeso(s));
  for (const n of [0, 0.04, 0.05, 0.15, 1.25, 1.45, 9.95, 99.94, 99.95, 100, 100.5, 122.4, 1234.5, -1.25])
    add('r1n', [n], hook.r1n(n));

  // a linha Preparo, Golpe e Recuperação de cada arma (o item como a ficha o
  // guarda: o do catálogo achatado, e variantes)
  const items = [];
  for (const w of weapons) {
    items.push(w);
    items.push({ ...w, ticks: (w.ticks ?? 5) + 1 });
    const { classe, ...semClasse } = w; void classe;
    items.push(semClasse);
    const { id, ...semId } = w;
    items.push({ ...semId, origem: id });
  }
  for (const t of [null, 3, 4, 5, 6, 7, 8, 9]) items.push(t === null ? { nome: 'Garras' } : { nome: 'Garras', ticks: t });
  items.push({}, { id: 'azagaia' }, { id: 'azagaia', ticks: 8 }, { nome: 'Azagaia', ticks: 6 });
  for (const w of items) {
    add('anatomiaDaFicha', [w], pgr.anatomiaDaFicha(structuredClone(w), regras));
    add('linhasComExtra', [w], pgr.linhasComExtra(structuredClone(w), regras));
    const cls = tempo.classeDeTempo(w?.id || w?.nome, w?.ticks, w?.classe);
    add('tetoDaRajada', [w], tempo.tetoDaRajada(cls));
  }

  // o alcance do arremesso
  for (const faa of [0, 1, 2, 5, 10, 20, 40]) {
    for (const max of [0, 20, 60, 150, 400]) {
      for (const kg of [-1, 0, 0.05, 0.1, 0.2, 0.5, 1, 2, 5, 10, 14, 15, 20, 30, 50, 80, 100]) {
        add('alcanceArremesso', [faa, kg, max], forca.alcanceArremesso(faa, kg, max, regras.forca));
      }
    }
  }

  return { inputs: Object.fromEntries(inputs.map((x) => [x.id, { ficha: x.s, massaEscolhida: x.chosen }])), cases };
}
