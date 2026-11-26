//! Paridade da ficha do jogador com o TypeScript (tests/parity/sheets.json).
//!
//! Para cada ficha do lote, seis saídas: a forma de leitura
//! (`fichaParaLeitura`), a gravada (`fichaParaGravar`), os números
//! (`numerosPersonagem`), o resumo de combate (`combatePersonagem`), o resumo
//! da mesa (`resumoFicha`) e o recorte do banco (`resumoParaBanco`). A saída
//! tipada do Rust é escrita na forma do objeto do TypeScript e comparada folha
//! por folha, como nas criaturas.

mod common;

use centelha_engine::creature::Soak;
use centelha_engine::sheet::{
    ArmorStack, Catalog, CombatSummary, DbSummary, HandItem, HandKind, HandSet, NearMiss,
    SheetNumbers, SheetSummary, SheetTables, combat_summary, db_summary, for_reading, for_saving,
    sheet_numbers, sheet_summary,
};
use common::{compare, num, opt, repo};
use serde_json::{Map, Value, json};

fn soak(s: &Soak) -> Value {
    json!({ "impacto": num(s.impact), "corte": num(s.cut), "perfuracao": num(s.pierce) })
}

fn insert_opt(m: &mut Map<String, Value>, k: &str, v: Option<Value>) {
    if let Some(v) = v {
        m.insert(k.into(), v);
    }
}

fn hand(h: &HandItem) -> Value {
    let mut m = Map::new();
    m.insert("kind".into(), json!(h.kind.id()));
    insert_opt(&mut m, "nome", h.name.clone().map(Value::String));
    m.insert("def".into(), num(h.defense));
    m.insert("pen".into(), num(h.penalty));
    match h.kind {
        HandKind::Nothing => {}
        HandKind::Shield => {
            insert_opt(&mut m, "vsProjetilRapido", h.vs_fast_projectile.clone());
            insert_opt(&mut m, "base", h.base.clone());
            insert_opt(&mut m, "s", h.shield.clone());
        }
        HandKind::Weapon | HandKind::Custom => {
            insert_opt(&mut m, "w", h.weapon.clone());
            insert_opt(&mut m, "base", h.base.clone());
        }
    }
    Value::Object(m)
}

fn hand_set(a: &HandSet) -> Value {
    let dual = a.dual.as_ref().map_or(Value::Null, |d| {
        let mut m = Map::new();
        m.insert("habilAp".into(), num(d.main_bonus));
        m.insert("inabilDados".into(), num(d.off_dice));
        m.insert("inabilBonus".into(), num(d.off_roll_bonus));
        m.insert("inabilFlat".into(), num(d.off_flat));
        insert_opt(&mut m, "inabilDado", d.off_die.clone());
        m.insert("inabilAp".into(), num(d.off_bonus));
        m.insert("inabilPen".into(), num(d.off_penalty));
        Value::Object(m)
    });
    json!({
        "habil": hand(&a.main_hand), "inabil": hand(&a.off_hand), "atk": a.attack_weapon,
        "dados": num(a.dice), "bonus": num(a.roll_bonus), "flat": num(a.flat), "dist": a.ranged,
        "versoes": a.versions.iter().map(|v| json!({ "rot": v.label, "ap": num(v.bonus) })).collect::<Vec<_>>(),
        "reqForca": num(a.strength_missing), "dupla": dual, "defSum": num(a.block),
        "penSum": num(a.penalty), "armorPen": num(a.armor_penalty), "forcaInvestida": num(a.charge_strength),
    })
}

fn armor(a: &ArmorStack) -> Value {
    json!({ "soak": soak(&a.soak), "resistPerf": num(a.pierce_level), "penalidade": num(a.penalty) })
}

fn numbers(n: &SheetNumbers) -> Value {
    json!({
        "pv": num(n.hp), "pesoErguido": num(n.lifted_weight), "esquiva": num(n.dodge),
        "bloqueio": num(n.block), "defesaSocial": num(n.social_defense),
        "defesaMental": num(n.mental_defense), "energia": num(n.energy), "mana": num(n.mana),
        "iniciativa": { "dado": num(n.initiative.die), "bonus": num(n.initiative.bonus), "str": n.initiative.text },
        "absorcao": soak(&n.soak), "act": hand_set(&n.hands), "arm": armor(&n.armor),
    })
}

