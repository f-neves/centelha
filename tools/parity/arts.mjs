// arts.mjs · a formatação dos Efeitos (src/lib/artes-fmt.ts) e o custo da
// conjuração (`custoDe`, src/lib/artes-grid.ts) na referência de paridade.
//
// Usado por gen-references.mjs. Cada caso: fn, args, out (-0 gravado como "-0").
//
//   idDe, formaDe, rank, ordemPar, valorPar, PAR_FORMA   de artes-fmt.ts
//   custoDe     o custo de cada Efeito (e do improviso) em níveis da Arte e
//               da Centelha variados, com parâmetros escolhidos de propósito
//   manaPaga    a Mana de um total de 0 a 12 pontos, Centelha de 0 a 6, com e
//               sem a fonte do elemento. A fonte não tem conta no TypeScript, só o
//               texto (`arcano.fonteElemental`: "+1 de desconto, somado ao da
//               Centelha", e a `zeraCusto`: Centelha 3 mais a fonte é Centelha 4
//               sem fonte): a referência é o `custoDe` do TypeScript com a
//               Centelha somada do desconto que o `regras.json` escreve.

const IDS = [
  'fogo', 'agua', 'terra', 'vento', null, undefined, 7, true, {}, { id: 'agua' }, { id: 3 }, { id: null },
  { id: '' }, '', { nome: 'x' }, [], ['fogo'],
];

const NAMES = [
  'Volume', 'volume', 'VOLUME', 'Volume (m³)', ' Volume', 'Área', 'área', 'AREA', 'Area', 'area', 'ÁREA', 'Áreas',
  'Alcance', 'Alvos', 'Dano', '', ' ', 'Duração', 'Cura', 'Comprimento', 'Raio', 'Altura', 'Profundidade',
  'Dano de fogo', 'dano da lança', 'Nuvem', 'Nuvem de Volume', 'Fah', 'FAA', 'Bônus', 'Força de vontade',
  'Duracao', 'Conjurar a Arte', 'Ataque', 'Jogada', 'Dificuldade', 'Penalidade', 'Visão', 'Voo', 'Quantidade',
];

const REPLACES = [undefined, null, '', 'area', 'Area', 'ÁREA', 'área', 'volume', 'alcance', 'Alvos', 'nada', 'dano', 'duração'];

