// gen-references.mjs · gera as referências de paridade em tests/parity/.
//
// Roda o próprio motor TypeScript da fonte (a pasta em RPG_SYSTEM_DIR), sem
// copiar código TypeScript para este repositório, e grava as entradas e as
// saídas em JSON. Roda na máquina do autor, não no CI: o CI só confere o Rust
// contra os arquivos gravados.
//
//   RPG_SYSTEM_DIR=/caminho/da/fonte node tools/parity/gen-references.mjs
//
// Antes de gravar, confere que a fonte está no mesmo commit da cópia de
// content/ (content/ORIGEM.txt) e sem mudança local em src/: as criaturas que
// entram são as de content/, e as tabelas que o motor lê são as da fonte. Com
// diferença, para (use --force para gravar assim mesmo, e diga no relatório).
//
// Arquivos gravados:
//   tests/parity/source.json      de onde veio (commit, data, gerador)
//   tests/parity/bestiary-ids.json  a lista dos ids das criaturas, em ordem
//   tests/parity/creatures.json   os derivados de cada criatura e das formas
//   tests/parity/mulberry32.json  sequências do gerador aleatório por semente
//   tests/parity/sheets.json      fichas de jogador e as saídas do motor (sheets.mjs)
//   tests/parity/character.json   as contas pequenas do personagem, função a função (character.mjs)
//   tests/parity/strikes.json     o lance e os dados (strikes.mjs)
//   tests/parity/techniques.json  o modificador, a Técnica oferecida e o fecho dos pré-requisitos (techniques.mjs)
//
// O JSON não guarda -0 (o JSON.stringify escreve 0). Para a referência não
// esconder um -0, todo -0 das saídas é gravado como o texto "-0", e o teste do
// Rust lê esse texto como o número -0. NaN e infinito param o gerador.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { buildSheetInputs, sheetOutputs } from './sheets.mjs';
import { buildCharacterCases } from './character.mjs';
import { buildStrikes, ROLLED_SEED, ROLLS_SEED } from './strikes.mjs';
import { buildTechniqueCases, loadTreeClosure } from './techniques.mjs';

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const OUT = path.join(REPO, 'tests', 'parity');
const CONTENT = path.join(REPO, 'content');
const FORCE = process.argv.includes('--force');

const SRC = process.env.RPG_SYSTEM_DIR;
if (!SRC || !fs.existsSync(path.join(SRC, 'src', 'lib', 'motor.ts'))) {
  console.error('RPG_SYSTEM_DIR precisa apontar para a fonte (a pasta que tem src/lib/motor.ts).');
  process.exit(2);
}

// ----- de onde veio -----
const git = (...args) => execFileSync('git', ['-C', SRC, ...args], { encoding: 'utf8' }).trim();
const commit = git('rev-parse', 'HEAD');
const commitDate = git('show', '-s', '--format=%cI', 'HEAD');
const dirty = git('status', '--porcelain', '--', 'src').split('\n').filter(Boolean);
const origem = fs.readFileSync(path.join(CONTENT, 'ORIGEM.txt'), 'utf8');
const contentCommit = (origem.match(/^commit: ([0-9a-f]{40})$/m) || [])[1];
const problems = [];
if (contentCommit !== commit) problems.push(`content/ veio do commit ${contentCommit}, a fonte está em ${commit}`);
if (dirty.length) problems.push(`a fonte tem ${dirty.length} mudança(s) local(is) em src/`);
if (problems.length) {
  for (const p of problems) console.error('aviso: ' + p);
  if (!FORCE) {
    console.error('nada gravado (use --force para gravar assim mesmo).');
    process.exit(1);
  }
}

// ----- carregar o TypeScript da fonte pelo esbuild dela -----
const requireFromSource = createRequire(path.join(SRC, 'package.json'));
const { build, transform } = requireFromSource('esbuild');

// O `astro:content` só existe dentro do Astro. Quem o importa só para montar páginas
// (src/lib/data.ts) recebe um módulo vazio; a função que o chamar quebra.
const noAstro = {
  name: 'sem-astro',
  setup(b) {
    b.onResolve({ filter: /^astro:content$/ }, () => ({ path: 'astro:content', namespace: 'sem-astro' }));
    b.onLoad({ filter: /.*/, namespace: 'sem-astro' }, () => ({
      contents: 'export const getCollection = () => { throw new Error("astro:content fora do Astro"); };',
      loader: 'js',
    }));
  },
};

