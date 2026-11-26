// Prova da busca: o índice do Pagefind acha "Iniciativa" no capítulo de
// combate, com a âncora do título certo, e uma página aberta sem buscar não
// baixa nada do Pagefind (nem a página, nem um worker dela).
//
//   node tools/ci/busca.mjs dist                 (serve o dist/ local)
//   node tools/ci/busca.mjs --url https://f-neves.github.io/centelha/
//
// Usa a API do pagefind.js no navegador sem janela, sem interface. Imprime os
// bytes que a página baixa sem buscar, os que a busca acrescenta na página e
// os que o worker do Pagefind baixa (índice e fragmentos), numa busca por
// "Iniciativa" e numa por uma palavra comum (`BUSCA_TERMO_COMUM`, padrão
// "dano"). O worker é seguido pelo `Target.setAutoAttach`: os pedidos dele
// não aparecem na rede da página.
import { resolve } from 'node:path';
import { detectBase, launch, serve } from '../site/cdp.mjs';

const TERMO = process.env.BUSCA_TERMO ?? 'Iniciativa';
const TERMO_COMUM = process.env.BUSCA_TERMO_COMUM ?? 'dano';
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
  // Pedidos de cada sessão (a página e cada worker), por id.
  const bytes = new Map();
  const urls = new Map();
  const doWorker = new Set();
  // Todo pedido conta, terminado ou não: um arquivo do Pagefind pedido e
  // falhado (ou ainda em curso) também quebra a regra "sem buscar, nada".
  let pedidos = [];
  const chave = (sessao, id) => `${sessao}|${id}`;
  const workers = new Set();
  browser.listen((msg) => {
    const s = msg.sessionId;
    if (s !== page.sessionId && !workers.has(s)) return;
    const p = msg.params;
    if (msg.method === 'Target.attachedToTarget' && s === page.sessionId) {
      // Um worker da página: rede ligada antes de ele começar a rodar.
      const w = p.sessionId;
      workers.add(w);
      (async () => {
        await browser.send('Network.enable', {}, w).catch(() => {});
        await browser.send('Network.setCacheDisabled', { cacheDisabled: true }, w).catch(() => {});
        await browser.send('Runtime.runIfWaitingForDebugger', {}, w).catch(() => {});
      })();
    } else if (msg.method === 'Network.requestWillBeSent') {
      urls.set(chave(s, p.requestId), p.request.url);
      if (s !== page.sessionId) doWorker.add(chave(s, p.requestId));
      pedidos.push(p.request.url);
    } else if (msg.method === 'Network.loadingFinished') {
      bytes.set(chave(s, p.requestId), p.encodedDataLength);
    } else if (msg.method === 'Network.loadingFailed') {
      pedidos.push(urls.get(chave(s, p.requestId)) ?? '');
    }
  });
  await page.send('Page.enable');
  await page.send('Runtime.enable');
  await page.send('Network.enable');
  await page.send('Network.setCacheDisabled', { cacheDisabled: true });
  await page.send('Target.setAutoAttach', { autoAttach: true, waitForDebuggerOnStart: true, flatten: true });

  const caminho = (u) => new URL(u, origin).pathname;
  const doPagefind = (u) => caminho(u).startsWith(`${base}pagefind/`);
  const soma = (filtro) => {
    let total = 0;
    for (const [k, n] of bytes) if (filtro(urls.get(k) ?? '', k)) total += n;
    return total;
  };
  const contar = (filtro) => [...bytes.keys()].filter((k) => filtro(urls.get(k) ?? '', k)).length;
  // Os bytes do worker, por pasta do Pagefind.
  const doWorkerEm = (pasta) => (u, k) => doWorker.has(k) && caminho(u).startsWith(`${base}pagefind/${pasta}`);
  const linhaWorker = () => {
    const idx = doWorkerEm('index/');
    const frag = doWorkerEm('fragment/');
    const outro = (u, k) => doWorker.has(k) && !idx(u, k) && !frag(u, k);
    const nomes = [...bytes.keys()].filter((k) => outro(urls.get(k) ?? '', k))
      .map((k) => caminho(urls.get(k)).slice(`${base}pagefind/`.length)).sort();
    return `index/ ${soma(idx)} B em ${contar(idx)} pedidos, fragment/ ${soma(frag)} B em ${contar(frag)}, `
      + `outros ${soma(outro)} B em ${contar(outro)}${nomes.length ? ` (${nomes.join(', ')})` : ''}`
      + ` (total ${soma((u, k) => doWorker.has(k))} B)`;
  };
  const zerar = () => { bytes.clear(); doWorker.clear(); pedidos = []; };

  // 1. A página aberta, sem buscar.
  await page.goto(`${origin}${base}${PAGINA}`);
  await new Promise((ok) => setTimeout(ok, 1500));
  const semBusca = soma(() => true);
  const pagefindSemBusca = soma(doPagefind);
  const pedidosSemBusca = [...new Set(pedidos.filter(doPagefind))];
  zerar();

  // 2. A busca, pela API do pagefind.js.
  const buscar = (termo) => page.eval(`(async () => {
    const pf = await import(${JSON.stringify(`${base}pagefind/pagefind.js`)});
    await pf.init();
    const busca = await pf.search(${JSON.stringify(termo)});
    const dados = await Promise.all(busca.results.slice(0, 5).map((x) => x.data()));
    return { total: busca.results.length, dados: dados.map((d) => ({
      url: d.url, titulo: d.meta?.title,
      sub: (d.sub_results ?? []).slice(0, 5).map((s) => ({ url: s.url, titulo: s.title })),
    })) };
  })()`);
  const r = await buscar(TERMO);
  await new Promise((ok) => setTimeout(ok, 500));
  const daPagina = (u, k) => !doWorker.has(k) && doPagefind(u);
  const daBusca = soma(daPagina);
  const pedidosDaBusca = contar(daPagina);
  const workerBusca = linhaWorker();
  const pedidosWorker = contar((u, k) => doWorker.has(k));
  zerar();

  // 3. Uma palavra comum, depois (o pagefind.js e o WebAssembly já vieram).
  const comum = await buscar(TERMO_COMUM);
  await new Promise((ok) => setTimeout(ok, 500));
  const workerComum = linhaWorker();

  const alvo = r.dados.find((d) => d.url.includes(PAGINA));
  const ancora = alvo?.sub.find((s) => s.url.includes('#') && s.titulo?.toLowerCase().includes(TERMO.toLowerCase()));
  console.log(`busca por "${TERMO}" em ${origin}${base}: ${r.total} resultados`);
  for (const d of r.dados) {
    console.log(`  ${d.url} (${d.titulo})`);
    for (const s of d.sub) console.log(`      ${s.url} (${s.titulo})`);
  }
  console.log(`bytes ao abrir ${PAGINA} sem buscar: ${semBusca} (do Pagefind: ${pagefindSemBusca} B em ${pedidosSemBusca.length} pedidos)`);
  console.log(`bytes que a busca acrescenta na página (o pagefind.js; o resto vem pelo worker, nas linhas abaixo): ${daBusca} B em ${pedidosDaBusca} pedidos`);
  console.log(`bytes que o worker baixou na busca por "${TERMO}": ${workerBusca}`);
  console.log(`bytes que o worker baixou na busca por "${TERMO_COMUM}" (${comum.total} resultados, logo depois): ${workerComum}`);
  if (pedidosDaBusca === 0) { console.log('FALHA: a busca não pediu nenhum arquivo do Pagefind (a contagem de pedidos está quebrada?)'); falhou = true; }
  if (pedidosWorker === 0) { console.log('FALHA: nenhum pedido do worker do Pagefind na busca (o worker não foi seguido?)'); falhou = true; }

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