fn near_miss(q: &NearMiss) -> Value {
    json!({
        "armaBonus": num(q.weapon_bonus), "armaDano": num(q.weapon_damage),
        "armaClasse": q.weapon_class, "armaduraBonus": num(q.armor_bonus),
        "armaduraReducao": num(q.armor_reduction), "armaduraClasses": q.armor_classes,
    })
}

fn combat(c: &CombatSummary) -> Value {
    let mut m = Map::new();
    insert_opt(&mut m, "arma", c.weapon_name.clone().map(Value::String));
    let rest = json!({
        "ataque": c.attack, "dano": c.damage, "defesa": num(c.dodge),
        "defesaMental": num(c.mental_defense), "soak": soak(&c.soak),
        "resistPerf": num(c.pierce_level), "perfArma": opt(c.weapon_pierce),
        "qa": near_miss(&c.near_miss),
        "passo": { "batalha": num(c.step.battle), "arranque": num(c.step.dash), "corrida": num(c.step.run) },
        "atributos": c.attributes, "pericias": c.skills, "velocidade": opt(c.speed),
        "classe": c.class.clone().unwrap_or(Value::Null), "forcaInvestida": num(c.charge_strength),
    });
    if let Value::Object(r) = rest {
        m.extend(r);
    }
    Value::Object(m)
}

fn summary(r: &SheetSummary) -> Value {
    json!({
        "nome": r.name, "conceito": r.concept, "jogador": r.player, "raca": r.ancestry,
        "centelha": num(r.spark), "vontade": num(r.willpower), "aparencia": num(r.appearance),
        "attrs": r.attributes, "skills": r.skills, "skills2": r.secondary_skills, "virtudes": r.virtues,
        "pv": num(r.hp), "energia": num(r.energy), "mana": num(r.mana),
        "defEsquiva": num(r.dodge), "defBloqueio": num(r.block), "defSocial": num(r.social_defense),
        "defMental": num(r.mental_defense), "soak": soak(&r.soak), "resistPerf": num(r.pierce_level),
        "penFisica": num(r.physical_penalty), "iniciativa": r.initiative,
        "iniciativaMedia": num(r.initiative_average), "arma": r.weapon, "ataque": r.attack, "dano": r.damage,
        "passivos": r.passives.iter().map(|p| json!({
            "id": p.id, "nome": p.name, "dados": num(p.dice), "bonus": num(p.bonus),
            "valor": num(p.value), "str": p.text,
        })).collect::<Vec<_>>(),
        "tecnicas": r.feats,
        "artes": r.arts.iter().map(|a| json!({ "id": a.id, "nivel": a.level })).collect::<Vec<_>>(),
        "armaduras": r.armors, "qa": near_miss(&r.near_miss),
    })
}

fn db(b: &DbSummary) -> Value {
    json!({
        "pv": num(b.hp), "energia": num(b.energy), "mana": num(b.mana), "arma": b.weapon,
        "ataque": b.attack, "dano": b.damage, "defesa": num(b.dodge), "defBloqueio": num(b.block),
        "defesaSocial": num(b.social_defense), "defesaMental": num(b.mental_defense),
        "soak": soak(&b.soak), "resistPerf": num(b.pierce_level), "iniciativa": b.initiative,
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

#[test]
fn sheets_match_typescript() {
    let t = tables();
    let reference = common::read_reference("sheets.json");
    let sheets = reference["sheets"].as_array().unwrap();
    assert_eq!(sheets.len() as u64, reference["count"].as_u64().unwrap());
    assert!(sheets.len() > 150);

    let mut diffs = Vec::new();
    let mut fields = 0;
    for s in sheets {
        let id = s["id"].as_str().unwrap();
        let input = &s["ficha"];
        let r = sheet_summary(&t, input);
        let outputs = [
            ("leitura", for_reading(t.catalog(), input)),
            ("gravar", for_saving(t.catalog(), input)),
            ("numeros", numbers(&sheet_numbers(&t, input))),
            ("combate", combat(&combat_summary(&t, input))),
            ("banco", db(&db_summary(&r))),
            ("resumo", summary(&r)),
        ];
        for (k, got) in outputs {
            fields += compare(&format!("{id}:{k}"), &got, &s[k], &mut diffs);
        }
    }
    println!(
        "fichas: {} fichas, 6 saídas cada, {fields} campos comparados, {} diferenças",
        sheets.len(),
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