async function loadTs(rel) {
  const outfile = path.join(os.tmpdir(), `parity-${path.basename(rel, '.ts')}-${process.pid}.mjs`);
  await build({
    entryPoints: [path.join(SRC, rel)], outfile, bundle: true, format: 'esm', platform: 'node',
    loader: { '.json': 'json' }, logLevel: 'error', define: { 'import.meta.env': 'globalThis.__ENV__' },
    plugins: [noAstro],
  });
  globalThis.__ENV__ = globalThis.__ENV__ || { BASE_URL: '/', MODE: 'test' };
  try {
    return await import(pathToFileURL(outfile).href);
  } finally {
    try { fs.unlinkSync(outfile); } catch { /* o sistema limpa */ }
  }
}

// Vários módulos num pacote só (o estado do acaso é do pacote).
async function loadBundle(name, contents) {
  const outfile = path.join(os.tmpdir(), `parity-${name}-${process.pid}.mjs`);
  await build({
    stdin: { contents, resolveDir: path.join(SRC, 'src', 'lib'), loader: 'ts' }, outfile, bundle: true, format: 'esm',
    platform: 'node', loader: { '.json': 'json' }, logLevel: 'error', define: { 'import.meta.env': 'globalThis.__ENV__' },
  });
  globalThis.__ENV__ = globalThis.__ENV__ || { BASE_URL: '/', MODE: 'test' };
  try {
    return await import(pathToFileURL(outfile).href);
  } finally {
    try { fs.unlinkSync(outfile); } catch { /* o sistema limpa */ }
  }
}

const motor = await loadTs('src/lib/motor.ts');
const acaso = await loadTs('src/lib/acaso.ts');
const formato = await loadTs('src/lib/ficha-formato.ts');
const mesa = await loadTs('src/lib/mesa-ficha.ts');
const quase = await loadTs('src/lib/quase-acerto.ts');
const equip = await loadTs('src/lib/equip.ts');
const migra = await loadTs('src/lib/ficha-migra.ts');
const data = await loadTs('src/lib/data.ts');
const modulos = await loadTs('src/lib/modulos.ts');
const combat = await loadBundle('combate', "export * from './acaso'; export { rolarExpr } from './rolagem'; "
  + "export { resolverGolpe, fonteFixa, fonteRolada, defesaEfetiva } from './lance';");

const source = {
  commit,
  commitDate,
  contentCommit,
  forced: problems.length > 0,
  generator: 'tools/parity/gen-references.mjs',
};

// -0 vira o texto "-0" (ver o cabeçalho)
const keepNegativeZero = (_, v) => (Object.is(v, -0) ? '-0' : v);

// `lines`: a chave (ou as chaves) de uma lista que sai com um item por linha, sem recuo (as
// fichas: o arquivo fica com a metade do tamanho e o diff continua por ficha).
function write(name, data, lines) {
  fs.mkdirSync(OUT, { recursive: true });
  const file = path.join(OUT, name);
  let text;
  if (lines) {
    const listKeys = [lines].flat();
    const parts = Object.entries(data).map(([k, v]) => (listKeys.includes(k)
      ? `${JSON.stringify(k)}: [\n${v.map((x) => JSON.stringify(x, keepNegativeZero)).join(',\n')}\n]`
      : `${JSON.stringify(k)}: ${JSON.stringify(v, keepNegativeZero)}`));
    text = `{\n${parts.join(',\n')}}`;
  } else {
    text = JSON.stringify(data, keepNegativeZero, 1);
  }
  fs.writeFileSync(file, text + '\n');
  console.log(`${path.relative(REPO, file)}: ${(fs.statSync(file).size / 1024).toFixed(1)} KB`);
}

// O JSON.stringify escreve NaN e infinito como null: a referência esconderia os
// dois. Antes de gravar, procura em cada saída do motor. O -0 só é contado: ele
// é gravado como "-0".
const badNumbers = [];
const negativeZeros = [];
function scanNumbers(value, where) {
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) badNumbers.push(`${where}: ${value}`);
    else if (Object.is(value, -0)) negativeZeros.push(where);
  } else if (Array.isArray(value)) {
    value.forEach((v, i) => scanNumbers(v, `${where}[${i}]`));
  } else if (value && typeof value === 'object') {
    for (const [k, v] of Object.entries(value)) scanNumbers(v, `${where}.${k}`);
  }
}

// ----- criaturas -----
const dir = path.join(CONTENT, 'data', 'bestiario');
const files = fs.readdirSync(dir).filter((f) => f.endsWith('.json')).sort();
const ids = [];
const creatures = {};
let formCount = 0;
let attackCount = 0;
for (const f of files) {
  const c = JSON.parse(fs.readFileSync(path.join(dir, f), 'utf8'));
  if (c.id !== f.slice(0, -5)) throw new Error(`${f}: o id ${c.id} não bate com o nome do arquivo`);
  ids.push(c.id);
  const derived = motor.criatura(c);
  scanNumbers(derived, c.id);
  attackCount += derived.ataques.length;
  const entry = { derived };
  if (c.formas?.length) {
    entry.forms = {};
    for (const forma of c.formas) {
      entry.forms[forma.id] = motor.criatura(motor.comForma(c, forma));
      scanNumbers(entry.forms[forma.id], `${c.id}/${forma.id}`);
      formCount++;
    }
  }
  creatures[c.id] = entry;
}

