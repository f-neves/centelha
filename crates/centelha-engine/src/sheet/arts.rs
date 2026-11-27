//! As Artes e os Efeitos na ficha, em dados: os níveis que a Arte alcançou,
//! o cartão de cada Efeito, o que o cartão de hover diz, e os grupos de
//! Efeitos ao alcance. É o que as funções de `ficha-engine.ts` escrevem em
//! HTML (`arteFx`, `artePrint`, `conteudoArtePop`, `cardEfeito`,
//! `conteudoEfeitoPop`, `corpoEfeitos`), sem a marcação: quem desenha monta o
//! HTML (ou o componente) a partir daqui.
//!
//! Quem confere é o teste de paridade da ficha interativa, que lê o HTML que a
//! própria fonte escreve.

use centelha_data::Art;
use centelha_data::arts::ArtLevel as ArtLevelData;
use centelha_data::effects::{Effect, EffectArt, EffectParameter, ParameterKind};
use serde_json::Value;

use super::editor::{EditorTables, effects_in_reach};
use super::value::{self as jv, truthy};
use crate::arts_fmt::{
    Form, PARAMETER_FORM, form_of, order_parameters, parameter_value, rank, scale_text,
};
use crate::character::effect_cost;
use crate::js;

/// O nível da ficha numa Arte (`S.arte[id] || 0`).
pub fn art_level(sheet: &Value, art_id: &str) -> f64 {
    jv::num_or0(jv::get(sheet.get("arte"), art_id))
}

/// `nomeArte`: o nome da Arte, ou o próprio id se ela não existe.
pub fn art_name<'a>(t: &'a EditorTables, id: &'a str) -> &'a str {
    t.arts()
        .iter()
        .find(|a| a.id == id)
        .map_or(id, |a| a.name.as_str())
}

/// `artesDe`: as Artes que comportam o Efeito e em que a ficha já tem o nível
/// dele.
pub fn arts_reached<'a>(sheet: &Value, effect: &'a Effect) -> Vec<&'a EffectArt> {
    effect
        .arts
        .iter()
        .filter(|a| art_level(sheet, &a.id) >= effect.level)
        .collect()
}

/// Uma linha dos níveis de uma Arte.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtLine {
    pub level: f64,
    pub name: String,
    pub effect: String,
    /// O nível já foi alcançado (a classe `hi`).
    pub reached: bool,
}

fn line(l: &ArtLevelData, reached: bool) -> ArtLine {
    ArtLine {
        level: l.level,
        name: l.name.clone(),
        effect: l.effect.clone(),
        reached,
    }
}

/// `arteFx`: todos os níveis da Arte, os alcançados marcados.
pub fn art_lines(art: &Art, level: f64) -> Vec<ArtLine> {
    art.levels
        .iter()
        .map(|l| line(l, l.level <= level))
        .collect()
}

/// `artePrint`: os níveis que a ficha alcançou, para o papel; Arte em zero não
/// imprime nada.
pub fn art_print_lines(art: &Art, level: f64) -> Vec<ArtLine> {
    if level == 0.0 || level.is_nan() {
        return Vec::new();
    }
    art.levels
        .iter()
        .filter(|l| l.level <= level)
        .map(|l| line(l, true))
        .collect()
}

/// O cartão de hover de uma Arte (`conteudoArtePop`).
#[derive(Debug, Clone, PartialEq)]
pub struct ArtPopup {
    pub title: String,
    /// "nível N" ou "nenhum nível".
    pub level_label: String,
    pub lines: Vec<ArtLine>,
}

/// `conteudoArtePop`.
pub fn art_popup(sheet: &Value, art: &Art) -> ArtPopup {
    let level = art_level(sheet, &art.id);
    ArtPopup {
        title: art.name.clone(),
        level_label: if level != 0.0 && !level.is_nan() {
            format!("nível {}", js::number_to_string(level))
        } else {
            "nenhum nível".into()
        },
        lines: art_lines(art, level),
    }
}

/// Um parâmetro no cartão curto de um Efeito (`parCurto`).
#[derive(Debug, Clone, PartialEq)]
pub struct ShortParameter {
    pub name: String,
    /// O cartão que o rótulo abre (`rotPar`).
    pub form: Option<Form>,
    /// O valor curto, só para os parâmetros de forma que o têm: o fixo e a
    /// régua da Duração.
    pub value: Option<String>,
}

impl ShortParameter {
    /// O texto, sem marcação: "Nome" ou "Nome: valor".
    pub fn text(&self) -> String {
        match &self.value {
            Some(v) => format!("{}: {v}", self.name),
            None => self.name.clone(),
        }
    }
}

