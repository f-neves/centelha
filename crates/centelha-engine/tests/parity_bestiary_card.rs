//! Paridade do card do bestiário com o site atual.
//!
//! - O card de cada criatura de `content/data/bestiario/` (e o de cada forma,
//!   dentro dele) contra a entrada do `content/data/monsters.json`, que o
//!   `gen-monsters.mjs` da fonte escreveu no mesmo commit do `content/`: campo a
//!   campo, o card inteiro. `CARD_REFERENCE` aponta para uma cópia, no teste
//!   negativo.
//! - As três contas que o card faz na hora (a linha de PV na zona, o pool com a
//!   penalidade e a Aparência do card) contra `tests/parity/bestiary-card.json`,
//!   de `tools/parity/bestiary.mjs`.

mod common;

use std::collections::HashMap;
use std::path::Path;

use centelha_data::bestiary::HpLine;
use centelha_engine::bestiary_card::{
    CardTables, bestiary_card, card_appearance_mod, card_mass, hp_line_in_zone, pool_with_penalty,
};
use centelha_engine::creature::Tables;
use common::{compare, num, opt, repo};
use serde_json::{Value, json};

fn raw(content: &Path, name: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(content.join("data").join(name)).unwrap())
        .unwrap()
}

fn tables() -> CardTables {
    let c = repo().join("content");
    let creature = Tables::new(
        centelha_data::load_rules(&c).unwrap(),
        centelha_data::load_weapons(&c).unwrap(),
        centelha_data::load_armors(&c).unwrap(),
        centelha_data::load_shields(&c).unwrap(),
    );
    CardTables::new(
        creature,
        raw(&c, "armas.json"),
        raw(&c, "regras.json"),
        &raw(&c, "habilidades.json"),
        &centelha_data::load_techniques(&c).unwrap(),
        &centelha_data::load_arts(&c).unwrap(),
    )
    .unwrap()
}

fn sources() -> Vec<(String, Value)> {
    let dir = repo().join("content/data/bestiario");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|p| {
            let v: Value = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
            (v["id"].as_str().unwrap().to_owned(), v)
        })
        .collect()
}

#[test]
fn cards_match_monsters_json() {
    let t = tables();
    // `CARD_REFERENCE` troca o monsters.json por uma cópia (o teste negativo,
    // sem mexer em content/)
    let monsters: Value = match std::env::var("CARD_REFERENCE") {
        Ok(p) => serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap(),
        Err(_) => raw(&repo().join("content"), "monsters.json"),
    };
    let by_id: HashMap<&str, &Value> = monsters
        .as_array()
        .unwrap()
        .iter()
        .map(|m| (m["id"].as_str().unwrap(), m))
        .collect();
    let all = sources();
    assert_eq!(all.len(), 324);
    assert_eq!(by_id.len(), 324);
    let (mut fields, mut forms) = (0, 0);
    for (id, src) in &all {
        let got = bestiary_card(&t, src).unwrap_or_else(|e| panic!("{id}: {e:?}"));
        let want = by_id[id.as_str()];
        forms += want["formas"].as_array().unwrap().len();
        let mut diffs = Vec::new();
        fields += compare(id, &got, want, &mut diffs);
        assert!(
            diffs.is_empty(),
            "primeira criatura diferente: {id} ({} diferenças):\n{}",
            diffs.len(),
            diffs[..diffs.len().min(40)].join("\n")
        );
    }
    assert_eq!(forms, 20);
    println!("cards: 324 criaturas e {forms} formas, {fields} campos comparados, 0 diferenças");
}

fn line(v: &Value) -> Option<HpLine> {
    match v.as_str() {
        Some("acima") => Some(HpLine::Above),
        Some("abaixo") => Some(HpLine::Below),
        _ => None,
    }
}

#[test]
fn card_time_functions_match_typescript() {
    let t = tables();
    let rules = t.creature_tables().rules();
    let r = common::read_reference("bestiary-card.json");
    let cases = r["cases"].as_array().unwrap();
    assert_eq!(cases.len() as u64, r["count"].as_u64().unwrap());
    let mut fields = 0;
    for (i, c) in cases.iter().enumerate() {
        let a = c["args"].as_array().unwrap();
        let name = c["fn"].as_str().unwrap();
        let got = match name {
            "linhaPVNaZona" => {
                let kg = a[0].as_f64();
                match hp_line_in_zone(rules, kg, a[1].as_f64().unwrap(), line(&a[2])) {
                    Some(z) => json!({
                        "zonaKg": [num(z.zone_kg[0]), num(z.zone_kg[1])],
                        "linha": if z.line == HpLine::Above { "acima" } else { "abaixo" },
                        "pvAcima": num(z.hp_above), "pvAbaixo": num(z.hp_below),
                    }),
                    None => Value::Null,
                }
            }
            "massaDaConta" => opt(card_mass(rules, &a[0])),
            "poolComPenalidade" => json!(pool_with_penalty(
                a[0].as_str().unwrap(),
                a[1].as_f64().unwrap()
            )),
            "apMod" => opt(card_appearance_mod(a[0].as_f64().unwrap())),
            other => panic!("função sem par no Rust: {other}"),
        };
        let mut diffs = Vec::new();
        fields += compare(
            &format!("{i}:{name}{}", c["args"]),
            &got,
            &c["out"],
            &mut diffs,
        );
        assert!(
            diffs.is_empty(),
            "primeiro caso diferente: {i}:\n{}",
            diffs.join("\n")
        );
    }
    println!(
        "contas do card: {} casos, {fields} campos comparados, 0 diferenças",
        cases.len()
    );
}

#[test]
fn the_comparison_catches_differences() {
    common::check_comparator();
}
