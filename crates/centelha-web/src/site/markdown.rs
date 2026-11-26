//! O Markdown dos capítulos vira HTML aqui, e só aqui (no gerador).
//!
//! Reproduz o que o site atual faz com o mesmo texto:
//! - GFM (tabelas, riscado, listas de tarefa);
//! - pontuação tipográfica (`typography`);
//! - o `id` de cada título (`slug`);
//! - o bloco `mermaid` trocado pelo SVG já desenhado em
//!   `content/data/diagramas.json`, pela mesma chave (os 12 primeiros
//!   caracteres hexadecimais do SHA-1 do código do diagrama, com fim de linha
//!   LF e sem os espaços do fim);
//! - o caminho base nos links do Markdown que começam com `/`;
//! - cada tabela envolvida num `<div class="table-wrap">`, que rola sozinho
//!   no celular.
//!
//! Os comentários `<!-- gen:... -->` e o HTML escrito no próprio Markdown
//! passam como estão, com uma troca só: o endereço do site atual
//! (`/centelha-rpg/`) nos `href` vira o caminho base deste site.

use std::collections::HashMap;

use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd, TextMergeStream};
use sha1::{Digest, Sha1};

use super::slug::Slugger;
use super::typography;

/// O caminho base do site atual, que o HTML escrito nos capítulos traz
/// gravado nos links (lá o caminho base não chega no HTML cru).
pub const SOURCE_BASE: &str = "/centelha-rpg/";

/// `diagramas.json` não é um objeto de chave para SVG.
#[derive(Debug)]
pub struct DiagramsError(leptos::serde_json::Error);

impl std::fmt::Display for DiagramsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "diagramas.json: {}", self.0)
    }
}

impl std::error::Error for DiagramsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

/// Os diagramas já desenhados, pela chave.
#[derive(Debug, Default, Clone)]
pub struct Diagrams {
    by_key: HashMap<String, String>,
}

impl Diagrams {
    /// Lê o texto de `diagramas.json` (um objeto de chave para SVG).
    pub fn parse(json: &str) -> Result<Diagrams, DiagramsError> {
        let by_key: HashMap<String, String> =
            leptos::serde_json::from_str(json).map_err(DiagramsError)?;
        Ok(Diagrams { by_key })
    }

    /// O SVG do diagrama com este código, se houver.
    pub fn get(&self, code: &str) -> Option<&str> {
        self.by_key.get(&diagram_key(code)).map(String::as_str)
    }

    /// Quantos diagramas há.
    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    /// Se não há nenhum.
    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }
}

/// A chave de um diagrama, como o site atual calcula.
pub fn diagram_key(code: &str) -> String {
    let normalized = code.replace("\r\n", "\n");
    let digest = Sha1::digest(normalized.trim_end().as_bytes());
    let mut hex = String::with_capacity(12);
    for byte in digest.iter().take(6) {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// O que a conversão precisa saber do site.
pub struct Markdown<'a> {
    /// O caminho base, com `/` no começo e no fim (`/centelha/` ou `/`).
    pub base: &'a str,
    pub diagrams: &'a Diagrams,
}

impl Markdown<'_> {
    /// Os eventos do Markdown depois de todas as trocas, prontos para virar
    /// HTML. A prova do texto completo lê os mesmos eventos.
    pub fn events<'t>(&self, body: &'t str) -> Vec<Event<'t>> {
        let parser = Parser::new_ext(body, options());
        let events: Vec<Event<'t>> = TextMergeStream::new(parser).collect();
        let events = self.replace_diagrams(events);
        let events = self.prefix_links(events);
        let events = wrap_tables(events);
        let events = typography_pass(events);
        heading_ids(events)
    }

    /// O HTML do corpo de um capítulo.
    pub fn to_html(&self, body: &str) -> String {
        let mut html = String::with_capacity(body.len() * 3 / 2);
        pulldown_cmark::html::push_html(&mut html, self.events(body).into_iter());
        html
    }

    fn replace_diagrams<'t>(&self, events: Vec<Event<'t>>) -> Vec<Event<'t>> {
        let mut out = Vec::with_capacity(events.len());
        let mut code: Option<String> = None;
        for event in events {
            match event {
                Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref info)))
                    if info.split_whitespace().next() == Some("mermaid") =>
                {
                    code = Some(String::new());
                }
                Event::Text(ref text) if code.is_some() => {
                    if let Some(code) = code.as_mut() {
                        code.push_str(text);
                    }
                }
                Event::End(TagEnd::CodeBlock) if code.is_some() => {
                    let source = code.take().unwrap_or_default();
                    let html = match self.diagrams.get(&source) {
                        Some(svg) => format!("<figure class=\"diagrama-caixa\">{svg}</figure>\n"),
                        None => "<p class=\"erro\">Diagrama sem desenho gravado. Rode <code>node scripts/gen-mermaid.mjs</code>.</p>\n".to_string(),
                    };
                    out.push(Event::Html(html.into()));
                }
                other => out.push(other),
            }
        }
        out
    }

    fn prefix_links<'t>(&self, events: Vec<Event<'t>>) -> Vec<Event<'t>> {
        let base_no_slash = self.base.trim_end_matches('/');
        events
            .into_iter()
            .map(|event| match event {
                Event::Start(Tag::Link {
                    link_type,
                    dest_url,
                    title,
                    id,
                }) => {
                    let dest_url = prefix_href(&dest_url, base_no_slash)
                        .map(CowStr::from)
                        .unwrap_or(dest_url);
                    Event::Start(Tag::Link {
                        link_type,
                        dest_url,
                        title,
                        id,
                    })
                }
                Event::Html(html) => Event::Html(self.rebase_raw_html(html)),
                Event::InlineHtml(html) => Event::InlineHtml(self.rebase_raw_html(html)),
                other => other,
            })
            .collect()
    }

    fn rebase_raw_html<'t>(&self, html: CowStr<'t>) -> CowStr<'t> {
        let needle = format!("href=\"{SOURCE_BASE}");
        if html.contains(&needle) {
            html.replace(&needle, &format!("href=\"{}", self.base))
                .into()
        } else {
            html
        }
    }
}

