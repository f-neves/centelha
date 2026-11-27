//! A Força e a carga na ficha: o alcance do arremesso (port de
//! `alcanceArremesso`, `src/lib/forca-empurrao.ts`) e as contas da seção de
//! força do `renderForca` (`src/lib/ficha-engine.ts`): a Força aplicada ao
//! arremesso (de 2 a 24), as alturas e o teto, a velocidade com carga em terra
//! e na água, as faixas de carga, os pesos da tabela de arremesso, e a carga
//! que a ficha leva (as bolsas, o arsenal e a armadura vestida).

use centelha_data::Rules;
use serde_json::Value;

use super::Catalog;
use super::value::{self as jv, is_js_space};
use crate::js;

/// `alcanceArremesso`: até onde vai um objeto de `weight_kg`, pela Força
/// aplicada (`reserve`, o FAA) e pelo peso máximo erguido (`max_kg`). O
/// objeto acima do teto do arremesso (uma fração do peso máximo) não voa;
/// abaixo do ápice a velocidade do braço satura; perto do teto o alcance cai
/// até zero.
pub fn throw_range(rules: &Rules, reserve: f64, weight_kg: f64, max_kg: f64) -> f64 {
    let f = &rules.strength.throw;
    let get = |x: Option<f64>| x.unwrap_or(f64::NAN);
    let cap_kg = max_kg * get(f.cap_fraction);
    if weight_kg <= 0.0 || weight_kg > cap_kg {
        return 0.0;
    }
    let apex = get(f.apex_kg);
    let head = get(f.constant) * js::pow(reserve, get(f.reserve_exponent));
    let heavy = |w: f64| head / js::pow(w, get(f.mass_exponent));
    let light = |w: f64| heavy(apex) * js::pow(w / apex, get(f.light_exponent));
    let drop_from = cap_kg * get(f.drop_fraction);
    let d = if weight_kg < apex {
        light(weight_kg)
    } else {
        heavy(weight_kg)
    };
    if weight_kg > drop_from {
        d * (cap_kg - weight_kg) / (cap_kg - drop_from)
    } else {
        d
    }
}

fn get(x: Option<f64>) -> f64 {
    x.unwrap_or(f64::NAN)
}

/// A Força aplicada ao arremesso (FAA): `2 × Força + Atletismo + Arremesso`,
/// entre 2 e 24.
pub fn throw_reserve(strength: f64, athletics: f64, throwing: f64) -> f64 {
    js::max(2.0, js::min(24.0, 2.0 * strength + athletics + throwing))
}

/// O que se ergue acima da cabeça: uma fração do peso máximo.
pub fn overhead_lift(rules: &Rules, max_kg: f64) -> f64 {
    max_kg * get(rules.strength.load.overhead_fraction)
}

/// O teto do arremesso: acima dele o objeto não sai da mão.
pub fn throw_cap(rules: &Rules, max_kg: f64) -> f64 {
    max_kg * get(rules.strength.throw.cap_fraction)
}

/// A velocidade com carga, em fração da sem carga (`velFrac` e
/// `velFracAgua`): `1 - (peso / máximo / corte)^expoente`, sem ficar abaixo de
/// zero. Na água o corte e o expoente são outros.
pub fn load_speed_fraction(rules: &Rules, weight_kg: f64, max_kg: f64, water: bool) -> f64 {
    let l = &rules.strength.load;
    let (cut, exp) = if water {
        (get(l.water_cut), get(l.water_exponent))
    } else {
        (get(l.cut), get(l.exponent))
    };
    js::max(0.0, 1.0 - js::pow(weight_kg / max_kg / cut, exp))
}

/// Uma faixa de carga: de onde a onde, e o nome.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadBand {
    pub from: f64,
    pub to: f64,
    pub name: &'static str,
}

/// As cinco faixas de carga: Mínima até um oitavo do máximo, Leve até um
/// quarto, Média até a metade, Pesada até o corte, Máxima até o máximo.
pub fn load_bands(rules: &Rules, max_kg: f64) -> Vec<LoadBand> {
    let min = max_kg / 8.0;
    let light = max_kg / 4.0;
    let medium = max_kg / 2.0;
    let heavy = max_kg * get(rules.strength.load.cut);
    [
        (0.0, min, "Mínima"),
        (min, light, "Leve"),
        (light, medium, "Média"),
        (medium, heavy, "Pesada"),
        (heavy, max_kg, "Máxima"),
    ]
    .into_iter()
    .map(|(from, to, name)| LoadBand { from, to, name })
    .collect()
}

/// `includes` do JavaScript entre números (o `NaN` acha o `NaN`).
fn has(list: &[f64], v: f64) -> bool {
    list.iter().any(|&x| x == v || (x.is_nan() && v.is_nan()))
}

