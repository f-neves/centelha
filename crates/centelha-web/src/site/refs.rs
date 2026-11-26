//! As referências do glossário no corpo do texto, marcadas no gerador.
//!
//! Port do auto-link de `src/components/Referencias.astro` do site atual, que
//! faz isto no navegador depois de baixar um índice de 68 KB. Aqui o gerador
//! marca o HTML pronto do capítulo com a mesma regra, e o texto da página não
//! muda (o termo só ganha um `<a>` em volta):
//!
//! - os termos mais longos primeiro (o nome do termo e os sinônimos, na ordem
//!   do glossário), sem acento e sem caixa, só em palavra inteira (antes e
//!   depois, nada de `a` a `z` nem algarismo), sem sobrepor;
//! - cada termo uma vez por bloco (`p`, `li`, `td`, `th`, `blockquote`,
//!   `figcaption`, `dd`, `dt`, `caption`, `summary`, `h5`, `h6`; fora deles, o
//!   bloco é a página);
//! - pula o texto cujo pai direto é `a`, `code`, `pre`, botão, campo de
//!   formulário, `h1` a `h4`, `script`, `style`, `option` ou `label`, o que
//!   estiver dentro de `.ref`, `.no-gloss`, `.arv-node` ou `.badge`, e o texto
//!   de até dois caracteres (sem os espaços das pontas).
//!
//! Uma diferença de propósito: o texto dos diagramas (dentro de `<svg>`) não é
//! marcado. No site atual o link entra dentro do `<tspan>` do desenho, onde o
//! navegador não o desenha, e o rótulo some.
//!
//! Primeira etapa: só os termos do glossário. Atributos, Habilidades,
//! Virtudes, Caminhos, Técnicas e Artes entram com os módulos da fase 2.

use std::collections::{BTreeSet, HashMap, HashSet};

use centelha_data::GlossaryEntry;

use super::escape_html;
use super::nav;
use super::text::decode_entities;

/// Pai direto que faz o texto ser pulado.
const SKIP: &[&str] = &[
    "a", "code", "pre", "button", "input", "select", "textarea", "h1", "h2", "h3", "h4", "script",
    "style", "option", "label",
];
/// Elementos que formam um bloco (o escopo de "uma vez por termo").
const BLOCK: &[&str] = &[
    "p",
    "li",
    "td",
    "th",
    "blockquote",
    "figcaption",
    "dd",
    "dt",
    "caption",
    "summary",
    "h5",
    "h6",
];
/// Classes que fazem pular tudo o que estiver dentro.
const SKIP_CLASSES: &[&str] = &["ref", "no-gloss", "arv-node", "badge"];
/// Elementos sem fechamento.
const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

/// Um termo marcado: em que bloco e com que texto (para a comparação com o
/// site atual).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mark {
    pub id: String,
    pub text: String,
}

/// A marcação de um trecho de HTML.
#[derive(Debug, Clone, Default)]
pub struct Marked {
    pub html: String,
    pub marks: Vec<Mark>,
}

impl Marked {
    /// Os ids marcados, sem repetir.
    pub fn ids(&self) -> BTreeSet<String> {
        self.marks.iter().map(|m| m.id.clone()).collect()
    }
}

struct Needle {
    folded: Vec<char>,
    entry: usize,
}

/// O glossário pronto para marcar.
pub struct Refs<'a> {
    base: &'a str,
    entries: &'a [GlossaryEntry],
    needles: Vec<Needle>,
}

struct Element {
    name: String,
    skip_inside: bool,
    uid: usize,
}

