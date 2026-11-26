// collate.mjs · a colação do `localeCompare` com 'pt' na referência de
// paridade.
//
// Usado por gen-references.mjs. Cada par [a, b, pt, base, numerico] grava o
// sinal de três comparações do Node (o ICU dele):
//   pt        a.localeCompare(b, 'pt')
//   base      a.localeCompare(b, 'pt', { sensitivity: 'base' })
//   numerico  a.localeCompare(b, 'pt', { numeric: true, sensitivity: 'base' })
//
// Os pares: todos os pares de uma lista de bordas (hífen, espaço, algarismo,
// pontuação, acento, caixa, ç, números de tamanhos diferentes); os vizinhos
// dos nomes reais do livro (Efeitos, Técnicas, Artes, Caminhos, Secundárias,
// criaturas e termos do glossário) depois de ordenados pela colação; e pares
// sorteados entre esses nomes (semente fixa).

export const COLLATE_SEED = 20261127;

const EDGES = ['', ' ', 'a', 'A', 'á', 'Á', 'à', 'â', 'ã', 'ä', 'b', 'B', 'c', 'ç', 'Ç', 'C', 'ca', 'ça', 'cb',
  'e', 'é', 'ê', 'É', 'i', 'í', 'o', 'ó', 'ô', 'õ', 'u', 'ú', 'ü', 'n', 'ñ', 'z',
  'a-b', 'a b', 'ab', 'a_b', 'a.b', 'a,b', 'a(b', 'a/b', "a'b", 'a+b', 'a1', 'a2', 'a10', 'a02', 'a 2', 'a-2',
  '1', '2', '10', '02', '1,5', '1.5', '−1', '-1', '×2', '(I)', '[x]', '#1', '*', 'Ação', 'Acao', 'acao', 'ação',
  'Água', 'Agua', 'agua', 'Aguardar', 'Pool', 'Po ol', 'P2', 'Força', 'Força de Vontade', 'Forca', 'Forçado',
  'Nível 2', 'Nível 10', 'nivel 2', 'Nível 02', 'Cão', 'Cao', 'Caos', 'Mão', 'Mao', 'Maos', 'Éter', 'Esquiva',
  'Golpe do Titã', 'Golpe Pesado', 'golpe-pesado', 'Quebra-Montanhas', 'Quebra Montanhas', 'Quebramontanhas'];

export function buildCollateCases({ acaso, lists }) {
  const sign = (x) => Math.sign(x) || 0;
  const row = (a, b) => [a, b, sign(a.localeCompare(b, 'pt')), sign(a.localeCompare(b, 'pt', { sensitivity: 'base' })),
    sign(a.localeCompare(b, 'pt', { numeric: true, sensitivity: 'base' }))];
  const pairs = [];
  for (let i = 0; i < EDGES.length; i++) for (let j = i + 1; j < EDGES.length; j++) pairs.push(row(EDGES[i], EDGES[j]));
  const names = [...new Set(lists.flat().filter((s) => typeof s === 'string' && s))];
  const sorted = [...names].sort((a, b) => a.localeCompare(b, 'pt'));
  for (let i = 0; i + 1 < sorted.length; i++) pairs.push(row(sorted[i], sorted[i + 1]));
  const rnd = acaso.semeadoDe(COLLATE_SEED);
  for (let k = 0; k < 2000; k++) {
    const a = names[Math.floor(rnd() * names.length)];
    const b = names[Math.floor(rnd() * names.length)];
    pairs.push(row(a, b));
  }
  return { names: names.length, pairs };
}
