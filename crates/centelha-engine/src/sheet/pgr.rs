//! A linha Preparo, Golpe e Recuperação que a ficha mostra para uma arma
//! (port de `src/lib/ficha-pgr.ts`): a da tabela do livro
//! (`combate.pgr.reforma`) quando a arma tem linha nela, a da régua do motor
//! quando não tem; e as linhas a mais do item extra (o atlatl da azagaia).

use centelha_data::rules::PgrRow;
use centelha_data::{Rules, Weapon};
use serde_json::Value;

use super::value as jv;
use crate::timeline::{self, Anatomy, AnatomyInput, TimeClass, TimeSystem};

/// O tiro que é arremesso na tabela do livro (`^(arremesso|funda)`): a Funda
/// é `arremesso` no catálogo e tem linha própria.
fn is_throw_row(id: &str) -> bool {
    id.starts_with("arremesso") || id.starts_with("funda")
}

fn text<'a>(w: &'a Value, k: &str) -> Option<&'a str> {
    w.get(k).and_then(Value::as_str)
}

/// `anatomiaDaFicha`: a anatomia de uma arma da ficha (o item, com `id` ou
/// `nome`, `ticks` e `classe`), pela régua do P/G/R, com a linha do livro por
/// cima quando houver. A linha do livro vale pela arma citada com a mesma
/// Velocidade; senão, pela única Velocidade do grupo da classe.
pub fn sheet_anatomy(rules: &Rules, weapons: &[Weapon], w: &Value) -> Anatomy {
    let ticks = w
        .get("ticks")
        .filter(|x| !x.is_null())
        .map(|x| jv::to_number(Some(x)));
    let speed = ticks.unwrap_or(5.0);
    let id = text(w, "id").filter(|s| !s.is_empty());
    let name_or_id = id.or_else(|| text(w, "nome"));
    let class = timeline::time_class(weapons, name_or_id, ticks, text(w, "classe"));
    let mut a = timeline::anatomy(
        rules,
        &AnatomyInput {
            class,
            speed: Some(speed),
            system: TimeSystem::Pgr,
            maneuver: None,
            blows: None,
        },
    );
    let reform = rules.combat.pgr.as_ref().and_then(|p| p.reform.as_ref());
    let empty: Vec<PgrRow> = Vec::new();
    let group: Vec<&PgrRow> = match (class, reform) {
        (TimeClass::Light | TimeClass::Medium | TimeClass::Polearm | TimeClass::Heavy, Some(r)) => {
            r.melee.iter().collect()
        }
        (TimeClass::Thrown, Some(r)) => r.shot.iter().filter(|c| is_throw_row(&c.id)).collect(),
        (TimeClass::Ranged, Some(r)) => r.shot.iter().filter(|c| !is_throw_row(&c.id)).collect(),
        _ => empty.iter().collect(),
    };
    let row = group
        .iter()
        .find(|c| c.speed == speed && id.is_some_and(|i| c.weapons.iter().any(|x| x == i)))
        .or_else(|| {
            group
                .iter()
                .find(|c| c.id != "punhos" && !c.weapons.is_empty() && c.speed == speed)
        });
    if let Some(c) = row {
        a.windup = c.windup;
        a.blows = c.blow;
        a.recovery = c.recovery;
        a.cycle = c.speed;
    }
    a
}

/// Uma linha a mais de uma arma: a do item extra que a muda.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtraLine {
    pub id: String,
    pub name: String,
    pub windup: f64,
    pub blows: f64,
    pub recovery: f64,
    pub cycle: f64,
}

/// `linhasComExtra`: as linhas dos itens extras (`armas-extras.json`, cru)
/// que aceitam esta arma (`so`) e têm linha em `combate.pgr.reforma.tiro`.
pub fn extra_lines(rules: &Rules, extras: &Value, w: &Value) -> Vec<ExtraLine> {
    let shot: &[PgrRow] = rules
        .combat
        .pgr
        .as_ref()
        .and_then(|p| p.reform.as_ref())
        .map_or(&[], |r| r.shot.as_slice());
    let id = w.get("id");
    let mut out = Vec::new();
    for ex in extras.as_array().map_or(&[][..], Vec::as_slice) {
        let accepts = ex
            .get("so")
            .and_then(Value::as_array)
            .is_some_and(|so| so.iter().any(|x| jv::strict_eq(Some(x), id)));
        if !accepts {
            continue;
        }
        let ex_id = text(ex, "id");
        if let Some(c) = shot
            .iter()
            .find(|x| x.extra.as_deref() == ex_id && ex_id.is_some())
        {
            out.push(ExtraLine {
                id: ex_id.unwrap_or_default().to_owned(),
                name: jv::to_js_string(ex.get("nome")),
                windup: c.windup,
                blows: c.blow,
                recovery: c.recovery,
                cycle: c.speed,
            });
        }
    }
    out
}

/// `tetoDaRajada` pela classe de tempo da arma da ficha: quantos golpes a
/// rajada dá.
pub fn sheet_flurry_cap(rules: &Rules, weapons: &[Weapon], w: &Value) -> f64 {
    let ticks = w
        .get("ticks")
        .filter(|x| !x.is_null())
        .map(|x| jv::to_number(Some(x)));
    let id = text(w, "id").filter(|s| !s.is_empty());
    let class = timeline::time_class(
        weapons,
        id.or_else(|| text(w, "nome")),
        ticks,
        text(w, "classe"),
    );
    timeline::flurry_cap(rules, class)
}
