//! Paridade dos derivados das criaturas com o TypeScript
//! (tests/parity/creatures.json).
//!
//! A saída do Rust é escrita na mesma forma do objeto do TypeScript (as mesmas
//! chaves, `null` onde o TypeScript escreve `null`, chave ausente onde ele não
//! escreve) e comparada folha por folha: número por `to_bits()` (o gerador
//! grava -0 como o texto `"-0"`), texto e booleano por igualdade.

mod common;

use centelha_engine::creature::{CreatureDerived, Defense, Soak, Tables, derive, with_form};
use common::{compare, num, opt, repo};
use serde_json::{Map, Value, json};

fn defense(d: Defense) -> Value {
    match d {
        Defense::Value(x) => num(x),
        Defense::Immune => json!("imune"),
    }
}

fn soak(s: &Soak) -> Value {
    json!({ "impacto": num(s.impact), "corte": num(s.cut), "perfuracao": num(s.pierce) })
}

/// O objeto que o `criatura()` do TypeScript devolve.
fn as_typescript(d: &CreatureDerived) -> Value {
    let mut o = Map::new();
    if let Some(s) = &d.swarm {
        o.insert(
            "enxame".into(),
            json!({
                "pvCheio": num(s.full_hp), "pv": num(s.hp), "individuos": num(s.individuals),
                "minimo": num(s.minimum), "aindaEnxame": s.still_swarm,
            }),
        );
    }
    o.insert("pv".into(), opt(d.hp));
    o.insert("pesoErguido".into(), opt(d.lifted_weight));
    o.insert("faixa".into(), opt(d.mass_band));
    o.insert(
        "tamanho".into(),
        d.size.map_or(Value::Null, |s| json!(s.id())),
    );
    o.insert(
        "segundoTamanho".into(),
        d.second_size.map_or(Value::Null, |s| json!(s.id())),
    );
    o.insert("esquiva".into(), num(d.dodge));
    o.insert("bloqueio".into(), num(d.block));
    o.insert("defesaMental".into(), defense(d.mental_defense));
    o.insert("defesaSocial".into(), defense(d.social_defense));
    o.insert("iniciativa".into(), num(d.initiative));
    o.insert("energia".into(), num(d.energy));
    o.insert("mana".into(), opt(d.mana));
    o.insert("absorcao".into(), soak(&d.soak));
    let attacks = d
        .attacks
        .iter()
        .map(|a| {
            json!({
                "nome": a.name, "acerto": num(a.accuracy),
                "habilidade": a.skill.as_ref().map_or(Value::Null, |s| json!(s)),
                "velocidade": opt(a.speed), "danoMedio": opt(a.average_damage), "jogada": opt(a.roll),
            })
        })
        .collect();
    o.insert("ataques".into(), Value::Array(attacks));
    if let Some(e) = &d.with_equipment {
        let mut ce = Map::new();
        ce.insert("esquiva".into(), num(e.dodge));
        ce.insert("bloqueio".into(), num(e.block));
        ce.insert("impacto".into(), num(e.soak.impact));
        ce.insert("corte".into(), num(e.soak.cut));
        ce.insert("perfuracao".into(), num(e.soak.pierce));
        ce.insert("penalidade".into(), num(e.penalty));
        if let Some(n) = e.pierce_level {
            ce.insert("nivelPerfuracao".into(), num(n));
        }
        if let Some(n) = e.against_shots {
            ce.insert("contraTiro".into(), num(n));
        }
        o.insert("comEquipamento".into(), Value::Object(ce));
    }
    Value::Object(o)
}

#[test]
fn creatures_match_typescript() {
    let content = repo().join("content");
    let tables = Tables::new(
        centelha_data::load_rules(&content).unwrap(),
        centelha_data::load_weapons(&content).unwrap(),
        centelha_data::load_armors(&content).unwrap(),
        centelha_data::load_shields(&content).unwrap(),
    );
    let creatures = centelha_data::load_bestiary(&content).unwrap();
    let text = std::fs::read_to_string(repo().join("tests/parity/creatures.json")).unwrap();
    let reference: Value = serde_json::from_str(&text).unwrap();
    let expected = reference["creatures"].as_object().unwrap();
    assert_eq!(creatures.len(), expected.len());

    let mut diffs = Vec::new();
    let (mut cases, mut fields, mut forms, mut attacks) = (0, 0, 0, 0);
    for c in &creatures {
        let want = &expected[&c.id];
        let got = derive(&tables, c);
        attacks += got.attacks.len();
        fields += compare(&c.id, &as_typescript(&got), &want["derived"], &mut diffs);
        cases += 1;
        let want_forms = want.get("forms").and_then(Value::as_object);
        let got_forms = c.forms.as_deref().unwrap_or_default();
        assert_eq!(
            got_forms.len(),
            want_forms.map_or(0, |f| f.len()),
            "{}: formas",
            c.id
        );
        for form in got_forms {
            let w = &want_forms.unwrap()[&form.id];
            let got = derive(&tables, &with_form(c, form));
            attacks += got.attacks.len();
            fields += compare(
                &format!("{}/{}", c.id, form.id),
                &as_typescript(&got),
                w,
                &mut diffs,
            );
            cases += 1;
            forms += 1;
        }
    }
    println!(
        "criaturas: {cases} casos de paridade ({} criaturas, {forms} formas, {attacks} ataques), {fields} campos comparados, {} diferenças",
        creatures.len(),
        diffs.len()
    );
    assert!(
        diffs.is_empty(),
        "{} diferenças:\n{}",
        diffs.len(),
        diffs[..diffs.len().min(60)].join("\n")
    );
}

#[test]
fn the_comparison_catches_differences() {
    common::check_comparator();
}
