// timeline.mjs · a régua, a agenda e a fila do tempo do combate na referência
// de paridade (combate-tempo.ts).
//
// Usado por gen-references.mjs. Cada caso é { fn, args, out }, como em
// character.mjs: o nome da função do TypeScript, os argumentos e o que ela
// devolveu. Nos argumentos, `null` no lugar de um parâmetro opcional quer dizer
// "não passado" (undefined). O caso em que o TypeScript lança erro não entra, e
// o caso cuja saída tem NaN ou infinito também não (o JSON não guarda os dois).
//
// Nenhuma destas funções sorteia: elas não leem o acaso. As ações sorteadas
// abaixo saem de um Mulberry32 próprio do gerador (semente SAMPLE_SEED), só
// para variar a entrada; o Rust lê a entrada gravada e não refaz o sorteio.
//
// O desempate da fila. Na mesa e no laço do espelho, o quarto critério do
// `ordemDaFila` (a `chegada`) é o id da peça desde 03/09/2026 (`naOrdem` no
// grid.astro e no scripts/sim/motor.mjs); o carimbo `movido_em` saiu da fila e
// ficou só no desenho da pilha de peças na mesma casa. Então não há relógio de
// movimento a fabricar: a chegada entra como texto (o id). O quinto critério é
// o nome. Os dois são comparados por `localeCompare`; o Rust compara texto por
// ponto de código. As duas ordens coincidem nos textos daqui (letras minúsculas
// sem acento, dígitos), e o gerador confere isso caso a caso e para se não
// coincidir.
//
// Grupos:
//   regua    fisicaDe, armaDoCatalogo, classeDaArma, classeDeTempo,
//            velocidadeDaArma, preparoDe, reguaDaArte, tetoDaRajada,
//            cicloExtraDaDupla, anatomia, comOverride, anatomiaLivre
//   entrada  ticksDeEntrada, contrapeEm, contrapeDe, contrapeAcaba,
//            faseDeQuemVaiAgir
//   fila     ordemDaFila (o sinal) e a fila ordenada (o sort estável do
//            JavaScript com o comparador, como o naFila do laço)
//   agenda   acaoVazia, temGesto, declarar, golpeDaAgenda, penDadosDaRegua,
//            agendar, golpesNoAr, proximoGolpe, golpeDevido, golpeResolvido,
//            faseEm, golpesDados, defesaPerdida, podeSerInterrompido
//   mesa     adiaGolpe, ehSimultaneo

export const SAMPLE_SEED = 20261126;

const SYSTEMS = ['normal', 'pgr', 'simultaneo'];
const CLASSES = ['leve', 'media', 'haste', 'pesada', 'distancia', 'arremesso', 'arte', 'haste-media', 'haste-guerra'];

function finite(v) {
  if (typeof v === 'number') return Number.isFinite(v);
  if (Array.isArray(v)) return v.every(finite);
  if (v && typeof v === 'object') return Object.values(v).every(finite);
  return true;
}

// o texto em ordem de ponto de código, como o Rust compara
const byCodePoint = (a, b) => {
  const x = [...a].map((c) => c.codePointAt(0));
  const y = [...b].map((c) => c.codePointAt(0));
  for (let i = 0; i < Math.min(x.length, y.length); i++) if (x[i] !== y[i]) return x[i] < y[i] ? -1 : 1;
  return Math.sign(x.length - y.length);
};