impl<'a> Refs<'a> {
    /// `entries` na ordem do arquivo (a ordem de desempate entre termos do
    /// mesmo tamanho, como no site atual).
    pub fn new(base: &'a str, entries: &'a [GlossaryEntry]) -> Refs<'a> {
        let mut needles = Vec::new();
        for (i, e) in entries.iter().enumerate() {
            for needle in std::iter::once(&e.term).chain(e.aliases.iter()) {
                let folded = fold(needle);
                if !folded.is_empty() {
                    needles.push(Needle { folded, entry: i });
                }
            }
        }
        // Ordenação estável, do mais longo para o mais curto (o tamanho do
        // termo original, que no português é o mesmo do termo sem acento).
        needles.sort_by_key(|n| std::cmp::Reverse(n.folded.len()));
        Refs {
            base,
            entries,
            needles,
        }
    }

    /// Marca o HTML de um capítulo.
    pub fn mark(&self, html: &str) -> Marked {
        let mut out = Marked {
            html: String::with_capacity(html.len() + html.len() / 8),
            marks: Vec::new(),
        };
        let mut stack: Vec<Element> = Vec::new();
        let mut used: HashMap<usize, HashSet<usize>> = HashMap::new();
        let mut next_uid = 1usize;
        let mut rest = html;
        while !rest.is_empty() {
            if let Some(after) = rest.strip_prefix("<!--") {
                let end = after.find("-->").map(|i| i + 3).unwrap_or(after.len());
                let len = 4 + end;
                out.html.push_str(&rest[..len]);
                rest = &rest[len..];
                continue;
            }
            if rest.starts_with('<')
                && rest[1..].starts_with(|c: char| c == '/' || c.is_ascii_alphabetic())
            {
                let len = tag_len(rest);
                let tag = &rest[..len];
                out.html.push_str(tag);
                rest = &rest[len..];
                let (closing, name) = tag_name(tag);
                if closing {
                    if let Some(pos) = stack.iter().rposition(|e| e.name == name) {
                        stack.truncate(pos);
                    }
                    continue;
                }
                // `<svg>`, `<script>` e `<style>` passam inteiros, sem marcar.
                if matches!(name.as_str(), "svg" | "script" | "style") {
                    let close = format!("</{name}");
                    let end = find_ci(rest, &close)
                        .map(|i| i + tag_len(&rest[i..]))
                        .unwrap_or(rest.len());
                    out.html.push_str(&rest[..end]);
                    rest = &rest[end..];
                    continue;
                }
                if VOID.contains(&name.as_str()) || tag.ends_with("/>") {
                    continue;
                }
                let skip_inside = classes(tag)
                    .iter()
                    .any(|c| SKIP_CLASSES.contains(&c.as_str()));
                stack.push(Element {
                    name,
                    skip_inside,
                    uid: next_uid,
                });
                next_uid += 1;
                continue;
            }
            // Texto até a próxima etiqueta ou comentário.
            let end = rest
                .char_indices()
                .skip(1)
                .find(|&(_, c)| c == '<')
                .map(|(i, _)| i)
                .unwrap_or(rest.len());
            let raw = &rest[..end];
            rest = &rest[end..];
            self.mark_text(raw, &stack, &mut used, &mut out);
        }
        out
    }

    fn mark_text(
        &self,
        raw: &str,
        stack: &[Element],
        used: &mut HashMap<usize, HashSet<usize>>,
        out: &mut Marked,
    ) {
        let parent_skips = stack
            .last()
            .is_some_and(|p| SKIP.contains(&p.name.as_str()));
        let inside_skip = stack.iter().any(|e| e.skip_inside);
        let text = decode_entities(raw);
        if parent_skips || inside_skip || text.trim().chars().count() <= 2 {
            out.html.push_str(raw);
            return;
        }
        let block = stack
            .iter()
            .rev()
            .find(|e| BLOCK.contains(&e.name.as_str()))
            .map(|e| e.uid)
            .unwrap_or(0);
        let used = used.entry(block).or_default();
        let chars: Vec<char> = text.chars().collect();
        let folded: Vec<char> = chars.iter().map(|&c| fold_char(c)).collect();
        let mut taken = vec![false; folded.len()];
        let mut hits: Vec<(usize, usize, usize)> = Vec::new();
        for needle in &self.needles {
            if used.contains(&needle.entry) {
                continue;
            }
            let n = &needle.folded;
            let mut from = 0;
            while let Some(i) = find(&folded, n, from) {
                let end = i + n.len();
                let before = if i == 0 { ' ' } else { folded[i - 1] };
                let after = folded.get(end).copied().unwrap_or(' ');
                let overlap = taken[i..end].iter().any(|&t| t);
                if !overlap && !is_word(before) && !is_word(after) {
                    taken[i..end].iter_mut().for_each(|t| *t = true);
                    hits.push((i, n.len(), needle.entry));
                    used.insert(needle.entry);
                    break;
                }
                from = i + 1;
            }
        }
        if hits.is_empty() {
            out.html.push_str(raw);
            return;
        }
        hits.sort_unstable();
        let mut pos = 0;
        for (start, len, entry) in hits {
            let e = &self.entries[entry];
            let piece: String = chars[pos..start].iter().collect();
            out.html.push_str(&escape_html(&piece));
            let matched: String = chars[start..start + len].iter().collect();
            out.html.push_str(&format!(
                "<a class=\"ref\" href=\"{}\" data-ref=\"{}\" title=\"{}\">{}</a>",
                nav::url(self.base, &format!("glossario#{}", e.id)),
                escape_html(&e.id),
                escape_html(&e.definition),
                escape_html(&matched),
            ));
            out.marks.push(Mark {
                id: e.id.clone(),
                text: matched,
            });
            pos = start + len;
        }
        let tail: String = chars[pos..].iter().collect();
        out.html.push_str(&escape_html(&tail));
    }
}

