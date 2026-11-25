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

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';

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
const { build } = requireFromSource('esbuild');

async function loadTs(rel) {
  const outfile = path.join(os.tmpdir(), `parity-${path.basename(rel, '.ts')}-${process.pid}.mjs`);
  await build({
    entryPoints: [path.join(SRC, rel)], outfile, bundle: true, format: 'esm', platform: 'node',
    loader: { '.json': 'json' }, logLevel: 'error', define: { 'import.meta.env': 'globalThis.__ENV__' },
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

const source = {
  commit,
  commitDate,
  contentCommit,
  forced: problems.length > 0,
  generator: 'tools/parity/gen-references.mjs',
};

function write(name, data) {
  fs.mkdirSync(OUT, { recursive: true });
  const file = path.join(OUT, name);
  fs.writeFileSync(file, JSON.stringify(data, null, 1) + '\n');
  console.log(`${path.relative(REPO, file)}: ${(fs.statSync(file).size / 1024).toFixed(1)} KB`);
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
  attackCount += derived.ataques.length;
  const entry = { derived };
  if (c.formas?.length) {
    entry.forms = {};
    for (const forma of c.formas) {
      entry.forms[forma.id] = motor.criatura(motor.comForma(c, forma));
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
  -1, -2, -12345, -(2 ** 31), -(2 ** 32) + 1, 3.7, -3.7, 0x6d2b79f5];
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

console.log(`${ids.length} criaturas, ${formCount} formas, ${attackCount} ataques; ${SEEDS.length} sementes × ${STEPS}`);
if (differs.length) {
  console.log(`a fixture criaturas-esperado.json difere em ${differs.length} caso(s):`);
  for (const d of differs.slice(0, 20)) console.log('  ' + d);
} else {
  console.log('a fixture criaturas-esperado.json dá o mesmo resultado que content/ nas 324 criaturas');
}
