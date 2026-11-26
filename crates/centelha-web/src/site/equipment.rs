//! A página de Equipamentos: o catálogo de armas, armaduras e escudos.
//!
//! Port de `src/pages/equipamentos.astro` do site atual. A lista e a ordem das
//! peças vêm dos leitores do `centelha-data`; cada peça, já achatada (o bloco
//! de números junto da raiz), vem do catálogo do motor
//! (`centelha_engine::sheet::Catalog`), e os números das células saem dos
//! textos do motor (`weapon_stats`, `damage_text`, `armor_stats`,
//! `shield_stats`), iguais byte a byte aos do site atual. Só a forma de cada
//! célula (nome da classe, dos modos de ataque, junção das tags) é desta
//! página, como lá.

use std::collections::BTreeSet;
use std::path::Path;

use centelha_engine::sheet::{Catalog, armor_stats, damage_text, shield_stats, weapon_stats};
use leptos::serde_json::Value;

use super::SiteError;

/// A ordem das classes de arma na página (e no filtro).
pub const CLASS_ORDER: &[&str] = &["leve", "media", "pesada", "haste", "distancia", "arremesso"];

/// O nome de uma classe (`CLASSE` do site atual).
pub fn class_name(id: &str) -> &str {
    match id {
        "leve" => "Leve",
        "media" => "Média",
        "pesada" => "Pesada",
        "haste" => "Haste",
        "distancia" => "Distância",
        "arremesso" => "Arremesso",
        "nenhuma" => "Nenhuma",
        other => other,
    }
}

/// Uma linha de arma.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeaponRow {
    pub name: String,
    /// O id da classe (para o filtro); `class` é o nome dela (a célula).
    pub class_id: String,
    pub class: String,
    pub damage: String,
    pub accuracy: String,
    /// A Defesa (corpo a corpo) ou a Efetiva (à distância).
    pub defense_or_range: String,
    pub hands: String,
    pub speed: String,
    pub modes: String,
    pub skill: String,
    pub tags: Vec<String>,
    /// O texto da busca (nome, tags e notas, em minúsculas).
    pub search: String,
}

/// Uma linha de armadura.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArmorRow {
    pub name: String,
    pub class: String,
    pub impact: String,
    pub cut: String,
    pub pierce: String,
    pub penalty: String,
    pub notes: String,
}

/// Uma linha de escudo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShieldRow {
    pub name: String,
    pub defense: String,
    pub vs_fast: String,
    pub penalty: String,
    pub notes: String,
}

/// O que a página mostra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Equipment {
    pub melee: Vec<WeaponRow>,
    pub ranged: Vec<WeaponRow>,
    pub armors: Vec<ArmorRow>,
    pub shields: Vec<ShieldRow>,
    /// As tags das armas, em ordem de código (o `sort()` do site atual).
    pub tags: Vec<String>,
}

