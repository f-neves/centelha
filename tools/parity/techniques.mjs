// techniques.mjs · as contas das Técnicas na referência de paridade.
//
// Usado por gen-references.mjs. Três grupos, cada caso { fn, args, out }:
//
//   modProeza           src/lib/data.ts, o modificador pela trilha e o nível:
//                       cada trilha de regras.json (e chaves que não existem)
//                       nos níveis de -1 a 7 e 1,5
//   tecnicaDisponivel   src/lib/modulos.ts, para cada Técnica de tecnicas.json
//   closure             o fecho dos pré-requisitos da árvore
//                       (src/components/ArvoreTecnicas.astro), para cada
//                       Técnica disponível e alguns ids de fora; out = { chain,
//                       xp }, a cadeia na ordem da fonte e o XP somado como a
//                       árvore soma (custoTecnica do nível, 0 para quem não
//                       está na lista)
//
// O `closure` mora num <script> do componente e não é exportado: o gerador
// recorta a função do arquivo da fonte, tira os tipos com o esbuild e a roda
// com o mesmo `byId` que a árvore monta (as Técnicas disponíveis). Se o recorte
// não achar a função, o gerador para.

import fs from 'node:fs';

export async function loadTreeClosure({ transform, file }) {
  const text = fs.readFileSync(file, 'utf8');
  const m = text.match(/\n(\s*)function closure\(id: string\): Set<string> \{\n[\s\S]*?\n\1\}\n/);
  if (!m) throw new Error(`não achei a função closure em ${file}`);
  const { code } = await transform(m[0], { loader: 'ts' });
  // eslint-disable-next-line no-new-func
  return (byId) => new Function('byId', `${code}\nreturn closure;`)(byId);
}

export function buildTechniqueCases({ data, modulos, motor, regras, techniques, closureFor }) {
  const cases = [];
  const add = (fn, args, out) => cases.push({ fn, args, out: out === undefined ? null : out });

  const tracks = [...Object.keys(regras.escalasProeza?.trilhas || {}), 'inexistente', ''];
  for (const t of tracks) {
    for (const n of [-1, 0, 1, 2, 3, 4, 5, 6, 7, 1.5]) add('modProeza', [t, n], data.modProeza(t, n));
  }

  for (const t of techniques) add('tecnicaDisponivel', [t.id], modulos.tecnicaDisponivel(t));

  const all = techniques.filter((t) => modulos.tecnicaDisponivel(t));
  const byId = Object.fromEntries(all.map((t) => [t.id, t]));
  const closure = closureFor(byId);
  const ids = [...all.map((t) => t.id), 'inexistente',
    ...techniques.filter((t) => !modulos.tecnicaDisponivel(t)).map((t) => t.id)];
  for (const id of ids) {
    const chain = [...closure(id)];
    const xp = chain.reduce((s, cid) => s + motor.custoTecnica(byId[cid]?.nivel ?? 0), 0);
    add('closure', [id], { chain, xp });
  }
  return cases;
}