// a fixture conferida pelo teste do motor da fonte: a entrada dela é a que o
// conversor das fichas escreve. Se o motor der o mesmo para ela e para o JSON
// de content/, as referências acima cobrem os dois caminhos.
const expectedFixture = JSON.parse(fs.readFileSync(path.join(SRC, 'scripts', 'fixtures', 'criaturas-esperado.json'), 'utf8'));
const differs = [];
for (const [id, x] of Object.entries(expectedFixture.criaturas)) {
  if (!creatures[id]) { differs.push(`${id}: não está em content/`); continue; }
  const a = JSON.stringify(motor.criatura(x.entrada));
  const b = JSON.stringify(creatures[id].derived);
  if (a !== b) differs.push(`${id}: o motor dá outro resultado para a entrada da fixture`);
}
for (const id of ids) if (!expectedFixture.criaturas[id]) differs.push(`${id}: não está na fixture`);

// ----- fichas de jogador -----
const readData = (name) => JSON.parse(fs.readFileSync(path.join(CONTENT, 'data', name), 'utf8'));
const kael = JSON.parse(fs.readFileSync(path.join(SRC, 'scripts', 'fixtures', 'kael.json'), 'utf8'));
const sheetInputs = buildSheetInputs({
  kael, formato, acaso,
  weapons: readData('armas.json'), shields: readData('escudos.json'), armors: readData('armaduras.json'),
  races: readData('racas.json'),
});
const sheets = [];
const sheetErrors = [];
const groups = {};
for (const c of sheetInputs) {
  try {
    const out = sheetOutputs({ formato, motor, mesa }, c.ficha);
    scanNumbers(out, `ficha ${c.id}`);
    sheets.push({ ...c, ...out });
    groups[c.grupo] = (groups[c.grupo] || 0) + 1;
  } catch (e) {
    sheetErrors.push(`${c.id}: ${e.message}`);
  }
}
if (sheetErrors.length) {
  for (const e of sheetErrors) console.error(`erro: o motor falhou na ficha ${e}`);
  console.error('nada gravado.');
  process.exit(1);
}
const sheetIds = new Set();
for (const s of sheets) {
  if (sheetIds.has(s.id)) throw new Error(`ficha repetida: ${s.id}`);
  sheetIds.add(s.id);
}

// ----- contas pequenas do personagem -----
const characterCases = buildCharacterCases({
  motor, quase, equip, migra, regras: readData('regras.json'), weapons: readData('armas.json'),
});
const characterCounts = {};
for (const c of characterCases) {
  scanNumbers(c.out, `${c.fn}(${JSON.stringify(c.args)})`);
  characterCounts[c.fn] = (characterCounts[c.fn] || 0) + 1;
}

// ----- o lance e os dados -----
const strikes = buildStrikes({
  combat, fixtureText: fs.readFileSync(path.join(SRC, 'scripts', 'fixtures', 'lances.jsonl'), 'utf8'),
});
strikes.fixed.forEach((c, i) => scanNumbers(c.saida, `lance fixo ${i}`));
strikes.rolled.forEach((c, i) => scanNumbers(c.saida, `lance rolado ${i}`));
strikes.rolls.forEach((c, i) => scanNumbers(c, `rolagem ${i}`));

// ----- as Técnicas -----
const techniqueCases = buildTechniqueCases({
  data, modulos, motor, regras: readData('regras.json'), techniques: readData('tecnicas.json'),
  closureFor: await loadTreeClosure({ transform, file: path.join(SRC, 'src', 'components', 'ArvoreTecnicas.astro') }),
});
const techniqueCounts = {};
for (const c of techniqueCases) {
  scanNumbers(c.out, `${c.fn}(${JSON.stringify(c.args)})`);
  techniqueCounts[c.fn] = (techniqueCounts[c.fn] || 0) + 1;
}

if (negativeZeros.length) console.log(`${negativeZeros.length} -0 nas saídas, gravados como "-0"`);
if (badNumbers.length) {
  for (const b of badNumbers) console.error(`erro: número não finito em ${b}`);
  console.error('nada gravado: o motor deu NaN ou infinito.');
  process.exit(1);
}

write('source.json', source);
write('bestiary-ids.json', { source, count: ids.length, ids });
write('creatures.json', {
  source,
  note: 'criatura() do motor para cada content/data/bestiario/<id>.json; forms: criatura(comForma(principal, forma))',
  count: ids.length,
  creatures,
});

