//! A página das regras das Artes (`artes/regras`).
//!
//! Port de `src/pages/artes/regras.astro` do site atual. O texto fixo da
//! página está aqui; o texto e as tabelas que vêm do livro saem do ramo
//! `arcano` de `regras.json` pelo leitor do `centelha-data`
//! (`centelha_data::rules::ArcaneRules`: a árvore de texto por caminho e os
//! campos tipados dos números). Os nomes das Artes vêm das Artes já lidas.

use centelha_data::arcane::Node;
use centelha_data::rules::ArcaneRules;
use leptos::prelude::*;

use super::arts::{Arts, number, shape_of};
use super::nav;
use crate::back_to_top::BackToTop;
use crate::shapes::Shapes;

/// O texto de um nó, como o Astro o escreve: o texto, o número sem `.0`, e
/// nada para o que falta ou é nulo.
fn show(node: Option<&Node>) -> String {
    match node {
        Some(Node::Text(s)) => s.clone(),
        Some(Node::Number(n)) => number(*n),
        Some(Node::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

/// O que a página lê do ramo `arcano`, com o nome das Artes.
#[derive(Clone)]
pub struct RulesData {
    pub arcane: ArcaneRules,
    /// `(id, nome)` de cada Arte.
    pub art_names: Vec<(String, String)>,
    pub art_count: usize,
    pub effect_count: usize,
}

impl RulesData {
    pub fn new(arcane: ArcaneRules, arts: &Arts) -> Self {
        let art_names = arts
            .elemental
            .iter()
            .chain(&arts.universal)
            .map(|a| (a.id.clone(), a.name.clone()))
            .collect();
        RulesData {
            arcane,
            art_names,
            art_count: arts.art_count,
            effect_count: arts.effect_count,
        }
    }

    fn t(&self, path: &str) -> String {
        show(self.arcane.get(path))
    }

    fn list(&self, path: &str) -> Vec<Node> {
        self.arcane
            .list(path)
            .map(<[Node]>::to_vec)
            .unwrap_or_default()
    }

    fn strings(&self, path: &str) -> Vec<String> {
        self.list(path).iter().map(|n| show(Some(n))).collect()
    }

    fn art_name(&self, id: &str) -> String {
        self.art_names
            .iter()
            .find(|(k, _)| k == id)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| id.to_string())
    }
}

fn field(node: &Node, key: &str) -> String {
    show(node.get(key))
}

fn bullets(items: Vec<String>) -> impl IntoView {
    items
        .into_iter()
        .map(|x| view! { <li>{x}</li> })
        .collect_view()
}

/// As colunas dos graus, de 0 a 6.
fn grade_heads() -> impl IntoView {
    (0..=6).map(|g| view! { <th>{g}</th> }).collect_view()
}

fn cells(values: Vec<String>) -> impl IntoView {
    values
        .into_iter()
        .map(|v| view! { <td>{v}</td> })
        .collect_view()
}

/// Uma tabela de linhas tiradas de uma lista de mapas: a primeira coluna em
/// negrito, as outras com a classe dada.
fn map_table(
    heads: &'static [&'static str],
    rows: Vec<Node>,
    keys: &'static [&'static str],
    classes: &'static [&'static str],
) -> impl IntoView + use<> {
    let heads = heads
        .iter()
        .map(|h| view! { <th>{h.to_string()}</th> })
        .collect_view();
    let keys = keys.to_vec();
    let classes = classes.to_vec();
    let body = rows
        .into_iter()
        .map(|row| {
            let first = field(&row, keys[0]);
            let rest = keys[1..]
                .iter()
                .enumerate()
                .map(|(i, k)| {
                    let class = classes.get(i).copied().filter(|c| !c.is_empty());
                    view! { <td class=class>{field(&row, k)}</td> }
                })
                .collect_view();
            view! { <tr><td><strong>{first}</strong></td>{rest}</tr> }
        })
        .collect_view();
    view! {
        <div class="table-wrap">
            <table>
                <thead><tr>{heads}</tr></thead>
                <tbody>{body}</tbody>
            </table>
        </div>
    }
}

/// As conjurações de exemplo da conta: descrição, contas e o total.
const EXAMPLES: [(&str, &str, f64); 4] = [
    ("Acender uma vela: tudo no grau 0", "0", 0.0),
    ("Toque ardente: Dano 1 (1d6)", "1", 1.0),
    ("Dardo: Alcance 3 (4 m) + Dano 2 (2d6)", "3 + 2", 5.0),
    (
        "Labareda de 12 Ticks: Volume 3 (3 m de base) + Dano 2 (2d6) + Duração 2 (12 Ticks)",
        "3 + 2 + 2",
        7.0,
    ),
];

/// O Mana de uma conjuração de `points` pontos com a Centelha dada: o que
/// passa da Centelha, ou "grátis" (a conta da tabela do site atual).
fn mana_text(points: f64, spark: f64) -> String {
    let mana = (points - spark).max(0.0);
    if mana == 0.0 {
        "grátis".to_string()
    } else {
        number(mana)
    }
}

const RETURN_HEADS: &[&str] = &["Quem tenta", "Borda, 1 m", "2 m", "3 m", "Núcleo, 4 m"];
const RETURN_KEYS: &[&str] = &["quem", "m1", "m2", "m3", "m4"];
const NUM4: &[&str] = &["num", "num", "num", "num"];

const KICKER_STYLE: &str =
    "letter-spacing:.3em;text-transform:uppercase;font-size: var(--fs-micro);margin:0";