/// O link com o caminho base, se ele precisar (o mesmo critério do site
/// atual: começa com `/`, não com `//`, e ainda não tem o caminho base).
fn prefix_href(href: &str, base_no_slash: &str) -> Option<String> {
    if base_no_slash.is_empty() {
        return None;
    }
    let has_base = href == base_no_slash || href.starts_with(&format!("{base_no_slash}/"));
    if href.starts_with('/') && !href.starts_with("//") && !has_base {
        Some(format!("{base_no_slash}{href}"))
    } else {
        None
    }
}

fn options() -> Options {
    Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS
}

fn wrap_tables(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    let mut out = Vec::with_capacity(events.len());
    for event in events {
        match event {
            Event::Start(Tag::Table(_)) => {
                out.push(Event::Html("<div class=\"table-wrap\">".into()));
                out.push(event);
            }
            Event::End(TagEnd::Table) => {
                out.push(event);
                out.push(Event::Html("</div>\n".into()));
            }
            other => out.push(other),
        }
    }
    out
}

/// Aplica a pontuação tipográfica nos eventos de texto. O texto emendado
/// segue a árvore do site atual: parágrafo começa com espaço (inclusive o
/// parágrafo implícito do item de lista curta), código em linha conta como
/// letras, quebra de linha conta como `\n`, código em bloco e texto de
/// imagem ficam de fora.
fn typography_pass(mut events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    let mut all: Vec<char> = Vec::new();
    // (índice do evento, começo no texto emendado, tamanho em caracteres)
    let mut pieces: Vec<(usize, usize, usize)> = Vec::new();
    let mut code_block = 0usize;
    let mut image = 0usize;

    for i in 0..events.len() {
        match &events[i] {
            Event::Start(Tag::CodeBlock(_)) => code_block += 1,
            Event::End(TagEnd::CodeBlock) => code_block = code_block.saturating_sub(1),
            Event::Start(Tag::Image { .. }) => image += 1,
            Event::End(TagEnd::Image) => image = image.saturating_sub(1),
            _ => {}
        }
        if code_block > 0 || image > 0 {
            continue;
        }
        match &events[i] {
            Event::Start(Tag::Paragraph) => all.push(' '),
            Event::Start(Tag::Item) => {
                if events.get(i + 1).is_some_and(is_inline) {
                    all.push(' ');
                }
            }
            Event::Text(text) => {
                let start = all.len();
                all.extend(text.chars());
                pieces.push((i, start, all.len() - start));
            }
            Event::Code(code) => all.extend(std::iter::repeat_n('A', code.chars().count())),
            Event::SoftBreak => all.push('\n'),
            _ => {}
        }
    }

    let quoted = typography::quotes(&all);
    for (index, start, len) in pieces {
        let piece: String = quoted[start..start + len].iter().collect();
        events[index] = Event::Text(typography::dashes_and_ellipses(&piece).into());
    }
    events
}

fn is_inline(event: &Event<'_>) -> bool {
    match event {
        Event::Text(_)
        | Event::Code(_)
        | Event::InlineHtml(_)
        | Event::SoftBreak
        | Event::HardBreak
        | Event::TaskListMarker(_) => true,
        Event::Start(tag) => matches!(
            tag,
            Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Link { .. } | Tag::Image { .. }
        ),
        _ => false,
    }
}

