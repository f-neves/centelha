// O script do <head>, antes da primeira pintura: aplica o tema gravado, a barra
// lateral recolhida e as preferências de diagramação (`centelha:config`), com as
// mesmas chaves do `localStorage` do site atual (mesma origem: quem ajustou lá
// traz as escolhas). Port do script do <head> de src/layouts/Base.astro e de
// aplicarConfig() de src/lib/config-site.ts e cssDasCores() de
// src/lib/cores-site.ts. O padrão é o tema escuro, que já vem no <html>.
(function () {
  var d = document.documentElement;
  try {
    d.dataset.theme = localStorage.getItem('tema') || 'escuro';
    if (localStorage.getItem('sidebar') === 'collapsed') d.classList.add('sidebar-collapsed');
    var c = JSON.parse(localStorage.getItem('centelha:config') || '{}') || {};
    var st = d.style;
    if (c.largTexto) st.setProperty('--cfg-larg-texto', c.largTexto + 'rem');
    if (c.largTabela === 0) st.setProperty('--cfg-larg-tabela', 'var(--cfg-larg-texto, 46rem)');
    else if (c.largTabela) st.setProperty('--cfg-larg-tabela', c.largTabela + 'rem');
    if (c.largFicha === 0) st.setProperty('--cfg-larg-ficha', '200rem');
    else if (c.largFicha) st.setProperty('--cfg-larg-ficha', c.largFicha + 'rem');
    if (c.fonteCorpo) st.setProperty('--fs-corpo', (c.fonteCorpo / 16) + 'rem');
    if (c.entrelinha) st.setProperty('--cfg-entrelinha', String(c.entrelinha));
    if (c.margem != null) st.setProperty('--cfg-margem', c.margem + 'rem');
    if (c.colunas === 2) d.dataset.colunas = '2';
    // O fundo decorativo: sem atributo vale "site", o padrão.
    d.dataset.fundo = c.fundo === 'ficha' ? 'ficha' : 'site';
    if (c.decoForca != null) st.setProperty('--deco-strength', String(c.decoForca));
    if (c.justificar === false) d.dataset.justif = '0';
    if (c.alinha === 'centro') d.dataset.alinha = 'centro';
    if (c.barra && c.barra !== 'fina') d.dataset.barra = c.barra;
    // A paleta editada: uma folha por tema, só com nomes e cores válidos.
    if (c.cores) {
      var css = '', arrow = { classico: 'accent-strong', escuro: 'accent', legivel: 'accent-strong' };
      ['classico', 'escuro', 'legivel'].forEach(function (tm) {
        var m = c.cores[tm];
        if (!m) return;
        var l = [];
        for (var k in m) if (/^[a-z0-9-]+$/.test(k) && /^#[0-9a-f]{6}$/i.test(m[k])) l.push('--' + k + ':' + m[k]);
        if (!l.length) return;
        var s = m[arrow[tm]];
        if (s && /^#[0-9a-f]{6}$/i.test(s)) {
          l.push("--select-arrow:url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 10 6'%3E%3Cpath d='M1 1l4 4 4-4' fill='none' stroke='" + encodeURIComponent(s) + "' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E\")");
        }
        css += "html[data-theme='" + tm + "']{" + l.join(';') + '}\n';
      });
      if (css) {
        var tag = document.createElement('style');
        tag.id = 'cfg-cores';
        tag.textContent = css;
        document.head.appendChild(tag);
      }
    }
  } catch (e) {
    d.dataset.theme = d.dataset.theme || 'classico';
  }
})();
