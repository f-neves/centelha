//! Os `id` dos títulos das páginas que não vêm do Markdown (a inicial,
//! Equipamentos, as das Artes) e o "Nesta página" delas.
//!
//! No site atual, o `NestaPagina.astro` dá `id` no navegador aos `<h2>` e
//! `<h3>` do `<main>` que não têm, a partir do texto (sem acento, em
//! minúscula, o que não é letra nem algarismo vira hífen, e `-2`, `-3` no
//! repetido), e mostra o "Nesta página" quando há dois títulos ou mais. Aqui
//! o gerador faz o mesmo no HTML: os endereços com essas âncoras continuam
//! valendo, e o sumário sai pronto.

use std::collections::HashSet;

use crate::toc::TocEntry;

/// O `slug` do `NestaPagina.astro`.
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in super::refs::fold(text) {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            if dash && !out.is_empty() {
                out.push('-');
            }
            dash = false;
            out.push(c);
        } else {
            dash = true;
        }
    }
    out
}

/// O texto de um trecho de HTML (sem etiquetas, com as entidades comuns
/// desfeitas), como o `textContent`.
fn text_of(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

fn existing_ids(html: &str) -> HashSet<String> {
    let mut ids = HashSet::new();
    let mut rest = html;
    while let Some(i) = rest.find(" id=\"") {
        rest = &rest[i + 5..];
        if let Some(end) = rest.find('"') {
            ids.insert(rest[..end].to_string());
            rest = &rest[end..];
        }
    }
    ids
}

/// Dá `id` aos `<h2>` e `<h3>` de `html` que não têm e devolve o HTML novo e
/// os títulos (todos os que têm `id`, na ordem), para o "Nesta página".
pub fn add_ids(html: &str) -> (String, Vec<TocEntry>) {
    let mut seen = existing_ids(html);
    let mut out = String::with_capacity(html.len() + 256);
    let mut entries = Vec::new();
    let mut rest = html;
    loop {
        let next = ["<h2", "<h3"]
            .iter()
            .filter_map(|t| rest.find(t).map(|i| (i, *t)))
            .min_by_key(|(i, _)| *i);
        let Some((at, tag)) = next else {
            out.push_str(rest);
            break;
        };
        let after = &rest[at + 3..];
        if !after.starts_with(['>', ' ']) {
            out.push_str(&rest[..at + 3]);
            rest = after;
            continue;
        }
        let open_end = at + 3 + after.find('>').unwrap_or(0);
        let attrs = &rest[at + 3..open_end];
        let close = format!("</{}>", &tag[1..]);
        let body_end = rest[open_end..]
            .find(&close)
            .map(|i| open_end + i)
            .unwrap_or(rest.len());
        let text = text_of(&rest[open_end + 1..body_end]).trim().to_string();
        let level = if tag == "<h2" { 2 } else { 3 };
        let id = match attrs.split_once(" id=\"") {
            Some((_, v)) => v.split('"').next().unwrap_or("").to_string(),
            None => {
                let base = Some(slug(&text))
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "sec".to_string());
                let mut id = base.clone();
                let mut n = 2;
                while seen.contains(&id) {
                    id = format!("{base}-{n}");
                    n += 1;
                }
                seen.insert(id.clone());
                id
            }
        };
        out.push_str(&rest[..at + 3]);
        if !attrs.contains(" id=\"") {
            out.push_str(&format!(" id=\"{id}\""));
        }
        out.push_str(&rest[at + 3..open_end]);
        rest = &rest[open_end..];
        entries.push(TocEntry { id, level, text });
    }
    (out, entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_like_the_current_site() {
        assert_eq!(slug("Armas à Distância"), "armas-a-distancia");
        assert_eq!(slug("Fogo 8 Efeitos"), "fogo-8-efeitos");
        assert_eq!(slug("  Os Capítulos "), "os-capitulos");
    }

    #[test]
    fn adds_missing_ids_and_lists_headings() {
        let (html, toc) = add_ids(
            "<h2>Escudos</h2><p id=\"escudos\">x</p><h3 class=\"t\">Fogo <span>8</span></h3><h2 id=\"a\">A</h2>",
        );
        assert_eq!(
            html,
            "<h2 id=\"escudos-2\">Escudos</h2><p id=\"escudos\">x</p><h3 id=\"fogo-8\" class=\"t\">Fogo <span>8</span></h3><h2 id=\"a\">A</h2>"
        );
        let ids: Vec<&str> = toc.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["escudos-2", "fogo-8", "a"]);
        assert_eq!(toc[1].text, "Fogo 8");
    }
}
