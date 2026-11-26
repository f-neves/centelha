// sheets.mjs · o lote de fichas de jogador da referência de paridade.
//
// Usado por gen-references.mjs. Monta as fichas de entrada a partir da ficha de
// fixação da fonte (scripts/fixtures/kael.json) e de variações, sem nada
// aleatório fora do Mulberry32 com semente fixa: rodar de novo dá o mesmo lote.
//
// Como o lote é escolhido (cada grupo leva o nome no campo `grupo`):
//   kael          a ficha de fixação como está, e gravada (fichaParaGravar)
//   formato       os formatos antigos que a leitura aceita: armaduras como
//                 lista de ids com `armMod`, peças com `mod` (o modelo
//                 "referência + ajuste"), a 41 gravada com `ajuste`, `img` e
//                 `enq`, as três Secundárias livres, `virtues.valor`, a peça
//                 personalizada (`c`, `ce` e a armadura `c`), conjunto sem
//                 nenhum ativo, ficha sem conjuntos (o `equip.arma` antigo), o
//                 item inteiro editado, a massa escrita e o peso por extenso,
//                 o arsenal não vazio, a ficha de leitura com img e enq, e o
//                 item guardado com arma.folego
//   raca          a mesma ficha em cada raça de racas.json
//   arma          cada arma da lista do jogador na mão hábil, com a mão inábil
//                 girando entre nada, punho, escudos e outra arma, e a Força
//                 girando de 1 a 6 (o Arco Composto pede 4)
//   armadura      cada armadura sozinha, e pares vestidos e não vestidos
//   escudo        cada escudo na mão inábil
//   sorteio       fichas sorteadas (raça, Atributos, Habilidades, Centelha,
//                 Vontade, Arte de Mana, conjuntos, armaduras), metade gravada
//                 no formato novo antes de entrar