// um sorteio de semente fixa (Mulberry32), só para embaralhar as listas de parâmetros
function seeded(seed) {
  let s = seed >>> 0;
  return () => {
    s = (s + 0x6d2b79f5) | 0;
    let t = Math.imul(s ^ (s >>> 15), 1 | s);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
export const ARTS_SEED = 20261128;

export function buildArtsCases({ fmt, grid, arts, effects, regras }) {
  const cases = [];
  const add = (fn, args, out) => cases.push({ fn, args, out: out === undefined ? null : out });
  const rnd = seeded(ARTS_SEED);
  const pick = (list) => list[Math.floor(rnd() * list.length)];

  add('PAR_FORMA', [], fmt.PAR_FORMA);
  for (const x of IDS) add('idDe', [x === undefined ? null : x], fmt.idDe(x) ?? null);
  for (const e of effects) for (const a of e.artes) add('idDe', [a.id], fmt.idDe(a.id));
  for (const e of effects) for (const a of e.artes) add('idDe', [{ id: a.id, sabor: a.sabor }], fmt.idDe(a));

  const nomes = [...new Set([...NAMES, ...effects.flatMap((e) => e.parametros.map((p) => p.nome))])];
  for (const n of nomes) add('formaDe', [n], fmt.formaDe(n) ?? null);

  // rank: cada parâmetro de cada Efeito, mais os nomes de propósito com cada `substitui`
  const seen = new Set();
  const once = (key, fn) => { if (!seen.has(key)) { seen.add(key); fn(); } };
  for (const e of effects) {
    for (const p of e.parametros) {
      once(`rank:${p.nome}:${p.substitui}`, () => add('rank', [p.nome, p.substitui ?? null], fmt.rank(p)));
    }
  }
  for (const n of NAMES) {
    for (const r of REPLACES) {
      once(`rank:${n}:${r}`, () => add('rank', [n, r ?? null], fmt.rank(r === undefined || r === null ? { nome: n } : { nome: n, substitui: r })));
    }
  }

  // ordemPar: cada Efeito (os índices dos parâmetros em ordem), e listas sorteadas
  // de nomes de propósito (com muitos empates de rank, que só o índice desempata)
  for (const e of effects) {
    add('ordemPar', [e.id], fmt.ordemPar(e.parametros).map((p) => e.parametros.indexOf(p)));
  }
  const pool = [...new Set(effects.flatMap((e) => e.parametros.map((p) => ({ nome: p.nome, tipo: p.tipo, ...(p.substitui ? { substitui: p.substitui } : {}) }))
    .map((p) => JSON.stringify(p))))].map((s) => JSON.parse(s));
  for (let k = 0; k < 400; k++) {
    const n = 2 + Math.floor(rnd() * 7);
    const list = Array.from({ length: n }, () => ({ ...pick(pool) }));
    add('ordemParLista', [list], fmt.ordemPar(list).map((p) => list.indexOf(p)));
  }

  // valorPar: cada parâmetro, em cada combinação de Artes que o cartão pode passar
  const artSets = [[], ['fogo'], ['terra'], ['fogo', 'terra'], ['agua', 'terra', 'vento']];
  for (const e of effects) {
    for (const p of e.parametros) {
      for (const as of artSets) {
        once(`valor:${JSON.stringify(p)}:${as}`, () => add('valorPar', [p, as], fmt.valorPar(p, as) ?? null));
      }
    }
  }
  // e os que o catálogo não tem: régua sem tipo, nota vazia, unidade vazia, Cura e Dano em outras caixas
  const extra = [
    { nome: 'Duração', tipo: 'padrao', regua: 'longa' }, { nome: 'Duração', tipo: 'padrao', regua: 'breve' },
    { nome: 'Duração', tipo: 'padrao', regua: 'manifestacao' }, { nome: 'Duração', tipo: 'padrao', regua: '' },
    { nome: 'Cura', tipo: 'padrao' }, { nome: 'CURA total', tipo: 'padrao' }, { nome: 'cura', tipo: 'fixo', valor: '4' },
    { nome: 'Dano', tipo: 'padrao' }, { nome: 'dano de gelo', tipo: 'padrao' }, { nome: 'Danos', tipo: 'padrao' },
    { nome: 'Volume', tipo: 'substitui', substitui: 'area', escala: ['1', '2', '3'] },
    { nome: 'Volume', tipo: 'substitui', substitui: 'area', escala: ['1', '2'], unidade: '' },
    { nome: 'Volume', tipo: 'substitui', substitui: 'area', escala: ['1', '2'], unidade: 'm³' },
    { nome: 'Volume', tipo: 'substitui', substitui: 'area', escala: ['1'], nota: '' },
    { nome: 'Volume', tipo: 'substitui', substitui: 'area', escala: ['1'], nota: 'a nota', unidade: 'm' },
    { nome: 'Alcance', tipo: 'fixo', valor: 'Toque' }, { nome: 'Alcance', tipo: 'fixo', valor: '' },
    { nome: 'Alvos', tipo: 'padrao', nota: 'tem nota' }, { nome: 'Alvos', tipo: 'padrao' },
  ];
  for (const p of extra) for (const as of artSets) once(`valor:${JSON.stringify(p)}:${as}`, () => add('valorPar', [p, as], fmt.valorPar(p, as) ?? null));

  // ---- o custo da conjuração ----
  const arte = arts[0];
  const cost = (efeito, nivel, escolhas, centelha) => {
    const c = grid.custoDe(efeito, arte, nivel, escolhas, centelha);
    return {
      base: c.base, parametros: c.parametros,
      esticados: c.esticados.map((x) => ({ nome: x.nome, acima: x.acima, custo: x.custo })),
      total: c.total, centelha: c.centelha, mana: c.mana,
    };
  };
  const ajustaveis = (e) => e.parametros.filter((p) => p.tipo !== 'fixo');
  for (const e of effects) {
    const pars = ajustaveis(e);
    for (const nivel of [0, 2, 4]) {
      const patterns = [
        {},
        ...pars.slice(0, 3).map((p) => ({ [p.nome]: 1 })),
        ...pars.slice(0, 3).map((p) => ({ [p.nome]: nivel + 2 })),
        Object.fromEntries(pars.map((p, i) => [p.nome, (i % 3) + 1])),
        { ...Object.fromEntries(pars.map((p) => [p.nome, 2])), 'Inexistente': 5 },
      ];
      for (const escolhas of patterns) {
        for (const centelha of [0, 3]) add('custoDe', [e.id, nivel, escolhas, centelha], cost(e, nivel, escolhas, centelha));
      }
    }
  }
  // os Efeitos com um parâmetro fixo que cobra Mana (`custaMana`) em cada nível da Arte, e o negativo
  for (const e of effects.filter((x) => x.parametros.some((p) => p.custaMana))) {
    for (const nivel of [-1, 0, 1, 3, 6]) add('custoDe', [e.id, nivel, {}, 0], cost(e, nivel, {}, 0));
  }
  // o improviso (sem Efeito): cada parâmetro dele, esticado ou não, e a Cura não está na lista
  const improviso = ['Alcance', 'Volume', 'Alvos', 'Dano', 'Duração', 'Cura', 'Inexistente'];
  for (const nivel of [0, 1, 3, 6]) {
    for (const nome of improviso) {
      for (const n of [-2, 0, 1, 3, 7]) add('custoDe', [null, nivel, { [nome]: n }, 0], cost(null, nivel, { [nome]: n }, 0));
    }
    const tudo = Object.fromEntries(improviso.map((n, i) => [n, i + 1]));
    for (const centelha of [0, 2, 9]) add('custoDe', [null, nivel, tudo, centelha], cost(null, nivel, tudo, centelha));
  }
  // a Cura por nível: o Efeito que cura, escolhendo a Cura
  for (const e of effects.filter((x) => ajustaveis(x).some((p) => p.nome === 'Cura'))) {
    for (const nivel of [1, 3]) for (const n of [1, 3, 5]) add('custoDe', [e.id, nivel, { Cura: n }, 0], cost(e, nivel, { Cura: n }, 0));
  }

  // ---- a Mana paga, com e sem a fonte do elemento ----
  const desconto = regras.arcano.fonteElemental.desconto;
  for (let pontos = 0; pontos <= 12; pontos++) {
    for (let centelha = 0; centelha <= 6; centelha++) {
      for (const fonte of [false, true]) {
        // `Alcance: pontos` com a Arte no nível 12 custa exatamente `pontos` (sem esticar)
        const c = grid.custoDe(null, arte, 12, { Alcance: pontos }, centelha + (fonte ? desconto : 0));
        add('manaPaga', [pontos, centelha, fonte], c.mana);
      }
    }
  }
  return cases;
}