/// `parCurto`: os parâmetros de forma (alcance, área, dano, duração) com nome e
/// valor curto; a jogada e a Dificuldade entram só pelo nome.
pub fn short_parameters(effect: &Effect) -> Vec<ShortParameter> {
    order_parameters(&effect.parameters)
        .into_iter()
        .map(|p| {
            let value = if rank(p) > PARAMETER_FORM {
                None
            } else if p.kind == ParameterKind::Fixed {
                // o `${p.valor}` da fonte: sem valor escreve "undefined"
                Some(p.value.clone().unwrap_or_else(|| "undefined".into()))
            } else {
                p.ruler
                    .as_deref()
                    .filter(|r| !r.is_empty())
                    .map(|r| if r == "longa" { "Longa" } else { "Breve" }.to_owned())
            };
            ShortParameter {
                name: p.name.clone(),
                form: form_of(&p.name),
                value,
            }
        })
        .collect()
}

/// A linha de parâmetros do cartão: os itens separados por " · ".
pub fn short_parameters_text(effect: &Effect) -> String {
    short_parameters(effect)
        .iter()
        .map(ShortParameter::text)
        .collect::<Vec<_>>()
        .join(" · ")
}

/// O cartão de um Efeito numa lista de Efeitos (`cardEfeito`).
#[derive(Debug, Clone, PartialEq)]
pub struct EffectCard {
    pub id: String,
    pub name: String,
    pub level: f64,
    /// O Efeito já foi comprado.
    pub bought: bool,
    /// O preço em XP (`custoEfeito`).
    pub xp: f64,
    /// Os nomes das outras Artes em que o Efeito também cabe (as que a ficha
    /// alcança), sem a Arte do grupo.
    pub also_in: Vec<String>,
    pub text: String,
    pub parameters: Vec<ShortParameter>,
}

/// `cardEfeito`: o cartão do Efeito sob a Arte `art_id`.
pub fn effect_card(t: &EditorTables, sheet: &Value, effect: &Effect, art_id: &str) -> EffectCard {
    let rules = t.sheet_tables().rules();
    EffectCard {
        id: effect.id.clone(),
        name: effect.name.clone(),
        level: effect.level,
        bought: truthy(jv::get(sheet.get("efeito"), &effect.id)),
        xp: effect_cost(rules, effect.level),
        also_in: arts_reached(sheet, effect)
            .into_iter()
            .filter(|a| a.id != art_id)
            .map(|a| art_name(t, &a.id).to_owned())
            .collect(),
        text: effect.text.clone(),
        parameters: short_parameters(effect),
    }
}

/// Um parâmetro no cartão de hover de um Efeito.
#[derive(Debug, Clone, PartialEq)]
pub struct PopupParameter {
    pub name: String,
    pub form: Option<Form>,
    pub value: Option<String>,
    /// A nota pequena depois do valor.
    pub note: Option<String>,
}

/// Um sabor: o que uma Arte do Efeito acrescenta.
#[derive(Debug, Clone, PartialEq)]
pub struct Flavor {
    pub art: String,
    pub text: String,
}

/// O cartão de hover de um Efeito (`conteudoEfeitoPop`).
#[derive(Debug, Clone, PartialEq)]
pub struct EffectPopup {
    pub title: String,
    /// "nível N · X XP", mais " · comprado".
    pub header: String,
    pub text: String,
    pub flavors: Vec<Flavor>,
    pub parameters: Vec<PopupParameter>,
    pub notes: Option<String>,
}

/// A nota pequena de um parâmetro: quem substitui outro mostra a escala (se
/// tem nota); os outros mostram a nota.
fn popup_note(p: &EffectParameter) -> Option<String> {
    let has_note = p.note.as_deref().is_some_and(|n| !n.is_empty());
    if !has_note {
        return None;
    }
    Some(if p.kind == ParameterKind::Replaces {
        scale_text(p)
    } else {
        p.note.clone().unwrap_or_default()
    })
}

/// Os sabores das Artes alcançadas: só as que têm texto (`filter(x => x.sabor)`).
fn flavors(reached: &[&EffectArt], name: impl Fn(&str) -> String) -> Vec<Flavor> {
    reached
        .iter()
        .filter(|a| !a.flavor.is_empty())
        .map(|a| Flavor {
            art: name(&a.id),
            text: a.flavor.clone(),
        })
        .collect()
}

