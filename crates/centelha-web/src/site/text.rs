//! O texto de um HTML, sem as etiquetas: usado pela prova do texto completo.
//!
//! Tira etiquetas, comentários e o conteúdo de `<script>` e `<style>`,
//! decodifica as entidades e junta todo espaço em branco num espaço só. Não é
//! um leitor de HTML completo: basta para o HTML que o próprio gerador grava.

use pulldown_cmark::{Event, Tag, TagEnd};

/// O texto do HTML, com o espaço em branco normalizado.
pub fn html_to_text(html: &str) -> String {
    let mut raw = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(lt) = rest.find('<') {
        raw.push_str(&rest[..lt]);
        rest = &rest[lt..];
        if let Some(after) = rest.strip_prefix("<!--") {
            rest = after.find("-->").map(|end| &after[end + 3..]).unwrap_or("");
            continue;
        }
        let Some(gt) = tag_end(rest) else {
            // `<` sem etiqueta: é texto.
            raw.push('<');
            rest = &rest[1..];
            continue;
        };
        let tag = &rest[1..gt];
        rest = &rest[gt + 1..];
        let name: String = tag
            .trim_start_matches('/')
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        if !tag.starts_with('/') && (name == "script" || name == "style") {
            let close = format!("</{name}");
            let lower = rest.to_ascii_lowercase();
            rest = match lower.find(&close) {
                Some(pos) => {
                    let after = &rest[pos..];
                    after.find('>').map(|g| &after[g + 1..]).unwrap_or("")
                }
                None => "",
            };
        }
        // Etiqueta de bloco separa palavras; a de texto (negrito, link) não.
        if !INLINE.contains(&name.as_str()) {
            raw.push(' ');
        }
    }
    raw.push_str(rest);
    normalize_space(&decode_entities(&raw))
}

/// As etiquetas de texto: não separam palavras.
pub const INLINE: &[&str] = &[
    "a", "abbr", "b", "bdi", "bdo", "cite", "code", "data", "del", "dfn", "em", "i", "ins", "kbd",
    "mark", "q", "s", "samp", "small", "span", "strong", "sub", "sup", "time", "u", "var", "wbr",
];

/// O texto de referência de um capítulo: o que os eventos do Markdown (já com
/// as trocas) dizem, lido sem passar pelo HTML que o gerador escreve. Texto e
/// código entram escapados, o HTML cru entra como está, cada bloco separa
/// palavras e o texto de imagem fica de fora (vira atributo `alt`).
pub fn events_to_text(events: &[Event<'_>]) -> String {
    let mut pseudo = String::new();
    let mut image = 0usize;
    for event in events {
        match event {
            Event::Start(Tag::Image { .. }) => image += 1,
            Event::End(TagEnd::Image) => image = image.saturating_sub(1),
            _ if image > 0 => {}
            Event::Text(t) | Event::Code(t) => {
                for c in t.chars() {
                    match c {
                        '&' => pseudo.push_str("&amp;"),
                        '<' => pseudo.push_str("&lt;"),
                        '>' => pseudo.push_str("&gt;"),
                        c => pseudo.push(c),
                    }
                }
            }
            Event::Html(h) | Event::InlineHtml(h) => pseudo.push_str(h),
            Event::SoftBreak | Event::HardBreak | Event::Rule => pseudo.push('\n'),
            Event::Start(tag) if !is_inline_tag(tag) => pseudo.push('\n'),
            Event::End(tag) if !is_inline_end(tag) => pseudo.push('\n'),
            _ => {}
        }
    }
    html_to_text(&pseudo)
}

fn is_inline_tag(tag: &Tag<'_>) -> bool {
    matches!(
        tag,
        Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Link { .. } | Tag::Image { .. }
    )
}

fn is_inline_end(tag: &TagEnd) -> bool {
    matches!(
        tag,
        TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link | TagEnd::Image
    )
}

/// O fim de uma etiqueta que começa em `s[0] == '<'`, respeitando aspas nos
/// atributos. `None` quando o `<` não abre etiqueta.
fn tag_end(s: &str) -> Option<usize> {
    let mut chars = s.char_indices().skip(1);
    let (_, first) = chars.next()?;
    if !(first.is_ascii_alphabetic() || first == '/' || first == '!') {
        return None;
    }
    let mut quote: Option<char> = None;
    for (i, c) in chars {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == '>' => return Some(i),
            None => {}
        }
    }
    None
}

/// Junta todo espaço em branco num espaço só e apara as pontas.
pub fn normalize_space(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Decodifica as entidades numéricas e as nomeadas que o HTML do site usa.
pub fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let decoded = rest[1..]
            .find(';')
            .filter(|&end| end <= 10)
            .and_then(|end| {
                let name = &rest[1..1 + end];
                decode_one(name).map(|c| (c, end + 2))
            });
        match decoded {
            Some((c, used)) => {
                out.push(c);
                rest = &rest[used..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn decode_one(name: &str) -> Option<char> {
    if let Some(num) = name.strip_prefix('#') {
        let code = match num.strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
            None => num.parse().ok()?,
        };
        return char::from_u32(code);
    }
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => '\u{a0}',
        "mdash" => '\u{2014}',
        "ndash" => '\u{2013}',
        "hellip" => '\u{2026}',
        "middot" => '\u{b7}',
        "laquo" => '\u{ab}',
        "raquo" => '\u{bb}',
        "times" => '\u{d7}',
        "minus" => '\u{2212}',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_tags_and_decodes() {
        let html = "<p class=\"a>b\">Um &amp; <b>dois</b><!-- x -->\n três&nbsp;4 &#8212; &#x2014;</p><style>p{}</style><script>var a='<p>';</script>fim < meio";
        assert_eq!(
            html_to_text(html),
            "Um & dois três 4 \u{2014} \u{2014} fim < meio"
        );
        assert_eq!(html_to_text("<p><b>a</b>b</p><p>c</p>"), "ab c");
    }
}
