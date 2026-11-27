// Conferência dos links internos do `dist/`: todo `href` e `src` de cada
// página cai num arquivo que existe e, com `#id`, num `id` que existe nele.
//
//   node tools/ci/links.mjs <dist>
//
// Cada link interno sai numa de três classes:
// - quebrado (o job falha): arquivo que não existe (imagem, folha de estilo,
//   script, ícone, ou qualquer `src`), âncora que não existe numa página que
//   existe, link fora do caminho base do site;
// - página ainda não portada (só lista): `<a href>`, `<form action>`,
//   `<meta http-equiv="refresh">` e `<link>` que aponta para página
//   (`canonical`, `alternate`, `next`, `prev`, `prefetch`, `prerender`, ou
//   um `rel` fora das listas, escrito na lista) para uma página que o `dist/`
//   não tem, com quantos links apontam para ela. `<link>` de recurso
//   (`stylesheet`, `icon`, `preload`, `modulepreload`, `manifest` e outros)
//   que falta é quebrado;
// - externo (só conta, sem visitar): outro endereço, `mailto:`, `tel:`.
// O caminho base vem do próprio `dist/` (o `href` da marca no topo). A lista e
// as contas vão para `$GITHUB_STEP_SUMMARY` quando existe, e para a saída.
import { appendFileSync, existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { extname, join, relative, resolve, sep } from 'node:path';
import { detectBase } from '../site/cdp.mjs';

const dist = resolve(process.argv[2] ?? 'dist');
const base = detectBase(dist);
const ORIGEM = 'http://site.invalid';

function arquivosHtml(dir) {
  const out = [];
  for (const nome of readdirSync(dir).sort()) {
    const p = join(dir, nome);
    if (statSync(p).isDirectory()) out.push(...arquivosHtml(p));
    else if (nome.endsWith('.html')) out.push(p);
  }
  return out;
}

// O texto da página sem o que não é marcação: comentários e o corpo dos
// `<script>` e `<style>` (JSON e código podem ter "<a href" dentro de texto).
function marcacao(html) {
  return html
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(/(<script\b[^>]*>)[\s\S]*?<\/script>/gi, '$1</script>')
    .replace(/(<style\b[^>]*>)[\s\S]*?<\/style>/gi, '$1</style>');
}

const entidades = { amp: '&', quot: '"', apos: "'", lt: '<', gt: '>', '#39': "'" };
const desfazer = (s) => s.replace(/&(amp|quot|apos|lt|gt|#39);/g, (_, e) => entidades[e]);

const idsPorArquivo = new Map();
function ids(arquivo) {
  if (!idsPorArquivo.has(arquivo)) {
    const html = marcacao(readFileSync(arquivo, 'utf8'));
    const set = new Set();
    for (const m of html.matchAll(/<[a-z][^>]*?\s(?:id|name)\s*=\s*"([^"]*)"/gi)) set.add(desfazer(m[1]));
    for (const m of html.matchAll(/<[a-z][^>]*?\s(?:id|name)\s*=\s*'([^']*)'/gi)) set.add(desfazer(m[1]));
    idsPorArquivo.set(arquivo, set);
  }
  return idsPorArquivo.get(arquivo);
}

/** O arquivo do `dist/` que o servidor daria para o caminho (sem a base), ou null. */
function arquivoDe(sub) {
  const tenta = [];
  if (sub === '' || sub.endsWith('/')) tenta.push(`${sub}index.html`);
  else tenta.push(sub, `${sub}/index.html`, `${sub}.html`);
  for (const t of tenta) {
    const p = join(dist, ...t.split('/'));
    if (!p.startsWith(dist)) continue;
    if (existsSync(p) && statSync(p).isFile()) return p;
  }
  return null;
}

const ehPagina = (sub) => sub === '' || sub.endsWith('/') || extname(sub) === '' || extname(sub) === '.html';

// Os atributos que levam a um endereço, por elemento.
const ATRIBUTOS = /<(a|area|link|img|script|source|iframe|video|audio|track|use|image|form|meta)\b([^>]*)>/gi;
const ATRIBUTO = /\s(href|src|srcset|xlink:href|poster|action)\s*=\s*("([^"]*)"|'([^']*)')/gi;
const valorDe = (attrs, nome) => {
  const m = attrs.match(new RegExp(String.raw`\s${nome}\s*=\s*("([^"]*)"|'([^']*)'|([^\s>]+))`, 'i'));
  return m ? desfazer(m[2] ?? m[3] ?? m[4] ?? '') : null;
};
// `<link rel>`: o que é recurso da página (falta = quebrado) e o que aponta
// para outra página (falta = página ainda não portada, como um `<a href>`).
const REL_RECURSO = new Set(['stylesheet', 'icon', 'apple-touch-icon', 'apple-touch-icon-precomposed', 'mask-icon',
  'preload', 'modulepreload', 'manifest', 'shortcut']);
const REL_PAGINA = new Set(['canonical', 'alternate', 'next', 'prev', 'prefetch', 'prerender']);

const paginas = arquivosHtml(dist);
const quebrados = [];
const naoPortadas = new Map();
const externos = new Map();
let internos = 0;
let ancoras = 0;

/**
 * Os endereços de uma tag: `{ v, como, pagina, nota }`. `pagina` diz se a
 * falta do alvo é "página ainda não portada" (link para página) ou quebrado
 * (recurso); `nota` vai junto na lista (o `rel` fora das duas listas).
 */
function enderecos(elemento, attrs) {
  if (elemento === 'meta') {
    // `<meta http-equiv="refresh" content="0; url=...">`: a página que só
    // redireciona segue a regra do `<a href>`.
    if ((valorDe(attrs, 'http-equiv') ?? '').toLowerCase() !== 'refresh') return [];
    const m = (valorDe(attrs, 'content') ?? '').match(/url\s*=\s*['"]?([^'"]+)/i);
    return m ? [{ v: m[1].trim(), como: `<meta http-equiv="refresh" url="${m[1].trim()}">`, pagina: true, nota: "" }] : [];
  }
  let pagina = elemento === 'a' || elemento === 'area' || elemento === 'form';
  let nota = '';
  if (elemento === 'link') {
    const rels = (valorDe(attrs, 'rel') ?? '').toLowerCase().split(/\s+/).filter(Boolean);
    if (rels.some((r) => REL_RECURSO.has(r))) pagina = false;
    else if (rels.some((r) => REL_PAGINA.has(r))) pagina = true;
    else {
      pagina = true;
      nota = ` (link rel="${rels.join(' ')}")`;
    }
  }
  const out = [];
  for (const at of attrs.matchAll(ATRIBUTO)) {
    const nomeAt = at[1].toLowerCase();
    const valor = desfazer(at[3] ?? at[4] ?? '').trim();
    const valores = nomeAt === 'srcset' ? valor.split(',').map((v) => v.trim().split(/\s+/)[0]).filter(Boolean) : [valor];
    const ehLink = pagina && ['href', 'xlink:href', 'action'].includes(nomeAt);
    for (const v of valores) out.push({ v, como: `<${elemento} ${nomeAt}="${v}">`, pagina: ehLink, nota: ehLink ? nota : '' });
  }
  return out;
}

for (const arquivo of paginas) {
  const rel = relative(dist, arquivo).split(sep).join('/');
  const urlPagina = `${ORIGEM}${base}${rel.endsWith('index.html') ? rel.slice(0, -'index.html'.length) : rel}`;
  const html = marcacao(readFileSync(arquivo, 'utf8'));
  for (const tag of html.matchAll(ATRIBUTOS)) {
    for (const { v, como, pagina, nota } of enderecos(tag[1].toLowerCase(), tag[2])) {
      if (!v || v.startsWith('data:') || v.startsWith('javascript:')) continue;
      let u;
      try {
        u = new URL(v, urlPagina);
      } catch {
        quebrados.push(`${rel}: endereço inválido em ${como}`);
        continue;
      }
      if (u.origin !== ORIGEM) {
        const chave = u.protocol === 'http:' || u.protocol === 'https:' ? u.host : u.protocol;
        externos.set(chave, (externos.get(chave) ?? 0) + 1);
        continue;
      }
      internos++;
      let caminho;
      try {
        caminho = decodeURIComponent(u.pathname);
      } catch {
        caminho = u.pathname;
      }
      if (!caminho.startsWith(base)) {
        quebrados.push(`${rel}: fora do caminho base ${base}: ${como}`);
        continue;
      }
      const sub = caminho.slice(base.length);
      const alvo = arquivoDe(sub);
      if (!alvo) {
        if (pagina && ehPagina(sub)) {
          const k = `${base}${sub}${nota}`;
          const e = naoPortadas.get(k) ?? { links: 0, de: new Set() };
          e.links++;
          e.de.add(rel);
          naoPortadas.set(k, e);
        } else {
          quebrados.push(`${rel}: arquivo que não existe: ${como}`);
        }
        continue;
      }
      const frag = u.hash.slice(1);
      if (frag && alvo.endsWith('.html')) {
        ancoras++;
        let id;
        try {
          id = decodeURIComponent(frag);
        } catch {
          id = frag;
        }
        // `#top` sem elemento com esse id: o navegador sobe para o alto da
        // página (regra do HTML), então não é quebrado.
        if (!ids(alvo).has(id) && id.toLowerCase() !== 'top') {
          const onde = relative(dist, alvo).split(sep).join('/');
          quebrados.push(`${rel}: âncora #${id} não existe em ${onde}: ${como}`);
        }
      }
    }
  }
}

const linhas = [];
linhas.push('### Links internos do site');
linhas.push('');
linhas.push(`${paginas.length} páginas lidas em \`${base}\`; ${internos} links internos conferidos, ${ancoras} com âncora.`);
linhas.push('');
if (quebrados.length) {
  linhas.push(`**${quebrados.length} quebrados** (o passo falha):`);
  linhas.push('');
  for (const q of quebrados) linhas.push(`- ${q}`);
} else {
  linhas.push('Nenhum link quebrado.');
}
linhas.push('');
if (naoPortadas.size) {
  linhas.push(`Páginas ainda não portadas (só lista): ${naoPortadas.size}.`);
  linhas.push('');
  linhas.push('| Página | links | de quantas páginas |');
  linhas.push('|---|---:|---:|');
  for (const [k, e] of [...naoPortadas].sort((a, b) => b[1].links - a[1].links || a[0].localeCompare(b[0]))) {
    linhas.push(`| \`${k}\` | ${e.links} | ${e.de.size} |`);
  }
} else {
  linhas.push('Nenhum link para página ainda não portada.');
}
linhas.push('');
const totalExternos = [...externos.values()].reduce((a, b) => a + b, 0);
linhas.push(`Externos (contados, não visitados): ${totalExternos}${externos.size ? ` (${[...externos].sort((a, b) => b[1] - a[1]).map(([h, n]) => `${h} ${n}`).join(', ')})` : ''}.`);
linhas.push('');
const texto = `${linhas.join('\n')}\n`;
console.log(texto);
if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, texto);
process.exit(quebrados.length ? 1 : 0);