/// `conteudoEfeitoPop`.
pub fn effect_popup(t: &EditorTables, sheet: &Value, effect: &Effect) -> EffectPopup {
    let reached = arts_reached(sheet, effect);
    let reached_ids: Vec<&str> = reached.iter().map(|a| a.id.as_str()).collect();
    let rules = t.sheet_tables().rules();
    let bought = truthy(jv::get(sheet.get("efeito"), &effect.id));
    EffectPopup {
        title: effect.name.clone(),
        header: format!(
            "nível {} · {} XP{}",
            js::number_to_string(effect.level),
            js::number_to_string(effect_cost(rules, effect.level)),
            if bought { " · comprado" } else { "" }
        ),
        text: effect.text.clone(),
        flavors: flavors(&reached, |id| art_name(t, id).to_owned()),
        parameters: order_parameters(&effect.parameters)
            .into_iter()
            .map(|p| PopupParameter {
                name: p.name.clone(),
                form: form_of(&p.name),
                value: parameter_value(p, &reached_ids),
                note: popup_note(p),
            })
            .collect(),
        notes: effect.notes.clone().filter(|n| !n.is_empty()),
    }
}

/// Os Efeitos de uma Arte na lista (`gruposEfeito`).
#[derive(Debug, Clone, PartialEq)]
pub struct EffectGroup<'a> {
    pub art: &'a Art,
    pub effects: Vec<&'a Effect>,
}

/// `gruposEfeito`: um grupo por Arte, na ordem das Artes, com os Efeitos ao
/// alcance que cabem nela; com `only_bought`, só os comprados. Arte sem Efeito
/// não tem grupo.
pub fn effect_groups<'a>(
    t: &'a EditorTables,
    sheet: &Value,
    only_bought: bool,
) -> Vec<EffectGroup<'a>> {
    let in_reach = effects_in_reach(t, sheet);
    t.arts()
        .iter()
        .map(|art| EffectGroup {
            art,
            effects: in_reach
                .iter()
                .copied()
                .filter(|e| arts_reached(sheet, e).iter().any(|a| a.id == art.id))
                .filter(|e| !only_bought || truthy(jv::get(sheet.get("efeito"), &e.id)))
                .collect(),
        })
        .filter(|g| !g.effects.is_empty())
        .collect()
}

/// O cabeçalho de um grupo na lista (`corpoEfeitos`).
#[derive(Debug, Clone, PartialEq)]
pub struct GroupSummary {
    pub art_id: String,
    pub art_name: String,
    pub count: usize,
    pub bought: usize,
}

impl GroupSummary {
    /// O contador do cabeçalho: "N", ou "N · M comprado(s)".
    pub fn counter(&self) -> String {
        if self.bought == 0 {
            self.count.to_string()
        } else {
            format!(
                "{} · {} comprado{}",
                self.count,
                self.bought,
                if self.bought > 1 { "s" } else { "" }
            )
        }
    }
}

/// O corpo da lista de Efeitos (`corpoEfeitos`): a mensagem de lista vazia ou
/// os grupos.
#[derive(Debug, Clone, PartialEq)]
pub enum EffectsBody {
    Empty(&'static str),
    Groups(Vec<GroupSummary>),
}

/// `corpoEfeitos`, sem o estado de aberto e fechado de cada grupo (que é da
/// tela).
pub fn effects_body(t: &EditorTables, sheet: &Value, only_bought: bool) -> EffectsBody {
    let groups = effect_groups(t, sheet, only_bought);
    if groups.is_empty() {
        return EffectsBody::Empty(if only_bought {
            "Nenhum Efeito comprado ainda. Desmarque a caixa para ver os que estão ao alcance."
        } else {
            "Os Efeitos Especiais aparecem aqui quando você tiver uma Arte no nível que cada um exige."
        });
    }
    EffectsBody::Groups(
        groups
            .iter()
            .map(|g| GroupSummary {
                art_id: g.art.id.clone(),
                art_name: g.art.name.clone(),
                count: g.effects.len(),
                bought: g
                    .effects
                    .iter()
                    .filter(|e| truthy(jv::get(sheet.get("efeito"), &e.id)))
                    .count(),
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn art(id: &str, flavor: &str) -> EffectArt {
        serde_json::from_value(json!({"id": id, "sabor": flavor})).unwrap()
    }

    /// Nenhum Efeito do catálogo tem Arte sem sabor, então só aqui o filtro
    /// aparece: a Arte sem texto não entra no cartão.
    #[test]
    fn a_flavor_without_text_stays_out_of_the_popup() {
        let (fire, water, earth) = (
            art("fogo", "brasa"),
            art("agua", ""),
            art("terra", "poeira"),
        );
        let list = flavors(&[&fire, &water, &earth], |id| id.to_uppercase());
        assert_eq!(
            list,
            vec![
                Flavor {
                    art: "FOGO".into(),
                    text: "brasa".into()
                },
                Flavor {
                    art: "TERRA".into(),
                    text: "poeira".into()
                },
            ]
        );
    }
}