/// Dá a cada título o `id` que o site atual daria.
fn heading_ids(mut events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    let mut slugger = Slugger::default();
    let mut i = 0;
    while i < events.len() {
        if matches!(events[i], Event::Start(Tag::Heading { id: None, .. })) {
            let mut text = String::new();
            let mut j = i + 1;
            while j < events.len() && !matches!(events[j], Event::End(TagEnd::Heading(_))) {
                match &events[j] {
                    Event::Text(t) | Event::Code(t) => text.push_str(t),
                    Event::InlineHtml(h) if !is_bare_tag(h) => text.push_str(h),
                    _ => {}
                }
                j += 1;
            }
            let id = slugger.heading_id(&text);
            if let Event::Start(Tag::Heading {
                level,
                classes,
                attrs,
                ..
            }) = std::mem::replace(&mut events[i], Event::SoftBreak)
            {
                events[i] = Event::Start(Tag::Heading {
                    level,
                    id: Some(id.into()),
                    classes,
                    attrs,
                });
            }
            i = j;
        }
        i += 1;
    }
    events
}

/// HTML em linha que é só uma etiqueta (`<span>`, `</b>`): não entra no
/// texto do `id`.
fn is_bare_tag(html: &str) -> bool {
    let h = html.strip_prefix('\n').unwrap_or(html);
    let h = h.strip_suffix('\n').unwrap_or(h);
    h.starts_with('<') && h.ends_with('>') && !h.contains('\n')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn md(base: &str) -> (Diagrams, String) {
        let key = diagram_key("graph TD\nA-->B");
        let diagrams = Diagrams::parse(&format!("{{\"{key}\": \"<svg>d</svg>\"}}")).unwrap();
        (diagrams, base.to_string())
    }

    #[test]
    fn broken_diagrams_json_is_a_typed_error() {
        let err = Diagrams::parse("[\"não é objeto\"]").unwrap_err();
        assert!(err.to_string().starts_with("diagramas.json: "));
        assert!(std::error::Error::source(&err).is_some());
    }

    fn html(base: &str, body: &str) -> String {
        let (diagrams, base) = md(base);
        Markdown {
            base: &base,
            diagrams: &diagrams,
        }
        .to_html(body)
    }

    #[test]
    fn diagram_key_ignores_crlf_and_trailing_space() {
        assert_eq!(diagram_key("a\r\nb\n\n"), diagram_key("a\nb"));
        assert_eq!(diagram_key("abc").len(), 12);
    }

    #[test]
    fn mermaid_becomes_svg() {
        let out = html("/centelha/", "Antes\n\n```mermaid\ngraph TD\nA-->B\n```\n");
        assert!(
            out.contains("<figure class=\"diagrama-caixa\"><svg>d</svg></figure>"),
            "{out}"
        );
        let out = html("/centelha/", "```mermaid\ngraph LR\n```\n");
        assert!(out.contains("class=\"erro\""), "{out}");
    }

    #[test]
    fn links_get_the_base() {
        let out = html(
            "/centelha/",
            "[a](/regras/x) [b](//outro) [c](/centelha/y) [d](#z) [e](http://e)\n",
        );
        assert!(out.contains("href=\"/centelha/regras/x\""), "{out}");
        assert!(out.contains("href=\"//outro\""), "{out}");
        assert!(out.contains("href=\"/centelha/y\""), "{out}");
        assert!(out.contains("href=\"#z\""), "{out}");
        let out = html("/", "[a](/regras/x)\n");
        assert!(out.contains("href=\"/regras/x\""), "{out}");
    }

    #[test]
    fn raw_html_links_from_the_source_site_get_our_base() {
        let out = html(
            "/centelha/",
            "<p>Veja <a href=\"/centelha-rpg/regras/x#y\">x</a>.</p>\n",
        );
        assert!(out.contains("href=\"/centelha/regras/x#y\""), "{out}");
        let out = html("/", "Veja <a href=\"/centelha-rpg/ficha\">a ficha</a>.\n");
        assert!(out.contains("href=\"/ficha\""), "{out}");
    }

    #[test]
    fn tables_are_wrapped() {
        let out = html("/", "| a | b |\n|---|---|\n| 1 | 2 |\n");
        assert!(
            out.starts_with("<div class=\"table-wrap\"><table>"),
            "{out}"
        );
        assert!(out.trim_end().ends_with("</table>\n</div>"), "{out}");
    }

    #[test]
    fn headings_get_ids() {
        let out = html(
            "/",
            "## Preparo, Golpe e Recuperação\n\n## Exemplo\n\n## Exemplo\n",
        );
        assert!(
            out.contains("<h2 id=\"preparo-golpe-e-recuperação\">"),
            "{out}"
        );
        assert!(out.contains("<h2 id=\"exemplo-1\">"), "{out}");
    }

    #[test]
    fn typography() {
        let out = html("/", "Ele disse \"**olá**\" -- e foi...\n");
        assert!(out.contains("“<strong>olá</strong>”"), "{out}");
        assert!(out.contains("\u{2014} e foi\u{2026}"), "{out}");
    }

    #[test]
    fn generated_comments_pass_through() {
        let out = html(
            "/",
            "<!-- gen:x -->\n| a |\n|---|\n| 1 |\n<!-- /gen:x -->\n",
        );
        assert!(out.contains("<!-- gen:x -->"), "{out}");
    }
}
