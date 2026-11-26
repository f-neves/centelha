// strikes.mjs · o lance e os dados na referência de paridade.
//
// Usado por gen-references.mjs, com um pacote só de acaso.ts, rolagem.ts e
// lance.ts (o estado do acaso é do pacote: semear um acaso carregado à parte
// não semeia o rolarExpr de dentro do lance). Grava:
//
//   fixed    os lances de scripts/fixtures/lances.jsonl da fonte, cada um pelo
//            resolverGolpe com a fonteFixa dos dados que a mesa gravou
//   rolled   os mesmos lances, em ordem, pelo resolverGolpe com a fonteRolada
//            e um Mulberry32 só (semente ROLLED_SEED), com quantos números o
//            gerador já tinha entregado ao fim de cada lance
//   rolls    rolarExpr de expressões reais e de casos de borda, com ajustes,
//            por um Mulberry32 só (semente ROLLS_SEED), com a contagem
//
// A entrada de cada lance é cortada nos campos que o resolverGolpe lê, e é
// essa entrada cortada que o TypeScript recebe.

export const ROLLED_SEED = 20260903;
export const ROLLS_SEED = 771107;

const pick = (o, keys) => Object.fromEntries(keys.filter((k) => o[k] !== undefined).map((k) => [k, o[k]]));

function trim(e) {
  return {
    aid: e.aid,
    atacante: pick(e.atacante, ['ataque', 'dano', 'ajusteFlat', 'ajusteDados', 'penDados', 'danoDados', 'danoFlat']),
    alvo: pick(e.alvo, ['defesaBase', 'ferimento', 'condicoesDefesa', 'defesaPerdida', 'soak', 'pv']),
    ...pick(e, ['golpeIndice', 'tipoDano', 'modManual', 'margemQA', 'danoQA']),
  };
}

export function buildStrikes({ combat, fixtureText }) {
  const lines = fixtureText.split('\n').filter((l) => l.trim());
  const entries = lines.map((l) => JSON.parse(l));
  const clone = (x) => structuredClone(x);

  const fixed = entries.map((x) => {
    const entrada = trim(x.entrada);
    const sorteio = { acerto: x.sorteio.acerto, dano: x.sorteio.dano };
    return { entrada, sorteio, saida: combat.resolverGolpe(clone(entrada), combat.fonteFixa(clone(sorteio))) };
  });

  // um contador por fora do gerador: quantos números ele já entregou
  let draws = 0;
  const counted = (seed) => {
    const next = combat.semeadoDe(seed);
    draws = 0;
    return () => { draws++; return next(); };
  };

  combat.semear(counted(ROLLED_SEED));
  const rolled = fixed.map((f) => ({ saida: combat.resolverGolpe(clone(f.entrada), combat.fonteRolada), draws }));

  const exprs = new Set();
  for (const f of fixed) { exprs.add(f.entrada.atacante.ataque); exprs.add(f.entrada.atacante.dano); }
  for (const e of ['', '0d6', 'd6', 'D6', '2D6+1', '3d6+2 +5', '1d6 \u22122 (I)', '2d6 \u2013 1', '1d6 \u2014 3', '(2d6) +4', '1d6 (+9', '+ 7',
    '- 2', '10d6', '2d6-1d6', 'x', '1d66', '12d6+3-2', '+-3', '1d6 + 2 (P) + 1']) exprs.add(e);
  combat.semear(counted(ROLLS_SEED));
  const rolls = [];
  for (const expr of [...exprs].sort()) {
    for (const extraDados of [0, -1, -3, 2, 0.5]) {
      for (const extraFlat of [0, 3, -2]) {
        const r = combat.rolarExpr(expr, extraDados, extraFlat);
        rolls.push({ expr, extraDados, extraFlat, ...r, draws });
      }
    }
  }
  combat.semear(null);
  return { fixed, rolled, rolls };
}
