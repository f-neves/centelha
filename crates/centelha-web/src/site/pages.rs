//! As páginas do livro: a casca comum, a página inicial, a do capítulo e a do
//! glossário.
//!
//! São componentes comuns (não ilhas): saem em HTML no gerador e o
//! WebAssembly não os toca. As ilhas que aparecem aqui são o seletor de tema,
//! os botões da gaveta e da barra lateral (`crate::shell`) e o "Nesta página"
//! (`crate::toc`).

use centelha_data::{Chapter, GlossaryEntry};
use leptos::prelude::*;

use super::backdrop::BACKDROP_HTML;
use super::nav::{self, NAV, TOOLS};
use crate::cards::RefCards;
use crate::shell::{DrawerClose, DrawerToggle, Scrim, SidebarCollapse, SidebarExpand};
use crate::tables::TableSorter;
use crate::theme::ThemeSwitcher;
use crate::toc::{OnThisPage, TocEntry};

/// O endereço da página de um capítulo (o mesmo caminho do site atual).
pub fn chapter_url(base: &str, slug: &str) -> String {
    nav::url(base, &format!("regras/{slug}"))
}

/// O que a casca precisa saber da página.
#[derive(Debug, Clone)]
pub struct PageInfo {
    /// O caminho da página sem o caminho base e sem barras nas pontas
    /// (`regras/combate`, `glossario`; vazio na inicial).
    pub path: String,
    pub title: String,
    /// A coluna do texto (`prose`) ou a mais larga (`wide`), como no site
    /// atual.
    pub wide: bool,
    /// Os títulos para o "Nesta página".
    pub toc: Vec<TocEntry>,
    /// A página marca os termos do glossário e leva o cartão das referências
    /// (`crate::cards`).
    pub refs: bool,
}

/// O conteúdo provisório do `<script>` dos dados do cartão: o gerador marca os
/// termos no `<main>` da página pronta e troca este marcador pelo JSON dos
/// termos que apareceram.
pub const REF_DATA_SLOT: &str = "__ref_data__";

/// A casca de toda página, como `src/layouts/Base.astro` do site atual: o
/// pulo para o conteúdo, a barra de cima do celular, a barra lateral (que vira
/// gaveta no celular), a trilha de navegação, a coluna do texto e o "Nesta
/// página".
///
/// Diferenças de propósito: a busca ainda não existe (entra com a busca); o
/// seletor de tema fica na barra lateral (no site atual ele mora numa página
/// de configurações); os itens que dependem de login (Minha Conta, Admin,
/// Preferências e o "Sair") ficam para a fase da conta.
#[component]
pub fn Layout(base: String, page: PageInfo, children: Children) -> impl IntoView {
    let path = page.path.clone();
    let current = nav::chapter_of(&path);
    let is_active = |slug: &str| slug == path;

    let chapters = NAV
        .iter()
        .map(|n| {
            // Dentro de um capítulo de várias páginas, quem marca a página
            // atual é a sub-página, não o capítulo.
            let in_chapter = current.is_some_and(|c| c.slug == n.slug && c.sub == n.sub);
            let marked = is_active(n.slug) && !in_chapter;
            let subs = if in_chapter {
                n.sub
                    .iter()
                    .map(|s| {
                        view! {
                            <a
                                class="subcap"
                                href=nav::url(&base, s.slug)
                                aria-current=is_active(s.slug).then_some("page")
                            >
                                <span class="num">"·"</span>
                                <span>{s.title}</span>
                            </a>
                        }
                    })
                    .collect_view()
                    .into_any()
            } else {
                ().into_any()
            };
            let numeral =
                (!n.numeral.is_empty()).then(|| view! { <span class="num">{n.numeral}</span> });
            view! {
                <a href=nav::url(&base, n.slug) aria-current=marked.then_some("page")>
                    {numeral}
                    <span>{n.title}</span>
                </a>
                {subs}
            }
        })
        .collect_view();
    let tools = TOOLS
        .iter()
        .map(|t| {
            view! {
                <a href=nav::url(&base, t.slug) aria-current=is_active(t.slug).then_some("page")>
                    <span class="num">"◆"</span>
                    <span>{t.title}</span>
                </a>
            }
        })
        .collect_view();

    let crumbs = nav::crumbs(&base, &page.path, &page.title);
    let crumbs = (crumbs.len() > 1).then(|| {
        let steps = crumbs
            .into_iter()
            .enumerate()
            .map(|(i, c)| {
                let sep = (i > 0).then(|| view! { <span class="crumb-sep">"›"</span> });
                let step = match c.href {
                    Some(href) => view! { <a href=href>{c.label}</a> }.into_any(),
                    None => view! { <span aria-current="page">{c.label}</span> }.into_any(),
                };
                view! { {sep} {step} }
            })
            .collect_view();
        view! { <nav class="crumbs" aria-label="Trilha de navegação">{steps}</nav> }
    });

    let toc = (page.toc.len() >= 2).then(|| view! { <OnThisPage entries=page.toc/> });
    let refs = page.refs.then(|| {
        view! {
            <script type="application/json" id="ref-data" inner_html=REF_DATA_SLOT></script>
            <dialog class="ref-modal" aria-label="Detalhe">
                <button type="button" class="ref-modal-x" aria-label="Fechar">"✕"</button>
                <div class="ref-modal-body"></div>
            </dialog>
            <RefCards/>
        }
    });
    let article_class = if page.wide { "wide" } else { "prose" };
    let home = nav::url(&base, "");

    view! {
        <div class="cfundo" aria-hidden="true" inner_html=BACKDROP_HTML.as_str()></div>
        <a href="#conteudo" class="skip">"Pular para o conteúdo"</a>
        <div class="topbar">
            <DrawerToggle/>
            <a class="brand" href=home.clone()>"Centelha " <span class="d6">"D6"</span></a>
        </div>
        <div class="shell">
            <Scrim/>
            <SidebarExpand/>
            <aside class="sidebar" id="nav">
                <DrawerClose/>
                <SidebarCollapse/>
                <a class="brand" href=home>
                    "Centelha " <span class="d6">"D6"</span>
                    <small>"do mortal ao semideus"</small>
                </a>
                <div class="sidebar-theme">
                    <ThemeSwitcher/>
                </div>
                <nav class="toc" aria-label="Capítulos">
                    <div class="grp">"Regras"</div>
                    {chapters}
                    <div class="grp">"Ferramentas"</div>
                    {tools}
                    <div class="grp">"Jogo"</div>
                    <a href=nav::url(&base, "mesas")>
                        <span class="num">"◆"</span>
                        <span>"Mesas / Fichas"</span>
                    </a>
                    <a href=nav::url(&base, "entrar")>
                        <span class="num">"◆"</span>
                        <span>"Entrar"</span>
                    </a>
                </nav>
            </aside>
            <main class="content" id="conteudo">
                {crumbs}
                <article class=article_class>{children()}</article>
            </main>
        </div>
        {toc}
        {refs}
    }
}

