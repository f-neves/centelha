//! O HTML das páginas das Artes (`artes/catalogo`, `artes/efeitos`) e o
//! redirecionamento de `artes/`.
//!
//! O espaço entre as etiquetas segue o do site atual (que vem da formatação
//! do código-fonte), para o texto copiado e o lido sem estilo saírem iguais.
//!
//! O texto é o de `src/pages/artes/catalogo.astro` e
//! `src/pages/artes/efeitos.astro` do site atual. As ilhas leem os `data-*`.

use leptos::prelude::*;

use super::arts::{ArtCard, Arts, EffectBlock};
use super::nav;
use crate::art_filter::ArtFilter;
use crate::back_to_top::BackToTop;
use crate::effect_tabs::EffectTabs;
use crate::markers::Markers;
use crate::shapes::Shapes;

const KICKER_STYLE: &str =
    "letter-spacing:.3em;text-transform:uppercase;font-size: var(--fs-micro);margin:0";
const GRID_STYLE: &str =
    "grid-template-columns:repeat(auto-fill,minmax(min(var(--card-min-lg),100%),1fr))";

fn numeral() -> &'static str {
    nav::NAV
        .iter()
        .find(|n| n.slug == "artes/regras")
        .map(|n| n.numeral)
        .unwrap_or("")
}

/// A página de `artes/`, que leva às regras (como a do site atual, sem a
/// casca: só o `meta refresh` e o link).
pub fn redirect_html(base: &str) -> String {
    let target = nav::url(base, "artes/regras");
    format!(
        "<!doctype html>\n<html lang=\"pt-BR\">\n<head>\n<meta charset=\"UTF-8\">\n\
<link rel=\"icon\" href=\"{base}favicon.ico\" sizes=\"any\">\n\
<link rel=\"icon\" href=\"{base}favicon.svg\" type=\"image/svg+xml\">\n\
<meta http-equiv=\"refresh\" content=\"0;url={target}\">\n\
<link rel=\"canonical\" href=\"{target}\">\n\
<title>As Artes \u{b7} Centelha</title>\n</head>\n\
<body><p><a href=\"{target}\">Abrir as regras das Artes</a></p></body>\n</html>\n"
    )
}

fn art_cards(cards: Vec<ArtCard>) -> impl IntoView {
    cards
        .into_iter()
        .map(|a| {
            let mark = format!("arte:{}", a.id);
            let label = format!("Marcar {}", a.name);
            let levels = a
                .levels
                .into_iter()
                .map(|n| {
                    let examples = (!n.examples.is_empty()).then(|| {
                        let items =
                            n.examples.into_iter().map(|e| view! { <span>{e}</span> }).collect_view();
                        view! { <span class="nv-ex muted">{items}</span> }
                    });
                    view! {
                        <li>
                            <span class="lvl">{n.level}</span>" "
                            <span class="nv-corpo">
                                <strong>{n.name}</strong>": "{n.effect}" "{examples}
                            </span>
                        </li>
                    }
                })
                .collect_view();
            view! {
                <div class="panel arte" id=a.id data-permalink="" data-cat=a.category data-txt=a.search>
                    <div class="arte-head">
                        <span class="cinzel arte-nome">{a.name}</span>" "
                        <button class="bm" data-bm=mark title="Marcar" aria-label=label>"☆"</button>
                    </div>
                    <ul class="niveis">{levels}</ul>
                </div>
            }
        })
        .collect_view()
}

