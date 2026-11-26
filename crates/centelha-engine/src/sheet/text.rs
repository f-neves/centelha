//! Os textos curtos do equipamento que a ficha e o bestiário mostram (port de
//! `sinalNum`, `danoStr`, `statsArma`, `statsEscudo` e `statsArmadura`, de
//! `equip.ts`). Texto igual ao do TypeScript, byte a byte, com o sinal de
//! menos tipográfico (U+2212).
//!
//! Os textos recebem o item PLANO (o do catálogo achatado, ou o que a ficha
//! guarda já achatado), como `Value`: um campo ausente sai `undefined`, como
//! no TypeScript.

use serde_json::Value;

use super::value::{get, or, to_js_string, to_number, truthy};
use crate::js;

const MINUS: char = '\u{2212}';

/// `sinalNum`: o número com o sinal (`+2`, `−1`; o zero sai `+0`).
pub fn signed_number(n: f64) -> String {
    let sign = if n >= 0.0 { '+' } else { MINUS };
    format!("{sign}{}", js::number_to_string(n.abs()))
}

/// O `sinalNum` de um campo do item, com `|| 0`.
fn signed_field(item: &Value, k: &str) -> String {
    let v = get(Some(item), k);
    signed_number(if truthy(v) { to_number(v) } else { 0.0 })
}

/// `${v}` de um campo.
fn field(v: Option<&Value>, k: &str) -> String {
    to_js_string(get(v, k))
}

/// `−${v}` quando o campo é verdadeiro, senão `0` (a Penalidade).
fn penalty(item: &Value) -> String {
    let p = get(Some(item), "penalidade");
    if truthy(p) {
        format!("{MINUS}{}", to_js_string(p))
    } else {
        "0".to_owned()
    }
}

/// `danoStr`: o dano da arma (`2d6+1`, `1d6`).
pub fn damage_text(weapon: &Value) -> String {
    let bonus = get(Some(weapon), "danoBonus");
    let bonus = if truthy(bonus) {
        signed_number(to_number(bonus))
    } else {
        String::new()
    };
    format!("{}d6{bonus}", field(Some(weapon), "dado"))
}

/// `statsArma`: Velocidade/Acerto/dano/Defesa da arma (`4/+1/2d6+1/+0`).
pub fn weapon_stats(weapon: &Value) -> String {
    format!(
        "{}/{}/{}/{}",
        field(Some(weapon), "ticks"),
        signed_field(weapon, "acerto"),
        damage_text(weapon),
        signed_field(weapon, "defesaArma"),
    )
}

/// `statsEscudo`: `+1 Def · −1 Pen`.
pub fn shield_stats(shield: &Value) -> String {
    format!(
        "{} Def \u{b7} {} Pen",
        signed_field(shield, "bloqCaC"),
        penalty(shield)
    )
}

/// `statsArmadura`: `Imp 3 · Cor 4 · Perf 1 (N0) · Pen −1`, as colunas da
/// tabela de Armaduras.
pub fn armor_stats(armor: &Value) -> String {
    let soak = get(Some(armor), "soak");
    let level = or(get(Some(armor), "resistPerf"), None);
    let level = if truthy(level) {
        to_js_string(level)
    } else {
        "0".to_owned()
    };
    format!(
        "Imp {} \u{b7} Cor {} \u{b7} Perf {} (N{level}) \u{b7} Pen {}",
        field(soak, "impacto"),
        field(soak, "corte"),
        field(soak, "perfuracao"),
        penalty(armor),
    )
}
