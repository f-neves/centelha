//! Paridade das contas das Técnicas com o TypeScript
//! (tests/parity/techniques.json, de tools/parity/techniques.mjs): o
//! modificador pela trilha e o fecho dos pré-requisitos com o XP da cadeia.

mod common;

use centelha_data::{Rules, Technique};
use centelha_engine::technique::{chain_xp, prerequisite_closure, technique_modifier};
use common::{compare, num, repo};
use serde_json::{Value, json};

fn tables() -> (Rules, Vec<Technique>) {
    let c = repo().join("content");
    (
        centelha_data::load_rules(&c).unwrap(),
        centelha_data::load_techniques(&c).unwrap(),
    )
}

fn run(rules: &Rules, all: &[Technique], name: &str, a: &[Value]) -> Value {
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
        "closure" => {
            // o fecho da fonte é a própria Técnica seguida dos pré-requisitos
            let mut chain = vec![id];
            chain.extend(prerequisite_closure(all, id));
            json!({"chain": chain, "xp": num(chain_xp(rules, all, id))})
        }
        other => panic!("função sem par no Rust: {other}"),
    }
}

#[test]
fn techniques_match_typescript() {
    let (rules, all) = tables();
    let r = common::read_reference("techniques.json");
    let cases = r["cases"].as_array().unwrap();
    assert_eq!(cases.len() as u64, r["count"].as_u64().unwrap());
    // cada Técnica e um id de fora
    assert_eq!(r["counts"]["closure"].as_u64(), Some(all.len() as u64 + 1));
    let mut fields = 0;
    for (i, c) in cases.iter().enumerate() {
        let name = c["fn"].as_str().unwrap();
        let got = run(&rules, &all, name, c["args"].as_array().unwrap());
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

/// O id da Técnica não precisa viver tanto quanto a lista: a cadeia só
/// guarda textos da lista.
#[test]
fn the_chain_outlives_the_id() {
    let (_, all) = tables();
    let chain = {
        let id = String::from("velocidade-divina");
        prerequisite_closure(&all, &id)
    };
    assert!(chain.len() > 3, "{chain:?}");
}

#[test]
fn the_comparison_catches_differences() {
    common::check_comparator();
}
