//! Paridade da formatação dos Efeitos (`artes-fmt.ts`) e do custo da
//! conjuração (`custoDe`, de `artes-grid.ts`) com o TypeScript
//! (tests/parity/arts.json, de tools/parity/arts.mjs).

mod common;

use centelha_data::Rules;
use centelha_data::effects::{Effect, EffectParameter};
use centelha_engine::arts_fmt::{
    PARAMETER_FORM, form_of, id_of, order_parameters, parameter_value, rank, rank_by_name,
};
use centelha_engine::casting::{CastingCost, casting_cost, mana_paid};
use common::{compare, num, repo};
use serde_json::{Value, json};

struct Ctx {
    rules: Rules,
    effects: Vec<Effect>,
}

fn param(v: &Value) -> EffectParameter {
    serde_json::from_value(v.clone()).unwrap()
}

fn cost_json(c: &CastingCost) -> Value {
    json!({
        "base": num(c.base),
        "parametros": num(c.parameters),
        "esticados": c.stretched.iter().map(|s| json!({
            "nome": s.name, "acima": num(s.above), "custo": num(s.cost),
        })).collect::<Vec<_>>(),
        "total": num(c.total),
        "centelha": num(c.spark),
        "mana": num(c.mana),
    })
}

fn run(ctx: &Ctx, name: &str, a: &[Value]) -> Value {
    let effect = |v: &Value| {
        v.as_str()
            .map(|id| ctx.effects.iter().find(|e| e.id == id).unwrap())
    };
    match name {
        "PAR_FORMA" => json!(PARAMETER_FORM),
        "idDe" => id_of(&a[0]),
        "formaDe" => form_of(a[0].as_str().unwrap()).map_or(Value::Null, |f| json!(f.id())),
        "rank" => json!(rank_by_name(a[0].as_str().unwrap(), a[1].as_str())),
        "ordemPar" => {
            let e = effect(&a[0]).unwrap();
            let order = order_parameters(&e.parameters);
            json!(
                order
                    .iter()
                    .map(|p| e
                        .parameters
                        .iter()
                        .position(|q| std::ptr::eq(q, *p))
                        .unwrap())
                    .collect::<Vec<_>>()
            )
        }
        "ordemParLista" => {
            let list: Vec<EffectParameter> = a[0].as_array().unwrap().iter().map(param).collect();
            let order = order_parameters(&list);
            json!(
                order
                    .iter()
                    .map(|p| list.iter().position(|q| std::ptr::eq(q, *p)).unwrap())
                    .collect::<Vec<_>>()
            )
        }
        "valorPar" => {
            let p = param(&a[0]);
            let arts: Vec<&str> = a[1]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().unwrap())
                .collect();
            parameter_value(&p, &arts).map_or(Value::Null, Value::from)
        }
        "custoDe" => {
            let choices: Vec<(&str, f64)> = a[2]
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_f64().unwrap()))
                .collect();
            let c = casting_cost(
                &ctx.rules,
                effect(&a[0]),
                a[1].as_f64().unwrap(),
                &choices,
                a[3].as_f64().unwrap(),
                false,
            );
            cost_json(&c)
        }
        "manaPaga" => num(mana_paid(
            &ctx.rules,
            a[0].as_f64().unwrap(),
            a[1].as_f64().unwrap(),
            a[2].as_bool().unwrap(),
        )),
        other => panic!("função sem par no Rust: {other}"),
    }
}

#[test]
fn arts_match_typescript() {
    let content = repo().join("content");
    let ctx = Ctx {
        rules: centelha_data::load_rules(&content).unwrap(),
        effects: centelha_data::load_effects(&content).unwrap(),
    };
    let r = common::read_reference("arts.json");
    let cases = r["cases"].as_array().unwrap();
    assert_eq!(cases.len() as u64, r["count"].as_u64().unwrap());
    let mut fields = 0;
    let mut diffs = Vec::new();
    for (i, c) in cases.iter().enumerate() {
        let name = c["fn"].as_str().unwrap();
        let got = run(&ctx, name, c["args"].as_array().unwrap());
        fields += compare(
            &format!("{i}:{name}{}", c["args"]),
            &got,
            &c["out"],
            &mut diffs,
        );
        if !diffs.is_empty() {
            break;
        }
    }
    println!(
        "Artes (formatação e custo): {} casos, {fields} campos comparados, {} diferenças",
        cases.len(),
        diffs.len()
    );
    assert!(
        diffs.is_empty(),
        "primeiro caso diferente ({} diferenças):\n{}",
        diffs.len(),
        diffs[..diffs.len().min(30)].join("\n")
    );
}

#[test]
fn the_comparison_catches_differences() {
    common::check_comparator();
}

/// O `rank` de cada parâmetro do catálogo também sai pelo tipo, não só pelo nome.
#[test]
fn rank_of_a_parameter_matches_rank_by_name() {
    let content = repo().join("content");
    for e in centelha_data::load_effects(&content).unwrap() {
        for p in &e.parameters {
            assert_eq!(rank(p), rank_by_name(&p.name, p.replaces.as_deref()));
        }
    }
}

/// Com a fonte do elemento ao alcance a Mana é a da Centelha somada do
/// desconto do livro (`zeraCusto`: Centelha 3 com a fonte é Centelha 4 sem).
#[test]
fn the_source_discount_adds_to_the_spark() {
    let content = repo().join("content");
    let rules = centelha_data::load_rules(&content).unwrap();
    let discount = centelha_engine::casting::elemental_source_discount(&rules);
    assert_eq!(discount, 1.0);
    for total in 0..=12 {
        for spark in 0..=6 {
            let (total, spark) = (f64::from(total), f64::from(spark));
            assert_eq!(
                mana_paid(&rules, total, spark, true),
                mana_paid(&rules, total, spark + discount, false)
            );
        }
    }
    // a fonte nunca deixa a Mana abaixo de zero
    assert_eq!(mana_paid(&rules, 1.0, 5.0, true), 0.0);
}
