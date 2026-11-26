//! Paridade das contas da ficha interativa com o TypeScript
//! (tests/parity/sheet-editor.json, de tools/parity/sheet-editor.mjs): a ficha
//! nova, o XP gasto, os tetos e o piso, a troca de Raça, o recorte das
//! especialidades, o estado das Técnicas e os Efeitos ao alcance.
//!
//! As fichas de entrada estão em `inputs`, por id (a ficha depois do
//! `normalize` da fonte e o `massaEscolhida` que ele deixou). O teste para no
//! primeiro caso diferente.

mod common;

use centelha_data::Technique;
use centelha_engine::sheet::{
    Catalog, EditorTables, SheetTables, attribute_floor, change_ancestry, clamp_specialties,
    effects_in_reach, fresh_sheet, spent_xp, technique_status, trait_cap,
};
use common::{compare, num, repo};
use serde_json::{Map, Value, json};

fn tables() -> (EditorTables, Vec<Technique>) {
    let content = repo().join("content");
    let read = |name: &str| std::fs::read_to_string(content.join("data").join(name)).unwrap();
    let raw = |name: &str| -> Value { serde_json::from_str(&read(name)).unwrap() };
    let sheet = SheetTables::new(
        centelha_data::load_rules(&content).unwrap(),
        centelha_data::load_ancestries(&content).unwrap(),
        Catalog::from_json(
            &read("armas.json"),
            &read("armaduras.json"),
            &read("escudos.json"),
        )
        .unwrap(),
    );
    let techniques = centelha_data::load_techniques(&content).unwrap();
    let t = EditorTables::new(
        sheet,
        &raw("atributos.json"),
        &raw("habilidades.json"),
        &raw("habilidades-secundarias.json"),
        &raw("virtudes.json"),
        &raw("antecedentes.json"),
        techniques.clone(),
        centelha_data::load_arts(&content).unwrap(),
        centelha_data::load_effects(&content).unwrap(),
    );
    (t, techniques)
}

fn f(v: &Value) -> f64 {
    match v {
        Value::String(s) if s == "-0" => -0.0,
        v => v.as_f64().unwrap_or_else(|| panic!("não é número: {v}")),
    }
}

/// As pílulas que o `renderCaminhos` marca: as classes e o motivo do bloqueio.
fn technique_pills(sheet: &Value, techniques: &[Technique]) -> Value {
    let names: std::collections::HashMap<&str, &str> = techniques
        .iter()
        .map(|t| (t.id.as_str(), t.name.as_str()))
        .collect();
    let mut out = Map::new();
    for t in techniques {
        let s = technique_status(sheet, t);
        let mut classes = Vec::new();
        if s.owned {
            classes.push("on");
        }
        if !s.owned && !s.ok {
            classes.push("locked");
        }
        if s.owned && !s.ok {
            classes.push("invalid");
        }
        if classes.is_empty() {
            continue;
        }
        let mut reasons = Vec::new();
        if !s.spark_ok {
            reasons.push(format!(
                "Centelha {}",
                centelha_engine::js::number_to_string(t.level)
            ));
        }
        if !s.missing.is_empty() {
            let list: Vec<&str> = s
                .missing
                .iter()
                .map(|p| names.get(p.as_str()).copied().unwrap_or("undefined"))
                .collect();
            reasons.push(format!("pré: {}", list.join(", ")));
        }
        let mut m = Map::new();
        m.insert("classes".into(), json!(classes));
        if !reasons.is_empty() {
            m.insert(
                "requer".into(),
                json!(reasons.join(" \u{b7} ").replace('"', "")),
            );
        }
        out.insert(t.id.clone(), Value::Object(m));
    }
    Value::Object(out)
}

fn run(
    t: &EditorTables,
    techniques: &[Technique],
    inputs: &Value,
    name: &str,
    a: &[Value],
) -> Value {
    let input = |i: usize| &inputs[a[i].as_str().unwrap()];
    match name {
        "fresh" => fresh_sheet(t),
        "xpGasto" => {
            let x = spent_xp(t, &input(0)["ficha"]);
            json!({
                "xa": num(x.attributes), "xs": num(x.skills), "xsp": num(x.specialties), "xv": num(x.virtues),
                "xw": num(x.willpower), "xap": num(x.appearance), "xc": num(x.spark), "x2": num(x.secondary),
                "xt": num(x.techniques), "xar": num(x.arts), "xef": num(x.effects), "xan": num(x.backgrounds),
                "xr": num(x.ancestry), "total": num(x.total),
            })
        }
        "capFor" => {
            let sheet = json!({"raca": a[0], "centelha": a[1]});
            num(trait_cap(t, &sheet, a[2].as_str().unwrap(), a[3].as_str()))
        }
        "pisoAttr" => num(attribute_floor(
            t,
            &json!({"raca": a[0]}),
            a[1].as_str().unwrap(),
        )),
        "trocaDeRaca" => {
            let x = input(0);
            let mut sheet = x["ficha"].clone();
            change_ancestry(
                t,
                &mut sheet,
                a[1].as_str().unwrap(),
                x["massaEscolhida"].as_bool().unwrap(),
            );
            json!({"attrs": sheet["attrs"], "corpo": sheet.get("corpo").cloned().unwrap_or(Value::Null)})
        }
        "clampSpecs" => Value::Array(
            clamp_specialties(&a[0], f(&a[1]))
                .into_iter()
                .map(|e| json!({"s": e.name, "v": num(e.level)}))
                .collect(),
        ),
        "tecnicas" => technique_pills(&input(0)["ficha"], techniques),
        "efeitosAoAlcance" => Value::Array(
            effects_in_reach(t, &input(0)["ficha"])
                .into_iter()
                .map(|e| json!(e.id))
                .collect(),
        ),
        other => panic!("função sem par no Rust: {other}"),
    }
}

#[test]
fn sheet_editor_matches_typescript() {
    let (t, techniques) = tables();
    let r = common::read_reference("sheet-editor.json");
    let cases = r["cases"].as_array().unwrap();
    assert_eq!(cases.len() as u64, r["count"].as_u64().unwrap());
    let mut fields = 0;
    for (i, c) in cases.iter().enumerate() {
        let name = c["fn"].as_str().unwrap();
        let got = run(
            &t,
            &techniques,
            &r["inputs"],
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
            "primeiro caso diferente: {i} ({} diferenças):\n{}",
            diffs.len(),
            diffs[..diffs.len().min(30)].join("\n")
        );
    }
    println!(
        "ficha interativa: {} casos, {fields} campos comparados, 0 diferenças",
        cases.len()
    );
}

#[test]
fn the_comparison_catches_differences() {
    common::check_comparator();
}