/// `artes/catalogo`: as Artes com os seis níveis e o filtro.
#[component]
pub fn ArtsCatalogPage(base: String, arts: Arts) -> impl IntoView {
    let count = arts.art_count;
    let chips = |list: &[ArtCard]| {
        list.iter()
            .map(|a| {
                view! {
                    <label class="f-chip">
                        <input type="checkbox" data-fa="arte" value=a.id.clone()/>
                        {a.name.clone()}
                    </label>
                }
            })
            .collect_view()
    };
    let groups = [
        ("Elementais", chips(&arts.elemental)),
        ("Universais", chips(&arts.universal)),
    ]
    .into_iter()
    .map(|(title, chips)| {
        view! {
            <div class="f-grupo">
                <button class="f-tit" type="button" data-grupo=title>{title}</button>
                <div class="f-chips">{chips}</div>
            </div>
        }
    })
    .collect_view();
    let sections = [
        ("Artes Elementais", arts.elemental),
        ("Artes Universais", arts.universal),
    ]
    .into_iter()
    .map(|(title, list)| {
        view! {
            <section class="arte-sec">
                <h2 style="text-align:center;border:none">{title}</h2>
                <div class="grid-cards" style=GRID_STYLE>{art_cards(list)}</div>
            </section>
        }
    })
    .collect_view();
    view! {
        <p class="muted cinzel" style=KICKER_STYLE>
            "Capítulo " {numeral()} " · " <a href=nav::url(&base, "artes/regras")>"As Artes"</a>
        </p>
        <h1>"As " {count} " Artes"</h1>
        <p class="lead">
            "Os seis níveis de cada Arte são " <em>"exemplos de alocação típica"</em>
            " daquele patamar, não a regra: a mecânica de verdade vem dos "
            <a href=nav::url(&base, "artes/regras#parametros")>"parâmetros"</a>
            ". O nível máximo que um personagem alcança em cada Arte é Centelha + 2, até 6: o mortal chega ao 2, e os níveis 5 e 6 pedem Centelha 3 e 4. Sob cada nível vão outros exemplos do que aquele patamar alcança, gastando os pontos de formas diferentes."
        </p>
        <p class="muted">
            "O nome de cada nível é o rótulo daquele degrau da Arte. Quando um Efeito Especial tem o mesmo nome em outro nível, é outra coisa, com o nível dele na página dos "
            <a href=nav::url(&base, "artes/efeitos")>"Efeitos Especiais"</a> "."
        </p>
        <div class="filtros panel">
            <div class="f-linha">
                <label class="f-busca">
                    "Buscar " <input type="search" data-fa="q" placeholder="fogo, cura, sorte…"/>
                </label>
                <button class="btn" type="button" id="limpar-filtros">"Limpar"</button>
            </div>
            {groups}
        </div>
        {sections}
        <nav class="artes-nav" data-pagefind-ignore="">
            <a class="btn" href=nav::url(&base, "artes/efeitos")>"← Efeitos Especiais"</a>
            <a class="btn" href=nav::url(&base, "artes/regras")>"Regras das Artes"</a>
        </nav>
        <ArtFilter/>
        <Markers/>
        <BackToTop/>
    }
}

fn effect_blocks(blocks: Vec<EffectBlock>) -> impl IntoView {
    blocks
        .into_iter()
        .map(|b| {
            let count = format!("{} Efeitos", b.cards.len());
            let cards = b
                .cards
                .into_iter()
                .map(|e| {
                    let params = e
                        .params
                        .into_iter()
                        .map(|p| {
                            let label = format!("{}:", p.name);
                            let detail =
                                p.detail.map(|d| view! { <span class="ef-p-esc muted">{d}</span> });
                            view! {
                                <li>
                                    <span class="ef-p-n" data-formas=p.shape>{label}</span>" "
                                    <span class="ef-p-v">{p.value}" "{detail}</span>
                                </li>
                            }
                        })
                        .collect_view();
                    let notes = e.notes.map(|n| view! { <p class="muted ef-nota">{n}</p> });
                    let also = e.also.map(|t| view! { <p class="muted ef-tb">{t}</p> });
                    view! {
                        <div class="panel efeito" id=e.anchor>
                            <div class="ef-head">
                                <span class="cinzel ef-nome">{e.name}</span>" "
                                <span class="ef-nivel" title=e.title>{e.level}</span>
                            </div>
                            <p class="ef-sabor">{e.flavor}</p>
                            <p class="ef-txt">{e.text}</p>
                            <ul class="ef-par">{params}</ul>
                            {notes}
                            {also}
                        </div>
                    }
                })
                .collect_view();
            view! {
                <details class="ef-bloco" id=format!("ef-{}", b.art_id)>
                    <summary class="ef-bloco-sum">
                        <h3 class="ef-bloco-tit">{b.art_name} " " <span class="muted">{count}</span></h3>
                    </summary>
                    <div class="grid-cards" style=GRID_STYLE>{cards}</div>
                </details>
            }
        })
        .collect_view()
}

