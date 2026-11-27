// Tamanho do site montado em release: o WebAssembly e a cola das ilhas, o
// HTML do capítulo de combate e o índice da busca, em bytes, brutos e com
// `gzip -9 -n` (o programa gzip, como as medidas do ADR 0003). Sem o programa
// no caminho, usa o zlib do Node no nível 9, que dá uns 1% a mais, e diz isso
// na tabela.
//
//   node tools/ci/tamanho.mjs <dist> [--ref <main.json>] [--out <atual.json>]
//
// Com `--ref` (a medida do último main), mostra a diferença; sem o arquivo,
// só os números. A tabela vai para `$GITHUB_STEP_SUMMARY` quando existe (o
// resumo da execução no GitHub) e sempre para a saída. Só mostra: não falha
// por tamanho.
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync, appendFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { gzipSync } from 'node:zlib';

const args = process.argv.slice(2);
const opcao = (nome) => {
  const i = args.indexOf(nome);
  return i >= 0 ? args[i + 1] : null;
};
const dist = resolve(args.find((a, i) => !a.startsWith('--') && !args[i - 1]?.startsWith('--')) ?? 'dist');
const refPath = opcao('--ref');
const outPath = opcao('--out');

const temGzip = spawnSync('gzip', ['--version'], { stdio: 'ignore' }).status === 0;
const gz = (buf) => {
  if (!temGzip) return gzipSync(buf, { level: 9 }).length;
  const r = spawnSync('gzip', ['-9', '-n', '-c'], { input: buf, maxBuffer: 1 << 30 });
  if (r.status !== 0) throw new Error(`gzip saiu com ${r.status}: ${r.stderr}`);
  return r.stdout.length;
};
const GZ = temGzip ? 'gzip -9' : 'zlib 9';

/** Todos os arquivos de uma pasta, recursivo. */
function arquivos(dir) {
  const out = [];
  for (const nome of readdirSync(dir).sort()) {
    const p = join(dir, nome);
    if (statSync(p).isDirectory()) out.push(...arquivos(p));
    else out.push(p);
  }
  return out;
}

function medir(rotulo, caminhos) {
  const existentes = caminhos.filter((p) => existsSync(p));
  if (!existentes.length) return { rotulo, ausente: true };
  let bruto = 0;
  let gzip = 0;
  for (const p of existentes) {
    const buf = readFileSync(p);
    bruto += buf.length;
    gzip += gz(buf);
  }
  return { rotulo, bruto, gzip, arquivos: existentes.length };
}

const pagefind = join(dist, 'pagefind');
const itens = [
  medir('WebAssembly (`pkg/centelha_web_bg.wasm`)', [join(dist, 'pkg', 'centelha_web_bg.wasm')]),
  medir('cola JS (`pkg/centelha_web.js`)', [join(dist, 'pkg', 'centelha_web.js')]),
  medir('HTML do combate (`regras/combate/index.html`)', [join(dist, 'regras', 'combate', 'index.html')]),
  // Cada arquivo com gzip à parte (é como o servidor manda), somados.
  medir('índice da busca (`pagefind/`, todos os arquivos)', existsSync(pagefind) ? arquivos(pagefind) : []),
];

let ref = null;
if (refPath && existsSync(refPath)) {
  try {
    ref = JSON.parse(readFileSync(refPath, 'utf8'));
  } catch {
    ref = null;
  }
}

const n = (v) => v.toLocaleString('pt-BR');
const dif = (atual, antes) => {
  if (antes == null) return 'novo';
  const d = atual - antes;
  if (d === 0) return '=';
  const pct = antes ? ` (${d > 0 ? '+' : ''}${((d / antes) * 100).toFixed(1).replace('.', ',')}%)` : '';
  return `${d > 0 ? '+' : ''}${n(d)}${pct}`;
};

const linhas = [];
linhas.push('### Tamanho do site (release)');
linhas.push('');
if (ref) {
  linhas.push(`Diferença contra o último \`main\` medido (\`${ref.commit?.slice(0, 7) ?? '?'}\`).`);
  linhas.push('');
  linhas.push(`| | bruto (B) | dif. | ${GZ} (B) | dif. |`);
  linhas.push('|---|---:|---:|---:|---:|');
} else {
  linhas.push('Sem medida do `main` para comparar: só os números.');
  linhas.push('');
  linhas.push(`| | bruto (B) | ${GZ} (B) |`);
  linhas.push('|---|---:|---:|');
}
for (const it of itens) {
  const antes = ref?.itens?.find((r) => r.rotulo === it.rotulo);
  if (it.ausente) {
    linhas.push(ref ? `| ${it.rotulo} | ausente | | | |` : `| ${it.rotulo} | ausente | |`);
    continue;
  }
  const nome = it.arquivos > 1 ? `${it.rotulo}, ${it.arquivos} arquivos` : it.rotulo;
  if (ref) {
    const a = antes && !antes.ausente ? antes : null;
    linhas.push(`| ${nome} | ${n(it.bruto)} | ${dif(it.bruto, a?.bruto)} | ${n(it.gzip)} | ${dif(it.gzip, a?.gzip)} |`);
  } else {
    linhas.push(`| ${nome} | ${n(it.bruto)} | ${n(it.gzip)} |`);
  }
}
linhas.push('');
const texto = `${linhas.join('\n')}\n`;

console.log(texto);
if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, texto);
if (outPath) {
  mkdirSync(dirname(resolve(outPath)), { recursive: true });
  writeFileSync(outPath, `${JSON.stringify({ commit: process.env.GITHUB_SHA ?? null, gzip: GZ, itens }, null, 2)}\n`);
}