export function buildSheetInputs({ kael, formato, acaso, weapons, shields, armors, races }) {
  const clone = (x) => structuredClone(x);
  const cases = [];
  const add = (grupo, nome, ficha) => cases.push({ id: `${grupo}/${nome}`, grupo, ficha: clone(ficha) });

  const base = clone(kael);
  delete base._nota;

  // ----- kael -----
  add('kael', 'fixacao', base);
  add('kael', 'gravada', formato.fichaParaGravar(clone(base)));

  // ----- formato -----
  {
    const f = clone(base);
    f.equip = { armaduras: ['gambeson', 'malha'], armMod: { malha: { corte: 9, penalidade: 0 } } };
    add('formato', 'armaduras-lista-de-ids', f);
  }
  {
    const f = clone(base);
    f.equip = { armaduras: ['gambeson', 'nao-existe'] };
    add('formato', 'armadura-fora-do-catalogo', f);
  }
  {
    const f = clone(base);
    f.conjuntos[0].habil = { ref: 'a:espada-longa', mod: { acerto: 3, dado: 2, ticks: 99, danoBonus: 1.6 } };
    f.conjuntos[0].inabil = { ref: 'e:broquel', mod: { bloqCaC: 4, penalidade: -3 } };
    f.equip.armaduras[0] = { uid: 'arm1', base: 'gambeson', vestida: true, mod: { impacto: 7, resistPerf: 2, perfuracao: '' } };
    add('formato', 'modelo-antigo-mod', f);
  }
  {
    const f = clone(base);
    f.conjuntos[0].habil = { ref: 'c', nome: 'Pé de cabra', dado: 5, danoBonus: -9, acerto: 3 };
    f.conjuntos[0].inabil = { ref: 'ce', nome: 'Tampa de barril' };
    f.equip.armaduras.push({ uid: 'arm3', base: 'c', nome: 'Casaco do ferreiro', vestida: true, mod: { corte: 2, penalidade: 1 } });
    add('formato', 'pecas-personalizadas', f);
  }
  {
    const f = clone(base);
    f.conjuntos[0].habil = { ref: 'c' };
    f.conjuntos[0].inabil = { ref: 'a:adaga' };
    add('formato', 'personalizada-sem-numeros-e-adaga', f);
  }
  {
    const f = clone(base);
    f.skills2 = { 'livre-1': 3, 'livre-2': 0, halterofilismo: 2, atletismo: 1 };
    f.spec2 = { 'livre-1': [{ s: 'cantar', v: 1 }] };
    f.livreNome = { 'livre-1': 'Canto', 'livre-3': 'Sem valor' };
    f.spec = { armas: [{ s: 'espadas', v: 1 }], esquiva: [{ foco: 'rolar', valor: 1 }] };
    f.anteNom = { reputacao: [{ u: 'r1', n: 'Na vila', v: 2 }] };
    f.ante = { recursos: 2 };
    f.tech = { 'golpe-certeiro': true, outra: false, terceira: true };
    f.efeito = { brilho: true };
    f.arte = { 'manipulacao-mana': 3, fogo: 0, agua: 2 };
    add('formato', 'secundarias-livres-e-listas', f);
  }
  {
    const f = clone(base);
    f.virtues = { compaixao: 2, conviccao: 3, temperanca: 2, valor: 4, bravura: 1 };
    add('formato', 'virtudes-valor-e-bravura', f);
  }
  {
    const f = clone(base);
    f.conjuntos = f.conjuntos.map((c) => ({ ...c, ativo: false }));
    add('formato', 'nenhum-conjunto-ativo', f);
  }
  {
    const f = clone(base);
    delete f.conjuntos;
    f.equip.arma = 'machado';
    f.equip.escudo = 'redondo';
    add('formato', 'sem-conjuntos-arma-antiga', f);
  }
  {
    const f = clone(base);
    f.conjuntos = [];
    add('formato', 'conjuntos-vazios', f);
  }
  {
    const f = clone(base);
    f.corpo = { massa: 92.5, altura: 1.83 };
    add('formato', 'massa-escrita', f);
  }
  {
    const f = clone(base);
    f.id = { ...f.id, peso: '78,5 kg', altura: '1,74 m', titulo: 'Sir', idade: '31' };
    add('formato', 'peso-por-extenso', f);
  }
  {
    const f = clone(base);
    f.id = { ...f.id, peso: 'pesado' };
    f.corpo = { massa: -4 };
    add('formato', 'peso-sem-numero', f);
  }
  {
    // o item inteiro já guardado, editado pelo jogador (D-259)
    const f = formato.fichaParaLeitura(clone(base));
    const hab = f.conjuntos[0].habil;
    hab.item.arma.dado = 3;
    hab.item.arma.acerto = -1;
    hab.item.arma.tipoDano = 'impacto';
    hab.item.nome = 'Espada do avô';
    f.conjuntos[0].inabil.item.escudo.bloqCaC = 5;
    f.equip.armaduras[0].item.armadura.soak.corte = 8;
    f.equip.armaduras[0].item.armadura.classe = 'pesada';
    f.equip.armaduras[0].nome = 'Gambeson remendado';
    add('formato', 'item-inteiro-editado', f);
    add('formato', 'item-inteiro-editado-gravado', formato.fichaParaGravar(clone(f)));
  }
  {
    // a 41 gravada: equipamento.*, maoHabil/maoInabil, ajuste, img, enq
    const g = formato.fichaParaGravar(clone(base));
    for (const cj of g.equipamento.conjuntos) {
      for (const m of ['maoHabil', 'maoInabil']) if (cj[m]) delete cj[m].item;
    }
    g.equipamento.conjuntos[0].maoHabil.ajuste = { acerto: 2 };
    g.equipamento.conjuntos[0].maoHabil.imagem = 'espada.webp';
    g.equipamento.conjuntos[0].maoHabil.enquadramento = { zoom: 1.5, x: 10, y: -4 };
    for (const a of g.equipamento.armaduras) delete a.item;
    g.equipamento.armaduras[0].ajuste = { penalidade: 0 };
    g.habilidadesSec = { habilidadeSecLivre1: { nome: 'Pesca', valor: 2 }, halterofilismo: 3 };
    g.especialidadesSec = { habilidadeSecLivre1: [{ foco: 'rio', valor: 1 }] };
    g.antecedentesNomeados = { reputacao: [{ uid: 'r9', nome: 'Na corte', valor: 1 }] };
    g.virtudes = { compaixao: 1, conviccao: 1, temperanca: 1, valor: 3 };
    add('formato', 'gravada-41-sem-item', g);
  }
  {
    const g = formato.fichaParaGravar(clone(base));
    g.corpo = { altura: 1.7 };
    g.raca = 'gnomo';
    add('formato', 'gravada-sem-massa-gnomo', g);
  }

  {
    // o arsenal não vazio: do catálogo, com ajuste antigo, personalizada e o
    // id que saiu do catálogo
    const f = clone(base);
    f.arsenal = [{ ref: 'a:adaga' }, { ref: 'e:broquel', mod: { bloqCaC: 2 } }, { ref: 'c', nome: 'Bengala', dado: 1 },
      { ref: 'a:dardos' }, { ref: 'a:arco-longo', img: 'arco.webp' }];
    add('formato', 'arsenal', f);
    add('formato', 'arsenal-gravada', formato.fichaParaGravar(clone(f)));
  }
  {
    // a ficha de leitura com img e enq nas peças: a ida para a gravada leva o
    // enquadramento
    const f = clone(base);
    f.conjuntos[0].habil = { ref: 'a:espada-longa', img: 'espada.webp', enq: { z: 1.2, px: 5, py: -3 } };
    f.conjuntos[0].inabil = { ref: 'e:broquel', enq: { z: 2, px: 0, py: 0 } };
    f.equip.armaduras[0] = { ...f.equip.armaduras[0], img: 'gambeson.webp', enq: { z: 1, px: -2, py: 8 } };
    add('formato', 'leitura-img-enq', f);
    add('formato', 'leitura-img-enq-gravada', formato.fichaParaGravar(clone(f)));
  }
  {
    // o item já guardado com arma.folego: o Fôlego sai da ficha
    const w = weapons.find((x) => x.id === 'espada-longa');
    const { id, ...env } = clone(w);
    const item = { ...env, origem: id, arma: { ...env.arma, folego: 3 } };
    const f = clone(base);
    f.conjuntos[0].habil = { ref: 'a:espada-longa', item };
    f.arsenal = [{ ref: 'a:espada-longa', item: clone(item) }];
    add('formato', 'item-com-folego', f);
    const g = formato.fichaParaGravar(clone(base));
    g.equipamento.conjuntos[0].maoHabil = { ref: 'a:espada-longa', item: clone(item) };
    add('formato', 'item-com-folego-gravada', g);
    // o arsenal gravado com o item com Fôlego: a peça que fica no equip
    // perde o Fôlego junto com a lida
    const h = formato.fichaParaGravar(clone(base));
    h.equipamento.arsenal = [{ ref: 'a:espada-longa', item: clone(item) }, { ref: 'a:adaga', item: { origem: 'adaga', arma: { folego: 1 } } }];
    add('formato', 'arsenal-item-com-folego-gravada', h);
  }

  // ----- raca -----
  for (const r of races) {
    const f = clone(base);
    f.raca = r.id;
    add('raca', r.id, f);
    const g = clone(base);
    g.raca = r.id;
    g.attrs = { ...g.attrs, vigor: 6, forca: 1 };
    add('raca', `${r.id}-vigor6-forca1`, formato.fichaParaGravar(g));
  }
  {
    const f = clone(base);
    f.raca = 'nao-existe';
    add('raca', 'desconhecida', f);
  }

  // ----- arma -----
  const playerWeapons = weapons.filter((w) => !w.soCriatura && w.arma);
  const offHands = [{ ref: 'nada' }, { ref: 'a:desarmado' }, { ref: 'e:broquel' }, { ref: 'e:kite' }, { ref: 'a:adaga' },
    { ref: 'a:machadinha' }, { ref: 'a:arco-composto' }, { ref: 'ce', nome: 'Porta' }];
  playerWeapons.forEach((w, i) => {
    const f = clone(base);
    const forca = (i % 6) + 1;
    f.attrs = { ...f.attrs, forca, destreza: ((i * 5) % 6) + 1, percepcao: ((i * 7) % 6) + 1 };
    f.skills = { ...f.skills, armas: i % 7, briga: (i + 3) % 7, arremesso: (i + 1) % 7, atirador: (i + 2) % 7 };
    f.centelha = i % 5;
    f.conjuntos = [{ ativo: true, habil: { ref: `a:${w.id}` }, inabil: clone(offHands[i % offHands.length]) }];
    add('arma', w.id, f);
  });
  for (const forca of [1, 3, 4, 6]) {
    const f = clone(base);
    f.attrs = { ...f.attrs, forca };
    f.conjuntos = [{ ativo: true, habil: { ref: 'a:arco-composto' }, inabil: { ref: 'nada' } }];
    add('arma', `arco-composto-forca${forca}`, f);
  }
  for (const forca of [2, 9]) {
    const f = clone(base);
    f.attrs = { ...f.attrs, forca };
    f.conjuntos = [{ ativo: true, habil: { ref: 'a:arco-curto' }, inabil: { ref: 'nada' } },
      { ativo: false, habil: { ref: 'a:arco-longo' }, inabil: { ref: 'nada' } }];
    add('arma', `arco-curto-forca${forca}`, f);
    const g = clone(f);
    g.conjuntos = [{ ativo: true, habil: { ref: 'a:arco-longo' }, inabil: { ref: 'nada' } }];
    add('arma', `arco-longo-forca${forca}`, g);
  }
  {
    const f = clone(base);
    f.conjuntos = [{ ativo: true, habil: { ref: 'a:desarmado' }, inabil: { ref: 'a:desarmado' } }];
    add('arma', 'dois-punhos', f);
    const g = clone(base);
    g.conjuntos = [{ ativo: true, habil: { ref: 'a:desarmado' }, inabil: { ref: 'e:broquel' } }];
    add('arma', 'punho-e-broquel', g);
    const h = clone(base);
    h.conjuntos = [{ ativo: true, habil: { ref: 'a:espada-longa' }, inabil: { ref: 'nada' } }];
    add('arma', 'versatil-duas-maos', h);
    const k = clone(base);
    k.conjuntos = [{ ativo: true, habil: { ref: 'a:montante' }, inabil: { ref: 'e:broquel' } }];
    add('arma', 'duas-maos-com-escudo', k);
  }

  // ----- armadura -----
  for (const a of armors.filter((x) => x.armadura)) {
    const f = clone(base);
    f.equip = { armaduras: [{ uid: 'x', base: a.id, vestida: true }] };
    add('armadura', a.id, f);
  }
  {
    const ids = armors.filter((x) => x.armadura).map((x) => x.id);
    for (let i = 0; i < ids.length - 1; i += 2) {
      const f = clone(base);
      f.equip = { armaduras: [{ uid: 'a', base: ids[i], vestida: true }, { uid: 'b', base: ids[i + 1], vestida: i % 4 === 0 }] };
      add('armadura', `${ids[i]}+${ids[i + 1]}`, f);
    }
  }

  // ----- escudo -----
  for (const s of shields.filter((x) => x.escudo)) {
    const f = clone(base);
    f.conjuntos = [{ ativo: true, habil: { ref: 'a:maca' }, inabil: { ref: `e:${s.id}` } }];
    add('escudo', s.id, f);
  }

  // ----- sorteio -----
  const next = acaso.semeadoDe(20261125);
  const int = (lo, hi) => lo + Math.floor(next() * (hi - lo + 1));
  const pick = (arr) => arr[Math.floor(next() * arr.length)];
  const ATTRS = ['forca', 'destreza', 'vigor', 'influencia', 'perspicacia', 'compostura', 'percepcao', 'inteligencia', 'raciocinio'];
  const SKILLS = ['armas', 'briga', 'esquiva', 'bloqueio', 'prontidao', 'atletismo', 'resistencia', 'furtividade', 'sobrevivencia',
    'integridade', 'investigacao', 'empatia', 'ocultismo', 'conhecimentos', 'sociabilidade', 'arremesso', 'atirador'];
  const SEC = ['halterofilismo', 'atletismo', 'cura', 'gastronomia', 'natacao'];
  const shieldIds = shields.filter((x) => x.escudo).map((x) => x.id);
  const armorIds = armors.filter((x) => x.armadura).map((x) => x.id);
  for (let n = 0; n < 80; n++) {
    const f = clone(base);
    f.raca = pick(races).id;
    f.attrs = Object.fromEntries(ATTRS.map((k) => [k, int(0, 6)]));
    f.skills = Object.fromEntries(SKILLS.filter(() => next() < 0.7).map((k) => [k, int(0, 6)]));
    f.skills2 = Object.fromEntries(SEC.filter(() => next() < 0.4).map((k) => [k, int(0, 6)]));
    f.centelha = int(0, 6);
    f.willpower = int(0, 12);
    f.arte = next() < 0.5 ? { 'manipulacao-mana': int(0, 7) } : {};
    const hand = () => {
      const r = next();
      if (r < 0.55) return { ref: `a:${pick(playerWeapons).id}` };
      if (r < 0.8) return { ref: `e:${pick(shieldIds)}` };
      if (r < 0.9) return { ref: 'nada' };
      return { ref: 'a:desarmado' };
    };
    const k = int(1, 3);
    f.conjuntos = Array.from({ length: k }, () => ({ ativo: false, habil: hand(), inabil: hand() }));
    f.conjuntos[int(0, k - 1)].ativo = next() < 0.9;
    f.equip = { armaduras: Array.from({ length: int(0, 3) }, (_, i) => ({ uid: `s${i}`, base: pick(armorIds), vestida: next() < 0.8 })) };
    if (next() < 0.3) f.corpo = { massa: int(30, 140) + (next() < 0.5 ? 0.5 : 0) };
    if (next() < 0.5) add('sorteio', `${n}-gravada`, formato.fichaParaGravar(f));
    else add('sorteio', String(n), f);
  }

  return cases;
}

/** As saídas do motor para uma ficha (cada chamada recebe a sua cópia). */
export function sheetOutputs({ formato, motor, mesa }, ficha) {
  const c = () => structuredClone(ficha);
  const resumo = mesa.resumoFicha(c());
  return {
    leitura: formato.fichaParaLeitura(c()),
    gravar: formato.fichaParaGravar(c()),
    numeros: motor.numerosPersonagem(c()),
    combate: motor.combatePersonagem(c()),
    resumo,
    banco: mesa.resumoParaBanco(resumo),
  };
}
