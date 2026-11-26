//! Paridade das contas das Técnicas com o TypeScript
//! (tests/parity/techniques.json, de tools/parity/techniques.mjs): o
//! modificador pela trilha, a Técnica que o site oferece e o fecho dos
//! pré-requisitos com o XP da cadeia.

mod common;

use centelha_data::{Rules, Technique};
use centelha_engine::technique::{
    chain_xp, prerequisite_closure, technique_available, technique_modifier,
};
use common::{compare, num, repo};
use serde_json::{Value, json};

fn tables() -> (Rules, Vec<Technique>) {
    let c = repo().join("content");
    (
        centelha_data::load_rules(&c).unwrap(),
        centelha_data::load_techniques(&c).unwrap(),
    )
}

fn run(
    rules: &Rules,
    all: &[Technique],
    available: &[Technique],
    name: &str,
    a: &[Value],
) -> Value {
    let id = a[0].as_str().unwrap();
    match name {
        "modProeza" => {
            let level = match &a[1] {
                Value::String(s) if s == "-0" => -0.0,
                v => v.as_f64().unwrap(),
            };
            technique_modifier(rules, id, level)
                .map_or(Value::Null, |m| json!({"v": m.value, "t": m.title}))
        }
        "tecnicaDisponivel" => json!(technique_available(
            all.iter().find(|t| t.id == id).unwrap()
        )),
        "closure" => {
            let chain = prerequisite_closure(available, id);
            json!({"chain": chain, "xp": num(chain_xp(rules, available, &chain))})
        }
        other => panic!("função sem par no Rust: {other}"),
    }
}

#[test]
fn techniques_match_typescript() {
    let (rules, all) = tables();
    let available: Vec<Technique> = all
        .iter()
        .filter(|t| technique_available(t))
        .cloned()
        .collect();
    let r = common::read_reference("techniques.json");
    let cases = r["cases"].as_array().unwrap();
    assert_eq!(cases.len() as u64, r["count"].as_u64().unwrap());
    assert_eq!(
        r["counts"]["closure"].as_u64(),
        Some(available.len() as u64 + 10)
    );
    let mut fields = 0;
    for (i, c) in cases.iter().enumerate() {
        let name = c["fn"].as_str().unwrap();
        let got = run(
            &rules,
            &all,
            &available,
            name,
            c["args"].as_array().unwrap(),
        );
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
        "Técnicas: {} casos, {fields} campos comparados, 0 diferenças",
        cases.len()
    );
}

#[test]
fn the_comparison_catches_differences() {
    common::check_comparator();
}
