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
use super::equipment::{CLASS_ORDER, Equipment, WeaponRow, class_name};
use super::nav::{self, NAV, TOOLS};
use crate::back_to_top::BackToTop;
use crate::bookmarks::Bookmarks;
use crate::cards::RefCards;
use crate::equipment_filter::EquipmentFilter;
use crate::search::Search;
use crate::shapes::Shapes;
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

/// A busca da barra lateral: o botão e a caixa de diálogo (texto de
/// `src/components/Busca.astro` do site atual), com a ilha que os liga
/// (`crate::search`). O `data-pf` diz onde está o `pagefind.js`, baixado só
/// ao abrir a caixa.
fn search_box(base: &str) -> impl IntoView + use<> {
    let pagefind = format!("{base}pagefind/pagefind.js");
    view! {
        <button class="btn busca-abrir" aria-label="Buscar (tecle /)" style="width:100%;text-align:left">
            "🔍 Buscar… " <kbd class="busca-kbd">"/"</kbd>
        </button>
        <dialog class="busca-modal" aria-label="Busca" data-pf=pagefind>
            <div class="busca-head">
                <strong class="cinzel">"Busca"</strong>
                <button class="btn busca-fechar" aria-label="Fechar">"✕"</button>
            </div>
            <div class="busca-opcoes">
                <fieldset class="busca-modos">
                    <legend class="busca-sr">"Modo de busca"</legend>
                    <label class="busca-chip">
                        <input type="radio" name="busca-modo" value="palavras" checked=true/>
                        <span>"Palavras"</span>
                    </label>
                    <label class="busca-chip">
                        <input type="radio" name="busca-modo" value="frase"/>
                        <span>"Frase exata"</span>
                    </label>
                </fieldset>
                <label class="busca-chip busca-aa" title="Diferenciar maiúsculas de minúsculas">
                    <input type="checkbox" class="busca-aa-caixa"/>
                    <span><b>"Aa"</b> " diferenciar maiúsculas"</span>
                </label>
            </div>
            <input class="busca-entrada" type="search" placeholder="Buscar no livro" aria-label="Termo da busca"
                autocomplete="off" autocapitalize="off" spellcheck="false" enterkeyhint="search" autofocus=true/>
            <p class="busca-msg" role="status" aria-live="polite"></p>
            <ol class="busca-lista"></ol>
            <button class="btn busca-mais" type="button" hidden=true></button>
            <p class="muted busca-hint">
                <b>"Palavras"</b> " acha as páginas que têm todas as palavras digitadas, em qualquer ordem. "
                <b>"Frase exata"</b> " só acha as palavras em sequência, como foram digitadas. "
                <b>"Aa"</b> " diferencia maiúscula de minúscula: " <em>"Kael"</em> " não acha " <em>"kael"</em>
                ". Só descarta os resultados em que a palavra achada tem outra caixa; as variações que a busca já acha ("
                <em>"luta"</em> ", " <em>"lutando"</em> ", " <em>"lutador"</em>
                ") continuam valendo. Os acentos continuam sendo ignorados. Conceitos também funcionam: tente "
                <em>"willpower"</em> ", " <em>"iniciativa"</em> ", " <em>"absorção"</em> "."
            </p>
        </dialog>
        <Search/>
    }
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
/// Diferenças de propósito: o
/// seletor de tema fica na barra lateral (no site atual ele mora numa página
/// de configurações); os itens que dependem de login (Minha Conta, Admin,
/// Preferências e o "Sair") ficam para a fase da conta.
#[component]
pub fn Layout(base: String, page: PageInfo, children: Children) -> impl IntoView {
    let page_base = base.clone();
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
        view! { <nav class="crumbs" aria-label="Trilha de navegação" data-pagefind-ignore="">{steps}</nav> }
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
    // A busca (Pagefind) indexa só o conteúdo (`data-pagefind-body`, como no
    // site atual); a página de não encontrada fica fora do índice.
    let indexed = (page.path != "404").then_some("");
    let home = nav::url(&base, "");

    view! {
        <div class="cfundo" aria-hidden="true" inner_html=BACKDROP_HTML.as_str()></div>
        <a href="#conteudo" class="skip">"Pular para o conteúdo"</a>
        <div class="topbar" data-pagefind-ignore="">
            <DrawerToggle/>
            <a class="brand" href=home.clone()>"Centelha " <span class="d6">"D6"</span></a>
            <button class="btn busca-abrir topbar-busca" aria-label="Buscar no livro">"🔍"</button>
        </div>
        <div class="shell">
            <Scrim/>
            <SidebarExpand/>
            <aside class="sidebar" id="nav" data-pagefind-ignore="">
                <DrawerClose/>
                <SidebarCollapse/>
                <a class="brand" href=home>
                    "Centelha " <span class="d6">"D6"</span>
                    <small>"do mortal ao semideus"</small>
                </a>
                <div style="margin-top:1rem">{search_box(&page_base)}</div>
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
            <main class="content" id="conteudo" data-pagefind-body=indexed>
                {crumbs}
                <article class=article_class>{children()}</article>
            </main>
        </div>
        {toc}
        {refs}
        <Bookmarks/>
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
/// os contadores, os capítulos (da barra lateral), as ferramentas e a frase
/// final da busca.
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
        <p class="muted" style="margin-top:2rem;text-align:center">
            "Edição viva e jogável de ponta a ponta. Tecle " <kbd>"/"</kbd> " para buscar qualquer regra."
        </p>
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
/// termos já em ordem, cada um com a sua âncora, e a abertura com o balão e a
/// busca.
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
        <p class="lead">
            "Os termos que aparecem por todo o livro. No corpo do texto, eles viram um balão ao passar o mouse. Os sinônimos ajudam a busca: procure por "
            <em>"willpower"</em> " e chegue a Força de Vontade."
        </p>
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

/// A página de Equipamentos, como `src/pages/equipamentos.astro` do site
/// atual: o filtro (busca, classe e tag, ligado pela ilha
/// `EquipmentFilter`) e as quatro tabelas. O contador sai do gerador com o
/// número de armas sem filtro, que é o que o filtro escreve ao abrir.
#[component]
pub fn EquipmentPage(base: String, equipment: Equipment) -> impl IntoView {
    let weapon_rows = |rows: Vec<WeaponRow>| {
        rows.into_iter()
            .map(|a| {
                let tags = a.tags.join(" ");
                let tag_list = a.tags.join(", ");
                view! {
                    <tr data-arma="" data-classe=a.class_id data-tags=tags data-txt=a.search>
                        <th>{a.name}</th>
                        <td>{a.class}</td>
                        <td class="c">{a.damage}</td>
                        <td class="c">{a.accuracy}</td>
                        <td class="c">{a.defense_or_range}</td>
                        <td class="c">{a.hands}</td>
                        <td class="c">{a.speed}</td>
                        <td>{a.modes}</td>
                        <td>{a.skill}</td>
                        <td class="muted">{tag_list}</td>
                    </tr>
                }
            })
            .collect_view()
    };
    let count = equipment.melee.len() + equipment.ranged.len();
    let count_text = format!("{count} arma{}", if count == 1 { "" } else { "s" });
    let classes = CLASS_ORDER
        .iter()
        .map(|c| {
            view! {
                <label class="chk">
                    <input type="checkbox" data-ef="classe" value=*c/>
                    {class_name(c).to_string()}
                </label>
            }
        })
        .collect_view();
    let tags = equipment
        .tags
        .iter()
        .map(|t| {
            view! {
                <label class="chk">
                    <input type="checkbox" data-ef="tag" value=t.clone()/>
                    {t.clone()}
                </label>
            }
        })
        .collect_view();
    let armors = equipment
        .armors
        .into_iter()
        .map(|a| {
            view! {
                <tr>
                    <th>{a.name}</th>
                    <td class="c">{a.class}</td>
                    <td class="c">{a.impact}</td>
                    <td class="c">{a.cut}</td>
                    <td class="c">{a.pierce}</td>
                    <td class="c">{a.penalty}</td>
                    <td class="muted">{a.notes}</td>
                </tr>
            }
        })
        .collect_view();
    let shields = equipment
        .shields
        .into_iter()
        .map(|s| {
            view! {
                <tr>
                    <th>{s.name}</th>
                    <td class="c">{s.defense}</td>
                    <td class="c">{s.vs_fast}</td>
                    <td class="c">{s.penalty}</td>
                    <td class="muted">{s.notes}</td>
                </tr>
            }
        })
        .collect_view();
    let melee = weapon_rows(equipment.melee);
    let ranged = weapon_rows(equipment.ranged);
    view! {
        <h1>"Equipamentos"</h1>
        <p class="lead">
            "A arma define o " <em>"estilo"</em> ", não a potência bruta. As regras completas estão em "
            <a href=nav::url(&base, "regras/armas-e-armaduras")>"Armas & Armaduras"</a>
            "; aqui está o catálogo para filtrar e comparar."
        </p>
        <div class="filtros panel">
            <label>"Buscar " <input type="search" data-ef="q" placeholder="nome, tag…"/></label>
            <fieldset><legend>"Classe"</legend>{classes}</fieldset>
            <fieldset><legend>"Tag"</legend>{tags}</fieldset>
        </div>
        <section data-bloco="">
            <h2>"Armas Corpo a Corpo"</h2>
            <div class="table-wrap">
                <table>
                    <thead>
                        <tr>
                            <th>"Arma"</th><th>"Classe"</th><th class="c">"Dano"</th><th class="c">"Acerto"</th>
                            <th class="c">"Def."</th><th class="c">"Mãos"</th><th class="c">"Velocidade"</th>
                            <th>"Modos"</th><th>"Habilidade"</th><th>"Tags"</th>
                        </tr>
                    </thead>
                    <tbody>{melee}</tbody>
                </table>
            </div>
        </section>
        <section data-bloco="">
            <h2>"Armas à Distância"</h2>
            <p class="muted">
                "No lugar da Defesa entra a " <strong>"Efetiva"</strong>
                ", a distância até a qual se acerta sem a distância atrapalhar (alvo Médio, a mesma para qualquer personagem): arma de projétil não guarda, então não acrescenta nada ao Bloqueio."
            </p>
            <p class="muted">
                "Além dela a mira custa −3 por meia Efetiva, sem teto. A distância " <em>"máxima"</em>
                " é outra conta: nos arcos vem da Força, nas bestas é fixa, e no " <strong>"Arremesso"</strong>
                " vem do seu " <strong>"FAA"</strong>
                " e do peso (a tabela de Arremessar, em Corpo e Movimento, e o bloco Peso, Arremesso e Corrida da ficha fazem essa conta)."
            </p>
            <div class="table-wrap">
                <table>
                    <thead>
                        <tr>
                            <th>"Arma"</th><th>"Classe"</th><th class="c">"Dano"</th><th class="c">"Acerto"</th>
                            <th class="c">"Efetiva"</th><th class="c">"Mãos"</th><th class="c">"Velocidade"</th>
                            <th>"Modos"</th><th>"Habilidade"</th><th>"Tags"</th>
                        </tr>
                    </thead>
                    <tbody>{ranged}</tbody>
                </table>
            </div>
        </section>
        <p class="muted ef-count">{count_text}</p>
        <p class="muted">
            "A coluna " <strong>"Modos"</strong> " lista os modos de ataque da arma; " <strong>"★"</strong>
            " = modo principal (sem custo), os sem marca são secundários (−2 ao acerto e −1d6 no dano). "
            <em>"(N0)–(N2)"</em>
            " = Nível de Perfuração da arma (só o modo Perfurante passa pelo gate da armadura)."
        </p>
        <h2>"Armaduras"</h2>
        <p class="muted">
            "Peitoral, Camisa de malha, Peitoral reforçado, Placa articulada e Malha completa: valores provisórios, sujeitos a balanceamento final."
        </p>
        <p class="muted">
            "Três Absorções: " <strong>"Impacto"</strong> ", " <strong>"Corte"</strong> " e " <strong>"Perfuração"</strong>
            " (o dano Perfurante usa o de Perfuração). O " <strong>"Nível"</strong>
            " entre parênteses é a Resistência à Perfuração: a Perfuração da arma precisa ser ≥ ele, senão resvala, e ele "
            <em>"não soma"</em> " ao empilhar (vale o maior)."
        </p>
        <p class="muted">
            "A " <strong>"Penalidade"</strong>
            " incide em qualquer ação física (Ataque, Esquiva, Bloqueio, Deslocamento, Salto; no Deslocamento, na Corrida e nos Saltos, só a metade dela, arredondada para baixo: −1 m a cada 2 pontos nas medidas horizontais, e no "
            <a href=nav::url(&base, "regras/combate#salto-velocidade-3")>"Salto Vertical"</a>
            " −10 cm a cada 2 pontos, até o mínimo de 0) e " <strong>"dobra"</strong>
            " para Furtividade e atividades delicadas (na Furtividade, esse dobro é a Circunstância da armadura na Dificuldade, e é uma cobrança só; na natação, a Circunstância da armadura substitui a Penalidade). Empilhar vale a maior Absorção de cada categoria; as penalidades somam."
        </p>
        <div class="table-wrap">
            <table>
                <thead>
                    <tr>
                        <th>"Armadura"</th><th class="c">"Classe"</th><th class="c">"Impacto"</th><th class="c">"Corte"</th>
                        <th class="c">"Perfuração (Nível)"</th><th class="c">"Penalidade"</th><th>"Notas"</th>
                    </tr>
                </thead>
                <tbody>{armors}</tbody>
            </table>
        </div>
        <h2>"Escudos"</h2>
        <p class="muted">
            "Escudo é " <strong>"bônus de Defesa"</strong> " (ajuda a não ser acertado), não absorção: some ao seu "
            <strong>"Bloqueio"</strong> ", como a Defesa de uma arma."
        </p>
        <p class="muted">
            "Contra " <strong>"projéteis rápidos"</strong>
            " (flecha, virote, bala de funda, Shuriken, Mini-faca, Kunai, Adaga de Arremesso) ele é a " <em>"única"</em>
            " forma de bloquear: só quem tem um escudo " <strong>"hábil"</strong>
            " (cobre ≥30% do corpo, do Hoplon para cima num usuário Médio) e está " <strong>"apto"</strong>
            " a manejá-lo bloqueia; sem isso, resta a Esquiva."
        </p>
        <p class="muted">
            "A " <strong>"Penalidade"</strong> " incide nas outras ações físicas (esquiva etc.), nunca no bloqueio do próprio escudo."
        </p>
        <div class="table-wrap">
            <table>
                <thead>
                    <tr>
                        <th>"Escudo"</th><th class="c">"Defesa"</th><th class="c">"vs Projétil rápido"</th>
                        <th class="c">"Penalidade"</th><th>"Estilo"</th>
                    </tr>
                </thead>
                <tbody>{shields}</tbody>
            </table>
        </div>
        <EquipmentFilter/>
    }
}

/// A página de teste das ilhas sem página portada: dois rótulos de forma, um
/// texto longo (para o voltar ao topo aparecer) e as duas ilhas.
#[component]
pub fn IslandsTestPage() -> impl IntoView {
    let filler = (1..=60)
        .map(|i| view! { <p>{format!("Parágrafo {i} de enchimento, para a página rolar mais de duas telas.")}</p> })
        .collect_view();
    view! {
        <h1>"Teste das ilhas"</h1>
        <p>
            "Uma " <span data-formas="area">"Área 3"</span> " e um " <span data-formas="volume">"Volume 2"</span> "."
        </p>
        {filler}
        <Shapes/>
        <BackToTop/>
    }
}