/// `artes/regras`.
#[component]
pub fn ArtsRulesPage(base: String, data: RulesData) -> impl IntoView {
    let u = |path: &str| nav::url(&base, path);
    let numeral = nav::NAV
        .iter()
        .find(|n| n.slug == "artes/regras")
        .map(|n| n.numeral)
        .unwrap_or("");
    let d = &data;

    // Os parâmetros por grau.
    let grade_rows = [
        ("Alcance", "improviso.graus.alcance", ""),
        (
            "Volume \u{b7} manifestação",
            "improviso.graus.volumeImproviso",
            "lado da base, no improviso",
        ),
        (
            "Volume \u{b7} matéria",
            "improviso.graus.volume",
            "lado, no Efeito Especial",
        ),
        (
            "Área",
            "improviso.graus.area",
            "diâmetro, no Efeito Especial",
        ),
        ("Alvos", "improviso.graus.alvos", ""),
        ("Dano", "improviso.graus.dano", ""),
        ("Duração breve", "improviso.graus.duracaoBreve", ""),
        ("Duração longa", "improviso.graus.duracaoLonga", ""),
    ]
    .into_iter()
    .map(|(name, path, scope)| {
        // O cartão das formas não abre na régua da manifestação.
        let shape = (!name.contains("manifesta"))
            .then(|| shape_of(name))
            .flatten();
        let scope = (!scope.is_empty()).then(|| view! { <small class="p-escopo">{scope}</small> });
        view! {
            <tr>
                <td><strong data-formas=shape>{name}</strong>{scope}</td>
                {cells(d.strings(path))}
            </tr>
        }
    })
    .collect_view();
    let anchors = d
        .strings("improviso.graus.volumeAncora")
        .iter()
        .enumerate()
        .map(|(i, a)| format!("grau {i}, {a}"))
        .collect::<Vec<_>>()
        .join(" \u{b7} ");

    let molds = d
        .list("moldes.lista")
        .into_iter()
        .map(|m| {
            let scale = m
                .list("escala")
                .map(|l| l.iter().map(|n| show(Some(n))).collect())
                .unwrap_or_default();
            let hex = m.list("hexagonos").map(|l| {
                let cells = l
                    .iter()
                    .map(|n| view! { <td><em>{show(Some(n))}</em></td> })
                    .collect_view();
                view! {
                    <tr class="hex-row"><td></td><td class="muted"><em>"hexágonos"</em></td>{cells}</tr>
                }
            });
            view! {
                <tr>
                    <td><strong>{field(&m, "nome")}</strong></td>
                    <td class="muted">{field(&m, "compra")}</td>
                    {cells(scale)}
                </tr>
                {hex}
            }
        })
        .collect_view();
    let serves = |path: &str| {
        d.list(path)
            .into_iter()
            .map(|m| {
                view! { <li><strong>{format!("{}:", field(&m, "nome"))}</strong> " " {field(&m, "serve")}</li> }
            })
            .collect_view()
    };
    let solids = d
        .list("moldes.solidos.lista")
        .into_iter()
        .map(|m| {
            let name = field(&m, "nome");
            let footprint = if field(&m, "pegada") == "circulo" {
                "redonda"
            } else if name == "Bloco" {
                "retangular (2 : 1)"
            } else {
                "quadrada"
            };
            view! {
                <tr>
                    <td><strong>{name}</strong></td>
                    <td class="muted">{field(&m, "proporcao")}</td>
                    <td class="muted">{footprint}</td>
                </tr>
            }
        })
        .collect_view();
    let reading = d
        .list("tempoDaArte.area.desvio.leitura.tabela")
        .into_iter()
        .map(|l| {
            view! {
                <li>{format!("{}: o alvo sofre ", field(&l, "resultado"))}<strong>{field(&l, "sofre")}</strong> "."</li>
            }
        })
        .collect_view();

    let examples = EXAMPLES
        .iter()
        .map(|(desc, sum, total)| {
            view! {
                <tr>
                    <td>{*desc}</td>
                    <td class="muted">{*sum}</td>
                    <td class="num">{number(*total)}</td>
                    <td class="num">{mana_text(*total, 2.0)}</td>
                    <td class="num">{mana_text(*total, 4.0)}</td>
                </tr>
            }
        })
        .collect_view();
    let abundance = d
        .list("fonteElemental.abundancia")
        .into_iter()
        .map(|a| {
            let levels = a
                .list("niveis")
                .map(|l| {
                    l.iter()
                        .map(|n| view! { <td class="ab-cel">{show(Some(n))}</td> })
                        .collect_view()
                })
                .unwrap_or_default();
            view! { <tr><td><strong>{d.art_name(&field(&a, "arte"))}</strong></td>{levels}</tr> }
        })
        .collect_view();
    let foci = d
        .list("fonteElemental.focos.lista")
        .into_iter()
        .map(|f| {
            view! { <li><strong>{d.art_name(&field(&f, "arte"))}</strong> {format!(": {}", field(&f, "foco"))}</li> }
        })
        .collect_view();
    let healing = [
        ("Alcance", "cura.graus.alcance"),
        ("Alvos", "cura.graus.alvos"),
        ("Cura", "cura.graus.cura"),
    ]
    .into_iter()
    .map(|(name, path)| view! { <tr><td><strong>{name}</strong></td>{cells(d.strings(path))}</tr> })
    .collect_view();
    let healing_cost = d
        .arcane
        .healing_cost_per_level
        .map(number)
        .unwrap_or_default();
    let source_discount = d
        .arcane
        .elemental_source_discount
        .map(number)
        .unwrap_or_default();
    let stretch = d
        .arcane
        .stretch_table
        .iter()
        .map(|e| {
            view! {
                <tr><td class="num">{number(e.above)}</td><td>{e.cost.clone()}</td><td class="num">{e.speed.clone()}</td></tr>
            }
        })
        .collect_view();
    let review = {
        let source = d.strings("fonteElemental.pendencias");
        let manifestation = d.strings("improviso.manifestacao.emAvaliacao");
        let time = d.strings("tempoDaArte.aRevisar");
        let molds_review = d.t("moldes.aRevisar");
        view! {
            <li>
                "A " <strong>"conjuração por Tradição"</strong>
                ": se um efeito Moldado rola por Habilidade própria. (A rolagem geral já está fechada: os efeitos "
                <strong>"mirados"</strong>
                " rolam Percepção + Acerto Arcano; os que tocam a mente sem projétil rolam Influência + uma Habilidade social, ver Conjurar e resistir; o resto se resolve por Dificuldade fixa, por resistência do alvo ou por tabela.)"
            </li>
            <li>
                "Revisar a " <strong>"primeira leva"</strong>
                " de Efeitos das Artes não elementais: os números e os limites de cada um ainda não passaram por mesa."
            </li>
            <li>
                "O Efeito Especial nas criaturas do " <strong>"bestiário"</strong> " está em "
                <a href=u("regras/poderes-das-criaturas")>"Os Poderes das Criaturas"</a> "."
            </li>
            {bullets(source)}
            <li>
                "Preencher " <strong>"fraquezas"</strong> " e " <strong>"resistências"</strong>
                " nas criaturas do bestiário; a regra das duas já está fechada acima."
            </li>
            <li>"Treze Efeitos " <strong>"elementais"</strong> " ainda estão sem número."</li>
            <li>"Se a penalidade de guardar feitiço acumula por feitiço, e se há teto de quantos dá para carregar."</li>
            <li>
                <strong>"Área × Volume está em revisão."</strong>
                " A escada de Volume, a tabela de estado da matéria e o corte entre corpo e jurisdição podem mudar juntos."
            </li>
            {manifestation
                .into_iter()
                .map(|p| view! { <li><strong>"Manifestação:"</strong> " " {p}</li> })
                .collect_view()}
            {time
                .into_iter()
                .map(|p| view! { <li><strong>"Tempo e área:"</strong> " " {p}</li> })
                .collect_view()}
            <li><strong>"Moldes:"</strong> " " {molds_review}</li>
            <li>
                "A " <strong>"Fonte do Elemento"</strong> " tem uma escada por elemental e nenhuma para "
                <strong>"Areia"</strong> ", " <strong>"Som"</strong>
                " e o que mais vier. Hoje um deserto cai perto do fundo da régua da Terra, que trata material solto como fraco."
            </li>
            <li>
                "Uma " <strong>"régua de luz"</strong>
                " (escuro, penumbra, tocha, dia): o que cada degrau faz nas rolagens e o que cada raça enxerga. Dela depende o "
                <strong>"halo"</strong>
                " de Luz e Sombra, que vaza para fora do volume comprado e muda a luz do lugar sem causar o efeito."
            </li>
            <li>
                <strong>"Os 46 Efeitos que ainda declaram Área precisam de molde."</strong>
                " A Área ficou sem dono quando o chão passou a se comprar por forma e o improviso passou a comprar a base com o Volume: ela só segue na tabela acima porque esses Efeitos ainda a usam. Cada um precisa do molde que lhe cabe, e isso é julgamento de Efeito, um a um. O "
                <strong>"Parar"</strong>
                " já tem o molde dele (a Explosão). A troca da Área pelos moldes está em andamento; o grau de partida de cada Efeito depende de decisão pendente. Dez deles nem chão medem, medem corpo, e deveriam usar Volume (Lascas, Onda, Maremoto, Sopro do Norte, Tromba, Sarçal, Erguer a Montanha, Clarão Cegante, Aurora, Escudo de Força). A Aura chama de "
                <strong>"Volume"</strong>
                " um parâmetro que é raio, e a escala própria da " <strong>"Neblina"</strong>
                " (125 m³ no topo) precisa ser reconciliada com o gás da tabela acima (64 m³)."
            </li>
        }
    };

    view! {
        <p class="muted cinzel" style=KICKER_STYLE>"Capítulo " {numeral}</p>
        <h1>"Regras das Artes"</h1>
        <p class="lead">
            "Como se conjura, o que custa e o que o alvo pode fazer a respeito. Cada " <strong>"Arte"</strong> " tem "
            <strong>"6 níveis"</strong> ": não é preciso " <strong>"Centelha"</strong>
            " para tocar a magia (o mortal conjura com a Mana, uma reserva própria com o valor da Força de Vontade, que não se mistura com ela), e o "
            <strong>"nível máximo"</strong> " de cada Arte é " <strong>"Centelha + 2"</strong>
            ", até 6 (Centelha 0 → 2; 1 → 3; 2 → 4; 3 → 5; 4, 5 ou 6 → 6); a profundidade (o nível da Arte) você compra e evolui um a um."
        </p>
        <p class="lead">
            "Dentro do seu nível, você monta o efeito girando os parâmetros. O panorama, os nomes da feitiçaria e as Tradições de ensino estão em "
            <a href=u("arcano")>"O Arcano"</a> "."
        </p>
        <nav class="artes-nav artes-nav-topo" data-pagefind-ignore="">
            <a class="cap-card" href=u("artes/efeitos")>
                <span class="cc-tit">"Efeitos Especiais"</span>
                <span class="cc-sub">
                    {format!("Os {} efeitos com nome próprio, agrupados por Arte: o que fazem, o que custam em XP e quais parâmetros você escolhe na conjuração.", d.effect_count)}
                </span>
            </a>
            <a class="cap-card" href=u("artes/catalogo")>
                <span class="cc-tit">{format!("As {} Artes", d.art_count)}</span>
                <span class="cc-sub">"O catálogo: os seis níveis de cada Arte, com exemplos do que aquele patamar alcança."</span>
            </a>
        </nav>
        <div class="callout regra">
            <span class="lbl">"Os dois modos"</span>
            <strong>"Feitiço"</strong>
            ", rápido, em combate: custa Ticks como uma ação (" <strong>"5"</strong>
            " quando o maior grau investido na conjuração vai de 0 a 3, " <strong>"6"</strong> " quando é 4, "
            <strong>"7"</strong>
            " quando é 5 ou 6; o nível da Arte de quem conjura não entra nessa conta), mais o Mana. Esses Ticks são a Velocidade da conjuração, e a Arte sai no "
            <a href="#tempo"><strong>"Tick do Golpe, o penúltimo"</strong></a>
            ", ao contrário da ação comum, que no sistema Normal resolve no primeiro Tick. \u{a0} "
            <strong><a href="#rituais">"Ritual"</a></strong>
            ", estendido: a conjuração passa a levar minutos ou horas e, em troca, sai mais barato ou alcança mais alto do que a sua Arte permitiria."
        </div>
        <section class="arcano-regras">
            <h2>"Como se aprende uma Arte"</h2>
            <p>
                "Nenhuma Arte se aprende no vácuo: você chega a ela por uma "
                <a href=u("arcano#trilhas")>"Tradição de Feitiçaria"</a> ", o método que explica " <em>"como"</em>
                " aquele personagem passou a lançá-la, e que ele declara por Arte. A mesma Arte chega por Tradições diferentes (a do erudito, a do monge, a do pactuário). Comprar e subir uma Arte custa "
                <strong>"XP"</strong> " como qualquer traço, e a Tradição " <strong>"não muda número nenhum"</strong>
                ": ela diz o que é preciso ter vivido para aquilo fazer sentido, e com que cara o feitiço sai."
            </p>
            <p>
                "Preço: cada nível de Arte custa 10 + 5 × o nível em XP, pagos um a um (a Arte 4 inteira custa 15 + 20 + 25 + 30 = 90 XP). Cada Efeito Especial custa 4 × o nível dele, pago cheio, sem somar os de baixo."
            </p>
        </section>
        <section class="arcano-regras">
            <ResistSection base=base.clone()/>
            <h2 id="parametros">"Os parâmetros"</h2>
            <p>
                "Você não decora feitiços prontos: você " <strong>"monta"</strong>
                " cada conjuração investindo pontos nos parâmetros. Cada grau custa " <strong>"1 ponto"</strong>
                ", e nenhum parâmetro passa do " <strong>"nível da Arte"</strong> " que você tem."
            </p>
            <p>
                "Nível da Arte e grau investido não são a mesma coisa. O nível da Arte é o que você comprou com XP, e limita os parâmetros: nenhum passa dele. O grau investido é o que você paga em pontos naquela conjuração. Quando um verbete diz \"por nível\", o sentido é um destes, e o próprio verbete deixa claro qual. Num parâmetro que você compra, é o grau que você comprou naquele parâmetro, até o nível da Arte (acima disso é esticar): o Dano da Arma Elemental é assim. Quando um Efeito tira a Dificuldade ou a Cura do \"nível\", a conta usa o maior grau investido naquela conjuração, e não o nível da Arte de quem conjura; com todos os graus no piso o Efeito não produz nada, então também não há o que resistir. O que se mede pelo nível da Arte de quem conjura (a Penalidade, a Resistência, o peso e a distância) diz \"nível da Arte\" no próprio verbete. E parâmetro fixo não tem nível: é um valor pronto, como o toque do Alcance ou a cura lenta e constante da Mão Firme."
            </p>
            <p>
                <strong>"O grau 0 é o piso de tudo, e é grátis."</strong>
                " Toque, uma manifestação mínima do elemento, nenhum alvo escolhido, nenhum dano, instantâneo. Um feitiço inteiro no grau 0 custa zero pontos, e é assim que se acende uma vela."
            </p>
            <p class="muted">
                "A tabela abaixo vale para as " <strong>"Artes elementais"</strong>
                " e, até cada uma ganhar régua própria, também para as universais: Alcance, Alvos, Área e as duas Durações saem daqui, e os parâmetros de efeito (Dano, Cura, Bônus, Absorção) estão em cada Efeito. A Cura é a exceção: tem régua própria ("
                <a href="#cura">"A economia da Cura"</a> ")."
            </p>
            <div class="table-wrap">
                <table class="graus-tab">
                    <thead><tr><th>"Parâmetro"</th>{grade_heads()}</tr></thead>
                    <tbody>{grade_rows}</tbody>
                </table>
            </div>
            <div class="callout regra">
                <span class="lbl">"O Volume tem duas réguas"</span> " " {d.t("improviso.graus.notaVolumeImproviso")}
            </div>
            <p class="muted">{format!("A régua da matéria em números redondos: {anchors}.")}</p>
            <div class="callout regra">
                <span class="lbl">"Área ou Volume, nunca as duas"</span> " " {d.t("improviso.graus.notaAreaOuVolume")}
            </div>
            <p>{d.t("improviso.graus.volumePorEstado.nota")}</p>
            {map_table(
                &["Estado", "Volume", "Artes", "Por quê"],
                d.list("improviso.graus.volumePorEstado.tabela"),
                &["estado", "fator", "artes", "porque"],
                &["", "", "muted"],
            )}
            <p class="formula">{d.t("improviso.graus.volumePorEstado.exemplo")}</p>
            <p class="muted">
                {d.t("improviso.graus.volumePorEstado.notaGranular")} " Sobre as Escolas, ver "
                <a href=u("arcano#escolas")>"O Arcano"</a> "."
            </p>
            <p class="muted">{d.t("improviso.graus.notaVolume")}</p>
            <p class="muted">{d.t("improviso.graus.notaArea")}</p>
            <p class="muted">{d.t("improviso.graus.notaAlcance")}</p>
            <p class="muted">{d.t("improviso.graus.notaAlvos")}</p>

            <h2 id="combinacoes">"O que o improviso pode combinar"</h2>
            <p>{d.t("improviso.combinacoes.nota")}</p>
            <p>{d.t("improviso.combinacoes.definicoes")}</p>
            <div class="callout regra">
                <span class="lbl">"As três travas"</span> " "
                <strong>"1."</strong> " " {d.t("improviso.combinacoes.areaNuncaColocada")} <br/>
                <strong>"2."</strong> " " {d.t("improviso.combinacoes.duracaoPedeConcentracao")} <br/>
                <strong>"3."</strong> " " {d.t("improviso.combinacoes.teto")}
            </div>
            {map_table(
                &["Improviso", "O que paga", "Como se manifesta", "Quem é atingido"],
                d.list("improviso.combinacoes.lista"),
                &["nome", "paga", "forma", "atinge"],
                &[],
            )}
            <div class="callout regra">
                <span class="lbl">"O que sai de graça"</span> " " {d.t("improviso.combinacoes.grauZero")}
            </div>
            <p>
                <strong>"Sustentar cobra a cada 6 Ticks."</strong>
                {format!(
                    " Cada janela de 6 Ticks gasta a ação daquele lance e não cobra Mana: a Duração se paga uma vez, na conjuração. A janela não se soma à Duração comprada: é a Duração sendo sustentada, e quem para de sustentar perde o resto. A janela é de 6 Ticks mesmo quando a conjuração levou 5, porque é a régua da Duração (6, 12, 24 Ticks) que a define. Sustentar {}, {}, e {}. Ao sofrer dano, {}.",
                    d.t("improviso.combinacoes.concentracao.acao"),
                    d.t("improviso.combinacoes.concentracao.posicao"),
                    d.t("improviso.combinacoes.concentracao.defesa"),
                    d.t("improviso.combinacoes.concentracao.aoSofrerDano"),
                )}
            </p>
            <p>
                "A Duração 1 de um sustentado compra só presença: a chama fica de pé 6 Ticks e fere quem estiver nela no fim da janela. Quem sair antes do 6º Tick não leva o pulso. Para ferir duas vezes é preciso Duração 2 (12 Ticks, dois pulsos)."
            </p>
            <ul class="fraq">{bullets(d.strings("improviso.combinacoes.consequencias"))}</ul>
            <p class="muted">{d.t("improviso.combinacoes.escopo")}</p>

            <h2 id="manifestacao">"Como a Arte crua se manifesta"</h2>
            <p>{d.t("improviso.manifestacao.nota")}</p>
            <div class="callout regra">
                <span class="lbl">"Quem paga a base"</span> " " {d.t("improviso.manifestacao.oParametro")}
            </div>
            <ol class="mani-regra">{bullets(d.strings("improviso.manifestacao.regra"))}</ol>
            <div class="table-wrap">
                <table class="graus-tab">
                    <thead><tr><th>"Grau"</th>{grade_heads()}</tr></thead>
                    <tbody>
                        <tr><td><strong>"Lado da base"</strong></td>{cells(d.strings("improviso.manifestacao.ladoBase"))}</tr>
                    </tbody>
                </table>
            </div>
            <p class="formula mat">{d.t("improviso.manifestacao.formula")}</p>
            <p class="muted">{d.t("improviso.manifestacao.formulaNota")}</p>

            <h3>"As três aberturas"</h3>
            <p>{d.t("improviso.manifestacao.aberturas.porque")}</p>
            {map_table(
                &["Abertura", "Raio", "Volume", "Fração"],
                d.list("improviso.manifestacao.aberturas.tabela"),
                &["abertura", "raio", "volume", "fracao"],
                &["", "", "muted"],
            )}
            <p class="muted">{d.t("improviso.manifestacao.aberturas.nota")}</p>
            <div class="callout regra">
                <span class="lbl">"A trava das fatias"</span> " " {d.t("improviso.manifestacao.travaFatias.regra")}
                <ul class="fraq">{bullets(d.strings("improviso.manifestacao.travaFatias.consequencias"))}</ul>
            </div>

            <h3>"O que sobe e o que desce quando se abre"</h3>
            {map_table(
                &["Botão", "O que sobe", "O que desce"],
                d.list("improviso.manifestacao.botoes.tabela"),
                &["botao", "sobe", "desce"],
                &[],
            )}
            <p>{d.t("improviso.manifestacao.botoes.cercarSe")}</p>

            <h3>"As duas alturas: o ápice e o pé da base"</h3>
            <p>{d.t("improviso.manifestacao.alturas.regra")}</p>
            <p class="muted">{d.t("improviso.manifestacao.alturas.exemplo")}</p>
            <p class="muted">{d.t("improviso.manifestacao.alturas.deGraca")}</p>
            {map_table(
                &["Ápice", "A face de baixo é", "Toca o chão"],
                d.list("improviso.manifestacao.alturas.tabela"),
                &["apice", "faceDeBaixo", "tocaOChao"],
                &[],
            )}
            <p>{d.t("improviso.manifestacao.alturas.consequencia")}</p>
            <p class="formula">{d.t("improviso.manifestacao.estadoNoLado.formula")}</p>
            <p class="muted">{d.t("improviso.manifestacao.estadoNoLado.nota")}</p>
            <div class="callout regra">
                <span class="lbl">"O que a manifestação não faz"</span>
                <ul class="fraq">{bullets(d.strings("improviso.manifestacao.naoFaz"))}</ul>
            </div>
            <p class="muted">{d.t("improviso.manifestacao.escopo")}</p>

            <h2 id="moldes">"Os moldes: o chão se compra por forma"</h2>
            <p>{d.t("moldes.nota")}</p>
            <p class="muted">{d.t("moldes.porque")}</p>
            <div class="callout regra">
                <span class="lbl">"A fronteira"</span> " " {d.t("moldes.fronteira.nota")} <br/>
                <strong>"Improviso:"</strong> " " {d.t("moldes.fronteira.improviso")} <br/>
                <strong>"Efeito Especial:"</strong> " " {d.t("moldes.fronteira.efeito")}
            </div>
            <div class="table-wrap">
                <table class="graus-tab">
                    <thead><tr><th>"Molde"</th><th>"Compra"</th>{grade_heads()}</tr></thead>
                    <tbody>{molds}</tbody>
                </table>
            </div>
            <p class="muted">{d.t("moldes.hexagonosNota")}</p>
            <ul class="fraq">{serves("moldes.lista")}</ul>
            <p>{d.t("moldes.alcance.nota")}</p>
            {map_table(
                &["Molde", "g3", "g4", "g5", "g6"],
                d.list("moldes.alcance.tabela"),
                &["molde", "g3", "g4", "g5", "g6"],
                &[],
            )}
            <p class="muted">{d.t("moldes.aura.nota")}</p>
            <p>{d.t("moldes.cadeiaRessalva")}</p>
            <p>{d.t("moldes.cuboide")}</p>
            <p class="muted">{d.t("moldes.volume")}</p>

            <h3 id="solidos">"Os moldes do volume: a matéria também tem forma"</h3>
            <p>{d.t("moldes.solidos.nota")}</p>
            <p class="muted">{d.t("moldes.solidos.porque")}</p>
            <div class="callout regra">
                <span class="lbl">"A proporção é fixa"</span> " " {d.t("moldes.solidos.trava")}
            </div>
            <div class="table-wrap">
                <table>
                    <thead><tr><th>"Molde"</th><th>"Proporção"</th><th>"Pegada"</th></tr></thead>
                    <tbody>{solids}</tbody>
                </table>
            </div>
            <ul class="fraq">{serves("moldes.solidos.lista")}</ul>
            <p class="muted">{d.t("moldes.solidos.padraoPorque")}</p>
            <p class="muted">{d.t("moldes.solidos.aRevisar")}</p>

            <h2 id="tempo">"O tempo da Arte, e quem está no caminho"</h2>
            <p>{d.t("tempoDaArte.nota")}</p>
            <div class="callout regra">
                <span class="lbl">"A Arte sai no Tick do Golpe"</span> " " {d.t("tempoDaArte.ultimoTick.regra")}
            </div>
            <p class="muted">{d.t("tempoDaArte.ultimoTick.excecao")}</p>
            <p>{d.t("tempoDaArte.ultimoTick.porque")}</p>
            {map_table(
                &["Momento", "O que fica decidido"],
                d.list("tempoDaArte.ultimoTick.tabela"),
                &["momento", "decide"],
                &[],
            )}
            <p><strong>{d.t("tempoDaArte.ultimoTick.resumo")}</strong></p>

            <h3>"Ler o sinal: identificar o que está sendo conjurado"</h3>
            <p>{d.t("tempoDaArte.identificar.nota")}</p>
            <p><strong>"De graça, por conhecer o ofício."</strong> " " {d.t("tempoDaArte.identificar.deGraca")}</p>
            <p><strong>"Por teste, para todos os outros."</strong> " " {d.t("tempoDaArte.identificar.teste")}</p>
            <ul class="fraq">{bullets(d.strings("tempoDaArte.identificar.regras"))}</ul>
            <p class="muted">{d.t("tempoDaArte.identificar.ocultismo")}</p>

            <h3>"A área não se esquiva nem se bloqueia. Ela se abandona."</h3>
            <p>{d.t("tempoDaArte.area.regra")}</p>
            <p>
                <strong>{format!("1. {}.", d.t("tempoDaArte.area.deslocamentoLivre.titulo"))}</strong> " "
                {d.t("tempoDaArte.area.deslocamentoLivre.texto")}
            </p>
            <p class="muted">{d.t("tempoDaArte.area.deslocamentoLivre.semGabarito")}</p>
            <p>
                <strong>{format!("2. {}.", d.t("tempoDaArte.area.desvio.titulo"))}</strong> " "
                {d.t("tempoDaArte.area.desvio.texto")}
            </p>
            <p class="formula">{d.t("tempoDaArte.area.desvio.formula")}</p>
            <p>{d.t("tempoDaArte.area.desvio.dificuldadeEDaTarefa")}</p>
            <p class="muted">{d.t("tempoDaArte.area.desvio.escada")}</p>
            {map_table(
                &["Metros até ficar fora", "Custa", "Dif. metade do dano", "Dif. dano nenhum"],
                d.list("tempoDaArte.area.desvio.tabela"),
                &["metros", "custa", "metade", "nada"],
                &[],
            )}
            <div class="callout regra">
                <span class="lbl">"Como se lê o resultado"</span> " " {d.t("tempoDaArte.area.desvio.leitura.nota")}
                <ul class="fraq">{reading}</ul>
            </div>
            <p>{d.t("tempoDaArte.area.desvio.leitura.porqueDobrar")}</p>
            <ul class="fraq">{bullets(d.strings("tempoDaArte.area.desvio.condicoes"))}</ul>
            <p>{d.t("tempoDaArte.area.desvio.ficarParado")}</p>
            <p class="muted">{d.t("tempoDaArte.area.desvio.retorno.nota")}</p>
            {map_table(RETURN_HEADS, d.list("tempoDaArte.area.desvio.retorno.tabela"), RETURN_KEYS, NUM4)}
            <p class="muted">{d.t("tempoDaArte.area.desvio.retorno.notaCentelha")}</p>
            {map_table(RETURN_HEADS, d.list("tempoDaArte.area.desvio.retorno.tabelaCentelha"), RETURN_KEYS, NUM4)}
            <p>{d.t("tempoDaArte.area.desvio.retorno.leitura")}</p>
            <p>{d.t("tempoDaArte.area.desvio.direcoes")}</p>
            <div class="callout regra">
                <span class="lbl">"O molde medido contra o Deslocamento de Batalha"</span> " "
                {d.t("tempoDaArte.area.contraOPasso")}
            </div>

            <h2 id="custo">"O que custa, e o desconto da Centelha"</h2>
            <div class="callout regra">
                <span class="lbl">"A conta"</span> " Some os níveis investidos: " <strong>"1 ponto por nível"</strong>
                ", igual em todos os parâmetros. Do total, subtraia a sua " <strong>"Centelha"</strong>
                ". O que sobrar é o custo em " <strong>"Mana"</strong>
                ", um por um. Se o total couber dentro da Centelha, o efeito é " <strong>"grátis e ilimitado"</strong> "."
            </div>
            <div class="table-wrap">
                <table>
                    <thead>
                        <tr><th>"Conjuração"</th><th>"Conta"</th><th>"Pontos"</th><th>"Mana com Centelha 2"</th><th>"com Centelha 4"</th></tr>
                    </thead>
                    <tbody>{examples}</tbody>
                </table>
            </div>
            <p class="muted">"A Labareda de 12 Ticks fere duas vezes, no 6º e no 12º Tick depois de sair."</p>
            <p>
                "O desconto muda o jeito de jogar conforme a Centelha sobe: o mesmo dardo que era caro no começo vira troco, e lá pelo fim os truques pequenos deixam de ter custo nenhum. E o que está escrito nos seis níveis de cada Arte, mais abaixo, é "
                <em>"exemplo de alocação típica"</em> " daquele patamar, não a regra."
            </p>

            <h2 id="fonte">"A fonte do elemento"</h2>
            <p>{d.t("fonteElemental.resumo")}</p>
            <div class="callout regra">
                <span class="lbl">"Mais um de desconto"</span> " Conjurar a partir de uma fonte do elemento vale "
                <strong>{format!("+{source_discount}")}</strong>
                {format!(", somado à sua Centelha, na hora de calcular o Mana. {}", d.t("fonteElemental.requisito"))}
            </div>
            <p><strong>"A fonte se gasta."</strong> " " {d.t("fonteElemental.consumo")}</p>
            <p>
                <strong>"O que fica no mundo."</strong>
                {format!(" {} {}", d.t("fonteElemental.permanencia.comMateria"), d.t("fonteElemental.permanencia.semMateria"))}
            </p>
            <p class="muted">{d.t("fonteElemental.permanencia.excecao")}</p>
            <div class="table-wrap">
                <table class="ab-tab">
                    <thead>
                        <tr><th>"Elemento"</th><th>"1"</th><th>"2"</th><th>"3"</th><th>"4"</th><th>"5"</th><th>"6"</th></tr>
                    </thead>
                    <tbody>{abundance}</tbody>
                </table>
            </div>
            <p class="muted">{d.t("fonteElemental.notaAr")}</p>
            <p>
                <strong>"No Ritual."</strong>
                {format!(" {} {}", d.t("fonteElemental.ritual.texto"), d.t("fonteElemental.ritual.contrapartida"))}
            </p>
            <p class="muted">{format!("{} {}", d.t("fonteElemental.zeraCusto"), d.t("fonteElemental.penalidade"))}</p>
            <h3>"Focos das Artes que não têm elemento"</h3>
            <p>{d.t("fonteElemental.focos.texto")}</p>
            <ul class="focos">{foci}</ul>

            <h2 id="cura">"A economia da Cura"</h2>
            <p>{d.t("cura.resumo")}</p>
            <p class="muted">
                "A régua da Cura " <strong>"não tem grau 0"</strong>
                ": ela começa em 1, diferente dos outros parâmetros do catálogo (que têm um grau 0 de graça). Não há Cura \"de brinde\". A exceção é o "
                <strong>"primeiro alvo"</strong>
                ": o nível 1 de Alvos é grátis. A partir do 2º, cada nível custa 2 de Mana, como em todo parâmetro da Cura (dois alvos, 2; três alvos, 4), e o toque continua pagando o Alcance 1."
            </p>
            <div class="table-wrap">
                <table class="graus-tab">
                    <thead>
                        <tr><th>"Parâmetro"</th><th>"1"</th><th>"2"</th><th>"3"</th><th>"4"</th><th>"5"</th><th>"6"</th></tr>
                    </thead>
                    <tbody>{healing}</tbody>
                </table>
            </div>
            <div class="callout regra">
                <span class="lbl">"Dobro do preço"</span> " Cada nível de parâmetro da Cura custa "
                <strong>{format!("{healing_cost} de Mana")}</strong>
                ", não 1. Um toque que devolve 2d6 é Cura 4 mais Alcance 1: " <strong>"10 de Mana"</strong>
                " antes do desconto da Centelha."
            </div>
            <p><strong>"Vale para toda Arte que cura."</strong> " " {d.t("cura.outrasArtes")}</p>
            <p>
                <strong>"Tratar."</strong> " Em quem está em 0 PV ou menos, a Arte de cura soma ao teste de Tratar do dia: "
                <strong>"+3 por nível da Arte de Cura"</strong> ", ou " <strong>"+1 por nível da Arte de Vida"</strong>
                ", o que for maior (não se somam). A Mana é a do nível usado, 2 por nível como toda cura, menos a Centelha (pode sair de graça). Um Efeito específico vale o que ele disser. Ver "
                <a href=u("regras/vida-ferimentos-cura#tratar")>"Vida, Ferimentos & Cura"</a> "."
            </p>
            <p><strong>"Sem área."</strong> " " {d.t("cura.semArea")}</p>
            <p><strong>"Dividida entre os alvos."</strong> " " {d.t("cura.divide")}</p>
            <p class="muted">{d.t("cura.nota")}</p>

            <TailSection base=base.clone() stretch=stretch.into_any() ritual_scale=d.strings("ritual.escalaTempo").join(" \u{b7} ") ritual_discount=d.t("ritual.modoDesconto.exemplo") ritual_above=d.t("ritual.modoAcima.exemplo") recovery_normal=d.t("recuperacaoMana.normal") recovery_rest=d.t("recuperacaoMana.descanso") resistance=d.t("fraquezas.resistencia") resistance_examples=d.strings("fraquezas.resistenciaExemplos") review=review.into_any()/>
        </section>
        <nav class="artes-nav" data-pagefind-ignore="">
            <a class="btn" href=u("artes/efeitos")>"Efeitos Especiais →"</a>
            <a class="btn" href=u("artes/catalogo")>{format!("As {} Artes →", d.art_count)}</a>
        </nav>
        <Shapes/>
        <BackToTop/>
    }
}

