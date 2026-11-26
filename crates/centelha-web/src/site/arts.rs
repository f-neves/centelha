//! As páginas das Artes: o catálogo (`artes/catalogo`) e os Efeitos Especiais
//! (`artes/efeitos`).
//!
//! Port de `src/pages/artes/catalogo.astro`, `src/pages/artes/efeitos.astro` e
//! da formatação de `src/lib/artes-fmt.ts` do site atual. Os dados vêm dos
//! leitores do `centelha-data` (`load_arts`, `load_effects`) e o custo de cada
//! Efeito em XP vem do motor (`centelha_engine::character::effect_cost`). A
//! ordem dos parâmetros e o texto curto de cada um são formatação, como lá.

use std::path::Path;

use centelha_data::arts::ArtCategory;
use centelha_data::effects::{EffectParameter, ParameterKind};
use centelha_data::{Art, Effect, Rules};

use super::SiteError;

/// Um nível de uma Arte no catálogo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtLevelRow {
    pub level: String,
    pub name: String,
    pub effect: String,
    pub examples: Vec<String>,
}

/// O cartão de uma Arte no catálogo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtCard {
    pub id: String,
    pub name: String,
    pub category: &'static str,
    /// O texto da busca (nome, nomes e efeitos dos níveis, em minúsculas).
    pub search: String,
    pub levels: Vec<ArtLevelRow>,
}

/// Uma linha de parâmetro no cartão de um Efeito.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamRow {
    pub name: String,
    /// `area` ou `volume`: o rótulo abre o cartão das formas.
    pub shape: Option<&'static str>,
    pub value: String,
    /// A escala ou a nota, em letra menor, debaixo do valor.
    pub detail: Option<String>,
}

/// O cartão de um Efeito sob uma Arte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectCard {
    /// `ef-<arte>-<efeito>`.
    pub anchor: String,
    pub name: String,
    pub level: String,
    /// "exige <Arte> <nível> · custa <XP> de XP".
    pub title: String,
    pub flavor: String,
    pub text: String,
    pub params: Vec<ParamRow>,
    pub notes: Option<String>,
    /// "Também em ...", quando o Efeito serve a outras Artes.
    pub also: Option<String>,
}

/// Os Efeitos de uma Arte (um `<details>` na página).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectBlock {
    pub art_id: String,
    pub art_name: String,
    pub category: &'static str,
    pub cards: Vec<EffectCard>,
}

/// O que as duas páginas mostram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arts {
    pub elemental: Vec<ArtCard>,
    pub universal: Vec<ArtCard>,
    pub effect_blocks_elemental: Vec<EffectBlock>,
    pub effect_blocks_universal: Vec<EffectBlock>,
    /// As Artes sem Efeito nenhum, pelo nome.
    pub without_effects: Vec<String>,
    pub art_count: usize,
    pub effect_count: usize,
}

/// Um número do JSON como o JavaScript o escreve (`3`, e não `3.0`).
pub fn number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// `formaDe`: os rótulos que abrem o cartão das formas.
pub fn shape_of(name: &str) -> Option<&'static str> {
    let n = name.to_lowercase();
    if n.starts_with("volume") {
        Some("volume")
    } else if n.starts_with("área") || n.starts_with("area") {
        Some("area")
    } else {
        None
    }
}

/// `RANK` do site atual: Alcance, Alvos, Área (e as medidas dela), Dano (e as
/// outras magnitudes), Duração, a jogada, a Dificuldade e a Penalidade.
fn rank_of_word(word: &str) -> Option<u8> {
    Some(match word {
        "alcance" => 0,
        "alvos" => 1,
        "área" | "area" | "volume" | "comprimento" | "altura" | "raio" | "profundidade" => 2,
        "dano" | "cura" | "bônus" | "absorção" | "absorve" | "resistência" | "quantidade"
        | "ganho" | "voo" | "visão" | "força" | "efeito" | "fah" | "faa" => 3,
        "duração" | "duracao" => 4,
        "ataque" | "conjurar" | "jogada" => 5,
        "dificuldade" => 6,
        "penalidade" => 7,
        _ => return None,
    })
}