/// A faísca da marca (`src/components/CentelhaSpark.astro` do site atual):
/// seis losangos que respiram entre o anel fechado e as pétalas soltas, em
/// SMIL, com o brilho em CSS. O SMIL não se desliga por CSS; para quem pede
/// menos movimento, o site atual para a animação por script. Aqui vai, ao
/// lado, a mesma faísca parada no pico, e o CSS mostra uma ou outra pela
/// preferência, sem JavaScript.
#[component]
pub fn Spark() -> impl IntoView {
    const VALE: &str = "M 512.00 462.00 L 627.47 262.00 L 512.00 62.00 L 396.53 262.00 Z \
M 555.30 487.00 L 786.24 487.00 L 901.71 287.00 L 670.77 287.00 Z \
M 555.30 537.00 L 670.77 737.00 L 901.71 737.00 L 786.24 537.00 Z \
M 512.00 562.00 L 396.53 762.00 L 512.00 962.00 L 627.47 762.00 Z \
M 468.70 537.00 L 237.76 537.00 L 122.29 737.00 L 353.23 737.00 Z \
M 468.70 487.00 L 353.23 287.00 L 122.29 287.00 L 237.76 487.00 Z";
    const PICO: &str = "M 512.00 412.00 L 582.73 237.00 L 512.00 62.00 L 441.27 237.00 Z \
M 598.60 462.00 L 785.52 435.75 L 901.71 287.00 L 714.79 313.25 Z \
M 598.60 562.00 L 714.79 710.75 L 901.71 737.00 L 785.52 588.25 Z \
M 512.00 612.00 L 441.27 787.00 L 512.00 962.00 L 582.73 787.00 Z \
M 425.40 562.00 L 238.48 588.25 L 122.29 737.00 L 309.21 710.75 Z \
M 425.40 462.00 L 309.21 313.25 L 122.29 287.00 L 238.48 435.75 Z";
    const VIEWBOX: &str = "122.29 62 779.42 900";
    let values = format!("{VALE};{PICO};{VALE}");
    view! {
        <svg class="centelha-spark animated" viewBox=VIEWBOX fill="currentColor" aria-hidden="true">
            <g transform="translate(512 512)">
                <g>
                    <animateTransform
                        attributeName="transform"
                        type="scale"
                        values="0.96;1;0.96"
                        dur="5s"
                        repeatCount="indefinite"
                        calcMode="spline"
                        keyTimes="0;0.5;1"
                        keySplines="0.4 0 0.6 1;0.4 0 0.6 1"
                    />
                    <g transform="translate(-512 -512)">
                        <path d=PICO>
                            <animate
                                attributeName="d"
                                values=values
                                dur="5s"
                                repeatCount="indefinite"
                                calcMode="spline"
                                keyTimes="0;0.5;1"
                                keySplines="0.4 0 0.6 1;0.4 0 0.6 1"
                            />
                        </path>
                    </g>
                </g>
            </g>
        </svg>
        <svg class="centelha-spark still" viewBox=VIEWBOX fill="currentColor" aria-hidden="true">
            <path d=PICO></path>
        </svg>
    }
}