/// Sem acento e em minúscula, caractere a caractere (o tamanho não muda, como
/// no `normalize('NFD')` do site atual sobre texto já composto).
pub fn fold(s: &str) -> Vec<char> {
    s.chars().map(fold_char).collect()
}

fn fold_char(c: char) -> char {
    let lower = c.to_lowercase().next().unwrap_or(c);
    match lower {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => 'a',
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => 'c',
        'ď' => 'd',
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => 'e',
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => 'g',
        'ĥ' => 'h',
        'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' => 'i',
        'ĵ' => 'j',
        'ķ' => 'k',
        'ĺ' | 'ļ' | 'ľ' => 'l',
        'ñ' | 'ń' | 'ņ' | 'ň' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ō' | 'ŏ' | 'ő' => 'o',
        'ŕ' | 'ŗ' | 'ř' => 'r',
        'ś' | 'ŝ' | 'ş' | 'š' => 's',
        'ţ' | 'ť' => 't',
        'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => 'u',
        'ŵ' => 'w',
        'ý' | 'ÿ' | 'ŷ' => 'y',
        'ź' | 'ż' | 'ž' => 'z',
        other => other,
    }
}

/// O que conta como letra de palavra na fronteira (como `[a-z0-9]` no site
/// atual, depois de tirar o acento).
fn is_word(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit()
}

fn find(hay: &[char], needle: &[char], from: usize) -> Option<usize> {
    if needle.len() > hay.len() {
        return None;
    }
    (from..=hay.len() - needle.len()).find(|&i| hay[i..i + needle.len()] == *needle)
}

/// O tamanho de uma etiqueta que começa em `s` (até o `>` fora de aspas).
fn tag_len(s: &str) -> usize {
    let mut quote: Option<char> = None;
    for (i, c) in s.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '>') => return i + 1,
            _ => {}
        }
    }
    s.len()
}

/// Se é fechamento, e o nome em minúsculas.
fn tag_name(tag: &str) -> (bool, String) {
    let inner = tag.trim_start_matches('<');
    let (closing, inner) = match inner.strip_prefix('/') {
        Some(rest) => (true, rest),
        None => (false, inner),
    };
    let name: String = inner
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    (closing, name.to_ascii_lowercase())
}

