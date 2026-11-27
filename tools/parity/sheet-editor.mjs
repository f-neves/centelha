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

// O gancho das Artes e dos Efeitos: as funções da fonte como estão (nenhum
// trecho copiado, então nada para guardar). Elas devolvem HTML; o gerador lê dele
// os dados que o Rust devolve em estrutura.
const HOOK_ARTES = String.raw`
  Object.assign(globalThis.__ficha, {
    idRef: (x) => idRef(x), nomeArte: (id) => nomeArte(id), artesDe: (e) => artesDe(e), rotPar: (n) => rotPar(n),
    parCurto: (e) => parCurto(e), gruposEfeito: () => gruposEfeito(), corpoEfeitos: () => corpoEfeitos(),
    arteFx: (a, l) => arteFx(a, l), artePrint: (a, l) => artePrint(a, l), conteudoArtePop: (a) => conteudoArtePop(a),
    conteudoEfeitoPop: (e) => conteudoEfeitoPop(e), cardEfeito: (e, arteId) => cardEfeito(e, arteId),
    setEF: (o) => Object.assign(EF, o), arteD: () => ARTE_D, efeitoD: () => EFEITO_D,
  });`;

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
    marcarUso: (u, p) => marcarUso(u, p), ehDuasMaos: (p) => ehDuasMaos(p), pecaPara: (r) => pecaPara(r),
    calcConj: (c) => calcConj(c), conjAtivo: () => conjAtivo(), duplaConj: (c) => duplaConj(c),
    // o normalize de uma ficha crua: a ficha que sai, o que subiu pelo piso, a
    // massa e os uid sorteados, na ordem
    normalizar: (raw) => {
      S = structuredClone(raw); SUBIU_PELO_PISO.length = 0; globalThis.__uids = [];
      normalize();
      return { ficha: structuredClone(S), subiu: SUBIU_PELO_PISO.map((x) => ({ ...x })), massa: massaEscolhida, uids: [...globalThis.__uids] };
    },
  };`;
// O uid das peças e das instâncias, que a fonte sorteia, passa a ser anotado em
// ordem: o Rust o recebe pela lista e a referência diz quantos o normalize pediu.
const UID_AT = "const novoUid = () => 'p' + Math.random().toString(36).slice(2, 9) + Date.now().toString(36).slice(-3);";
const UID = `const novoUid = () => {
    const u = 'p' + Math.random().toString(36).slice(2, 9) + Date.now().toString(36).slice(-3);
    (globalThis.__uids ||= []).push(u);
    return u;
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

// Mais gancho, para mexer nos traços: as funções da fonte como estão, e cópias
// conferidas do que os ouvintes fazem dentro de um modal que o DOM falso não
// monta (as especialidades, o diálogo da especialidade de defesa, a linha de
// identidade). Cada trecho copiado tem o original em GUARDAS: se ele mudar na
// fonte, o gerador para.
const HOOK2 = String.raw`
  Object.assign(globalThis.__ficha, {
    applyVal: (k, key, v) => applyVal(k, key, v), pisoDe: (k, key) => pisoDe(k, key), valOf: (k, key) => valOf(k, key),
    apEfetiva: (v) => apEfetiva(v), racApMod: () => racApMod(), tetoRacialAttr: (k) => tetoRacialAttr(k),
    pisoRacialAttr: (k) => pisoRacialAttr(k), anteInst: (k) => anteInst(k), ehSecLivre: (k) => ehSecLivre(k),
    nomeLivre: (k) => nomeLivre(k), specCount: (a) => specCount(a), specSkill: (sc, k) => specSkill(sc, k),
    setMassa: (v) => { massaEscolhida = v; },
    espAbrir: (scope, key) => { const cap = Math.floor(specSkill(scope, key) / 2); if (cap <= 0) return false; const arr = specArr(scope, key); if (!arr.length) arr.push({ s: '', v: 0 }); return true; },
    espQuad: (scope, key, i, d) => { const cap = Math.floor(specSkill(scope, key) / 2); if (d > cap) return; const arr = specArr(scope, key); arr[i].v = arr[i].v === d ? d - 1 : Math.min(cap, d); },
    espNome: (scope, key, i, v) => { const arr = specArr(scope, key); arr[i].s = v; },
    espTirar: (scope, key, i) => { const arr = specArr(scope, key); arr.splice(i, 1); if (!arr.length) arr.push({ s: '', v: 0 }); },
    espMais: (scope, key) => { const cap = Math.floor(specSkill(scope, key) / 2); const arr = specArr(scope, key); if (arr.length < cap) { arr.push({ s: '', v: 0 }); } },
    espFechar: (scope, key) => { const arr = scope === 'p' ? S.spec[key] : S.spec2[key]; if (Array.isArray(arr)) { const p = arr.filter((e) => e.v > 0); if (scope === 'p') S.spec[key] = p; else S.spec2[key] = p; } },
    defEspMais: (key, r) => { const s = r.s.trim(); const v = Math.max(0, Math.min(6, parseInt(r.v, 10) || 0)); if (!s || !v) return; (S.defSpec[key] ||= []).push({ s, v }); },
    idLinha: (id, value) => { S.id[id] = value; if (id === 'peso') { const massa = Number(value.replace(',', '.').match(/\d+(?:\.\d+)?/)?.[0]); if (massa > 0) { S.corpo ??= {}; S.corpo.massa = massa; massaEscolhida = true; } else if (!value.trim()) { if (S.corpo) delete S.corpo.massa; massaEscolhida = false; } } },
  });`;
const GUARDAS = [
  String.raw`const cap = Math.floor(specSkill(scope, key) / 2); if (cap <= 0) return;`,
  String.raw`if (!arr.length) arr.push({ s: '', v: 0 });`,
  String.raw`arr[i].v = arr[i].v === d ? d - 1 : Math.min(cap, d);`,
  String.raw`arr[+inp.dataset.spname!].s = inp.value; save();`,
  String.raw`arr.splice(+b.dataset.sprm!, 1); paint(arr.length - 1); recompute();`,
  String.raw`if (arr.length < cap) { arr.push({ s: '', v: 0 }); paint(arr.length - 1); }`,
  // o espFechar: a lista de que se parte e o teste de que é lista, antes do filtro
  String.raw`const arr = scope === 'p' ? S.spec[key] : S.spec2[key];`,
  String.raw`if (Array.isArray(arr)) { const p = arr.filter(`,
  String.raw`const p = arr.filter((e: any) => e.v > 0); if (scope === 'p') S.spec[key] = p; else S.spec2[key] = p;`,
  // a mesma regra do espQuad, vista de dois lados: o quadradinho acima do teto sai com a classe `dis`, e o
  // clique nele não faz nada
  "${d > cap ? ' dis' : ''}",
  String.raw`if (s.classList.contains('dis')) return;`,
  // o defEspMais: o texto da situação sem as bordas, e sem situação ou sem bônus nada entra
  String.raw`const s = r.s.trim();`,
  String.raw`if (!s || !v) return;`,
  String.raw`const v = Math.max(0, Math.min(6, parseInt(r.v, 10) || 0));`,
  String.raw`(S.defSpec[key] ||= []).push({ s, v }); recompute();`,
  String.raw`S.id[inp.dataset.id!] = inp.value; if (inp.dataset.id === 'peso') { const massa = Number(inp.value.replace(',', '.').match(/\d+(?:\.\d+)?/)?.[0]); if (massa > 0) { S.corpo ??= {}; S.corpo.massa = massa; massaEscolhida = true; renderForca(); } else if (!inp.value.trim()) { if (S.corpo) delete S.corpo.massa; massaEscolhida = false; renderForca(); } }`,
];

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
  const text = patch(patch(patch(patch(patch(fs.readFileSync(file, 'utf8'), HOOK_AT, HOOK_AT + HOOK + HOOK2 + HOOK_ARTES), TOTAL_AT, TOTAL),
    MOVE_AT, MOVE), FORCE_AT, FORCE), UID_AT, UID);
  // o original de cada trecho copiado no gancho tem de estar na fonte, uma vez só
  const fonte = fs.readFileSync(file, 'utf8');
  for (const g of GUARDAS) patch(fonte, g, g);
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

export function buildSheetEditorCases({ editor, sheets, races, attributes, techniques, arts, effects, pgr, tempo, forca, weapons, regras, equip }) {
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

  // ----- as armas e as peças (fatia 4) -----
  // a peça nova do arsenal: o uid é sorteado pela fonte, e o Rust recebe o mesmo
  for (const x of [{ prepare: () => hook.fresh() }, ...inputs.slice(0, 6)]) {
    for (const ref of ['c', 'ce', 'a:montante', 'a:adaga', 'a:espada-longa', 'e:broquel']) {
      x.prepare();
      const antes = structuredClone(hook.getS().arsenal ?? null);
      const peca = structuredClone(hook.pecaPara(ref));
      add('pecaNova', [antes, ref], { arsenal: hook.getS().arsenal, peca });
    }
  }
  // a edição de peça: o valor de cada campo no item achatado do catálogo
  const lists = [['arma', equip.ARMAS, [...equip.CAMPOS_ARMA, ...equip.CAMPOS_ARMA_MAIS]],
    ['escudo', equip.ESCUDOS, equip.CAMPOS_ESCUDO], ['armadura', equip.ARMADURAS, equip.CAMPOS_ARMADURA]];
  for (const [tipo, itens, campos] of lists) {
    for (const it of itens) for (const c of campos) add('baseCampo', [tipo, it.id, c.k], equip.baseCampo(it, c));
  }
  // escrever um campo no envelope copiado do catálogo
  const amostra = { arma: ['espada-longa', 'montante', 'arco-curto', 'adaga'], escudo: ['broquel', 'escudo-medio'],
    armadura: ['couro', 'cota-de-malha', 'placas'] };
  const valores = (c) => (c.tipo === 'lista' ? [['a', 'b'], 'x', null]
    : c.tipo === 'opcao' ? [null, ...c.opcoes.map((o) => o.v)] : [null, 0, 3, -2]);
  for (const [tipo, itens, campos] of lists) {
    for (const origem of amostra[tipo]) {
      if (!equip.copiaDoCatalogo(tipo, origem)) continue;
      for (const c of campos) {
        for (const v of valores(c)) {
          const item = equip.copiaDoCatalogo(tipo, origem);
          equip.escreveCampo(item, c, structuredClone(v));
          add('escreveCampo', [tipo, origem, c.k, v], item);
        }
      }
    }
  }
  // restaurar o item de peças das fichas (o item delas e um trocado à mão)
  let restauradas = 0;
  for (const x of inputs) {
    if (restauradas >= 40) break;
    x.prepare();
    const st = hook.getS();
    for (const p of [...(st.arsenal || []), ...(st.equip?.armaduras || []).filter((a) => typeof a === 'object')]) {
      if (restauradas >= 40) break;
      const mexida = structuredClone(p);
      if (mexida.item?.arma) mexida.item.arma.dado = 9;
      if (mexida.item?.armadura?.soak) mexida.item.armadura.soak.corte = 9;
      for (const q of [p, mexida]) {
        const r = structuredClone(q);
        equip.restaurarItem(r);
        add('restaurarItem', [q], r);
      }
      restauradas++;
    }
  }
  // as duas mãos: ehDuasMaos de cada peça, e o marcarUso de cada peça em cada papel
  const comArsenal = inputs.filter((x) => { x.prepare(); return (hook.getS().arsenal || []).length; });
  let marcados = 0;
  for (const x of comArsenal) {
    x.prepare();
    for (const p of hook.getS().arsenal) add('ehDuasMaos', [p], hook.ehDuasMaos(p));
    for (const p of structuredClone(hook.getS().arsenal)) {
      for (const papel of ['nada', 'habil', 'inabil']) {
        if (marcados >= 60) break;
        x.prepare();
        const antes = { arsenal: structuredClone(hook.getS().arsenal), conjuntos: structuredClone(hook.getS().conjuntos) };
        hook.marcarUso(p.uid, papel);
        add('marcarUso', [antes, p.uid, papel], hook.getS().conjuntos);
        marcados++;
      }
    }
  }
  // a arma de duas mãos e a dupla, de propósito: montante, adaga e espada curta
  const pares = [['a:adaga', 'a:espada-curta'], ['a:montante', 'a:adaga'], ['a:espada-curta', 'a:montante'],
    ['a:machadinha', 'a:adaga'], ['c', 'a:adaga']];
  for (const [r1, r2] of pares) {
    for (const ordem of [['habil', 'inabil'], ['inabil', 'habil']]) {
      hook.fresh();
      const p1 = hook.pecaPara(r1), p2 = hook.pecaPara(r2);
      const antes = { arsenal: structuredClone(hook.getS().arsenal), conjuntos: structuredClone(hook.getS().conjuntos) };
      hook.marcarUso(p1.uid, ordem[0]);
      add('marcarUso', [antes, p1.uid, ordem[0]], structuredClone(hook.getS().conjuntos));
      const meio = { arsenal: structuredClone(hook.getS().arsenal), conjuntos: structuredClone(hook.getS().conjuntos) };
      hook.marcarUso(p2.uid, ordem[1]);
      add('marcarUso', [meio, p2.uid, ordem[1]], structuredClone(hook.getS().conjuntos));
      // o ataque duplo do conjunto em uso
      for (const forca of [1, 3, 5]) {
        hook.getS().attrs.forca = forca;
        const c = hook.calcConj(hook.conjAtivo());
        if (c.dupla) {
          add('ataqueDuplo', [{ dados: c.dados, bonus: c.bonus, flat: c.flat, dado: c.atk.dado ?? null, dupla: c.dupla }],
            hook.duplaConj(c));
        }
      }
    }
  }

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


  // ----- abrir a ficha: o resto do `normalize` -----
  // Cada caso é a ficha crua (o formato velho, o gravado, o de hoje) e o que o
  // `normalize` da fonte deixa: a ficha, a lista do que subiu pelo piso, se a
  // massa foi escolhida e os `uid` que ele sorteou, na ordem (o Rust os recebe
  // por esta lista).
  const w1 = equip.ARMAS[0].id, w2 = equip.ARMAS[7].id, w3 = equip.ARMAS[12].id;
  const sh1 = equip.ESCUDOS.find((x) => x.id !== 'nenhum').id;
  const ar1 = equip.ARMADURAS[0].id, ar2 = equip.ARMADURAS[1].id;
  const base = () => { hook.fresh(); return structuredClone(hook.getS()); };
  const crua = [];
  const mk = (id, f) => { const s = base(); f(s); crua.push({ id, ficha: s }); };
  const so = (id, ficha) => crua.push({ id, ficha });
  so('vazia', {});
  so('so-raca', { raca: 'humano' });
  so('so-raca-elfo', { raca: 'elfo' });
  so('nulos', { attrs: null, skills: null, spec: null, skills2: null, spec2: null, virtues: null, willpower: null, aparencia: null, centelha: null, raca: null, budget: null });
  mk('raca-invalida', (s) => { s.raca = 'xyz'; });
  mk('raca-ausente', (s) => { delete s.raca; });
  mk('raca-vazia', (s) => { s.raca = ''; });
  mk('virtudes-valor', (s) => { s.virtues = { valor: 3 }; });
  mk('virtudes-valor-e-bravura', (s) => { s.virtues = { valor: 3, bravura: 2 }; });
  mk('escudos', (s) => { s.skills.escudos = 3; s.spec.escudos = [{ s: 'x', v: 1 }]; delete s.skills.bloqueio; delete s.spec.bloqueio; s.blkPericia = 'x'; });
  mk('escudos-e-bloqueio', (s) => { s.skills.escudos = 3; s.skills.bloqueio = 1; s.spec.escudos = [{ s: 'x', v: 1 }]; s.spec.bloqueio = []; });
  mk('armas-uma-mao', (s) => { s.skills['armas-uma-mao'] = 2; s.spec['armas-uma-mao'] = [{ s: 'espadas', v: 1 }]; });
  mk('armas-fundidas', (s) => {
    s.skills.armas = 1; s.skills['armas-uma-mao'] = 2; s.skills['armas-duas-maos'] = 4;
    s.spec.armas = [{ s: 'lanças', v: 1 }]; s.spec['armas-uma-mao'] = [{ s: 'espadas', v: 1 }]; s.spec['armas-duas-maos'] = [{ s: 'machados', v: 2 }];
  });
  mk('armas-duas-maos-so', (s) => { s.skills['armas-duas-maos'] = 5; s.skills.armas = '3'; });
  for (const [prim, sec] of [['lideranca', 'lideranca'], ['medicina', 'cura'], ['energia-espiritual', 'energia-espiritual']]) {
    mk(`primaria-${prim}`, (s) => { s.skills[prim] = 3; s.spec[prim] = [{ s: 'a', v: 1 }]; delete s.skills2[sec]; });
    mk(`primaria-${prim}-com-secundaria`, (s) => { s.skills[prim] = 3; s.spec[prim] = [{ s: 'a', v: 1 }]; s.skills2[sec] = 2; s.spec2[sec] = [{ s: 'b', v: 1 }]; });
  }
  for (const [velho, novo] of [['culinaria', 'gastronomia'], ['veterinario', 'veterinaria'], ['ferraria', 'ferreiro'], ['escrivania', 'escrivao'],
    ['comercio', 'comerciante'], ['abrir-fechaduras', 'abrir-mecanismos'], ['ladinagem', 'abrir-mecanismos'], ['marcenaria', 'carpintaria'],
    ['contrabando', 'ocultacao'], ['falsificacao', 'ocultacao']]) {
    mk(`renome-${velho}`, (s) => { s.skills2[velho] = 4; s.spec2[velho] = [{ s: 'x', v: 2 }]; delete s.skills2[novo]; });
    mk(`renome-${velho}-fusao`, (s) => { s.skills2[velho] = 2; s.spec2[velho] = [{ s: 'x', v: 1 }]; s.skills2[novo] = 3; s.spec2[novo] = [{ s: 'y', v: 1 }]; });
  }
  // as migrações juntas, em ficha sem nenhuma chave de destino: a ordem em que cada
  // uma roda é a ordem em que as chaves novas entram no objeto
  mk('migracoes-juntas', (s) => {
    delete s.skills.bloqueio; delete s.skills.armas; delete s.skills2.lideranca; delete s.skills2.cura; delete s.skills2.gastronomia;
    s.skills.escudos = 3; s.skills['armas-uma-mao'] = 2; s.skills.lideranca = 2; s.skills.medicina = 1;
    s.skills2.culinaria = 3; s.spec2.culinaria = [{ s: 'x', v: 1 }];
  });
  mk('renome-fusao-especialidades', (s) => {
    s.skills2['abrir-fechaduras'] = 2; s.skills2.ladinagem = 3; delete s.skills2['abrir-mecanismos']; s.spec2['abrir-mecanismos'] = [];
    s.spec2['abrir-fechaduras'] = [{ s: 'fechaduras', v: 1 }]; s.spec2.ladinagem = [{ s: 'ladrao', v: 1 }];
  });
  mk('renome-duplo-fusao', (s) => { s.skills2['abrir-fechaduras'] = 3; s.skills2.ladinagem = 2; s.skills2['abrir-mecanismos'] = 1; s.spec2.ladinagem = [{ s: 'l', v: 1 }]; });
  mk('renome-contrabando-falsificacao', (s) => { s.skills2.contrabando = 2; s.skills2.falsificacao = 5; s.spec2.falsificacao = [{ s: 'f', v: 2 }]; delete s.skills2.ocultacao; });
  for (const r of races) {
    mk(`piso-${r.id}`, (s) => { s.raca = r.id; for (const k of Object.keys(s.attrs)) s.attrs[k] = 0; });
    mk(`piso-${r.id}-faltando`, (s) => { s.raca = r.id; const ks = Object.keys(s.attrs); delete s.attrs[ks[0]]; delete s.attrs[ks[2]]; s.attrs[ks[1]] = '1'; s.attrs[ks[3]] = null; });
  }
  mk('sem-padroes', (s) => { delete s.willpower; delete s.aparencia; delete s.centelha; delete s.budget; delete s.derivCol; s.modo = 'criacao'; });
  mk('piso-nao-numerico', (s) => { s.raca = 'elfo'; for (const k of Object.keys(s.attrs)) s.attrs[k] = 'x'; });
  mk('reliquia', (s) => { s.ante.reliquia = 3; s.anteNom.reliquia = [{ n: 'Anel', v: 2 }]; });
  mk('reliquia-e-artefato', (s) => { s.ante.reliquia = 3; s.ante.artefato = 1; s.anteNom.reliquia = [{ n: 'Anel', v: 2 }]; s.anteNom.artefato = [{ u: 'u1', n: 'Cajado', v: 1 }]; });
  mk('nomeados-sem-u', (s) => { s.anteNom = { aliado: [{ n: 'Ana', v: 2 }, { u: 'fixo', n: 7, v: '3' }, { n: null, v: 'x' }, { u: '', n: 'b', v: -0 }], inimigo: 'texto', contato: [{ n: 'C', v: 1 }] }; });
  mk('nomeados-ausentes', (s) => { delete s.ante; delete s.anteNom; });
  mk('equip-legado', (s) => { delete s.conjuntos; delete s.arsenal; s.equip = { arma: w1, escudo: sh1, armadura: ar1 }; });
  mk('equip-legado-vazio', (s) => { delete s.conjuntos; s.equip = { arma: 'desarmado', escudo: 'nenhum', armadura: 'nenhuma' }; });
  mk('equip-legado-armas-renomeadas', (s) => { delete s.conjuntos; s.equip = { arma: 'dardos' }; s.arsenal = [{ uid: 'p1', ref: 'a:glaive-serrilhada' }]; });
  mk('equip-sem-equip', (s) => { delete s.equip; delete s.conjuntos; });
  mk('armaduras-texto', (s) => { s.equip = { armaduras: [ar1, ar2], armMod: { [ar1]: { impacto: 1, corte: 'x' } } }; });
  mk('armaduras-objetos', (s) => {
    s.equip = { armaduras: [
      { base: ar1 }, { uid: 'keep', base: ar2, nome: 'Boa', mod: { impacto: 1 }, vestida: false },
      { uid: 'livre', base: 'c', item: { origem: null, nome: 'X', tipo: 'armadura', armadura: { soak: { impacto: 1, corte: 1, perfuracao: 1 } } }, img: 'a.png', enq: { x: 1 } },
      { base: 'zzz' }, { base: ar1, nome: '' }, { uid: 'x', vestida: true }, null, { uid: 'it', item: { origem: 'zzz' } },
    ] };
  });
  mk('armaduras-nao-lista', (s) => { s.equip = { armaduras: 'couro', armadura: ar2 }; });
  mk('conjuntos-estranhos', (s) => {
    s.arsenal = [];
    s.conjuntos = [
      { habil: { ref: `a:${w1}`, nome: 'Minha', dado: '2', danoBonus: null, acerto: '', mod: { dado: '1', ticks: 'a', acerto: 2 } }, inabil: { ref: `e:${sh1}`, item: { origem: sh1 } }, uidH: 'h', uidI: null },
      { habil: { ref: `a:${w1}`, nome: 'Minha', dado: 2, danoBonus: 0, mod: { dado: 1, acerto: 2 } }, inabil: { ref: `a:${w2}` } },
      null,
      { habil: { nome: 'sem ref' }, inabil: 'texto', ativo: 1 },
      { habil: { ref: 'c', nome: 'Livre', dado: 1, danoBonus: 0, acerto: -2 }, inabil: { ref: 'ce', nome: 'Escudo livre' } },
    ];
  });
  mk('conjuntos-sem-ativo', (s) => { s.conjuntos = [{ habil: { ref: 'a:desarmado' }, inabil: { ref: 'nada' } }, { habil: { ref: `a:${w3}` }, inabil: { ref: 'nada' } }]; });
  mk('conjuntos-vazio', (s) => { s.conjuntos = []; s.equip = { arma: w2 }; });
  mk('arsenal-sem-uid', (s) => { s.arsenal = [{ ref: `a:${w1}` }, null, { nome: 'sem ref' }, { uid: '', ref: `e:${sh1}`, nome: 'Escudo' }, { uid: 'ok', ref: 'c', nome: 'Livre', dado: 1, danoBonus: 0, acerto: -2 }, { uid: 'ok', ref: `a:${w2}` }]; s.conjuntos = [{ habil: { ref: `a:${w1}`, uid: 'velho' }, inabil: { ref: `e:${sh1}`, nome: 'Escudo' }, ativo: true }]; });
  mk('arsenal-mao-sumida', (s) => { s.arsenal = [{ uid: 'a1', ref: `a:${w1}` }]; s.conjuntos = [{ habil: { uid: 'a1', ref: `a:${w1}` }, inabil: { uid: 'nao-existe', ref: `a:${w2}` }, ativo: true }]; });
  mk('arsenal-nao-lista', (s) => { s.arsenal = 'x'; });
  mk('bolsas-texto', (s) => { s.bolsas = [{ nome: 'Carroça', texto: 'corda\r\n  tocha  \n\n \u00a0lanterna\u00a0 \r\nsaco' }, { nome: 'Mochila' }, null]; });
  mk('bolsas-itens', (s) => { s.bolsas = [{ nome: 5, itens: [{ item: 'x', peso: 2, preco: null }, { item: 3 }, 'texto', {}], pesoTotal: 12 }, { nome: 'Carroça', itens: [] }, { nome: '', itens: [{ item: 'a', peso: '1,5', preco: '2' }], pesoTotal: ' 3 ' }, { itens: [] }, {}]; });
  mk('bolsas-vazias', (s) => { s.bolsas = []; s.cargaAtual = 12; });
  mk('bolsas-ausentes', (s) => { delete s.bolsas; delete s.cargaAtual; });
  mk('bolsas-carga', (s) => { s.cargaAtual = null; });
  mk('livres', (s) => { delete s.livreNome; delete s.skills2['livre-1']; delete s.spec2['livre-2']; s.skills2['livre-3'] = 4; s.spec2['livre-3'] = [{ s: 'a', v: 1 }, { s: 'b', v: 1 }, { s: 'c', v: 1 }]; });
  mk('especialidades', (s) => {
    const ks = Object.keys(s.skills);
    s.skills[ks[0]] = 4; s.spec[ks[0]] = 3; s.skills[ks[1]] = 6; s.spec[ks[1]] = ['a', 'b', 'a', 'c', 'a'];
    s.skills[ks[2]] = 2; s.spec[ks[2]] = [{ s: 'x', v: 9 }, { s: 'y', v: 1 }]; s.skills[ks[3]] = 1; s.spec[ks[3]] = [{ s: 'x', v: 1 }];
    s.skills[ks[4]] = '5'; s.spec[ks[4]] = [null, { s: 3, v: '2' }];
  });
  mk('atributos-e-virtudes-faltando', (s) => { for (const k of Object.keys(s.attrs).slice(0, 4)) delete s.attrs[k]; for (const k of Object.keys(s.virtues).slice(0, 2)) delete s.virtues[k]; for (const k of Object.keys(s.skills).slice(0, 5)) delete s.skills[k]; });
  mk('massa-escolhida', (s) => { s.corpo = { massa: 80 }; s.id = { peso: '80 kg' }; });
  mk('massa-no-peso', (s) => { s.corpo = {}; s.id = { peso: '70 kg' }; });
  // as fichas de entrada do gerador (cruas): uma a cada quatro, no formato que cada uma tem
  sheets.forEach((sh, i) => { if (i % 4 === 0) so(`ficha-${sh.id ?? i}`, sh.ficha); });
  for (const x of crua) {
    const r = hook.normalizar(structuredClone(x.ficha));
    add('normalize', [x.ficha, r.uids], { ficha: r.ficha, subiu: r.subiu, massa: r.massa });
  }


  // ----- mexer nos traços: os ouvintes de verdade, com um alvo falso -----
  // O clique, a seta, o campo de massa e o do orçamento são os ouvintes da fonte,
  // chamados como o navegador os chamaria. O que o DOM falso não monta (o modal
  // das especialidades, o diálogo da especialidade de defesa, a linha de
  // identidade) vai pelas cópias conferidas do gancho.
  const achaOuvinte = (nome, tipo, pista) => {
    const l = dom.listeners.find((x) => x.name === nome && x.type === tipo && (!pista || String(x.fn).includes(pista)));
    if (!l) throw new Error(`não achei o ouvinte ${tipo} de ${nome} (${pista ?? ''})`);
    return l.fn;
  };
  const onClick = achaOuvinte('document', 'click', 'data-ante-add');
  const onKey = achaOuvinte('document', 'keydown', 'ArrowRight');
  const onInput = achaOuvinte('document', 'input', 'data-ante-nome');
  const onMassa = achaOuvinte('#corpo-massa', 'input');
  const onOrcamento = achaOuvinte('#xpBudget', 'input');
  const onLivre = achaOuvinte('#secondary', 'input', 'data-livre');
  const elfake = (dataset, extra = {}) => ({ dataset, classList: { contains: (c) => (extra.classes ?? []).includes(c) }, ...extra });
  const clicar = (mapa) => onClick({ target: { closest: (sel) => mapa[sel] ?? null } });

  const estados = {};
  const mkEstado = (id, raca, meio) => {
    hook.fresh();
    const s = hook.getS();
    s.raca = raca;
    for (const k of Object.keys(s.attrs)) s.attrs[k] = hook.pisoAttr(k);
    if (meio) {
      let i = 0;
      for (const k of Object.keys(s.attrs)) s.attrs[k] = Math.min(6, hook.pisoAttr(k) + (i++ % 4));
      Object.keys(s.skills).forEach((k, j) => { s.skills[k] = j % 7; if (j % 7 >= 2) s.spec[k] = [{ s: 'um', v: 1 }]; });
      Object.keys(s.skills2).slice(0, 12).forEach((k, j) => { s.skills2[k] = j % 6; if (j % 6 >= 4) s.spec2[k] = [{ s: 'dois', v: 2 }]; });
      Object.keys(s.virtues).forEach((k, j) => { s.virtues[k] = 1 + (j % 4); });
      s.centelha = 5; s.willpower = 6; s.aparencia = 4;
      for (const a of arts.slice(0, 5)) s.arte[a.id] = 3;
      for (const k of [...ANTE_UNICOS].slice(0, 3)) s.ante[k] = 2;
      s.anteNom = { ...s.anteNom, [ANTE_NOMEADO]: [{ u: 'ua', n: 'Ana', v: 2 }, { u: 'ub', n: 'Bia', v: 5 }] };
      techniques.slice(0, 8).forEach((tc) => { s.tech[tc.id] = true; });
    }
    estados[id] = structuredClone(s);
  };
  const ANTE_UNICOS = new Set(sheets.flatMap((sh) => Object.keys(sh.ficha?.ante ?? {})));
  const ANTE_NOMEADO = Object.keys(hook.normalizar({}).ficha.anteNom)[0];
  mkEstado('traco-humano', 'humano', false);
  mkEstado('traco-elfo', 'elfo', false);
  mkEstado('meio-anao', 'anao', true);
  mkEstado('meio-humano', 'humano', true);

  const ordem = (o) => Object.keys(o);
  const grupoDe = { attr: 'attrs', skill: 'skills', skill2: 'skills2', virtue: 'virtues', arte2: 'arte', ante: 'ante' };
  // o que a trilha guarda depois da operação
  const resumo = (kind, key) => {
    const S = hook.getS();
    if (kind === 'anten') return { v: hook.anteInst(key)?.v ?? null };
    if (grupoDe[kind]) {
      const r = { v: S[grupoDe[kind]]?.[key] ?? null };
      if (kind === 'skill') r.spec = S.spec?.[key] ?? null;
      if (kind === 'skill2') r.spec2 = S.spec2?.[key] ?? null;
      return r;
    }
    return { v: S[kind] ?? null };
  };
  // as chaves de topo da ficha que a operação mudou
  const mudou = (fn) => {
    const S0 = hook.getS();
    const antes = Object.fromEntries(Object.entries(S0).map(([k, v]) => [k, JSON.stringify(v)]));
    fn();
    const S1 = hook.getS();
    return Object.keys(S1).filter((k) => antes[k] !== JSON.stringify(S1[k]));
  };
  const abrir = (id) => { hook.setS(structuredClone(estados[id])); hook.setMassa(false); globalThis.__uids = []; };
  const trilhas = [];
  const cada = (kind, keys) => keys.forEach((key) => trilhas.push([kind, key]));
  const E = estados['meio-anao'];
  cada('attr', Object.keys(E.attrs));
  cada('skill', ['briga', 'armas', Object.keys(E.skills)[3], Object.keys(E.skills)[9]]);
  cada('skill2', Object.keys(E.skills2).slice(0, 3).concat(['livre-1']));
  cada('virtue', Object.keys(E.virtues));
  cada('centelha', ['']); cada('willpower', ['']); cada('aparencia', ['']);
  cada('arte2', arts.slice(0, 2).map((a) => a.id));
  cada('ante', [...ANTE_UNICOS].slice(0, 2));
  cada('anten', [`${ANTE_NOMEADO}~ua`, `${ANTE_NOMEADO}~ub`, `${ANTE_NOMEADO}~fantasma`]);
  const valoresDoTraco = [-2, 0, 1, 2, 3, 5, 6, 7, 9, 12, 13, 14];
  const bolinhas = [0, 1, 2, 3, 4, 5, 6, 7, 8, 12, 13];
  for (const id of Object.keys(estados)) {
    for (const [kind, key] of trilhas) {
      for (const v of valoresDoTraco) {
        abrir(id);
        const m = mudou(() => hook.applyVal(kind, key, v));
        add('aplicarValor', [id, kind, key, v], { ...resumo(kind, key), mudou: m });
      }
      for (const d of bolinhas) {
        abrir(id);
        const cap = d > hook.capFor(kind, key || undefined);
        const m = mudou(() => clicar({ '.dots .dot': elfake({ d: String(d) }, { classes: cap ? ['cap'] : [], parentElement: { dataset: { kind, key } } }) }));
        add('clicarBolinha', [id, kind, key, d], { ...resumo(kind, key), mudou: m });
      }
      for (const tecla of ['ArrowRight', 'ArrowLeft', 'ArrowUp', 'ArrowDown', 'Enter']) {
        if (tecla !== 'ArrowRight' && tecla !== 'ArrowLeft' && id !== 'meio-anao') continue;
        abrir(id);
        const m = mudou(() => onKey({ key: tecla, preventDefault() {}, target: { closest: (sel) => (sel === '.dots' ? { dataset: { kind, key } } : null) } }));
        add('teclaSeta', [id, kind, key, tecla], { ...resumo(kind, key), mudou: m });
      }
    }
  }
  // a Raça no piso, no teto e na Aparência; o valor da trilha
  for (const id of Object.keys(estados)) {
    hook.setS(structuredClone(estados[id]));
    for (const [kind, key] of trilhas) {
      add('pisoDe', [id, kind, key], hook.pisoDe(kind, key));
      add('valOf', [id, kind, key], hook.valOf(kind, key));
    }
    for (const a of Object.keys(estados[id].attrs)) {
      add('tetoRacial', [id, a], { teto: hook.tetoRacialAttr(a), piso: hook.pisoRacialAttr(a) });
    }
    add('racApMod', [id], hook.racApMod());
    for (const v of [-1, 0, 3, 11, 12, 15]) add('apEfetiva', [id, v], hook.apEfetiva(v));
  }
  for (const r of races) {
    hook.fresh(); hook.getS().raca = r.id;
    add('racApModDaRaca', [r.id], hook.racApMod());
    for (const v of [0, 5, 12]) add('apEfetivaDaRaca', [r.id, v], hook.apEfetiva(v));
  }
  add('ehSecLivre', ['livre-1'], hook.ehSecLivre('livre-1'));
  add('ehSecLivre', ['livre-4'], hook.ehSecLivre('livre-4'));
  add('ehSecLivre', ['culinaria'], hook.ehSecLivre('culinaria'));
  hook.setS({ livreNome: { 'livre-1': '  Cantoria \u00a0', 'livre-2': '', 'livre-3': 'Tricô' } });
  for (const k of ['livre-1', 'livre-2', 'livre-3', 'livre-4']) add('nomeLivre', [k], hook.nomeLivre(k));
  for (const lista of [[], [{ s: 'a', v: 2 }, { s: 'b', v: 1 }], [{ s: 'a' }, { v: 2 }, { s: 'c', v: null }]]) add('specCount', [lista], hook.specCount(lista));

  // a instância nomeada: pela chave, a que sumiu, e as três operações
  abrir('meio-anao');
  for (const key of [`${ANTE_NOMEADO}~ua`, `${ANTE_NOMEADO}~ub`, `${ANTE_NOMEADO}~fantasma`, 'x', `outro~ua`, `${ANTE_NOMEADO}`]) {
    add('anteInst', ['meio-anao', key], hook.anteInst(key));
  }
  for (const id of ['meio-anao', 'traco-humano']) {
    for (const nome of [ANTE_NOMEADO, 'inexistente']) {
      abrir(id);
      const m = mudou(() => clicar({ '[data-ante-add]': elfake({ anteAdd: nome }) }));
      add('anteAdd', [id, nome], { lista: hook.getS().anteNom?.[nome] ?? null, uids: [...globalThis.__uids], mudou: m });
    }
    for (const key of [`${ANTE_NOMEADO}~ua`, `${ANTE_NOMEADO}~fantasma`, `${ANTE_NOMEADO}~ub`]) {
      abrir(id);
      const m = mudou(() => clicar({ '[data-ante-rm]': elfake({ anteRm: key }) }));
      add('anteRm', [id, key], { lista: hook.getS().anteNom?.[ANTE_NOMEADO] ?? null, mudou: m });
      abrir(id);
      const m2 = mudou(() => onInput({ target: { closest: (sel) => (sel === '[data-ante-nome]' ? { dataset: { anteNome: key }, value: 'Novo nome' } : null) } }));
      add('anteNome', [id, key, 'Novo nome'], { lista: hook.getS().anteNom?.[ANTE_NOMEADO] ?? null, mudou: m2 });
    }
  }

  // a especialidade de defesa
  for (const [situacao, bonus] of [['contra um grupo', '2'], ['  na floresta  ', '9'], ['vs sedução', '0'], ['', '3'], ['x', 'abc'], ['y', '-4'], ['z', ' 5 acima'], ['w', '3.9']]) {
    for (const key of ['esquiva', 'social']) {
      abrir('meio-humano');
      const m = mudou(() => hook.defEspMais(key, { s: situacao, v: bonus }));
      add('defEspMais', ['meio-humano', key, situacao, bonus], { lista: hook.getS().defSpec?.[key] ?? null, mudou: m });
    }
  }
  abrir('meio-humano');
  hook.getS().defSpec.mental = [{ s: 'a', v: 1 }, { s: 'b', v: 2 }, { s: 'c', v: 3 }];
  const comDef = structuredClone(hook.getS());
  estados['defesas'] = comDef;
  for (const idx of [0, 1, 2, 3]) {
    abrir('defesas');
    const m = mudou(() => clicar({ '[data-defspec-del]': elfake({ defspecDel: `mental:${idx}` }) }));
    add('defEspTira', ['defesas', 'mental', idx], { lista: hook.getS().defSpec?.mental ?? null, mudou: m });
  }

  // as Técnicas (o clique na pílula) e os Efeitos
  for (const id of ['traco-humano', 'meio-anao']) {
    for (const tc of techniques.slice(0, 70)) {
      abrir(id);
      hook.renderCaminhos();
      const classes = techniquesFromHtml(String(dom.elements.tecnicas?.innerHTML ?? ''))[tc.id]?.classes ?? [];
      const m = mudou(() => clicar({ '.tpill': elfake({ tech: tc.id }, { classes }) }));
      add('tecnicaClique', [id, tc.id], { v: hook.getS().tech?.[tc.id] ?? null, mudou: m });
    }
    for (const ef of effects.slice(0, 25)) {
      abrir(id);
      const m = mudou(() => clicar({ '[data-efeito], [data-efbuy]': elfake({ efeito: ef.id }) }));
      add('efeitoClique', [id, ef.id], { v: hook.getS().efeito?.[ef.id] ?? null, mudou: m });
    }
  }

  // a massa, a identidade, o orçamento e o nome da Secundária livre
  for (const id of ['traco-humano', 'meio-anao']) {
    for (const texto of ['', '  ', '70', '82.5', '0', '-3', 'abc', '1e2']) {
      for (const antes of [false, true]) {
        abrir(id);
        hook.setMassa(antes);
        if (antes) { hook.getS().corpo = { massa: 80 }; hook.getS().id = { peso: '80 kg' }; }
        const numero = texto.trim() === '' ? NaN : Number(texto);
        const m = mudou(() => onMassa({ target: { value: texto, valueAsNumber: numero } }));
        add('massaCampo', [id, texto, antes], { corpo: hook.getS().corpo ?? null, id: hook.getS().id ?? null, massaEscolhida: hook.massaEscolhida(), mudou: m });
      }
    }
    for (const [campo, texto] of [['nome', 'Aria'], ['peso', '70 kg'], ['peso', '82,5'], ['peso', 'umas 3,5 e 7'], ['peso', ''], ['peso', '   '], ['peso', 'leve'], ['peso', '0 kg'], ['idade', '30']]) {
      abrir(id);
      hook.getS().corpo = { massa: 55 };
      const m = mudou(() => hook.idLinha(campo, texto));
      add('identidade', [id, campo, texto], { corpo: hook.getS().corpo ?? null, id: hook.getS().id ?? null, massaEscolhida: hook.massaEscolhida(), mudou: m });
    }
    for (const v of ['1200', '', 'abc', '-5', ' 800 ', '0x10', '3.5']) {
      abrir(id);
      const m = mudou(() => onOrcamento({ target: { value: v } }));
      add('orcamento', [id, v], { budget: hook.getS().budget ?? null, mudou: m });
    }
    for (const [k, v] of [['livre-1', 'Cantoria'], ['livre-3', ''], ['outra', 'x']]) {
      abrir(id);
      const m = mudou(() => onLivre({ target: { closest: (sel) => (sel === '[data-livre]' ? { dataset: { livre: k }, value: v } : null) } }));
      add('livreNome', [id, k, v], { livreNome: hook.getS().livreNome ?? null, mudou: m });
    }
  }

  // as especialidades: o modal, uma operação por caso, sobre a lista dada
  const listas = [null, [], [{ s: 'a', v: 1 }], [{ s: 'a', v: 2 }, { s: 'b', v: 0 }], [{ s: 'a', v: 1 }, { s: 'b', v: 1 }, { s: 'c', v: 1 }]];
  const ops = [['abrir'], ['quad', 0, 1], ['quad', 0, 2], ['quad', 0, 3], ['quad', 1, 1], ['nome', 0, 'novo'], ['tirar', 0], ['tirar', 1], ['mais'], ['fechar']];
  for (const [scope, kind] of [['p', 'skill'], ['s', 'skill2']]) {
    const chave = scope === 'p' ? 'briga' : Object.keys(E.skills2)[1];
    for (const nivel of [0, 1, 2, 4, 6]) {
      for (const lista of listas) {
        for (const op of ops) {
          abrir('meio-humano');
          const S = hook.getS();
          (scope === 'p' ? S.skills : S.skills2)[chave] = nivel;
          if (lista === null) delete (scope === 'p' ? S.spec : S.spec2)[chave]; else (scope === 'p' ? S.spec : S.spec2)[chave] = structuredClone(lista);
          let aberto = null;
          // o modal só existe aberto: toda operação, menos abrir e fechar, vem depois de abrir
          const preabre = op[0] !== 'abrir' && op[0] !== 'fechar';
          const cap = Math.floor(nivel / 2);
          if (preabre && cap <= 0) continue;
          if (preabre) {
            const tam = lista === null ? 1 : Math.max(1, lista.length);
            if ((op[0] === 'quad' || op[0] === 'nome' || op[0] === 'tirar') && op[1] >= tam) continue;
          }
          const m = mudou(() => {
            if (preabre) hook.espAbrir(scope, chave);
            if (op[0] === 'abrir') aberto = hook.espAbrir(scope, chave);
            else if (op[0] === 'quad') hook.espQuad(scope, chave, op[1], op[2]);
            else if (op[0] === 'nome') hook.espNome(scope, chave, op[1], op[2]);
            else if (op[0] === 'tirar') hook.espTirar(scope, chave, op[1]);
            else if (op[0] === 'mais') hook.espMais(scope, chave);
            else hook.espFechar(scope, chave);
          });
          const S2 = hook.getS();
          add('especialidade', ['meio-humano', scope, chave, nivel, lista, op], { lista: (scope === 'p' ? S2.spec : S2.spec2)[chave] ?? null, aberto, mudou: m });
        }
      }
    }
  }


  // ----- Artes e Efeitos, em dados -----
  // As funções da fonte escrevem HTML; daqui saem os dados que o Rust devolve em
  // estrutura (os níveis, o cartão, o cartão de hover, os grupos).
  const ARTES = hook.arteD(), EFEITOS = hook.efeitoD();
  const tira = (s) => String(s).replace(/<[^>]+>/g, '');
  const formasDe = (s) => [...String(s).matchAll(/data-formas="([^"]+)"/g)].map((m) => m[1]);
  const parsePars = (h) => ({ texto: tira(h), formas: formasDe(h) });
  const linhasDe = (h) => [...String(h).matchAll(/<div class="fxline( hi)?">([^ ]+) · <b>([\s\S]*?)<\/b>: ([\s\S]*?)<\/div>/g)]
    .map((m) => ({ nivel: Number(m[2]), nome: m[3], efeito: m[4], alcancado: !!m[1] }));
  const titulo = (h) => { const m = /<span class="ficha-pop-tit">([\s\S]*?)<small>([\s\S]*?)<\/small><\/span>/.exec(h); return { titulo: m[1], rotulo: m[2] }; };
  const cartao = (h) => ({
    comprado: /class="ef-card on"/.test(h),
    nome: /<span class="ef-c-nm"[^>]*>([\s\S]*?)<\/span>/.exec(h)[1],
    nivel: Number(/<span class="ef-i-nv"[^>]*>([^<]*)<\/span>/.exec(h)[1]),
    xp: Number(/<span class="ef-c-xp">([^ ]*) XP<\/span>/.exec(h)[1]),
    tambem: (/<div class="ef-c-ar">também em ([\s\S]*?)<\/div>/.exec(h)?.[1] ?? '').split(' · ').filter(Boolean),
    texto: /<div class="ef-c-tx">([\s\S]*?)<\/div>/.exec(h)[1],
    parametros: parsePars(/<div class="ef-c-par muted">([\s\S]*?)<\/div>/.exec(h)[1]),
  });
  const cartaoHover = (h) => ({
    ...titulo(h),
    texto: /<div class="ef-pop-tx">([\s\S]*?)<\/div>/.exec(h)[1],
    sabores: [...h.matchAll(/<div class="ef-pop-sab"><b>([\s\S]*?):<\/b> ([\s\S]*?)<\/div>/g)].map((m) => ({ arte: m[1], texto: m[2] })),
    parametros: [...h.matchAll(/<div class="ef-pop-p"><b>([\s\S]*?):<\/b> ([\s\S]*?)<\/div>/g)].map((m) => {
      const q = /^([\s\S]*?)(?:<small>([\s\S]*?)<\/small>)?$/.exec(m[2]);
      return { nome: tira(m[1]), forma: formasDe(m[1])[0] ?? null, valor: q[1], nota: q[2] ?? null };
    }),
    notas: /<div class="ef-pop-nota muted">([\s\S]*?)<\/div>/.exec(h)?.[1] ?? null,
  });
  const corpo = (h) => {
    const vazio = /<div class="ef-vazio muted">([\s\S]*?)<\/div>/.exec(h);
    if (vazio) return { vazio: vazio[1] };
    return { grupos: [...h.matchAll(/data-eftog="([^"]+)"[\s\S]*?<span class="ef-g-nm">([\s\S]*?)<\/span>\s*<span class="ef-g-conta">([\s\S]*?)<\/span>/g)]
      .map((m) => ({ arte: m[1], nome: m[2], conta: m[3] })) };
  };

  for (const x of [{ id: 'fogo' }, { id: { id: 'agua' } }, { id: null }, { id: 3 }, { id: 'a b' }]) add('idRef', [x.id], hook.idRef(x.id) ?? null);
  for (const a of ARTES) add('nomeArte', [a.id], hook.nomeArte(a.id));
  add('nomeArte', ['nao-existe'], hook.nomeArte('nao-existe'));
  const nomesPar = [...new Set(EFEITOS.flatMap((e) => e.parametros.map((p) => p.nome)).concat(['Volume', 'volume (m³)', 'Área', 'area', 'Alcance', '']))];
  for (const n of nomesPar) add('rotPar', [n], parsePars(hook.rotPar(n)));
  for (const e of EFEITOS) add('parCurto', [e.id], parsePars(hook.parCurto(e)));

  // os estados com as Artes todas no mesmo nível, e metade dos Efeitos comprados
  for (let n = 0; n <= 6; n++) {
    const prepare = () => {
      hook.fresh();
      const st = hook.getS();
      ARTES.forEach((a) => { st.arte[a.id] = n; });
      EFEITOS.forEach((e, i) => { if (i % 2 === 0) st.efeito[e.id] = true; });
    };
    prepare();
    inputs.push({ id: `artes-${n}`, s: pick(hook.getS()), chosen: hook.massaEscolhida(), prepare });
  }
  for (const x of inputs) {
    hook.setS(structuredClone(x.s));
    for (const so of [false, true]) {
      hook.setEF({ soComprados: so });
      add('gruposEfeito', [x.id, so], hook.gruposEfeito().map((g) => ({ arte: g.arte.id, itens: g.itens.map((e) => e.id) })));
      add('corpoEfeitos', [x.id, so], corpo(hook.corpoEfeitos()));
    }
    hook.setEF({ soComprados: false });
  }
  const comNiveis = inputs.filter((x) => x.id.startsWith('compras-') || x.id.startsWith('artes-') || x.id === 'meio-humano');
  for (const x of comNiveis) {
    hook.setS(structuredClone(x.s));
    for (const a of ARTES) {
      const lvl = hook.getS().arte[a.id] || 0;
      add('arteFx', [x.id, a.id], linhasDe(hook.arteFx(a, lvl)));
      add('artePrint', [x.id, a.id], linhasDe(hook.artePrint(a, lvl)));
      add('conteudoArtePop', [x.id, a.id], { ...titulo(hook.conteudoArtePop(a)), linhas: linhasDe(hook.conteudoArtePop(a)) });
    }
  }
  const comEfeitos = inputs.filter((x) => ['compras-0', 'compras-3', 'artes-2', 'artes-6'].includes(x.id));
  for (const x of comEfeitos) {
    hook.setS(structuredClone(x.s));
    for (const e of EFEITOS) {
      add('artesDe', [x.id, e.id], hook.artesDe(e).map((a) => hook.idRef(a.id)));
      add('conteudoEfeitoPop', [x.id, e.id], cartaoHover(hook.conteudoEfeitoPop(e)));
      for (const a of e.artes) add('cardEfeito', [x.id, e.id, hook.idRef(a.id)], cartao(hook.cardEfeito(e, hook.idRef(a.id))));
    }
  }

  return { inputs: Object.fromEntries([...inputs.map((x) => [x.id, { ficha: x.s, massaEscolhida: x.chosen }]),
    ...Object.entries(estados).map(([id, s]) => [id, { ficha: s, massaEscolhida: false }])]), cases };
}