/// Os números da página inicial.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HomeCounts {
    /// Proezas: os Caminhos que têm Técnica.
    pub feats: usize,
    pub techniques: usize,
    pub arts: usize,
}

/// A página inicial, como `src/pages/index.astro` do site atual: a abertura,
/// os contadores, os capítulos (da barra lateral) e as ferramentas.
///
/// A frase final do site atual ("Tecle / para buscar qualquer regra") fica de
/// fora até a busca existir.
#[component]
pub fn HomePage(base: String, counts: HomeCounts) -> impl IntoView {
    let u = |path: &str| nav::url(&base, path);
    let counters = [
        ("Proezas", counts.feats, "caminhos"),
        ("Técnicas", counts.techniques, "glossario#tecnica"),
        ("Artes", counts.arts, "artes/catalogo"),
    ]
    .into_iter()
    .map(|(label, value, path)| {
        view! {
            <a class="panel counter" href=u(path)>
                <div class="cinzel counter-value">{value}</div>
                <div class="muted">{label}</div>
            </a>
        }
    })
    .collect_view();
    let chapters = NAV
        .iter()
        .filter(|n| !n.numeral.is_empty())
        .map(|n| {
            view! {
                <a class="panel card" href=u(n.slug)>
                    <div class="cinzel card-kicker">"Capítulo " {n.numeral}</div>
                    <div class="cinzel card-title">{n.title}</div>
                </a>
            }
        })
        .collect_view();
    let tools = TOOLS
        .iter()
        .map(|t| {
            view! {
                <a class="panel card" href=u(t.slug)>
                    <div class="cinzel card-tool">{t.title}</div>
                </a>
            }
        })
        .collect_view();
    let sparks = (0..6)
        .map(|i| {
            let gap = (i > 0).then_some(" ");
            view! { {gap} <Spark/> }
        })
        .collect_view();
    view! {
        <header class="home-head">
            <p class="muted cinzel home-kicker">"Sistema de Interpretação"</p>
            <h1>"Centelha " <span class="d6">"D6"</span></h1>
            <p class="tag-orn">{sparks}</p>
            <p class="lead home-lead">"Do mortal ao semideus, um dado de cada vez."</p>
            <p class="muted home-pitch">
                "Um sistema de " <a href=u("regras/coracao-do-sistema")>"pool de d6"</a>
                " que equilibra realismo tático e agilidade cinematográfica, onde a Habilidade, a vontade e a centelha de poder decidem o destino."
            </p>
            <div class="btn-row home-actions">
                <a class="btn primary" href=u("regras/coracao-do-sistema")>"Começar a ler"</a>
                <a class="btn" href=u("ficha")>"Criar personagem"</a>
                <a class="btn" href=u("caminhos")>"Explorar Proezas"</a>
            </div>
        </header>
        <section class="counters">{counters}</section>
        <h2>"Os Capítulos"</h2>
        <div class="grid-cards">{chapters}</div>
        <h2>"Ferramentas"</h2>
        <div class="grid-cards">{tools}</div>
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
        <p class="tag-orn tag-orn-left"><Spark/></p>
        <div class="chapter-body" inner_html=body_html></div>
        <TableSorter/>
        <nav class="cap-nav" aria-label="Navegação entre capítulos">{prev}{next}</nav>
    }
}

/// A página do glossário, como `src/pages/glossario.astro` do site atual: os
/// termos já em ordem, cada um com a sua âncora. A abertura ainda não leva as
/// duas frases do site atual sobre o balão no corpo do texto e sobre a busca:
/// elas voltam com os balões e com a busca.
#[component]
pub fn GlossaryPage(entries: Vec<GlossaryEntry>) -> impl IntoView {
    let items = entries
        .into_iter()
        .map(|g| {
            let aliases = (!g.aliases.is_empty()).then(|| {
                view! { <span class="muted al">" · também: " {g.aliases.join(", ")}</span> }
            });
            view! {
                <div id=g.id data-permalink="" class="gloss-item">
                    <dt class="cinzel">{g.term}</dt>
                    <dd>{g.definition} {aliases}</dd>
                </div>
            }
        })
        .collect_view();
    view! {
        <h1>"Glossário"</h1>
        <p class="lead">"Os termos que aparecem por todo o livro."</p>
        <dl class="gloss-list no-gloss">{items}</dl>
    }
}

/// A página de "não encontrada", como `src/pages/404.astro` do site atual.
#[component]
pub fn NotFoundPage(base: String) -> impl IntoView {
    view! {
        <div class="not-found">
            <p class="cinzel not-found-code">"404"</p>
            <p class="lead">"Este caminho não leva a lugar nenhum… ainda."</p>
            <div class="btn-row not-found-actions">
                <a class="btn primary" href=nav::url(&base, "")>"Voltar ao início"</a>
                <a class="btn" href=nav::url(&base, "caminhos")>"As Proezas"</a>
            </div>
        </div>
    }
}
