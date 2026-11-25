//! Paridade dos derivados das criaturas com o TypeScript
//! (tests/parity/creatures.json).
//!
//! A saída do Rust é escrita na mesma forma do objeto do TypeScript (as mesmas
//! chaves, `null` onde o TypeScript escreve `null`, chave ausente onde ele não
//! escreve) e comparada folha por folha: número por `to_bits()` (o JSON não
//! guarda -0, então 0 e -0 contam iguais), texto e booleano por igualdade.

use std::path::{Path, PathBuf};

use centelha_engine::creature::{CreatureDerived, Defense, Soak, Tables, derive, with_form};
use serde_json::{Map, Value, json};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn num(x: f64) -> Value {
    Value::Number(
        serde_json::Number::from_f64(x).unwrap_or_else(|| panic!("número não finito: {x}")),
    )
}

fn opt(x: Option<f64>) -> Value {
    x.map_or(Value::Null, num)
}

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

/// Compara folha por folha; devolve quantas folhas comparou e anota as
/// diferenças.
fn compare(path: &str, got: &Value, want: &Value, diffs: &mut Vec<String>) -> usize {
    match (got, want) {
        (Value::Object(g), Value::Object(w)) => {
            let mut n = 0;
            for k in w.keys().chain(g.keys().filter(|k| !w.contains_key(*k))) {
                match (g.get(k), w.get(k)) {
                    (Some(a), Some(b)) => n += compare(&format!("{path}.{k}"), a, b, diffs),
                    (None, Some(b)) => {
                        diffs.push(format!("{path}.{k}: falta no Rust (TypeScript: {b})"))
                    }
                    (Some(a), None) => diffs.push(format!("{path}.{k}: sobra no Rust ({a})")),
                    (None, None) => unreachable!(),
                }
            }
            n
        }
        (Value::Array(g), Value::Array(w)) => {
            if g.len() != w.len() {
                diffs.push(format!(
                    "{path}: {} itens no Rust, {} no TypeScript",
                    g.len(),
                    w.len()
                ));
                return 1;
            }
            g.iter()
                .zip(w)
                .enumerate()
                .map(|(i, (a, b))| compare(&format!("{path}[{i}]"), a, b, diffs))
                .sum()
        }
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            if a.to_bits() != b.to_bits() && !(a == 0.0 && b == 0.0) {
                diffs.push(format!("{path}: Rust {a:?}, TypeScript {b:?}"));
            }
            1
        }
        (a, b) => {
            if a != b {
                diffs.push(format!("{path}: Rust {a}, TypeScript {b}"));
            }
            1
        }
    }
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
    let mut diffs = Vec::new();
    let want = json!({"pv": 37, "tamanho": "medio", "mana": null, "ataques": [{"jogada": 7}]});
    let same = json!({"pv": 37.0, "tamanho": "medio", "mana": null, "ataques": [{"jogada": 7}]});
    assert_eq!(compare("x", &same, &want, &mut diffs), 4);
    assert!(diffs.is_empty());
    let cases = [
        json!({"pv": num(f64::from_bits(37f64.to_bits() + 1)), "tamanho": "medio", "mana": null, "ataques": [{"jogada": 7}]}),
        json!({"pv": 37, "tamanho": "grande", "mana": null, "ataques": [{"jogada": 7}]}),
        json!({"pv": 37, "tamanho": "medio", "mana": 0, "ataques": [{"jogada": 7}]}),
        json!({"pv": 37, "tamanho": "medio", "ataques": [{"jogada": 7}]}),
        json!({"pv": 37, "tamanho": "medio", "mana": null, "ataques": [{"jogada": 7}], "enxame": {}}),
        json!({"pv": 37, "tamanho": "medio", "mana": null, "ataques": []}),
    ];
    for got in cases {
        let mut diffs = Vec::new();
        compare("x", &got, &want, &mut diffs);
        assert_eq!(diffs.len(), 1, "{got}");
    }
}
