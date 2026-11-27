//! O custo de uma conjuração: os pontos investidos nos parâmetros, o esticar,
//! o desconto da Centelha e da fonte do elemento, e a Mana que sobra.
//!
//! Port de `custoDe` (`src/lib/artes-grid.ts`), sem a Velocidade (os Ticks da
//! conjuração são do tabuleiro, `ticksDe`). O desconto da fonte do elemento
//! não tem conta no TypeScript, só o texto do livro; ver [`mana_paid`].
//!
//! Os números saem de `regras.json → arcano`:
//! - `improviso.custoPorNivel.cura`: a Cura custa 2 pontos por grau, todo
//!   outro parâmetro 1;
//! - `fonteElemental.desconto`: o +1 de quem conjura com o elemento à mão.
//!
//! Quem confere é `tests/parity/arts.json`.

use centelha_data::Rules;
use centelha_data::effects::{Effect, ParameterKind};

/// O custo da Cura por grau, se o livro não disser.
const DEFAULT_HEALING_COST: f64 = 2.0;

/// Os parâmetros do improviso (conjurar sem um Efeito), na ordem em que a
/// fonte os lista (`parametrosDoImproviso`).
pub const IMPROVISATION_PARAMETERS: [&str; 5] = ["Alcance", "Volume", "Alvos", "Dano", "Duração"];

/// Um parâmetro esticado: passou do nível da Arte (`esticados`).
#[derive(Debug, Clone, PartialEq)]
pub struct Stretched {
    pub name: String,
    /// Quantos níveis acima do nível da Arte.
    pub above: f64,
    /// O que ele custou, já com o esticar.
    pub cost: f64,
}

/// O que uma conjuração custa (`Custo`, sem a Velocidade).
#[derive(Debug, Clone, PartialEq)]
pub struct CastingCost {
    /// O nível do Efeito, ou 0 no improviso.
    pub base: f64,
    /// A soma dos níveis investidos, já com o esticar e os fixos que cobram.
    pub parameters: f64,
    pub stretched: Vec<Stretched>,
    /// `base + parameters`.
    pub total: f64,
    /// A Centelha de quem conjura.
    pub spark: f64,
    /// O desconto da fonte do elemento que valeu (0 sem fonte).
    pub source_discount: f64,
    /// A Mana paga: o que sobra depois da Centelha e da fonte. 0 é grátis e
    /// ilimitado.
    pub mana: f64,
}

/// O desconto de quem conjura com o elemento ao alcance
/// (`arcano.fonteElemental.desconto`).
pub fn elemental_source_discount(rules: &Rules) -> f64 {
    rules
        .arcane
        .as_ref()
        .and_then(|a| a.elemental_source_discount)
        .unwrap_or(0.0)
}

/// A Mana paga por uma conjuração de `total` pontos.
///
/// A regra do livro (`arcano.improviso`): "total de pontos − Centelha
/// (mínimo 0; zero = grátis e ilimitado)". Com a fonte do elemento ao alcance,
/// o desconto dela entra somado ao da Centelha (`arcano.fonteElemental`: "+1 de
/// desconto, somado ao da Centelha"), de modo que a Centelha 3 com a fonte
/// paga o mesmo que a Centelha 4 sem fonte (`zeraCusto`). Só a regra escrita
/// tem a fonte: no TypeScript o `custoDe` recebe a Centelha pronta.
pub fn mana_paid(rules: &Rules, total: f64, spark: f64, elemental_source: bool) -> f64 {
    let source = if elemental_source {
        elemental_source_discount(rules)
    } else {
        0.0
    };
    (total - (spark + source)).max(0.0)
}

/// `custoDe`: o custo de conjurar `effect` (ou o improviso, sem Efeito) numa
/// Arte de nível `art_level`, com os níveis escolhidos em cada parâmetro
/// (`choices`, o nome do parâmetro e o nível) e a Centelha dada.
///
/// - Cada parâmetro que o jogador ajusta custa nível × preço por grau (1, ou
///   o da Cura); passar do nível da Arte é esticar e custa nível × (níveis
///   acima + 1).
/// - O parâmetro fixo que cobra Mana (`custaMana`) cobra por nível da Arte.
/// - O Efeito de ação livre custa Mana como qualquer outro.
pub fn casting_cost(
    rules: &Rules,
    effect: Option<&Effect>,
    art_level: f64,
    choices: &[(&str, f64)],
    spark: f64,
    elemental_source: bool,
) -> CastingCost {
    let base = effect.map_or(0.0, |e| e.level);
    let healing = rules
        .arcane
        .as_ref()
        .and_then(|a| a.improvisation_cost)
        .map_or(DEFAULT_HEALING_COST, |c| c.healing);
    let names: Vec<&str> = match effect {
        // os que o jogador escolhe: os fixos não entram na conta
        Some(e) => e
            .parameters
            .iter()
            .filter(|p| p.kind != ParameterKind::Fixed)
            .map(|p| p.name.as_str())
            .collect(),
        None => IMPROVISATION_PARAMETERS.to_vec(),
    };
    let mut stretched = Vec::new();
    let mut parameters = 0.0;
    for name in names {
        let chosen = choices
            .iter()
            .find(|(n, _)| *n == name)
            .map_or(0.0, |(_, level)| *level);
        let n = chosen.max(0.0);
        if n == 0.0 || n.is_nan() {
            continue;
        }
        let per_level = if name == "Cura" { healing } else { 1.0 };
        let above = (n - art_level).max(0.0);
        let cost = n * per_level * (above + 1.0);
        if above > 0.0 {
            stretched.push(Stretched {
                name: name.to_owned(),
                above,
                cost,
            });
        }
        parameters += cost;
    }
    // o preço dos parâmetros fixos que cobram: o nível da Arte, não uma escolha
    for p in effect.map_or(&[][..], |e| &e.parameters[..]) {
        let Some(mana_cost) = p.mana_cost.filter(|c| *c != 0.0) else {
            continue;
        };
        if p.kind != ParameterKind::Fixed {
            continue;
        }
        parameters += mana_cost * art_level.max(0.0);
    }
    let total = base + parameters;
    let source_discount = if elemental_source {
        elemental_source_discount(rules)
    } else {
        0.0
    };
    CastingCost {
        base,
        parameters,
        stretched,
        total,
        spark,
        source_discount,
        mana: mana_paid(rules, total, spark, elemental_source),
    }
}