fn text(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|x| x != 0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

/// `modoStr` e `modosOrd` do site atual: impacto, corte, perfurante; ★ no
/// modo principal; o Nível de Perfuração entre parênteses.
fn modes_text(w: &Value) -> String {
    if truthy(w.get("semDano")) {
        return "\u{2014}".to_string();
    }
    let modes: Vec<Value> = match w.get("modos").and_then(Value::as_array) {
        Some(list) => list.clone(),
        None => vec![leptos::serde_json::json!({
            "tipo": w.get("tipoDano").cloned().unwrap_or(Value::Null),
            "perf": w.get("pen").cloned().unwrap_or(Value::Null),
            "principal": true,
        })],
    };
    let rank = |m: &Value| match m.get("tipo").and_then(Value::as_str) {
        Some("impacto") => 0,
        Some("corte") => 1,
        Some("perfurante") => 2,
        _ => 9,
    };
    let mut modes = modes;
    modes.sort_by_key(rank);
    modes
        .iter()
        .map(|m| {
            let star = if truthy(m.get("principal")) {
                "\u{2605}"
            } else {
                ""
            };
            let name = match m.get("tipo").and_then(Value::as_str) {
                Some("corte") => "Cortante",
                Some("perfurante") => "Perfurante",
                Some("impacto") => "Impacto",
                _ => "undefined",
            };
            let level = match m.get("perf") {
                None | Some(Value::Null) => String::new(),
                Some(p) => format!(" (N{})", text(Some(p))),
            };
            format!("{star}{name}{level}")
        })
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
}

/// Os pedaços de `weapon_stats` ("ticks/acerto/dano/defesa").
fn weapon_parts(w: &Value) -> (String, String, String, String) {
    let stats = weapon_stats(w);
    let mut it = stats.splitn(4, '/');
    let ticks = it.next().unwrap_or_default().to_string();
    let accuracy = it.next().unwrap_or_default().to_string();
    let _damage = it.next();
    let defense = it.next().unwrap_or_default().to_string();
    (ticks, accuracy, damage_text(w), defense)
}

fn weapon_row(w: &Value, skills: &dyn Fn(&str) -> String, ranged: bool) -> WeaponRow {
    let (speed, accuracy, damage, defense) = weapon_parts(w);
    let damage = if truthy(w.get("semDano")) {
        "não causa dano".to_string()
    } else {
        damage
    };
    let defense_or_range = if ranged {
        if truthy(w.get("efetiva")) {
            format!("{} m", text(w.get("efetiva")))
        } else {
            "\u{2014}".to_string()
        }
    } else {
        defense
    };
    let tags: Vec<String> = w
        .get("tags")
        .and_then(Value::as_array)
        .map(|t| {
            t.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let skill_id = match w.get("pericia") {
        Some(Value::String(s)) => s.clone(),
        Some(other) => text(other.get("id")),
        None => String::new(),
    };
    let name = text(w.get("nome"));
    let search = format!("{} {} {}", name, tags.join(" "), text(w.get("notas"))).to_lowercase();
    let class_id = text(w.get("classe"));
    WeaponRow {
        name,
        class: class_name(&class_id).to_string(),
        class_id,
        damage,
        accuracy,
        defense_or_range,
        hands: text(w.get("maos")),
        speed,
        modes: modes_text(w),
        skill: skills(&skill_id),
        tags,
        search,
    }
}

/// Lê o catálogo e monta as quatro tabelas, na ordem do site atual.
pub fn load(content_dir: &Path, skill_names: &[(String, String)]) -> Result<Equipment, SiteError> {
    let read = |name: &str| -> Result<String, SiteError> {
        let path = content_dir.join("data").join(name);
        std::fs::read_to_string(&path).map_err(super::io_error(&path))
    };
    let catalog = Catalog::from_json(
        &read("armas.json")?,
        &read("armaduras.json")?,
        &read("escudos.json")?,
    )
    .map_err(SiteError::Equipment)?;
    let weapons = centelha_data::load_weapons(content_dir)?;
    let armors = centelha_data::load_armors(content_dir)?;
    let shields = centelha_data::load_shields(content_dir)?;
    let skills = |id: &str| {
        skill_names
            .iter()
            .find(|(k, _)| k == id)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| id.to_string())
    };

    // As armas da loja (sem as só de criatura), por classe e depois pelo nome.
    let mut list: Vec<&Value> = weapons
        .iter()
        .filter_map(|w| catalog.weapon(&w.id))
        .filter(|w| !truthy(w.get("soCriatura")))
        .collect();
    let class_rank = |w: &Value| {
        let c = text(w.get("classe"));
        CLASS_ORDER
            .iter()
            .position(|x| *x == c)
            .map(|i| i as i64)
            .unwrap_or(-1)
    };
    list.sort_by(|a, b| {
        class_rank(a).cmp(&class_rank(b)).then_with(|| {
            centelha_engine::collate::compare(&text(a.get("nome")), &text(b.get("nome")))
        })
    });
    let tags: BTreeSet<String> = list
        .iter()
        .flat_map(|w| {
            w.get("tags")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|t| t.as_str().map(str::to_string))
        })
        .collect();
    let is_ranged = |w: &Value| matches!(text(w.get("classe")).as_str(), "distancia" | "arremesso");
    let melee = list
        .iter()
        .filter(|w| !is_ranged(w))
        .map(|w| weapon_row(w, &skills, false))
        .collect();
    let mut ranged_list: Vec<&Value> = list.iter().copied().filter(|w| is_ranged(w)).collect();
    let range = |w: &Value| w.get("efetiva").and_then(Value::as_f64).unwrap_or(0.0);
    ranged_list.sort_by(|a, b| {
        range(b)
            .partial_cmp(&range(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let ranged = ranged_list
        .iter()
        .map(|w| weapon_row(w, &skills, true))
        .collect();

    let armors = armors
        .iter()
        .filter_map(|a| catalog.armor(&a.id))
        .map(|a| {
            // "Imp 3 · Cor 4 · Perf 1 (N0) · Pen −1"
            let stats = armor_stats(a);
            let parts: Vec<&str> = stats.split(" \u{b7} ").collect();
            let after = |i: usize, prefix: &str| {
                parts
                    .get(i)
                    .and_then(|p| p.strip_prefix(prefix))
                    .unwrap_or_default()
                    .to_string()
            };
            ArmorRow {
                name: text(a.get("nome")),
                class: class_name(&text(a.get("classe"))).to_string(),
                impact: after(0, "Imp "),
                cut: after(1, "Cor "),
                pierce: after(2, "Perf "),
                penalty: after(3, "Pen "),
                notes: text(a.get("notas")),
            }
        })
        .collect();

    let shields = shields
        .iter()
        .filter_map(|s| catalog.shield(&s.id))
        .filter(|s| s.get("bloqCaC").and_then(Value::as_f64).unwrap_or(0.0) > 0.0)
        .map(|s| {
            // "+1 Def · −1 Pen"
            let stats = shield_stats(s);
            let (def, pen) = stats.split_once(" \u{b7} ").unwrap_or((&stats, ""));
            let fast = s.get("vsProjetilRapido");
            let vs_fast = if fast.is_some_and(|f| truthy(f.get("bloqueia"))) {
                match fast.and_then(|f| f.get("bonus")) {
                    Some(b) if truthy(Some(b)) => format!("bloqueia +{}", text(Some(b))),
                    _ => "bloqueia".to_string(),
                }
            } else {
                "\u{2014}".to_string()
            };
            ShieldRow {
                name: text(s.get("nome")),
                defense: def.trim_end_matches(" Def").to_string(),
                vs_fast,
                penalty: pen.trim_end_matches(" Pen").to_string(),
                notes: text(s.get("notas")),
            }
        })
        .collect();

    Ok(Equipment {
        melee,
        ranged,
        armors,
        shields,
        tags: tags.into_iter().collect(),
    })
}
