// Prova da busca: o índice do Pagefind acha "Iniciativa" no capítulo de
// combate, com a âncora do título certo, e uma página aberta sem buscar não
// baixa nada do Pagefind.
//
//   node tools/ci/busca.mjs dist                 (serve o dist/ local)
//   node tools/ci/busca.mjs --url https://f-neves.github.io/centelha/
//
// Usa a API do pagefind.js no navegador sem janela, sem interface. Imprime os
// bytes que a página baixa sem buscar e os que a busca acrescenta.
import { resolve } from 'node:path';
import { detectBase, launch, serve } from '../site/cdp.mjs';

const TERMO = process.env.BUSCA_TERMO ?? 'Iniciativa';
const PAGINA = 'regras/combate/';

let origin;
let base;
let server = null;
if (process.argv[2] === '--url') {
  const u = new URL(process.argv[3]);
  origin = u.origin;
  base = u.pathname.endsWith('/') ? u.pathname : `${u.pathname}/`;
} else {
  const dist = resolve(process.argv[2] ?? 'dist');
  base = detectBase(dist);
  server = await serve(dist, base);
  origin = server.origin;
}

const browser = await launch();
let falhou = false;
try {
  const page = await browser.newPage();
  const bytes = new Map();
  const urls = new Map();
  // Todo pedido conta, terminado ou não: um arquivo do Pagefind pedido e
  // falhado (ou ainda em curso) também quebra a regra "sem buscar, nada".
  let pedidos = [];
  page.on('Network.requestWillBeSent', (p) => { urls.set(p.requestId, p.request.url); pedidos.push(p.request.url); });
  page.on('Network.loadingFinished', (p) => bytes.set(p.requestId, p.encodedDataLength));
  page.on('Network.loadingFailed', (p) => pedidos.push(urls.get(p.requestId) ?? ''));
  await page.send('Page.enable');
  await page.send('Runtime.enable');
  await page.send('Network.enable');
  await page.send('Network.setCacheDisabled', { cacheDisabled: true });

  const soma = (filtro) => {
    let total = 0;
    for (const [id, n] of bytes) if (filtro(urls.get(id) ?? '')) total += n;
    return total;
  };
  const doPagefind = (u) => new URL(u, origin).pathname.startsWith(`${base}pagefind/`);

  // 1. A página aberta, sem buscar.
  await page.goto(`${origin}${base}${PAGINA}`);
  await new Promise((ok) => setTimeout(ok, 1500));
  const semBusca = soma(() => true);
  const pagefindSemBusca = soma(doPagefind);
  const pedidosSemBusca = [...new Set(pedidos.filter(doPagefind))];
  bytes.clear();
  pedidos = [];

  // 2. A busca, pela API do pagefind.js.
  const r = await page.eval(`(async () => {
    const pf = await import(${JSON.stringify(`${base}pagefind/pagefind.js`)});
    await pf.init();
    const busca = await pf.search(${JSON.stringify(TERMO)});
    const dados = await Promise.all(busca.results.slice(0, 5).map((x) => x.data()));
    return { total: busca.results.length, dados: dados.map((d) => ({
      url: d.url, titulo: d.meta?.title,
      sub: (d.sub_results ?? []).slice(0, 5).map((s) => ({ url: s.url, titulo: s.title })),
    })) };
  })()`);
  await new Promise((ok) => setTimeout(ok, 500));
  const daBusca = soma(doPagefind);
  const pedidosDaBusca = [...new Set(pedidos.filter(doPagefind))].length;

  const alvo = r.dados.find((d) => d.url.includes(PAGINA));
  const ancora = alvo?.sub.find((s) => s.url.includes('#') && s.titulo?.toLowerCase().includes(TERMO.toLowerCase()));
  console.log(`busca por "${TERMO}" em ${origin}${base}: ${r.total} resultados`);
  for (const d of r.dados) {
    console.log(`  ${d.url} (${d.titulo})`);
    for (const s of d.sub) console.log(`      ${s.url} (${s.titulo})`);
  }
  console.log(`bytes ao abrir ${PAGINA} sem buscar: ${semBusca} (do Pagefind: ${pagefindSemBusca} B em ${pedidosSemBusca.length} pedidos)`);
  console.log(`bytes que a busca acrescenta (pagefind.js, WebAssembly, índice, fragmentos): ${daBusca} B em ${pedidosDaBusca} pedidos`);
  if (pedidosDaBusca === 0) { console.log('FALHA: a busca não pediu nenhum arquivo do Pagefind (a contagem de pedidos está quebrada?)'); falhou = true; }

  if (!alvo) { console.log(`FALHA: nenhum dos 5 primeiros resultados é ${PAGINA}`); falhou = true; }
  else if (!ancora) { console.log(`FALHA: ${PAGINA} veio sem âncora de título com "${TERMO}"`); falhou = true; }
  else console.log(`ok: ${ancora.url}`);
  if (pedidosSemBusca.length > 0) {
    console.log(`FALHA: a página pediu arquivo do Pagefind sem buscar: ${pedidosSemBusca.join(', ')}`);
    falhou = true;
  }
} finally {
  await browser.close();
  if (server) await server.close();
}
process.exit(falhou ? 1 : 0);