/// Os pesos da tabela de arremesso da ficha: o ápice, meio quilo e um quilo;
/// oito pesos em progressão geométrica até o teto, arredondados para números
/// que se dizem (meio quilo abaixo de 10, cinco quilos acima); três degraus no
/// vão até o teto; e o teto.
pub fn throw_table_weights(rules: &Rules, max_kg: f64) -> Vec<f64> {
    let cap = throw_cap(rules, max_kg);
    let apex = get(rules.strength.throw.apex_kg);
    let clean = |v: f64| js::round(v * 1000.0) / 1000.0;
    let nice = |v: f64| {
        if v < 10.0 {
            js::round(v * 2.0) / 2.0
        } else {
            js::round(v / 5.0) * 5.0
        }
    };
    let mut out: Vec<f64> = Vec::new();
    for i in 1..=8 {
        let mut v = nice(js::pow(cap, f64::from(i) / 9.0));
        while has(&out, v) || v <= 1.0 {
            v = nice(v + if v < 10.0 { 0.5 } else { 5.0 });
        }
        if v < cap {
            out.push(v);
        }
    }
    out.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let last = out.last().copied().unwrap_or(1.0);
    let gap = cap - last;
    let step = if gap >= 20.0 {
        5.0
    } else if gap >= 4.0 {
        1.0
    } else if gap >= 2.0 {
        0.5
    } else {
        0.1
    };
    let mut wall: Vec<f64> = Vec::new();
    for k in 1..=3 {
        let mut v = clean(js::round((last + (gap * f64::from(k)) / 4.0) / step) * step);
        while (v <= last || has(&wall, v)) && v < cap {
            v = clean(v + step);
        }
        if v > last && v < cap && !has(&wall, v) {
            wall.push(v);
        }
    }
    let mut light: Vec<f64> = Vec::new();
    for v in [apex, 0.5, 1.0] {
        if !light.contains(&v) {
            light.push(v);
        }
    }
    light.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    light
        .into_iter()
        .chain(out)
        .chain(wall)
        .chain(std::iter::once(js::round(cap * 10.0) / 10.0))
        .collect()
}

/// A velocidade de base com a metade da penalidade (armadura e mãos), como a
/// Vel. de Corrida da seção de força: `max(0, corrida - metade)`.
pub fn penalized_speed(speed: f64, penalty: f64) -> f64 {
    js::max(0.0, speed - super::editor::half_movement_penalty(penalty))
}

// ------------------------------------------------------------ a carga

/// `numPeso`: o primeiro número de um campo escrito à mão (`"1,5"`,
/// `"2 kg"`, `"≈3"` e `"3.2"`); 0 sem número.
pub fn weight_from_text(v: Option<&Value>) -> f64 {
    let s = if jv::nullish(v) {
        String::new()
    } else {
        jv::to_js_string(v)
    };
    let s = s.replacen(',', ".", 1);
    let b = s.as_bytes();
    let Some(start) = b.iter().position(u8::is_ascii_digit) else {
        return 0.0;
    };
    let mut end = start;
    while end < b.len() && b[end].is_ascii_digit() {
        end += 1;
    }
    if end + 1 < b.len() && b[end] == b'.' && b[end + 1].is_ascii_digit() {
        end += 1;
        while end < b.len() && b[end].is_ascii_digit() {
            end += 1;
        }
    }
    s[start..end].parse().unwrap_or(0.0)
}

fn written(v: Option<&Value>) -> String {
    let s = if jv::nullish(v) {
        String::new()
    } else {
        jv::to_js_string(v)
    };
    s.trim_matches(is_js_space).to_owned()
}

/// `pesoPeca`: o peso de uma peça (arma, escudo ou armadura): o do item que a
/// ficha guarda.
pub fn piece_weight(catalog: &Catalog, p: &Value) -> f64 {
    let item = catalog.piece_item(Some(p));
    js::or_zero(jv::to_number(item.as_ref().and_then(|i| i.get("peso"))))
}

/// `pesoBolsa`: a soma das linhas de uma bolsa, ou o total que o jogador
/// escreveu no lugar dela.
pub fn bag_weight(b: &Value) -> f64 {
    let w = written(b.get("pesoTotal"));
    if !w.is_empty() {
        return weight_from_text(Some(&Value::String(w)));
    }
    b.get("itens").and_then(Value::as_array).map_or(0.0, |l| {
        l.iter()
            .fold(0.0, |s, x| s + weight_from_text(x.get("peso")))
    })
}

fn list(v: Option<&Value>) -> &[Value] {
    v.and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

/// `cargaCalculada`: as bolsas, mais o arsenal inteiro, mais a armadura
/// vestida (a peça antiga, gravada só pelo id, pesa o da armadura do catálogo).
pub fn carried_load(catalog: &Catalog, sheet: &Value) -> f64 {
    let bags = list(sheet.get("bolsas"))
        .iter()
        .fold(0.0, |s, b| s + bag_weight(b));
    let arsenal = list(sheet.get("arsenal"))
        .iter()
        .fold(0.0, |s, p| s + piece_weight(catalog, p));
    let armor = list(jv::get(sheet.get("equip"), "armaduras"))
        .iter()
        .fold(0.0, |s, p| match p {
            Value::String(id) => {
                let base = if id == "c" { None } else { catalog.armor(id) };
                s + js::or_zero(jv::to_number(base.and_then(|a| a.get("peso"))))
            }
            p if p.get("vestida") == Some(&Value::Bool(false)) => s,
            p => s + piece_weight(catalog, p),
        });
    bags + arsenal + armor
}

/// `cargaAtual`: a carga que o jogador escreveu, ou a calculada.
pub fn current_load(catalog: &Catalog, sheet: &Value) -> f64 {
    let w = written(sheet.get("cargaAtual"));
    if w.is_empty() {
        carried_load(catalog, sheet)
    } else {
        weight_from_text(Some(&Value::String(w)))
    }
}

/// `r1n`: o número curto da ficha, com vírgula (uma casa, nenhuma de 100 em
/// diante): `122`, `10`, `1,5`.
pub fn short_number(n: f64) -> String {
    let s = if n >= 100.0 {
        js::number_to_string(js::round(n))
    } else {
        js::number_to_string(js::round(n * 10.0) / 10.0)
    };
    s.replacen('.', ",", 1)
}
