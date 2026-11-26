//! Paridade das contas pequenas do personagem com o TypeScript
//! (tests/parity/character.json): XP, Aparência, Absorção, portão da
//! Perfuração, limite da morte, Tamanho, peso da Arte de força, jogada da
//! manobra, o Quase-Acerto, os textos do equipamento e a troca de ids de arma.
//!
//! Cada caso traz o nome da função do TypeScript, os argumentos e a saída. O
//! teste chama a função Rust do mesmo nome com os mesmos argumentos, escreve a
//! saída na forma do TypeScript e compara folha por folha.

mod common;

use centelha_data::bestiary::Size;
use centelha_engine::character::{self, DamageMode};
use centelha_engine::sheet::{
    self, Catalog, NearMissAttacker, NearMissOutcome, NearMissTarget, SheetTables,
};
use common::{compare, num, opt, repo};
use serde_json::{Value, json};

/// Um número dos argumentos (o `"-0"` do gerador é -0).
fn f(v: &Value) -> f64 {
    match v {
        Value::String(s) if s == "-0" => -0.0,
        v => v
            .as_f64()
            .unwrap_or_else(|| panic!("argumento não é número: {v}")),
    }
}

/// Um número opcional (`null` é ausente).
fn of(v: &Value) -> Option<f64> {
    (!v.is_null()).then(|| f(v))
}

fn s(v: &Value) -> &str {
    v.as_str()
        .unwrap_or_else(|| panic!("argumento não é texto: {v}"))
}

fn size(v: &Value) -> Size {
    serde_json::from_value(v.clone()).unwrap()
}

fn mode(v: &Value) -> DamageMode {
    DamageMode::from_id(s(v)).unwrap()
}

fn list(v: Option<&Value>) -> Vec<Value> {
    v.and_then(Value::as_array).cloned().unwrap_or_default()
}

fn outcome(r: &NearMissOutcome) -> Value {
    json!({
        "margem": num(r.margin), "dano": num(r.damage), "ignoraSoak": r.ignores_soak,
        "arma": { "classe": r.weapon.class, "bonus": num(r.weapon.bonus), "dano": num(r.weapon.damage) },
        "armadura": { "bonus": num(r.armor.bonus), "reducao": num(r.armor.reduction), "classes": r.armor.classes },
    })
}

fn tables() -> SheetTables {
    let content = repo().join("content");
    let read = |name: &str| std::fs::read_to_string(content.join("data").join(name)).unwrap();
    SheetTables::new(
        centelha_data::load_rules(&content).unwrap(),
        centelha_data::load_ancestries(&content).unwrap(),
        Catalog::from_json(
            &read("armas.json"),
            &read("armaduras.json"),
            &read("escudos.json"),
        )
        .unwrap(),
    )
}

