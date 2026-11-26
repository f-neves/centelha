// character.mjs · as contas pequenas do personagem na referência de paridade.
//
// Usado por gen-references.mjs. Cada caso é { fn, args, out }: o nome da função
// do TypeScript, os argumentos e o que ela devolveu. As entradas são varridas
// em grades fixas (sem nada aleatório): rodar de novo dá o mesmo lote.
//
// Nos argumentos, `null` no lugar de um parâmetro opcional quer dizer "não
// passado" (undefined), salvo onde o TypeScript trata os dois igual (`== null`).
// O caso em que o TypeScript quebra (lança erro) não entra.
//
// Grupos:
//   xp          pisoXp, precoNivel, custoPontos e os custos nomeados, em cada
//               trilha de regras.json → xp
//   pequenas    aparenciaMod, soakNatural, soak, gatePerfuracaoAbre,
//               limiteDaMorte, passouDoLimite, porteDeRotulo, modificadorPorte,
//               levantamentoDaArte, jogadaDaManobra
//   quase       quaseAcertoDoEncontro, quaseAcerto, errouPor, raspa,
//               saidaDoAtaque
//   textos      sinalNum, danoStr, statsArma, statsEscudo, statsArmadura
//   migra       migrarRefsDeArma

export function buildCharacterCases({ motor, quase, equip, migra, regras, weapons }) {
  const cases = [];
  const und = (a) => (a === null ? undefined : a);
  const add = (fn, args, call) => {
    let out;
    try {
      out = call(...args.map((a) => structuredClone(a)));
    } catch {
      return;
    }
    cases.push({ fn, args, out: out === undefined ? null : out });
  };

  // ----- xp -----
  const keys = Object.keys(regras.xp).filter((k) => k !== 'modelo');
  const levels = [-1, -0, 0, 1, 2, 3, 6, 7, 12, 2.5];
  for (const k of keys) {
    add('pisoXp', [k], motor.pisoXp);
    for (const n of levels) add('precoNivel', [k, n], motor.precoNivel);
    for (const de of [null, -1, 0, 1, 2, 5, 1.5]) {
      for (const ate of [null, -1, 0, 1, 2, 3, 6, 9, 3.5]) {
        add('custoPontos', [k, de, ate], (a, b, c) => motor.custoPontos(a, und(b), und(c)));
      }
    }
  }
  for (const n of levels) {
    add('custoTecnica', [n], motor.custoTecnica);
    add('custoArte', [n], motor.custoArte);
    add('custoEfeito', [n], motor.custoEfeito);
    for (const sec of [null, false, true]) {
      add('custoEspecialidade', [n, sec], (a, b) => motor.custoEspecialidade(a, und(b)));
    }
  }

  // ----- pequenas -----
  for (let n = -2; n <= 14; n++) {
    for (const m of [null, 0, 1, 2, 3, 6, 10, -2, 1.5]) {
      add('aparenciaMod', [n, m], (a, b) => motor.aparenciaMod(a, und(b)));
    }
  }
  for (const n of [2.5, -0]) add('aparenciaMod', [n, 1], motor.aparenciaMod);
  for (const v of [0, 3, -1, 2.5]) {
    for (const cat of ['impacto', 'corte', 'perfurante', 'perfuracao', 'outro']) add('soakNatural', [v, cat], motor.soakNatural);
  }
  for (const vigor of [0, 2, 5]) {
    for (const centelha of [0, 1, 3]) {
      for (const modo of ['corte', 'perfurante', 'impacto']) {
        for (const armaduraSoak of [null, 0, 4]) {
          add('soak', [vigor, centelha, modo, armaduraSoak], (a, b, c, d) =>
            motor.soak({ vigor: a, centelha: b, modo: c, ...(d === null ? {} : { armaduraSoak: d }) }));
        }
      }
    }
  }
  for (const modo of ['corte', 'perfurante', 'impacto']) {
    for (const perf of [0, 1, 2, 3]) for (const resist of [0, 1, 2, 3]) add('gatePerfuracaoAbre', [modo, perf, resist], motor.gatePerfuracaoAbre);
  }
  for (const pv of [null, 0, -3, 1, 2, 7, 25, 26, 33.5]) {
    for (const c of [null, 0, 1, 3, -1]) add('limiteDaMorte', [pv, c], motor.limiteDaMorte);
  }
  for (const vida of [null, 0, -3, -4, -13, -14, 5]) {
    for (const pv of [null, 7, 26, 27]) for (const c of [null, 0, 2]) add('passouDoLimite', [vida, pv, c], motor.passouDoLimite);
  }
  for (const r of ['Miúdo', 'Minúsculo', 'minusculo', 'MÉDIO', 'Médio', 'médio', 'Grande', 'ENORME', 'Imenso', 'Colossal',
    'pequeno', 'Pequena', '', null, 'xyz', 'Mİudo', 'gigante', 'Grandé', ' medio']) {
    add('porteDeRotulo', [r], motor.porteDeRotulo);
  }
  const portes = regras.porteAcerto.ordem;
  for (const a of portes) for (const b of portes) for (const d of [false, true]) add('modificadorPorte', [a, b, d], motor.modificadorPorte);
  for (const n of [0, 1, 2, 3, 4, 5, 6, 7, 0.5, 0.7, -1, 10, 0.64, 2.5]) add('levantamentoDaArte', [n], motor.levantamentoDaArte);
  for (const regra of ['agarrar', 'derrubar']) {
    for (const forca of [1, 3, 4, null]) {
      for (const destreza of [2, 3, 5]) {
        for (const briga of [0, 1, 2, 4, null]) {
          for (const centelha of [0, 1, 3, null]) {
            const c = { atributos: {}, habilidades: {}, centelha: und(centelha) };
            if (forca !== null) c.atributos.forca = forca;
            c.atributos.destreza = destreza;
            if (briga !== null) c.habilidades.briga = briga;
            add('jogadaDaManobra', [regra, c], motor.jogadaDaManobra);
          }
        }
      }
    }
  }
  for (const regra of ['investida', null, 'rajada']) {
    add('jogadaDaManobra', [regra, { atributos: { forca: 3 }, habilidades: {}, centelha: 1 }], (a, b) => motor.jogadaDaManobra(und(a), b));
  }

  // ----- quase -----
  for (const ab of [0, 1, 2]) for (const ad of [0, 1, 3]) for (const ac of [0, 2]) {
    for (const tb of [0, 1]) for (const tr of [0, 2]) for (const tc of [0, 1]) {
      add('quaseAcertoDoEncontro', [{ atacante: { qaArmaBonus: ab, qaArmaDano: ad, centelha: ac },
        alvo: { qaArmaduraBonus: tb, qaArmaduraReducao: tr, centelha: tc } }], quase.quaseAcertoDoEncontro);
    }
  }
  const named = weapons.filter((w) => w.arma).map((w) => w.nome).slice(0, 4);
  const armorSets = [null, [], [{ classe: 'leve' }], [{ classe: 'pesada' }, { classe: 'media' }], [{}, null, { classe: 'nao-existe' }],
    [{ classe: 'media' }, { classe: 'media' }]];
  for (const arma of [null, 'adaga', 'espada-longa', 'montante', 'nao-existe', 'desarmado', ...named]) {
    for (const dano of [null, '1d6', '2d6+3', '3d6 -1', 'x', '']) {
      for (const centelha of [null, 0, 2]) {
        for (const armaduras of armorSets) {
          add('quaseAcerto', [{ arma, dano, centelha }, { armaduras }], (a, b) =>
            quase.quaseAcerto({ arma: und(a.arma), dano: und(a.dano), centelha: und(a.centelha) }, { armaduras: und(b.armaduras) }));
        }
      }
    }
  }
  for (const w of weapons) add('quaseAcerto', [{ arma: w.id, dano: null, centelha: 1 }, { armaduras: [{ classe: 'leve' }] }], (a, b) =>
    quase.quaseAcerto({ arma: a.arma, centelha: a.centelha }, b));
  for (let total = 4; total <= 14; total++) {
    for (const def of [8, 10, 12]) {
      add('errouPor', [total, def], quase.errouPor);
      for (const margem of [0, 1, 2, 3]) add('saidaDoAtaque', [total, def, margem], quase.saidaDoAtaque);
    }
  }
  for (const f of [-1, 0, 1, 2, 3, 4]) for (const m of [0, 1, 3]) add('raspa', [f, m], quase.raspa);

  // ----- textos -----
  for (const n of [0, -0, 1, -1, 2.5, -3.25, 12, 1e21, 1e-7]) add('sinalNum', [n], equip.sinalNum);
  const flatWeapons = Object.values(equip.ARMA);
  const odd = [{}, { ticks: 4, acerto: '2', dado: 'x', danoBonus: -0.5, defesaArma: true }, { ticks: null, acerto: -1, dado: 0, danoBonus: '3' },
    { ticks: 3.5, acerto: 'abc', dado: [2], danoBonus: [], defesaArma: {} }];
  for (const w of [...flatWeapons, ...odd]) {
    add('danoStr', [w], equip.danoStr);
    add('statsArma', [w], equip.statsArma);
  }
  for (const s of [...Object.values(equip.ESCUDO), {}, { bloqCaC: -2, penalidade: '1' }, { bloqCaC: '3', penalidade: -1 }]) {
    add('statsEscudo', [s], equip.statsEscudo);
  }
  for (const a of [...Object.values(equip.ARMADURA), { soak: {} }, { soak: { impacto: '2', corte: null, perfuracao: 1.5 }, resistPerf: '2', penalidade: 3 },
    { soak: { impacto: 0, corte: 0, perfuracao: 0 }, resistPerf: 0, penalidade: -1 }]) {
    add('statsArmadura', [a], equip.statsArmadura);
  }

  // ----- migra -----
  const sheets = [
    { equip: { arma: 'dardos' } },
    { equip: { arma: 'glaive-serrilhada', armaduras: [] }, conjuntos: [{ habil: { ref: 'a:dardos', mod: { dado: 2 } }, inabil: { ref: 'a:glaive-serrilhada' } }] },
    { conjuntos: [{ habil: { ref: 'e:dardos' }, inabil: { ref: 'a:adaga' } }, null, { habil: null }, { habil: { ref: 7 } }, { habil: 'a:dardos' }, []] },
    { arsenal: [{ ref: 'a:dardos', item: { origem: 'dardos' } }, null, { ref: '' }, { ref: 'a:' }, { ref: 'c', nome: 'dardos' }, 'a:dardos'] },
    { equip: { arma: 5 }, conjuntos: 'x', arsenal: {} },
    { equip: null, conjuntos: [{ inabil: { ref: 'a:glaive-serrilhada', img: 'x.webp' } }] },
    {},
  ];
  for (const s of sheets) {
    add('migrarRefsDeArma', [s, null], (a) => migra.migrarRefsDeArma(a));
    add('migrarRefsDeArma', [s, [['dardos', 'a'], ['adaga', 'faca'], ['dardos', 'b']]], migra.migrarRefsDeArma);
  }

  return cases;
}
