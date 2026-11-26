//! Os links internos sem destino: o gerador avisa, no fim do build, quantos
//! links apontam para páginas que o site ainda não tem e para onde. É um
//! aviso, não um erro: as páginas que ainda não foram portadas ficam sem
//! destino até a fase delas.

use std::collections::{BTreeMap, HashSet};

use super::Page;

/// Um destino sem página, com quantos links apontam para ele.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingLink {
    /// O caminho como aparece no `href`, sem a âncora (`/centelha/ficha/`).
    pub target: String,
    pub count: usize,
}

/// Os `href` das páginas que começam com o caminho base e não chegam a um
/// arquivo de `files` (caminhos dentro de `dist/`). O que mora em `pkg/` (o
/// WebAssembly, gravado depois do gerador) não conta. Sai em ordem de
/// caminho.
pub fn missing_links(base: &str, pages: &[Page], files: &[String]) -> Vec<MissingLink> {
    let files: HashSet<&str> = files.iter().map(String::as_str).collect();
    let mut missing: BTreeMap<String, usize> = BTreeMap::new();
    for page in pages {
        for href in hrefs(&page.html) {
            let Some(rest) = href.strip_prefix(base) else {
                continue;
            };
            let path = rest.split(['#', '?']).next().unwrap_or_default();
            if path.starts_with("pkg/") || exists(path, &files) {
                continue;
            }
            // Com e sem a barra do fim é o mesmo destino.
            let is_file = path.rsplit('/').next().is_some_and(|s| s.contains('.'));
            let target = if is_file {
                format!("{base}{path}")
            } else {
                format!("{base}{}/", path.trim_end_matches('/'))
            };
            *missing.entry(target).or_default() += 1;
        }
    }
    missing
        .into_iter()
        .map(|(target, count)| MissingLink { target, count })
        .collect()
}

/// Um caminho de link chega a um arquivo: ele mesmo, ou a página da pasta.
fn exists(path: &str, files: &HashSet<&str>) -> bool {
    if path.is_empty() || path.ends_with('/') {
        return files.contains(format!("{path}index.html").as_str());
    }
    files.contains(path) || files.contains(format!("{path}/index.html").as_str())
}

/// Os valores dos atributos `href="..."` do HTML, com as entidades de `&`
/// desfeitas.
fn hrefs(html: &str) -> impl Iterator<Item = String> + '_ {
    html.split("href=\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .map(|v| v.replace("&amp;", "&"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(html: &str) -> Page {
        Page {
            path: "index.html".to_string(),
            html: html.to_string(),
        }
    }

    #[test]
    fn lists_internal_links_without_a_page() {
        let pages = [
            page(r##"<a href="/c/">x</a><a href="/c/ficha/">y</a><a href="/c/ficha/#a">z</a>"##),
            page(r##"<a href="/c/ficha">sem barra</a><img src="/c/x.png"><a href="/c/y.png">"##),
            page(r##"<a href="/c/regras/combate/">ok</a><a href="https://x.org/">fora</a>"##),
            page(r##"<link href="/c/assets/site.css"><link href="/c/pkg/centelha_web.js">"##),
            page(r##"<a href="/c/regras/combate">sem barra</a><a href="#local">âncora</a>"##),
        ];
        let files = [
            "index.html".to_string(),
            "regras/combate/index.html".to_string(),
            "assets/site.css".to_string(),
        ];
        assert_eq!(
            missing_links("/c/", &pages, &files),
            [
                MissingLink {
                    target: "/c/ficha/".to_string(),
                    count: 3,
                },
                MissingLink {
                    target: "/c/y.png".to_string(),
                    count: 1,
                },
            ]
        );
    }
}