export function buildTimelineCases({ tempo, acaso, weapons }) {
  const cases = [];
  let dropped = 0;
  const und = (a) => (a === null ? undefined : a);
  const add = (fn, args, call) => {
    let out;
    try {
      out = call(...args.map((a) => structuredClone(a)));
    } catch {
      return;
    }
    if (out === undefined) out = null;
    if (!finite(out)) { dropped++; return; }
    cases.push({ fn, args, out });
  };
  const rnd = acaso.semeadoDe(SAMPLE_SEED);
  const pick = (xs) => xs[Math.floor(rnd() * xs.length)];
  const int = (lo, hi) => lo + Math.floor(rnd() * (hi - lo + 1));

  // ----- regua -----
  for (const s of SYSTEMS) add('fisicaDe', [s], tempo.fisicaDe);
  const names = [null, '', 'inexistente', 'Garras', 'Mordida', 'espada longa', 'ESPADA-LONGA'];
  for (const w of weapons) names.push(w.id, w.nome);
  for (const n of names) {
    add('armaDoCatalogo', [n], (x) => tempo.armaDoCatalogo(und(x))?.id ?? null);
    add('classeDaArma', [n], (x) => tempo.classeDaArma(und(x)));
    for (const p of [null, 3, 0, 7.5]) {
      add('velocidadeDaArma', [n, p], (x, y) => (y === null ? tempo.velocidadeDaArma(und(x)) : tempo.velocidadeDaArma(und(x), y)));
    }
  }
  const explicit = [null, '', 'leve', 'media', 'haste', 'pesada', 'distancia', 'arremesso', 'arte', 'haste-media', 'xyz'];
  for (const arma of [null, '', 'espada-longa', 'Montante', 'adaga', 'arco-curto', 'Garras', 'Mordida']) {
    for (const v of [null, -1, 1, 4, 5, 5.5, 6, 6.5, 7, 9]) {
      for (const e of explicit) add('classeDeTempo', [arma, v, e], (a, b, c) => tempo.classeDeTempo(und(a), und(b), und(c)));
    }
  }
  const speeds = [-2, 0, 0.5, 1, 2, 3, 4, 5, 5.5, 6, 7, 8, 10, 15];
  for (const c of CLASSES) {
    for (const s of SYSTEMS) {
      for (const v of speeds) add('preparoDe', [c, v, s], tempo.preparoDe);
      add('cicloExtraDaDupla', [c, s], tempo.cicloExtraDaDupla);
    }
    add('tetoDaRajada', [c], tempo.tetoDaRajada);
  }
  for (const s of SYSTEMS) for (const n of [-1, 0, 1, 2, 3, 4, 5, 10, 1.5]) add('reguaDaArte', [n, s], tempo.reguaDaArte);
  const anatomyOpts = [];
  for (const c of CLASSES) {
    for (const s of SYSTEMS) {
      for (const v of [null, -2, 0, 1, 2, 3, 5, 6, 7, 10, 5.5]) {
        for (const m of [null, 'simples', 'segura', 'dupla']) anatomyOpts.push({ classe: c, sistema: s, velocidade: v, manobra: m, golpes: null });
        for (const g of [null, -1, 0, 1, 2, 3, 4, 2.5]) anatomyOpts.push({ classe: c, sistema: s, velocidade: v, manobra: 'rajada', golpes: g });
      }
    }
  }
  const optsOf = (o) => Object.fromEntries(Object.entries(o).filter(([, v]) => v !== null));
  const anatomies = [];
  for (const o of anatomyOpts) {
    add('anatomia', [o], (x) => tempo.anatomia(optsOf(x)));
    anatomies.push(tempo.anatomia(optsOf(o)));
  }
  // uma amostra das anatomias para o resto (fixa: o índice sai do sorteio do gerador)
  const sample = Array.from({ length: 60 }, () => pick(anatomies));
  const overrides = [null, {}, { preparo: 3 }, { golpes: 2 }, { recuperacao: 0 }, { preparo: -1, golpes: 0, recuperacao: -2 },
    { preparo: 1, golpes: 3, recuperacao: 2 }, { golpes: 2.5 }, { preparo: null, golpes: 4 }, { preparo: 0, recuperacao: 5 }];
  for (const a of sample.slice(0, 25)) for (const p of overrides) add('comOverride', [a, p], (x, y) => tempo.comOverride(x, und(y)));
  for (const s of SYSTEMS) {
    for (const v of [-3, 0, 0.4, 1, 2, 2.4, 2.5, 3, 5, 7]) for (const q of ['agora', 'fim']) add('anatomiaLivre', [v, q, s], tempo.anatomiaLivre);
  }

  // ----- entrada -----
  const lists = [[], [10], [10, 10], [12, 5, 18, 0], [9, 3, 15, 16, 2], [7.5, 1], [-0, 0, 4], [2, 18]];
  for (let i = 0; i < 30; i++) lists.push(Array.from({ length: int(1, 8) }, () => int(2, 18)));
  for (const l of lists) add('ticksDeEntrada', [l], tempo.ticksDeEntrada);
  const feet = [null, {}];
  for (const n of [0, -1, -2, -3, 1]) for (const d of [null, 0, 2, 3]) feet.push(d === null ? { contrape: n } : { contrape: n, contrapeDesde: d });
  for (const a of feet) {
    add('contrapeDe', [a], (x) => tempo.contrapeDe(und(x)));
    add('contrapeAcaba', [a], (x) => tempo.contrapeAcaba(und(x)));
    for (let t = 0; t <= 8; t++) add('contrapeEm', [a, t], (x, y) => tempo.contrapeEm(und(x), y));
  }
  for (let ta = 0; ta <= 6; ta++) {
    for (const p of [-1, 0, 1, 2, 3]) for (let tg = 0; tg <= 8; tg++) add('faseDeQuemVaiAgir', [ta, p, tg], tempo.faseDeQuemVaiAgir);
  }

  // ----- fila -----
  const keyOf = () => {
    const k = {};
    const t = pick([undefined, null, 1, 2, 3]); if (t !== undefined) k.tick = t;
    const i = pick([undefined, null, 5, 9, 12]); if (i !== undefined) k.iniciativa = i;
    const r = pick([undefined, null, 0, 1, 3]); if (r !== undefined) k.raciocinio = r;
    const c = pick([undefined, null, '', 'a0', 'a1', 'b0', 'b10', 'b2']); if (c !== undefined) k.chegada = c;
    const n = pick([undefined, null, '', 'escudeiro', 'montanteiro', 'fatal']); if (n !== undefined) k.nome = n;
    return k;
  };
  const checkText = (a, b) => {
    for (const f of ['chegada', 'nome']) {
      const x = String(a[f] || ''); const y = String(b[f] || '');
      if (Math.sign(x.localeCompare(y)) !== byCodePoint(x, y)) throw new Error(`a ordem do texto difere: ${x} e ${y}`);
    }
  };
  for (let i = 0; i < 400; i++) {
    const a = keyOf(); const b = keyOf();
    checkText(a, b);
    add('ordemDaFila', [a, b], (x, y) => Math.sign(tempo.ordemDaFila(x, y)));
  }
  for (let i = 0; i < 80; i++) {
    const n = int(2, 8);
    // poucas opções por campo, para haver empate inteiro (a ordem de entrada decide)
    const pieces = Array.from({ length: n }, () => ({
      tick: pick([1, 2]), iniciativa: pick([9, 9, 12]), raciocinio: pick([1, 3]),
      chegada: pick(['a0', 'a1', 'b0']), nome: pick(['escudeiro', 'fatal']),
    }));
    for (let x = 0; x < n; x++) for (let y = 0; y < n; y++) checkText(pieces[x], pieces[y]);
    add('fila', [pieces], (ps) => ps.map((p, j) => ({ p, j })).sort((x, y) => tempo.ordemDaFila(x.p, y.p)).map((e) => e.j));
  }

  // ----- agenda -----
  const shapes = [null, {}, { golpes: [] }, { golpes: [3] }, { pressao: 0 }, { pressao: 2 }, { pressao: -1 },
    { interpoe: { aid: 'x1', golpe: 4, alvoOriginal: 'b0' } }, { interpoe: null }, { golpes: [], pressao: 1 },
    { contrape: -1, contrapeDesde: 2 }, { golpes: [2, 3], pressao: 0, interpoe: null }];
  for (const a of shapes) {
    add('acaoVazia', [a], (x) => tempo.acaoVazia(und(x)));
    add('temGesto', [a], (x) => tempo.temGesto(und(x)));
  }
  for (const a of sample.slice(0, 40)) {
    for (const t of [0, 1, 3, 7]) add('declarar', [t, a], tempo.declarar);
    for (const i of [0, 1, 2, 3, -1]) add('penDadosDaRegua', [a, i], tempo.penDadosDaRegua);
  }
  const actions = [];
  for (const a of sample.slice(0, 25)) {
    const t = pick([1, 2, 4]);
    const base = tempo.declarar(t, a);
    actions.push(base);
    actions.push(tempo.agendar(base, t));
    actions.push({ ...base, pressao: pick([1, 3]) });
    actions.push({ ...tempo.agendar(base, t), pressao: 2, tipo: 'rajada', aid: 'a1' });
  }
  actions.push({ golpes: [5, 3], livre: 8, pressao: 0 }, { golpes: [4], livre: 4 }, { golpes: [2, 2], livre: 6, aResolver: [] },
    { golpes: [3], pressao: 1 }, { golpes: [3, 4, 5], livre: 9, aResolver: [3, 5] }, { pressao: 2 });
  const guards = [null, {}, { segura: true }, { fase: 'preparo' }, { fase: 'golpe', segura: true }, { fase: 'golpe' },
    { fase: 'recuperacao' }, { fase: 'livre' }];
  // a escada inteira (todas as opções) numa ação em três; nas outras, sem opção
  for (const [k, a] of [null, ...actions].entries()) {
    add('golpesNoAr', [a], (x) => tempo.golpesNoAr(und(x)));
    add('proximoGolpe', [a], (x) => tempo.proximoGolpe(und(x)));
    const top = a ? Math.max(a.livre ?? 0, ...(a.golpes || [0])) : 3;
    for (let t = (a?.desde ?? 1) - 1; t <= top + 1; t++) {
      add('golpeDaAgenda', [a, t], (x, y) => tempo.golpeDaAgenda(und(x), y));
      add('golpeDevido', [a, t], (x, y) => tempo.golpeDevido(und(x), y));
      add('faseEm', [a, t], (x, y) => tempo.faseEm(und(x), y));
      add('golpesDados', [a, t], (x, y) => tempo.golpesDados(und(x), y));
      if (k % 3 === 0) add('podeSerInterrompido', [a, t], (x, y) => tempo.podeSerInterrompido(und(x), y));
      if (a) {
        add('agendar', [a, t], tempo.agendar);
        add('golpeResolvido', [a, t], tempo.golpeResolvido);
      }
      for (const g of (k % 3 === 0 ? guards : [null])) add('defesaPerdida', [a, t, g], (x, y, z) => (z === null ? tempo.defesaPerdida(und(x), y) : tempo.defesaPerdida(und(x), y, z)));
    }
  }

  // ----- mesa -----
  for (const c of [null, {}, { sistema: 'normal' }, { sistema: 'pgr' }, { sistema: 'pgr', golpeAdiado: true },
    { sistema: 'simultaneo' }, { sistema: 'normal', golpeAdiado: true }, { sistema: 'simultaneo', golpeAdiado: false }]) {
    add('adiaGolpe', [c], (x) => tempo.adiaGolpe(und(x)));
    add('ehSimultaneo', [c], (x) => tempo.ehSimultaneo(und(x)));
  }

  return { cases, dropped };
}