/// `artes/efeitos`: os Efeitos Especiais agrupados por Arte, com as abas.
#[component]
pub fn ArtsEffectsPage(base: String, arts: Arts) -> impl IntoView {
    let index = arts
        .effect_blocks_elemental
        .iter()
        .chain(&arts.effect_blocks_universal)
        .map(|b| {
            view! {
                <a href=format!("#ef-{}", b.art_id) class=b.category>
                    {b.art_name.clone()} " " <span class="muted">{b.cards.len()}</span>
                </a>
            }
        })
        .collect_view();
    let groups = [
        (
            "Artes Elementais",
            "Fogo, gelo, raio, água, vento, terra, luz e sombra: o que o mundo já tem, moldado.",
            "elemental",
            arts.effect_blocks_elemental,
        ),
        (
            "Artes Universais",
            "As linhas que não pegam um elemento e sim um princípio: a vida, a morte, a mente, o espaço, o tempo.",
            "universal",
            arts.effect_blocks_universal,
        ),
    ];
    let tabs = groups
        .iter()
        .enumerate()
        .map(|(i, (title, _, cat, list))| {
            let selected = if i == 0 { "true" } else { "false" };
            view! {
                <button class="ef-aba" type="button" role="tab" data-aba=*cat aria-selected=selected>
                    {*title} " " <span class="muted">{list.len()}</span>
                </button>
            }
        })
        .collect_view();
    let sections = groups
        .into_iter()
        .map(|(title, lead, cat, list)| {
            view! {
                <section class="ef-grupo" data-cat=cat>
                    <h2 class="ef-grupo-tit">{title}</h2>
                    <p class="muted ef-grupo-sub">{lead}</p>
                    {effect_blocks(list)}
                </section>
            }
        })
        .collect_view();
    let without = (!arts.without_effects.is_empty()).then(|| {
        let names = arts.without_effects.join(", ");
        view! { <p class="muted">"Sem Efeitos por enquanto: " {names} "."</p> }
    });
    let rules = |hash: &str| nav::url(&base, &format!("artes/regras{hash}"));
    view! {
        <p class="muted cinzel" style=KICKER_STYLE>
            "Capítulo " {numeral()} " · " <a href=rules("")>"As Artes"</a>
        </p>
        <h1>"Os Efeitos Especiais"</h1>
        <p class="lead">
            "A Arte dá o elemento em estado bruto; o " <strong>"Efeito"</strong>
            " dá forma, nome e comportamento. A régua é simples: "
            <em>"se precisa de um substantivo próprio para ser descrito, é Efeito."</em>
            " Fogo é Arte, muro é Efeito, e subir a Arte nunca substitui o Efeito, porque muro não é fogo maior, é fogo com forma."
        </p>
        <div class="callout regra">
            <span class="lbl">"O que custa"</span>
            " Cada Efeito custa " <strong>"4 × o nível dele"</strong>
            " em XP, e só se compra até o nível que você tem na Arte. Na hora de conjurar, o custo é o "
            <strong>"nível do Efeito mais a soma dos níveis dos parâmetros escolhidos"</strong> ", com o "
            <a href=rules("#custo")>"desconto da Centelha"</a> " por cima. O "
            <strong>"mesmo Efeito serve a várias Artes"</strong>
            ", mudando o sabor e o efeito colateral, e quem já o conhece por uma Arte não paga XP de novo pela outra: precisa só ter a segunda Arte no nível dele."
        </div>
        <p class="muted">
            "Um Efeito compartilhado aparece sob cada Arte que o comporta, com o sabor daquela Arte. Dentro de cada Arte, os menores vêm primeiro. Os parâmetros marcados como "
            <strong>"normal"</strong>
            " são os que você escolhe na hora, até o nível da Arte, na tabela dos parâmetros das "
            <a href=rules("#parametros")>"Regras das Artes"</a>
            ", que vale para as elementais e para as universais (a Cura usa a dela)."
        </p>
        <p class="muted">
            <strong>"Efeitos da mente."</strong> " O verbete que age na cabeça do alvo traz duas linhas: "
            <strong>"Conjurar"</strong>
            ", a jogada de quem conjura (Influência + uma Habilidade social, ver "
            <a href=rules("")>"Regras das Artes"</a> "), e " <strong>"Dificuldade"</strong>
            ", a Defesa do alvo."
        </p>
        <p class="muted">
            "A linha Dificuldade dos Efeitos com Ataque segue a regra de "
            <a href=rules("")>"Conjurar e resistir"</a> "."
        </p>
        <nav class="ef-indice" data-pagefind-ignore="">{index}</nav>
        <div class="ef-abas" role="tablist" data-pagefind-ignore="">{tabs}</div>
        {sections}
        {without}
        <nav class="artes-nav" data-pagefind-ignore="">
            <a class="btn" href=rules("")>"← Regras das Artes"</a>
            <a class="btn" href=nav::url(&base, "artes/catalogo")>
                "As " {arts.art_count} " Artes →"
            </a>
        </nav>
        <Shapes/>
        <EffectTabs/>
        <BackToTop/>
    }
}