// ----- Mulberry32 -----
// As sementes como o TypeScript as recebe: um número de JavaScript qualquer,
// reduzido a 32 bits sem sinal por `>>> 0`. `seedU32` é o resultado dessa
// redução; `u32` é cada saída vezes 2^32 (inteiro exato, as STEPS saídas) e
// `f64` são as primeiras F64_STEPS saídas como o JavaScript as devolve (a
// divisão por 2^32 é exata, então o resto se confere pelo `u32`).
const SEEDS = [0, 1, 2, 10, 42, 12345, 2 ** 31 - 1, 2 ** 31, 2 ** 32 - 1, 2 ** 32, 2 ** 32 + 5,
  -1, -2, -12345, -(2 ** 31), -(2 ** 32) + 1, 3.7, -3.7, 0x6d2b79f5,
  // as sementes que os testes e a bancada da fonte usam
  1234, 771107, 20260902, 20260903];
const STEPS = 1000;
const F64_STEPS = 50;
const sequences = SEEDS.map((seed) => {
  const next = acaso.semeadoDe(seed);
  const f64 = [];
  const u32 = [];
  for (let i = 0; i < STEPS; i++) {
    const x = next();
    if (i < F64_STEPS) f64.push(x);
    u32.push(x * 4294967296);
  }
  return { seed, seedU32: seed >>> 0, u32, f64 };
});
write('mulberry32.json', { source, steps: STEPS, f64Steps: F64_STEPS, sequences });
write('sheets.json', {
  source,
  note: 'ficha: a entrada; leitura: fichaParaLeitura; gravar: fichaParaGravar; numeros: numerosPersonagem; '
    + 'combate: combatePersonagem; resumo: resumoFicha (mesa-ficha.ts); banco: resumoParaBanco(resumo). '
    + 'O lote é montado por tools/parity/sheets.mjs. -0 gravado como "-0".',
  count: sheets.length,
  groups,
  sheets,
}, 'sheets');
write('character.json', {
  source,
  note: 'Cada caso: fn (a função do TypeScript), args e out. null num parâmetro opcional quer dizer não passado. '
    + 'O lote é montado por tools/parity/character.mjs. -0 gravado como "-0".',
  count: characterCases.length,
  counts: characterCounts,
  cases: characterCases,
}, 'cases');
write('strikes.json', {
  source,
  note: 'fixed: resolverGolpe(entrada, fonteFixa(sorteio)) de cada lance de scripts/fixtures/lances.jsonl; '
    + `rolled: os mesmos lances pela fonteRolada com um Mulberry32 só (semente ${ROLLED_SEED}), draws = números já entregues; `
    + `rolls: rolarExpr com um Mulberry32 só (semente ${ROLLS_SEED}). Montado por tools/parity/strikes.mjs. -0 gravado como "-0".`,
  rolledSeed: ROLLED_SEED,
  rollsSeed: ROLLS_SEED,
  counts: { fixed: strikes.fixed.length, rolled: strikes.rolled.length, rolls: strikes.rolls.length },
  fixed: strikes.fixed,
  rolled: strikes.rolled,
  rolls: strikes.rolls,
}, ['fixed', 'rolled', 'rolls']);
write('techniques.json', {
  source,
  note: 'Cada caso: fn, args e out. modProeza (src/lib/data.ts), tecnicaDisponivel (src/lib/modulos.ts) e closure '
    + '(o fecho dos pré-requisitos de src/components/ArvoreTecnicas.astro, recortado do arquivo; out = cadeia e XP). '
    + 'Montado por tools/parity/techniques.mjs.',
  count: techniqueCases.length,
  counts: techniqueCounts,
  cases: techniqueCases,
}, 'cases');
console.log(`${techniqueCases.length} casos das Técnicas`);
console.log(`${strikes.fixed.length} lances (fixos e rolados), ${strikes.rolls.length} rolagens`);
console.log(`${characterCases.length} casos das contas pequenas em ${Object.keys(characterCounts).length} funções`);
console.log(`${sheets.length} fichas: ${Object.entries(groups).map(([g, n]) => `${g} ${n}`).join(', ')}`);

console.log(`${ids.length} criaturas, ${formCount} formas, ${attackCount} ataques; ${SEEDS.length} sementes × ${STEPS}`);
if (differs.length) {
  console.log(`a fixture criaturas-esperado.json difere em ${differs.length} caso(s):`);
  for (const d of differs.slice(0, 20)) console.log('  ' + d);
} else {
  console.log('a fixture criaturas-esperado.json dá o mesmo resultado que content/ nas 324 criaturas');
}
