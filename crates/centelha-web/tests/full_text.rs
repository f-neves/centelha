//! Prova 1 do ADR 0003: cada página gravada tem o texto inteiro sem
//! WebAssembly.
//!
//! Gera o site numa pasta temporária e, para cada capítulo, compara o texto do
//! HTML gravado (sem etiquetas) com o texto que os eventos do Markdown de
//! origem dizem, depois das mesmas trocas (diagramas, caminho base,
//! pontuação). Na página inicial, confere numeral, título e resumo de cada
//! capítulo, em ordem. Roda com os dois caminhos base (`/centelha/` e `/`).

use std::path::{Path, PathBuf};

use centelha_web::site::Site;
use centelha_web::site::text::{events_to_text, html_to_text};

fn content_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

fn temp_out(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("centelha-full-text-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// Onde o texto da página deixa de seguir o esperado: o trecho do esperado a
/// partir do ponto em que falta, e o que a página tem no lugar.
fn first_gap(expected: &str, page: &str) -> String {
    // A posição na página onde o esperado começaria (o título vem antes).
    let probe: String = expected.chars().take(40).collect();
    let start = page.find(&probe).unwrap_or(0);
    let tail = &page[start..];
    let common = expected
        .char_indices()
        .zip(tail.chars())
        .find(|((_, a), b)| a != b)
        .map(|((i, _), _)| i)
        .unwrap_or_else(|| expected.len().min(tail.len()));
    let from = expected[..common]
        .char_indices()
        .rev()
        .nth(30)
        .map(|(i, _)| i)
        .unwrap_or(0);
    let wanted: String = expected[from..].chars().take(120).collect();
    let found: String = tail[from.min(tail.len())..].chars().take(120).collect();
    format!("falta a partir de: {wanted:?}\n   a página tem: {found:?}")
}

fn check_site(base: &str) {
    let content = content_dir();
    let site = Site::load(&content, base).expect("o site se lê");
    let out = temp_out(base.trim_matches('/'));
    let pages = site.write(&out).expect("o site se grava");
    assert_eq!(pages.len(), site.chapters().len() + 1);
    assert_eq!(site.chapters().len(), 28);

    let mut failures = Vec::new();
    let markdown = site.markdown();
    for chapter in site.chapters() {
        let path = out.join("regras").join(&chapter.slug).join("index.html");
        let html = std::fs::read_to_string(&path).expect("a página foi gravada");
        let page = html_to_text(&html);
        let expected = events_to_text(&markdown.events(&chapter.body));
        assert!(expected.len() > 200, "{}: texto curto demais", chapter.slug);
        let heading = format!("Capítulo {} {}", chapter.numeral, chapter.title);
        if !page.contains(&heading) {
            failures.push(format!("{}: falta o cabeçalho {heading:?}", path.display()));
        }
        if !page.contains(&expected) {
            failures.push(format!(
                "{}: {}",
                path.display(),
                first_gap(&expected, &page)
            ));
        }
    }

    let home_path = out.join("index.html");
    let home = html_to_text(&std::fs::read_to_string(&home_path).expect("a página inicial existe"));
    let mut rest = home.as_str();
    for chapter in site.chapters() {
        let card = format!(
            "Capítulo {} {} {}",
            chapter.numeral, chapter.title, chapter.summary
        );
        match rest.find(&card) {
            Some(pos) => rest = &rest[pos + card.len()..],
            None => failures.push(format!(
                "{}: falta, em ordem, o cartão {card:?}",
                home_path.display()
            )),
        }
    }

    let _ = std::fs::remove_dir_all(&out);
    assert!(
        failures.is_empty(),
        "{} página(s) sem o texto inteiro (base {base}):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn every_page_has_the_full_text_with_base() {
    check_site("/centelha/");
}

#[test]
fn every_page_has_the_full_text_without_base() {
    check_site("/");
}

#[test]
fn every_diagram_has_a_drawing() {
    let site = Site::load(&content_dir(), "/").unwrap();
    let markdown = site.markdown();
    let mut figures = 0;
    for chapter in site.chapters() {
        let html = markdown.to_html(&chapter.body);
        assert!(
            !html.contains("class=\"erro\""),
            "{}: diagrama sem desenho",
            chapter.slug
        );
        figures += html.matches("<figure class=\"diagrama-caixa\">").count();
    }
    assert_eq!(figures, 6, "o livro tem seis diagramas (qual-sistema)");
}

#[test]
fn no_link_to_the_source_site_is_left() {
    let site = Site::load(&content_dir(), "/centelha/").unwrap();
    for page in site.pages() {
        assert!(
            !page.html.contains("/centelha-rpg"),
            "{}: link para o site atual",
            page.path
        );
    }
}

/// As fontes saem em `assets/fonts/` byte a byte, cada `url()` da folha de
/// estilo tem o seu arquivo, e só a fonte do texto corrido é pré-carregada,
/// com o caminho base.
#[test]
fn fonts_are_copied_as_is_and_one_is_preloaded() {
    let site = Site::load(&content_dir(), "/centelha/").unwrap();
    let out = temp_out("fontes");
    site.write(&out).unwrap();
    let fonts_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");

    let css = std::fs::read_to_string(out.join("assets/site.css")).unwrap();
    let urls: Vec<&str> = css
        .split("url(fonts/")
        .skip(1)
        .map(|rest| &rest[..rest.find(')').unwrap()])
        .collect();
    assert_eq!(urls.len(), 4, "quatro arquivos de fonte na folha de estilo");
    for name in &urls {
        let copied = std::fs::read(out.join("assets/fonts").join(name)).unwrap();
        let family = if name.starts_with("cinzel") {
            "cinzel"
        } else {
            "eb-garamond"
        };
        let original = std::fs::read(fonts_src.join(family).join(name)).unwrap();
        assert!(copied == original, "{name}: o arquivo mudou na cópia");
    }

    let home = std::fs::read_to_string(out.join("index.html")).unwrap();
    assert_eq!(home.matches("as=\"font\"").count(), 1);
    assert!(home.contains(
        "<link rel=\"preload\" href=\"/centelha/assets/fonts/eb-garamond-latin-400-normal.woff2\" as=\"font\" type=\"font/woff2\" crossorigin>"
    ));
    let _ = std::fs::remove_dir_all(&out);
}
