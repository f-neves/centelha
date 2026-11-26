// A prova 2 das ilhas que ainda não têm página portada (o cartão das formas e
// o voltar ao topo, das páginas das Artes), numa página de teste fora do
// `dist/`.
//
//   node tools/site/islands-test.mjs [dist]
//
// Copia o `dist/` (montado antes por `cargo xtask site`) para uma pasta
// temporária, grava nela a página de teste com o gerador (`--test-page`) e roda
// a prova 2 (`hydration.mjs`) só nela. Apaga a pasta no fim. O gerador já tem
// de estar compilado (o `cargo xtask site` compila).

import { cpSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const dist = resolve(process.argv[2] ?? 'dist');
const root = resolve(new URL('../..', import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1'));
const work = mkdtempSync(join(tmpdir(), 'centelha-ilhas-'));
let code = 1;
try {
  cpSync(dist, work, { recursive: true });
  const gen = spawnSync('cargo', ['run', '--quiet', '-p', 'centelha-web', '--features', 'ssr', '--bin', 'centelha-site', '--',
    '--content', join(root, 'content'), '--out', work, '--test-page'], { cwd: root, stdio: 'inherit', env: process.env });
  if (gen.status !== 0) throw new Error('o gerador falhou');
  // O gerador refaz as páginas, mas não o WebAssembly: o `pkg/` veio na cópia.
  const run = spawnSync(process.execPath, [join(root, 'tools/site/hydration.mjs'), work], {
    stdio: 'inherit', env: { ...process.env, PAGES: 'teste-ilhas/' },
  });
  code = run.status ?? 1;
} finally {
  rmSync(work, { recursive: true, force: true });
}
process.exit(code);