fn rank(p: &EffectParameter) -> u8 {
    let name = p.name.to_lowercase();
    let first = name.split(' ').next().unwrap_or("");
    rank_of_word(first)
        .or_else(|| rank_of_word(&p.replaces.clone().unwrap_or_default().to_lowercase()))
        .unwrap_or(9)
}

/// `ordemPar`: os parâmetros pela ordem fixa, e empate pela ordem do arquivo.
pub fn ordered(params: &[EffectParameter]) -> Vec<&EffectParameter> {
    let mut list: Vec<&EffectParameter> = params.iter().collect();
    // `sort_by_key` é estável: o empate fica na ordem do arquivo.
    list.sort_by_key(|p| rank(p));
    list
}

fn scale_text(p: &EffectParameter) -> String {
    let scale = p.scale.clone().unwrap_or_default().join(" \u{b7} ");
    match &p.unit {
        Some(u) if !u.is_empty() => format!("{scale} ({u})"),
        _ => scale,
    }
}

/// `valorPar`: o valor curto de cada parâmetro. `arts` são as Artes em que o
/// Efeito aparece ali (sob a Terra, o dado do Dano dobra).
pub fn value_of(p: &EffectParameter, arts: &[&str]) -> String {
    match p.kind {
        ParameterKind::Fixed => return p.value.clone().unwrap_or_default(),
        ParameterKind::Replaces => {
            return match &p.note {
                Some(n) if !n.is_empty() => n.clone(),
                _ => scale_text(p),
            };
        }
        ParameterKind::Standard => {}
    }
    if let Some(r) = p.ruler.as_deref().filter(|r| !r.is_empty()) {
        return if r == "longa" { "Longa" } else { "Breve" }.to_string();
    }
    let name = p.name.to_lowercase();
    if name.starts_with("cura") {
        return "2 \u{b7} 1d6 \u{b7} 1d6+2 \u{b7} 2d6 \u{b7} 2d6+2 \u{b7} 3d6 (2 de Mana por nível)"
            .to_string();
    }
    if name.starts_with("dano") {
        return if arts.contains(&"terra") {
            "1d6 por nível (Terra: 2d6 por nível)"
        } else {
            "1d6 por nível"
        }
        .to_string();
    }
    "normal".to_string()
}

fn param_row(p: &EffectParameter, art_id: &str) -> ParamRow {
    let note = p.note.clone().filter(|n| !n.is_empty());
    let detail = match (p.kind, note) {
        (ParameterKind::Replaces, Some(_)) => Some(scale_text(p)),
        (ParameterKind::Replaces, None) => None,
        (_, note) => note,
    };
    ParamRow {
        name: p.name.clone(),
        shape: shape_of(&p.name),
        value: value_of(p, &[art_id]),
        detail,
    }
}

fn category_id(c: ArtCategory) -> &'static str {
    c.id()
}

fn art_card(a: &Art) -> ArtCard {
    let mut search = a.name.clone();
    for l in &a.levels {
        search.push(' ');
        search.push_str(&l.name);
        search.push(' ');
        search.push_str(&l.effect);
    }
    ArtCard {
        id: a.id.clone(),
        name: a.name.clone(),
        category: category_id(a.category),
        search: search.to_lowercase(),
        levels: a
            .levels
            .iter()
            .map(|l| ArtLevelRow {
                level: number(l.level),
                name: l.name.clone(),
                effect: l.effect.clone(),
                examples: l.examples.clone(),
            })
            .collect(),
    }
}

/// Os Efeitos de uma Arte, pelo nível e depois pelo nome (colação `pt`).
fn effects_of<'a>(effects: &'a [Effect], art_id: &str) -> Vec<(&'a Effect, String)> {
    let mut list: Vec<(&Effect, String)> = effects
        .iter()
        .filter_map(|e| {
            e.arts
                .iter()
                .find(|x| x.id == art_id)
                .map(|x| (e, x.flavor.clone()))
        })
        .collect();
    list.sort_by(|(a, _), (b, _)| {
        a.level
            .partial_cmp(&b.level)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| centelha_engine::collate::compare(&a.name, &b.name))
    });
    list
}