/// "Conjurar e resistir": só texto fixo da página.
#[component]
fn ResistSection(base: String) -> impl IntoView {
    let u = |path: &str| nav::url(&base, path);
    view! {
        <h2>"Conjurar e resistir"</h2>
        <p>
            "O que o alvo opõe depende da " <em>"natureza"</em>
            " do efeito: magia que voa se esquiva, magia que invade o corpo se aguenta, magia que toca a mente se enfrenta com a alma."
        </p>
        <div class="table-wrap">
            <table>
                <thead><tr><th>"Natureza do efeito"</th><th>"Como o alvo resiste"</th><th>"Exemplos"</th></tr></thead>
                <tbody>
                    <tr>
                        <td><strong>"Dano e projéteis"</strong></td>
                        <td><strong>"Defesa"</strong> " para acertar (Esquiva; Bloqueio só contra o que deixou matéria no mundo); depois o dano − " <strong>"Absorção"</strong></td>
                        <td>"dardo de fogo, raio, lança de gelo"</td>
                    </tr>
                    <tr>
                        <td><strong>"Corpo e veneno"</strong></td>
                        <td><strong>"Vigor + Resistência"</strong> " (fortitude)"</td>
                        <td>"transmutação, doença, frio que enrijece, paralisia"</td>
                    </tr>
                    <tr>
                        <td><strong>"Mente e alma"</strong></td>
                        <td><strong>"Defesa Mental"</strong> " (passiva), ou " <strong>"Defesa Social"</strong> " nos Efeitos que persuadem; sem projétil, quem conjura rola Influência + uma Habilidade social"</td>
                        <td>"medo, fascínio, ilusão, domínio"</td>
                    </tr>
                    <tr>
                        <td><strong>"Aprisionamento e contato"</strong></td>
                        <td><strong>"Defesa"</strong> " para prender; o preso fica " <strong>"Preso"</strong> " (não se desloca, mas age) e escapa com " <strong>"Força + Atletismo"</strong> " contra a Dificuldade do Efeito, uma tentativa por ação"</td>
                        <td>"correntes, lama, agarrão telecinético"</td>
                    </tr>
                    <tr>
                        <td><strong>"Em aliado, objeto ou cenário"</strong></td>
                        <td>"sem resistência"</td>
                        <td>"cura, escudo, muralha, luz"</td>
                    </tr>
                </tbody>
            </table>
        </div>
        <p>
            <strong>"A Arte que toca a mente."</strong>
            " Sem projétil, conjurar contra a mente é uma interação com o alvo: quem conjura rola "
            <strong>"Influência + uma Habilidade social"</strong>
            ", com 2 × o menor entre a Centelha e essa Habilidade (a conta do ataque social, em "
            <a href=u("regras/relacoes-sociais")>"Relações Sociais"</a>
            ", sem o Acerto da Abordagem). Cada Efeito diz a Habilidade na linha " <strong>"Conjurar"</strong>
            "; uma Arte da mente usada sem Efeito (as Sugestões, Simpatias e Encantos do catálogo) pede a Habilidade que a intenção declarada pedir, escolhida pelo Mestre como em qualquer cena social."
        </p>
        <p>
            "O nível da Arte não entra na jogada: compra o tamanho do efeito (Alvos, Área, Duração). O total é comparado à Defesa que o Efeito nomeia na linha "
            <strong>"Dificuldade"</strong> ": a " <strong>"Defesa Social"</strong>
            " nos Efeitos que persuadem (Boa Impressão), a " <strong>"Defesa Mental"</strong>
            " nos que impõem, plantam ou apagam algo na cabeça do alvo (ver "
            <a href=u("regras/defesas")>"As Três Defesas"</a>
            "). Com projétil, o efeito que sai da mão e voa até o alvo, a jogada é Percepção + Acerto Arcano, contra a Defesa Mental."
        </p>
        <p>
            "Um total vale para todos os alvos do Efeito: na área, ele é comparado à Defesa de cada um que estiver dentro; nos Efeitos em você, à de cada um que olhar enquanto durar; no Círculo, à de cada criatura da categoria que tentar cruzar a linha. A Margem é a de sempre, cada 6 pontos acima da Defesa, e a Força de Vontade do alvo vale como em toda Defesa. Como o alvo resiste depois que o total passou está em "
            <strong>"Resistir a um Efeito da mente"</strong> ", logo abaixo."
        </p>
        <p>
            <strong>"Resistir a um Efeito da mente."</strong>
            " Depois que o total superou a Defesa, o alvo pode pagar Força de Vontade para recusar o Efeito: "
            <strong>"1 + Margem"</strong> ", com teto de 4, fora do limite de 1 ponto por ação (ver "
            <a href=u("regras/defesas")>"As Três Defesas"</a> ", Queimar Força de Vontade). Com Margem 1 ou mais, pode pagar só "
            <strong>"1"</strong> ": o Efeito pega, mas dura " <strong>"um grau a menos na régua de Duração dele"</strong>
            ", a Breve ou a Longa da tabela dos parâmetros, a que o Efeito usa. Se o Efeito já está no menor grau, pagar 1 o anula: é o caso do Efeito sem linha de Duração, que é instantâneo, grau 0."
        </p>
        <p>
            "Exemplos: um Pavor de Duração breve 3 (24 Ticks) cai para 12 Ticks; uma Sugestão Plantada de Duração longa 2 (1 hora) cai para 10 minutos; um Pavor de Duração breve 1 (6 Ticks) cai para o grau 0, no máximo 1 Tick; Esquecer, que não tem Duração, é anulado."
        </p>
        <p>
            "Nos Efeitos contra a Defesa Social (Boa Impressão), o Resistir é o de Relações Sociais, que só tem recusar ou ceder: quem paga 1 + Margem recusa, e quem não paga sofre o Efeito inteiro."
        </p>
        <p>
            <strong>"A Centelha nas Artes."</strong>
            " A jogada de uma Arte soma 2 × o menor entre a sua Centelha e a Habilidade da jogada, como toda jogada do livro: no efeito mirado, Percepção + Acerto Arcano + 2 × o menor entre a Centelha e o Acerto Arcano (Acerto Arcano 0 dá bônus 0); na Arte que toca a mente sem projétil, o mesmo com a Habilidade social do Efeito."
        </p>
        <p>
            "O dano soma a sua Centelha inteira, uma vez por pulso, em toda Arte que causa dano. O alvo usa as próprias regras: as Defesas já trazem a Centelha dele, e jogadas como Vigor + Resistência somam 2 × o menor entre a Centelha e a Habilidade usada. O desconto de Mana é outra conta: a Centelha inteira, tirada do total de pontos. O mortal (Centelha 0) não soma nada em nenhuma das três. Exemplo: Veil, Centelha 4, Percepção 3 e Acerto Arcano 3, rola 3d6 + 6; com Acerto Arcano 0 rolaria 1d6 + 2."
        </p>
        <p>
            <strong>"Área é a exceção da primeira linha."</strong>
            " O que voa se esquiva, mas o que ocupa espaço não: contra área não há Esquiva nem Bloqueio, e a saída é "
            <a href="#tempo">"sair do lugar"</a>
            ", gastando um Tick do próprio deslocamento se você ainda tem a vez, ou um Tick por metro se já agiu. A jogada é a mesma nos dois casos."
        </p>
        <p>
            "Bloquear uma Arte segue o mesmo critério da armadura. O que deixou matéria no mundo se bloqueia como a arma de arremesso do mesmo tamanho, pelo "
            <a href=u("regras/combate")>"Cap. IX"</a>
            ": o Projétil Conjurado de Gelo, Água ou Terra, do tamanho de flecha, virote, dardo ou adaga, é projétil rápido, e só se bloqueia com escudo hábil e estando apto; a lança de gelo arremessada e a pedra grande (a da Terra, a do Arremesso de Forças) são lentas, e se bloqueiam com arma, escudo ou mão. Fogo, raio, luz, sombra e o gelo que evapora ao acertar não se bloqueiam: contra eles, só Esquiva. Quando o Efeito diz que é imbloqueável (a Prisão, a Paralisia), vale o Efeito."
        </p>
        <p>"Nos Efeitos com Ataque, a Dificuldade é a de resistir ao efeito secundário; o Mestre pede o teste que fizer sentido."</p>
        <p class="muted">
            "A " <strong>"Margem"</strong>
            " funciona como no combate: cada 6 pontos acima da resistência rende +1d6 de dano ou um efeito mais forte. A armadura só protege contra dano "
            <strong>"físico"</strong>
            ": o que decide é se o efeito deixou matéria no mundo. Fogo, raio e o gelo que evapora ao acertar são absorvidos só pela Centelha; a lança de gelo que se pega na mão, ou o cascalho que a Terra arranca do chão, entram nas trilhas normais."
        </p>
        <p class="muted">
            "Nos Efeitos vale o mesmo critério. O que é feito de Gelo, Água ou Terra e acerta como corpo (o projétil, a arma, as lascas) é matéria, e a armadura absorve normalmente, no modo do formato. O dano de Fogo, Raio, Luz e Sombra não passa pela armadura: a resistência ao elemento corta pela metade, e depois a Centelha de quem recebe absorve. O dano de "
            <strong>"Luz"</strong>
            " fere todos, mas pela metade: contra quem não tem fraqueza a luz, ela é uma resistência como as outras e segue a mesma ordem: a armadura (que não absorve Luz), depois a metade, arredondando para cima, e só então a Centelha de quem recebe (dano 10 e Centelha 2: 10 ÷ 2 = 5, menos 2 = 3); quem também resiste à Luz soma as duas metades e leva um quarto; contra quem tem fraqueza a luz, vale o inteiro e é o da Fraqueza das Artes (ignora a Absorção e agrava), salvo quando a Regeneração da criatura manda ("
            <a href=u("regras/poderes-das-criaturas")>"Os Poderes das Criaturas"</a> ")."
        </p>
        <p class="muted">
            "Dentro de uma ação Longa (" <a href=u("regras/acoes-e-sistema#longa")>"Cap. VIII"</a>
            "), que não rola, o Efeito só conta se durar o intervalo inteiro, ou se for relançado sem falha, pagando a Mana a cada vez. Cada +1d6 de bônus do Efeito vale +3, e um bônus fixo entra como o próprio valor. Nova rolagem e bônus atado a uma rolagem só (como o de Maré da Sorte) não valem na Longa."
        </p>
    }
}

