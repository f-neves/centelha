//! As páginas do livro: a casca comum, a página inicial e a do capítulo.
//!
//! São componentes comuns (não ilhas): saem em HTML no gerador e o
//! WebAssembly não os toca. A única ilha delas é o seletor de tema.

use centelha_data::Chapter;
use leptos::prelude::*;

use crate::theme::ThemeSwitcher;

/// O endereço da página inicial.
pub fn home_url(base: &str) -> String {
    base.to_string()
}

/// O endereço da página de um capítulo (o mesmo caminho do site atual).
pub fn chapter_url(base: &str, slug: &str) -> String {
    format!("{base}regras/{slug}/")
}

/// A casca de toda página: o pulo para o conteúdo, a barra de cima (marca e
/// seletor de tema) e a coluna do texto.
#[component]
pub fn Layout(base: String, children: Children) -> impl IntoView {
    view! {
        <a href="#conteudo" class="skip">"Pular para o conteúdo"</a>
        <header class="topbar">
            <a class="brand" href=home_url(&base)>"Centelha " <span class="d6">"D6"</span></a>
            <ThemeSwitcher/>
        </header>
        <main class="content" id="conteudo">
            <article class="prose">{children()}</article>
        </main>
    }
}

/// A página inicial: os capítulos em ordem, com numeral, título e resumo.
#[component]
pub fn HomePage(base: String, chapters: Vec<Chapter>) -> impl IntoView {
    let cards = chapters
        .into_iter()
        .map(|c| {
            view! {
                <li>
                    <a class="panel chapter-card" href=chapter_url(&base, &c.slug)>
                        <div class="cinzel kicker">"Capítulo " {c.numeral}</div>
                        <div class="cinzel card-title">{c.title}</div>
                        <div class="card-summary">{c.summary}</div>
                    </a>
                </li>
            }
        })
        .collect_view();
    view! {
        <header class="home-head">
            <p class="muted cinzel kicker">"Sistema de Interpretação"</p>
            <h1>"Centelha " <span class="d6">"D6"</span></h1>
            <p class="lead">"Do mortal ao semideus, um dado de cada vez."</p>
        </header>
        <h2>"Os Capítulos"</h2>
        <ol class="chapter-list">{cards}</ol>
    }
}

/// O vizinho de um capítulo, para a navegação do pé da página.
#[derive(Clone, Debug)]
pub struct Neighbor {
    pub url: String,
    pub label: String,
}

impl Neighbor {
    /// O vizinho `other` visto da página `current`: vindo de outro capítulo,
    /// a página que abre um capítulo de várias páginas leva o nome do
    /// capítulo na frente do título ("Ações & Sistema (A Régua Comum)").
    pub fn new(base: &str, current: &Chapter, other: &Chapter) -> Neighbor {
        let label = match &other.chapter_name {
            Some(name) if other.numeral != current.numeral => format!("{name} ({})", other.title),
            _ => other.title.clone(),
        };
        Neighbor {
            url: chapter_url(base, &other.slug),
            label,
        }
    }
}

/// A página de um capítulo. `body_html` é o Markdown já convertido.
#[component]
pub fn ChapterPage(
    numeral: String,
    title: String,
    body_html: String,
    prev: Option<Neighbor>,
    next: Option<Neighbor>,
) -> impl IntoView {
    let prev = match prev {
        Some(n) => view! { <a class="btn" href=n.url>"← " {n.label}</a> }.into_any(),
        None => view! { <span></span> }.into_any(),
    };
    let next = match next {
        Some(n) => view! { <a class="btn" href=n.url>{n.label} " →"</a> }.into_any(),
        None => view! { <span></span> }.into_any(),
    };
    view! {
        <p class="muted cinzel kicker">"Capítulo " {numeral}</p>
        <h1>{title}</h1>
        <div class="chapter-body" inner_html=body_html></div>
        <nav class="cap-nav" aria-label="Navegação entre capítulos">{prev}{next}</nav>
    }
}