fn effect_block(art: &Art, arts: &[Art], effects: &[Effect], rules: &Rules) -> EffectBlock {
    let cards = effects_of(effects, &art.id)
        .into_iter()
        .map(|(e, flavor)| {
            let cost = centelha_engine::character::effect_cost(rules, e.level);
            let also = (e.arts.len() > 1).then(|| {
                let names: Vec<String> = e
                    .arts
                    .iter()
                    .filter(|x| x.id != art.id)
                    .map(|x| {
                        arts.iter()
                            .find(|a| a.id == x.id)
                            .map(|a| a.name.clone())
                            .unwrap_or_else(|| "undefined".to_string())
                    })
                    .collect();
                format!("Também em {}", names.join(", "))
            });
            EffectCard {
                anchor: format!("ef-{}-{}", art.id, e.id),
                name: e.name.clone(),
                level: number(e.level),
                title: format!(
                    "exige {} {} \u{b7} custa {} de XP",
                    art.name,
                    number(e.level),
                    number(cost)
                ),
                flavor,
                text: e.text.clone(),
                params: ordered(&e.parameters)
                    .into_iter()
                    .map(|p| param_row(p, &art.id))
                    .collect(),
                notes: e.notes.clone().filter(|n| !n.is_empty()),
                also,
            }
        })
        .collect();
    EffectBlock {
        art_id: art.id.clone(),
        art_name: art.name.clone(),
        category: category_id(art.category),
        cards,
    }
}

/// Lê as Artes e os Efeitos e monta as duas páginas, na ordem do site atual.
pub fn load(content_dir: &Path, rules: &Rules) -> Result<Arts, SiteError> {
    let arts = centelha_data::load_arts(content_dir)?;
    let effects = centelha_data::load_effects(content_dir)?;
    let of = |c: ArtCategory| arts.iter().filter(move |a| a.category == c);
    let has_effects = |a: &Art| effects.iter().any(|e| e.arts.iter().any(|x| x.id == a.id));
    let blocks = |c: ArtCategory| -> Vec<EffectBlock> {
        of(c)
            .filter(|a| has_effects(a))
            .map(|a| effect_block(a, &arts, &effects, rules))
            .collect()
    };
    Ok(Arts {
        elemental: of(ArtCategory::Elemental).map(art_card).collect(),
        universal: of(ArtCategory::Universal).map(art_card).collect(),
        effect_blocks_elemental: blocks(ArtCategory::Elemental),
        effect_blocks_universal: blocks(ArtCategory::Universal),
        without_effects: arts
            .iter()
            .filter(|a| !has_effects(a))
            .map(|a| a.name.clone())
            .collect(),
        art_count: arts.len(),
        effect_count: effects.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_written_like_javascript() {
        assert_eq!(number(3.0), "3");
        assert_eq!(number(0.5), "0.5");
    }

    #[test]
    fn shapes_open_on_area_and_volume() {
        assert_eq!(shape_of("Área (diâmetro)"), Some("area"));
        assert_eq!(shape_of("Volume"), Some("volume"));
        assert_eq!(shape_of("Alcance"), None);
    }

    /// O total de cartões e as âncoras, contra o build da bancada.
    #[test]
    fn effect_cards_match_the_current_site() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let rules = centelha_data::load_rules(&dir).expect("regras");
        let arts = load(&dir, &rules).expect("Artes");
        let cards: usize = arts
            .effect_blocks_elemental
            .iter()
            .chain(&arts.effect_blocks_universal)
            .map(|b| b.cards.len())
            .sum();
        assert_eq!(cards, 189);
        assert_eq!((arts.elemental.len(), arts.universal.len()), (8, 16));
    }
}