/// Do "Passar do seu limite" ao fim: texto fixo e os poucos campos do livro.
#[component]
#[allow(clippy::too_many_arguments)]
fn TailSection(
    base: String,
    stretch: AnyView,
    ritual_scale: String,
    ritual_discount: String,
    ritual_above: String,
    recovery_normal: String,
    recovery_rest: String,
    resistance: String,
    resistance_examples: Vec<String>,
    review: AnyView,
) -> impl IntoView {
    let u = |path: &str| nav::url(&base, path);
    view! {
        <h2>"Passar do seu limite"</h2>
        <p>
            "Investir mais Mana não fura o teto sozinho: quem tem Fogo 3 leva todos os parâmetros até o nível 3 pagando o preço normal, mas para alcançar o "
            <strong>"4"</strong> " precisa esticar. Cada nível acima do seu nível de Arte "
            <strong>"encarece o parâmetro"</strong> " e " <strong>"alonga a ação"</strong> "."
        </p>
        <div class="table-wrap">
            <table>
                <thead><tr><th>"Níveis acima"</th><th>"Custo do parâmetro"</th><th>"Velocidade"</th></tr></thead>
                <tbody>{stretch}</tbody>
            </table>
        </div>
        <p class="muted">
            "A Velocidade que o esticar multiplica é a do conjuro antes de esticar, pelo maior grau investido até o seu nível de Arte. No exemplo abaixo, o maior grau é 2 (Velocidade 5), e esticar dá 10 e depois 15."
        </p>
        <p class="formula">
            "Centelha 2, Arte 2, um jorro sustentado. Volume 2 + Dano 2 + Duração 2 = " <strong>"6 pontos"</strong>
            ", Velocidade 5. Puxando para Volume 3, um nível acima: 3×2 + 2 + 2 = " <strong>"10 pontos"</strong>
            ", Velocidade 10. Para Volume 4, dois acima: 4×3 + 2 + 2 = " <strong>"16 pontos"</strong> ", Velocidade 15."
        </p>
        <div class="callout regra">
            <span class="lbl">"Você decide no fim"</span>
            " Não é preciso anunciar de saída até onde vai. No " <strong>"Tick do Golpe de cada ciclo"</strong>
            " você escolhe: soltar o efeito agora, ou segurar e subir mais um nível de um parâmetro. O Golpe do ciclo n cai no Tick "
            <strong>"n × V − 1"</strong>
            ", em que V é a Velocidade da conjuração: numa ação de Velocidade 5, você decide no Tick 4; se esticar, decide de novo no 9, e outra vez no 14. Quem interromper você no meio leva a conjuração junto. Só o ciclo final leva Recuperação, e o sinal esticado dura T − 1 Ticks (T é a Velocidade esticada: 9 Ticks quando a ação passa a 10)."
        </div>
        <p class="muted">
            "O Tick em que você decide esticar ainda é Preparo (−2 na Defesa), e só o Tick em que a Arte sai é Golpe (−4). Quem segura no Tick 4 da Velocidade 5 não tem Recuperação no 5: esse Tick já é Preparo do ciclo seguinte."
        </p>
        <p class="muted">
            "O preço sobe rápido de propósito: forçar o limite é recurso de momento decisivo, não atalho para o estudo. E o desconto da Centelha continua valendo sobre o total."
        </p>

        <h2>"Conjuração composta"</h2>
        <p>
            "Você pode trançar " <strong>"várias Artes numa única conjuração"</strong>
            ", o mesmo idioma dos combos de Técnica. Junte K Artes: some os pontos de todas, subtraia a sua Centelha "
            <strong>"uma vez só"</strong> ", do total, e some ao que sobrar a " <strong>"sobretaxa progressiva"</strong>
            " (a 2ª Arte +1, a 3ª +2, a 4ª +3: no total, 1 + 2 + … + (K − 1)). A sobretaxa não leva o desconto da Centelha e se paga mesmo quando os pontos cabem dentro dela: trançar Artes nunca sai de graça."
        </p>
        <p>
            "É uma " <strong>"só rolagem"</strong>
            " (você escolhe a Arte condutora); o nível que vale para os tetos é o do maior componente, e os Ticks seguem o maior grau investido em qualquer das Artes trançadas."
        </p>
        <p class="formula">
            "Fogo (Labareda: Volume 3 + Dano 2 = 5 pontos) + Forças (Alcance 1 = 1 ponto) = uma chama que persegue o alvo. Com Centelha 2: (5 + 1) − 2 = 4, mais 1 de sobretaxa = "
            <strong>"5"</strong> " de Mana, numa conjuração só."
        </p>

        <h2 id="rituais">"Rituais"</h2>
        <p>
            <strong>"Ritual é a conjuração estendida"</strong>
            ", não uma categoria de magia à parte. Um efeito que dura a cena não é Ritual: é um feitiço com o parâmetro de duração alto. O que faz um Ritual é "
            <em>"o tempo que você gasta conjurando"</em> ", subindo esta escada:"
        </p>
        <p class="formula">{ritual_scale}</p>
        <p>"Subir a escada compra uma de duas coisas, e você escolhe qual em cada conjuração."</p>
        <h3>"Mais barato"</h3>
        <p>
            "Cada grau de tempo acima do normal " <strong>"soma outra vez o desconto de Centelha"</strong>
            ": um grau dá Centelha × 2, dois graus dão Centelha × 3."
        </p>
        <p class="formula">{ritual_discount}</p>
        <h3>"Mais alto"</h3>
        <p>
            "Cada grau de tempo acima permite usar parâmetros " <strong>"um nível acima do seu nível de Arte"</strong>
            ", custando " <strong>"1 de Força de Vontade por grau"</strong>
            ". Aqui o Mana é o normal: você não paga o multiplicador do esticar, como se tivesse mesmo aquele nível de Arte."
        </p>
        <p class="formula">{ritual_above}</p>
        <div class="callout regra">
            <span class="lbl">"Ritual não é discreto"</span>
            " Nada disso é silencioso. Ritual tem luz, som e às vezes cheiro, e chama atenção de longe. Quem conjura por uma hora num lugar habitado está anunciando o que faz, e isso é parte do preço."
        </div>
        <p>
            "Vários feiticeiros que conheçam a Arte " <strong>"cooperam"</strong>
            ", somando Mana e Força de Vontade. Não há recuo mágico (" <em>"backlash"</em>
            ") e não há jogada de conjurar o Ritual: ele não falha por dado. Falha só se for interrompido, como no esticar (quem interromper você no meio leva a conjuração junto), e aí se perdem o tempo e o Mana investidos. Se o efeito for mirado, a jogada de ataque dele rola normalmente, quando ele sai."
        </p>

        <h2 id="mana-volta">"Quando o Mana volta"</h2>
        <p>
            "Pelo relógio, a reserva " <strong>"não"</strong>
            " volta por cena: quem se esvazia numa luta precisa de horas, não de um respiro. A única porta dentro da cena é a Firula (1 de Mana na de nível 2, 3 na de nível 3). Com Centelha 1 ou mais, você recupera "
            <strong>{recovery_normal}</strong> ", e " <strong>{recovery_rest}</strong>
            ". O mortal (Centelha 0) tem a Mana como uma " <strong>"reserva própria"</strong>
            ", com o valor da Força de Vontade, e ela volta " <strong>"1 por dia"</strong> ", ou "
            <strong>"2 por dia descansando"</strong>
            " (descanso completo ou sono profundo). Mana e Força de Vontade são reservas separadas: gastar ou recuperar uma não mexe na outra."
        </p>
        <p>
            <strong>"Meditação."</strong> " Meditar devolve " <strong>"1 de Mana por hora meditando"</strong>
            ", até o seu nível de Meditação por dia (ver " <a href=u("regras/habilidades-secundarias")>"Habilidades Secundárias"</a>
            "). Vale para todos, mortais ou não, e soma-se ao que a Mana já recupera."
        </p>
        <p>
            <strong>"Lugares de fluxo."</strong>
            " Em certos lugares a energia corre mais forte, e quem recupera Mana ali ganha " <strong>"+1 por hora"</strong>
            " (fluxo mediano), " <strong>"+2"</strong> " (fluxo muito alto) ou " <strong>"+3"</strong>
            " (fluxo enorme, e preocupante). Escolas de magia, templos e círculos de druidas costumam ficar nesses lugares."
        </p>
        <p class="muted">
            "É o que separa a economia do Arcano da das Proezas: a Energia é o combustível da cena, o Mana é o combustível do dia. Quem investe na Arte "
            <a href=u("artes/catalogo#manipulacao-mana")>"Mana"</a>
            " aumenta as duas coisas, a reserva e a velocidade com que ela volta, com os números fixos do catálogo: a Arte Mana não usa a Centelha em conta nenhuma."
        </p>
        <div class="callout regra">
            <span class="lbl">"O lugar do feiticeiro"</span>
            " Feitiçaria não é build barata nem para personagem iniciante: no começo as conjurações são escassas, como as do mago nos primeiros níveis de outros sistemas. O feiticeiro do Centelha precisa de um "
            <strong>"papel secundário"</strong> ", e não necessariamente de combate: quando o Mana acaba, é dele que você vive."
        </div>

        <h2>"Guardar um feitiço"</h2>
        <p>
            "Dá para conjurar agora e soltar depois. O Mana é pago na hora de guardar, e o feitiço fica "
            <strong>"comitado"</strong>
            ": parte da cabeça do feiticeiro continua segurando aquilo o tempo todo. Enquanto estiver guardado, o "
            <strong>"Raciocínio"</strong> " paga a conta."
        </p>
        <div class="callout regra">
            <span class="lbl">"O preço da espera"</span> " " <strong>"Passivo:"</strong> " o Raciocínio conta como "
            <strong>"um nível mais baixo"</strong>
            " em tudo que é calculado com ele (Iniciativa, Defesa Mental e afins). \u{a0} " <strong>"Ativo:"</strong> " "
            <strong>"−1d6"</strong>
            " em qualquer rolagem que use Raciocínio. Soltar o feitiço, ou desfazê-lo, devolve o Raciocínio na hora."
        </div>

        <h2>"Fraqueza e resistência"</h2>
        <p>
            "Quase nenhuma criatura tem uma. Elas aparecem no sobrenatural (a múmia com o fogo, a luz e o sagrado; o vrock, um demônio, com a luz e o sagrado; o golem de gelo com o fogo) e no material de que a criatura é feita: o que é de folha, madeira, palha ou gelo teme o fogo; o que é de fogo ou de terra teme a água; o que é de metal ou pedra resiste ao fogo."
        </p>
        <p>"Acertar a fraqueza de alguém não é um bônus qualquer, é outra categoria de golpe:"</p>
        <ul class="fraq">
            <li>"O dano " <strong>"ignora toda a Absorção"</strong> ", natural e de armadura."</li>
            <li>
                "O dano é " <strong>"agravado"</strong>
                ": não fecha com descanso nem com a Habilidade Cura, só com tratamento específico ou com a Arte de Cura."
            </li>
            <li><strong>"Aparar de mãos nuas fere"</strong> ": quem tenta bloquear o golpe com o corpo sofre o dano do mesmo jeito."</li>
        </ul>
        <p>
            "Quando o mesmo tipo é também a fraqueza da Regeneração da criatura (o fogo e o ácido do trol), vale a regeneração para esse tipo, na forma que a ficha der: o dano não ignora a Absorção nem é agravado (ver a "
            <a href=u("regras/poderes-das-criaturas#regeneração")>"Regeneração"</a>
            " em Os Poderes das Criaturas). O profano é o mesmo tipo que o maligno da regeneração; o sagrado não é a arma abençoada da regeneração, que é propriedade da arma, como a prata e o ferro frio."
        </p>
        <p class="formula">
            "Um elemental de madeira que tenta aparar uma lâmina feita inteira de fogo leva o dano cheio, sem absorver nada, e a queimadura não fecha sozinha."
        </p>
        <p>"A " <strong>"resistência"</strong> " é o avesso disso, e é bem mais comum:"</p>
        <div class="callout regra">
            <span class="lbl">"O que a resistência faz"</span> " " {resistance}
        </div>
        <ul class="fraq">{bullets(resistance_examples)}</ul>
        <div class="callout revisao">
            <span class="lbl">"Em revisão"</span> " Esta parte do capítulo está sendo reescrita. Ainda falta definir:"
            <ul>{review}</ul>
        </div>
    }
}