/// As classes de uma etiqueta de abertura.
fn classes(tag: &str) -> Vec<String> {
    let lower = tag.to_ascii_lowercase();
    let Some(i) = lower.find("class=") else {
        return Vec::new();
    };
    let rest = &tag[i + 6..];
    let value = match rest.chars().next() {
        Some(q @ ('"' | '\'')) => rest[1..].split(q).next().unwrap_or(""),
        _ => rest.split([' ', '>']).next().unwrap_or(""),
    };
    value.split_whitespace().map(str::to_string).collect()
}

fn find_ci(hay: &str, needle: &str) -> Option<usize> {
    hay.to_ascii_lowercase().find(&needle.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, term: &str, aliases: &[&str]) -> GlossaryEntry {
        GlossaryEntry {
            id: id.into(),
            term: term.into(),
            aliases: aliases.iter().map(|s| s.to_string()).collect(),
            definition: format!("def {id}"),
        }
    }

    fn glossary() -> Vec<GlossaryEntry> {
        vec![
            entry("ticks", "Ticks", &["velocidade", "linha do tempo"]),
            entry("defesa", "Defesa", &["esquiva"]),
            entry("defesa-mental", "Defesa Mental", &[]),
            entry("pool", "Pool", &["dados"]),
        ]
    }

    fn marked(html: &str) -> Marked {
        let g = glossary();
        Refs::new("/b/", &g).mark(html)
    }

    #[test]
    fn longest_first_once_per_block_whole_words() {
        let m = marked("<p>A Defesa Mental e a Defesa; Defesas não.</p><p>Defesa outra vez.</p>");
        assert_eq!(
            m.marks,
            [
                Mark {
                    id: "defesa-mental".into(),
                    text: "Defesa Mental".into()
                },
                Mark {
                    id: "defesa".into(),
                    text: "Defesa".into()
                },
                Mark {
                    id: "defesa".into(),
                    text: "Defesa".into()
                },
            ]
        );
        assert!(m.html.contains(
            "<a class=\"ref\" href=\"/b/glossario/#defesa-mental\" data-ref=\"defesa-mental\" title=\"def defesa-mental\">Defesa Mental</a>"
        ));
    }

    #[test]
    fn accents_and_case_do_not_matter() {
        let m = marked("<p>A VELOCIDADE e a linha do tempo.</p>");
        // Uma vez por bloco: o segundo sinônimo do mesmo termo não marca.
        assert_eq!(
            m.marks,
            [Mark {
                id: "ticks".into(),
                text: "linha do tempo".into()
            }]
        );
    }

    #[test]
    fn skips_links_code_headings_and_classes() {
        let m = marked(
            "<h2>Defesa</h2><p><a href=\"#\">Defesa</a> <code>pool</code></p><div class=\"no-gloss\"><p>Pool</p></div><p><span class=\"badge\">Pool</span></p>",
        );
        assert!(m.marks.is_empty(), "{:?}", m.marks);
    }

    #[test]
    fn only_the_direct_parent_counts_like_the_current_site() {
        // Dentro de <strong> dentro de <a>, o pai direto é <strong>: marca.
        let m = marked("<p><a href=\"#\"><strong>Pool</strong></a></p>");
        assert_eq!(m.marks.len(), 1);
    }

    #[test]
    fn svg_text_and_short_text_are_left_alone() {
        let m = marked("<figure><svg><text>Pool</text></svg></figure><p>Po</p>");
        assert!(m.marks.is_empty());
    }

    #[test]
    fn text_is_unchanged() {
        let html = "<p>Os dados &amp; a Defesa &lt;sempre&gt;.</p>";
        let m = marked(html);
        assert_eq!(
            super::super::text::html_to_text(&m.html),
            super::super::text::html_to_text(html)
        );
        assert_eq!(m.marks.len(), 2);
    }
}
