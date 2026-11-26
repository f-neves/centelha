// bestiary.mjs · as contas que o card do bestiário faz na hora, na referência
// de paridade.
//
// Usado por gen-references.mjs. Cada caso é { fn, args, out }; `null` num
// argumento é o `undefined` do TypeScript.
//
//   linhaPVNaZona       src/lib/motor.ts: a entrada de cada card do
//                       monsters.json (e de cada forma), como o card a monta
//                       (massaDaConta do corpo, o Vigor e a linha escolhida),
//                       mais uma grade em volta de cada zona da tabela
//   massaDaConta        src/lib/motor.ts: o corpo de cada card, e o incorpóreo
//   poolComPenalidade   src/lib/defesa-vestida.ts: cada pool do monsters.json
//                       e pools de borda, com Penalidade de 0 a 5
//   apMod               a curva da Aparência do próprio card
//                       (src/components/BestaCard.astro), recortada do arquivo
//                       da fonte e rodada como está; de -3 a 14 e frações
//
// O card em si não tem referência aqui: o teste compara o Rust com o
// content/data/monsters.json, que o gen-monsters.mjs da fonte escreveu no
// mesmo commit.

import fs from 'node:fs';

export function loadCardAppearance(file) {
  const text = fs.readFileSync(file, 'utf8');
  const curve = text.match(/\nconst AP_CURVA: Record<number, number> = (\{[^\n]*\});\n/);
  const fn = text.match(/\nconst apMod = \(n: number\) => ([^\n]*);\n/);
  if (!curve || !fn) throw new Error(`não achei a curva da Aparência em ${file}`);
  // eslint-disable-next-line no-new-func
  return new Function('n', `const AP_CURVA = ${curve[1]}; return ${fn[1]};`);
}

export function buildBestiaryCases({ motor, vestida, regras, monsters, apMod }) {
  const cases = [];
  const und = (a) => (a === null ? undefined : a);
  const add = (fn, args, out) => cases.push({ fn, args, out: out === undefined ? null : out });

  const cards = [];
  for (const m of monsters) {
    cards.push(m);
    for (const f of m.formas || []) cards.push(f.criatura);
  }
  for (const i of cards) {
    if (!i.corpo) continue;
    const kg = motor.massaDaConta(i.corpo);
    add('massaDaConta', [i.corpo], kg);
    const args = [kg ?? null, i.atributos?.vigor ?? 0, i.corpo.linhaPV ?? null];
    add('linhaPVNaZona', args, motor.linhaPVNaZona(und(args[0]), args[1], und(args[2])));
  }
  for (const corpo of [{ semMassa: true, volume: 2 }, { semMassa: true }, { massa: 0 }, {}]) {
    add('massaDaConta', [corpo], motor.massaDaConta(corpo));
  }
  for (const row of regras.derivados.pv.massa.conta) {
    if (!row.zonaKg) continue;
    const [a, b] = row.zonaKg;
    for (const kg of [a * 0.9, a, (a + b) / 2, b, b * 1.1]) {
      for (const vigor of [0, 3, 7, 12]) {
        for (const linha of [null, 'acima', 'abaixo']) add('linhaPVNaZona', [kg, vigor, linha], motor.linhaPVNaZona(kg, vigor, und(linha)));
      }
    }
  }
  add('linhaPVNaZona', [null, 3, 'acima'], motor.linhaPVNaZona(undefined, 3, 'acima'));

  const pools = new Set(['', 'x', ' 3d6+2 +5 ', '4d6 −1', '4d6 - 1', '2d6+2', '2d6+2+2', '2d6+21', '1d6 +', '3d6+2 +0', '0d6']);
  for (const i of cards) for (const a of i.combate?.ataques || []) if (a.pool) pools.add(a.pool);
  for (const p of [...pools].sort()) {
    for (const pen of [0, 1, 2, 3, 5]) add('poolComPenalidade', [p, pen], vestida.poolComPenalidade(p, pen));
  }

  for (let n = -3; n <= 14; n++) add('apMod', [n], apMod(n));
  for (const n of [0.5, 2.5, 10.5]) add('apMod', [n], apMod(n));
  return cases;
}