/// A função Rust do caso, com a saída na forma do TypeScript.
fn run(t: &SheetTables, name: &str, a: &[Value]) -> Value {
    let r = t.rules();
    match name {
        "pisoXp" => opt(character::xp_floor(r, s(&a[0]))),
        "precoNivel" => opt(character::level_price(r, s(&a[0]), f(&a[1]))),
        "custoPontos" => opt(character::xp_cost(
            r,
            s(&a[0]),
            of(&a[1]),
            of(&a[2]).unwrap_or(0.0),
        )),
        "custoTecnica" => num(character::feat_cost(r, f(&a[0]))),
        "custoArte" => num(character::art_cost(r, f(&a[0]))),
        "custoEfeito" => num(character::effect_cost(r, f(&a[0]))),
        "custoEspecialidade" => num(character::specialty_cost(
            r,
            f(&a[0]),
            a[1].as_bool().unwrap_or(false),
        )),
        "aparenciaMod" => num(character::appearance_mod(r, f(&a[0]), of(&a[1]))),
        "soakNatural" => num(character::natural_soak(f(&a[0]), s(&a[1]))),
        "soak" => num(character::mode_soak(
            r,
            f(&a[0]),
            f(&a[1]),
            mode(&a[2]),
            of(&a[3]),
        )),
        "gatePerfuracaoAbre" => json!(character::pierce_gate_opens(
            r,
            mode(&a[0]),
            f(&a[1]),
            f(&a[2])
        )),
        "limiteDaMorte" => opt(character::death_limit(r, of(&a[0]), of(&a[1]))),
        "passouDoLimite" => json!(character::past_death_limit(
            r,
            of(&a[0]),
            of(&a[1]),
            of(&a[2])
        )),
        "porteDeRotulo" => json!(character::size_from_label(a[0].as_str()).id()),
        "modificadorPorte" => num(character::size_modifier(
            r,
            size(&a[0]),
            size(&a[1]),
            a[2].as_bool().unwrap(),
        )),
        "levantamentoDaArte" => opt(character::art_lifting(r, f(&a[0]))),
        "jogadaDaManobra" => {
            let c = &a[1];
            let n = |v: Option<&Value>| v.map_or(0.0, f);
            json!(character::maneuver_roll(
                a[0].as_str(),
                n(c["atributos"].get("forca")),
                n(c["atributos"].get("destreza")),
                n(c["habilidades"].get("briga")),
                n(c.get("centelha")),
            ))
        }
        "quaseAcertoDoEncontro" => {
            let (at, al) = (&a[0]["atacante"], &a[0]["alvo"]);
            let g = sheet::encounter_near_miss(
                NearMissAttacker {
                    weapon_bonus: f(&at["qaArmaBonus"]),
                    weapon_damage: f(&at["qaArmaDano"]),
                    spark: f(&at["centelha"]),
                },
                NearMissTarget {
                    armor_bonus: f(&al["qaArmaduraBonus"]),
                    armor_reduction: f(&al["qaArmaduraReducao"]),
                    spark: f(&al["centelha"]),
                },
            );
            json!({ "margem": num(g.margin), "dano": num(g.damage) })
        }
        "quaseAcerto" => {
            let at = &a[0];
            let weapon = at.get("arma").filter(|v| !v.is_null());
            outcome(&t.near_miss_between(
                weapon,
                at.get("dano").and_then(Value::as_str),
                at.get("centelha").and_then(of),
                &list(a[1].get("armaduras")),
            ))
        }
        "errouPor" => num(sheet::missed_by(f(&a[0]), f(&a[1]))),
        "raspa" => json!(sheet::grazes(f(&a[0]), f(&a[1]))),
        "saidaDoAtaque" => json!(sheet::attack_outcome(f(&a[0]), f(&a[1]), f(&a[2])).id()),
        "sinalNum" => json!(sheet::signed_number(f(&a[0]))),
        "danoStr" => json!(sheet::damage_text(&a[0])),
        "statsArma" => json!(sheet::weapon_stats(&a[0])),
        "statsEscudo" => json!(sheet::shield_stats(&a[0])),
        "statsArmadura" => json!(sheet::armor_stats(&a[0])),
        "migrarRefsDeArma" => {
            let mut sheet = a[0].clone();
            let custom: Vec<(String, String)> = list(Some(&a[1]))
                .iter()
                .map(|p| (s(&p[0]).to_owned(), s(&p[1]).to_owned()))
                .collect();
            let custom: Vec<(&str, &str)> = custom
                .iter()
                .map(|(x, y)| (x.as_str(), y.as_str()))
                .collect();
            let table: &[(&str, &str)] = if a[1].is_null() {
                &sheet::WEAPON_RENAMES
            } else {
                &custom
            };
            sheet::migrate_weapon_refs(&mut sheet, table);
            sheet
        }
        other => panic!("função sem par no Rust: {other}"),
    }
}

#[test]
fn character_matches_typescript() {
    let t = tables();
    let reference = common::read_reference("character.json");
    let cases = reference["cases"].as_array().unwrap();
    assert_eq!(cases.len() as u64, reference["count"].as_u64().unwrap());
    let functions = reference["counts"].as_object().unwrap().len();
    assert!(functions >= 28, "faltam funções na referência: {functions}");

    let mut diffs = Vec::new();
    let mut fields = 0;
    for (i, c) in cases.iter().enumerate() {
        let name = c["fn"].as_str().unwrap();
        let args = c["args"].as_array().unwrap();
        let got = run(&t, name, args);
        fields += compare(
            &format!("{i}:{name}{}", c["args"]),
            &got,
            &c["out"],
            &mut diffs,
        );
    }
    println!(
        "contas pequenas: {} casos em {functions} funções, {fields} campos comparados, {} diferenças",
        cases.len(),
        diffs.len()
    );
    assert!(
        diffs.is_empty(),
        "{} diferenças:\n{}",
        diffs.len(),
        diffs[..diffs.len().min(60)].join("\n")
    );
}
