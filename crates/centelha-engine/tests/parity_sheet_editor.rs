//! Paridade das contas da ficha interativa com o TypeScript
//! (tests/parity/sheet-editor.json, de tools/parity/sheet-editor.mjs): a ficha
//! nova, o XP gasto, os tetos e o piso, a troca de Raça, o recorte das
//! especialidades, o estado das Técnicas e os Efeitos ao alcance.
//!
//! As fichas de entrada estão em `inputs`, por id (a ficha depois do
//! `normalize` da fonte e o `massaEscolhida` que ele deixou). O teste para no
//! primeiro caso diferente. `SHEET_EDITOR_REFERENCE` aponta para uma cópia da
//! referência, no teste negativo.

mod common;

use centelha_data::{Technique, Weapon};
use centelha_engine::sheet::{
    ARMOR_EDIT_FIELDS, Catalog, DualWield, EditField, EditorCatalogs, EditorTables, HandRole,
    ItemKind, Movement, SHIELD_EDIT_FIELDS, SheetTables, WEAPON_EDIT_FIELDS, WEAPON_MORE_FIELDS,
    attribute_floor, carried_load, change_ancestry, clamp_specialties, current_load,
    dual_attack_from, dual_damage_text, edit_field_base, effects_in_reach, extra_lines,
    fresh_sheet, load_bands, load_speed_fraction, mass_chosen, normalize_sheet, overhead_lift,
    penalized_movement, penalized_speed, piece_for, pool_text, sheet_anatomy, sheet_flurry_cap,
    short_number, spent_xp, technique_status, throw_cap, throw_range, throw_reserve,
    throw_table_weights, trait_cap, weight_from_text, write_edit_field,
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
        EditorCatalogs {
            attributes: &raw("atributos.json"),
            skills: &raw("habilidades.json"),
            secondary: &raw("habilidades-secundarias.json"),
            virtues: &raw("virtudes.json"),
            backgrounds: &raw("antecedentes.json"),
            techniques: techniques.clone(),
            arts: centelha_data::load_arts(&content).unwrap(),
            effects: centelha_data::load_effects(&content).unwrap(),
        },
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

/// O que as contas leem além do caso: as tabelas e os catálogos.
struct Ctx {
    t: EditorTables,
    techniques: Vec<Technique>,
    weapons: Vec<Weapon>,
    extras: Value,
}

fn run(ctx: &Ctx, inputs: &Value, name: &str, a: &[Value], want: &Value) -> Value {
    let (t, techniques, weapons, extras) =
        (&ctx.t, &ctx.techniques[..], &ctx.weapons[..], &ctx.extras);
    let input = |i: usize| &inputs[a[i].as_str().unwrap()];
    match name {
        "fresh" => fresh_sheet(t),
        "massaEscolhida" => json!(mass_chosen(&input(0)["ficha"])),
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
        "tecnicas" => {
            // a ficha de uma Proeza grava só as pílulas daquela Proeza
            let id = a[0].as_str().unwrap();
            let only: Vec<Technique> = match id.strip_prefix("proeza-") {
                Some(p) => techniques.iter().filter(|t| t.path == p).cloned().collect(),
                None => techniques.to_vec(),
            };
            let names = technique_pills(&input(0)["ficha"], techniques);
            let wanted: std::collections::HashSet<&str> =
                only.iter().map(|t| t.id.as_str()).collect();
            Value::Object(
                names
                    .as_object()
                    .unwrap()
                    .iter()
                    .filter(|(k, _)| wanted.contains(k.as_str()))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            )
        }
        "efeitosAoAlcance" => Value::Array(
            effects_in_reach(t, &input(0)["ficha"])
                .into_iter()
                .map(|e| json!(e.id))
                .collect(),
        ),
        "deslocamento" => {
            let d = &a[1];
            let m = Movement {
                battle: f(&d["normal"]),
                dash: f(&d["arranque"]),
                run: f(&d["corrida"]),
                swim: f(&d["natacao"]),
                vertical_jump: f(&d["saltoVertical"]),
                standing_jump: f(&d["saltoHorizontalParado"]),
                running_jump: f(&d["saltoHorizontalCorrendo"]),
            };
            let p = penalized_movement(&m, f(&a[0]));
            json!({
                "batalha": num(p.battle), "arranque": num(p.dash), "corrida": num(p.run), "natacao": num(p.swim),
                "saltoVertical": num(p.vertical_jump), "saltoHorizontalParado": num(p.standing_jump),
                "saltoHorizontalCorrendo": num(p.running_jump),
            })
        }
        "anatomiaDaFicha" => {
            let x = sheet_anatomy(t.sheet_tables().rules(), weapons, &a[0]);
            let mut m = Map::new();
            m.insert("preparo".into(), num(x.windup));
            m.insert("golpes".into(), num(x.blows));
            m.insert("recuperacao".into(), num(x.recovery));
            m.insert("ciclo".into(), num(x.cycle));
            m.insert("offs".into(), Value::Array(x.offsets.iter().map(|v| num(*v)).collect()));
            m.insert("penDados".into(), Value::Array(x.dice_penalties.iter().map(|v| num(*v)).collect()));
            if let Some(w) = x.warning {
                m.insert("aviso".into(), json!(w));
            }
            Value::Object(m)
        }
        "linhasComExtra" => Value::Array(
            extra_lines(t.sheet_tables().rules(), extras, &a[0])
                .into_iter()
                .map(|e| {
                    json!({"id": e.id, "nome": e.name, "preparo": num(e.windup), "golpes": num(e.blows),
                           "recuperacao": num(e.recovery), "ciclo": num(e.cycle)})
                })
                .collect(),
        ),
        "tetoDaRajada" => num(sheet_flurry_cap(t.sheet_tables().rules(), weapons, &a[0])),
        "alcanceArremesso" => num(throw_range(t.sheet_tables().rules(), f(&a[0]), f(&a[1]), f(&a[2]))),
        "normalize" => {
            // os uid chegam na ordem em que a fonte os sorteou
            let mut uids = a[1]
                .as_array()
                .unwrap()
                .iter()
                .map(|u| u.as_str().unwrap().to_owned())
                .collect::<Vec<_>>()
                .into_iter();
            let n = normalize_sheet(t, &a[0], &mut || {
                uids.next().expect("o Rust pediu mais uid que a fonte")
            });
            assert!(uids.next().is_none(), "o Rust pediu menos uid que a fonte");
            let raised: Vec<Value> = n
                .raised
                .iter()
                .map(|r| json!({"nome": r.name, "de": r.from, "para": num(r.to)}))
                .collect();
            json!({"ficha": n.sheet, "subiu": raised, "massa": n.mass_chosen})
        }
        "forca" => {
            // a entrada da conta é o que a ficha leu (Força, Atletismo,
            // Arremesso, o peso máximo, a penalidade e as velocidades sem ela);
            // o resto sai do Rust e é comparado com o que a fonte calculou
            let o = &a[1];
            let r = t.sheet_tables().rules();
            let max = f(&o["maxKg"]);
            let tw = throw_table_weights(r, max);
            let reserve = throw_reserve(f(&o["forca"]), f(&o["atl"]), f(&o["arr"]));
            let v_base = penalized_speed(f(&o["corridaBruta"]), f(&o["pen"]));
            let v_swim = penalized_speed(f(&o["natacaoBruta"]), 0.0);
            let sample: Vec<f64> = o["amostra"].as_array().unwrap().iter().map(f).collect();
            let nums = |v: Vec<f64>| Value::Array(v.into_iter().map(num).collect());
            json!({
                "forca": o["forca"], "atl": o["atl"], "arr": o["arr"], "faa": num(reserve), "maxKg": o["maxKg"],
                "acimaKg": num(overhead_lift(r, max)), "tetoKg": num(throw_cap(r, max)), "pen": o["pen"],
                "penMov": num(centelha_engine::sheet::half_movement_penalty(f(&o["pen"]))),
                "corridaBruta": o["corridaBruta"], "vBase": num(v_base),
                "natacaoBruta": o["natacaoBruta"], "vBaseNatacao": num(v_swim),
                "bandas": Value::Array(load_bands(r, max).into_iter()
                    .map(|b| json!({"de": num(b.from), "ate": num(b.to), "nome": b.name})).collect()),
                "dist": nums(tw.iter().map(|w| throw_range(r, reserve, *w, max)).collect()),
                "tw": nums(tw),
                "amostra": o["amostra"],
                "velFrac": nums(sample.iter().map(|w| load_speed_fraction(r, *w, max, false)).collect()),
                "velFracAgua": nums(sample.iter().map(|w| load_speed_fraction(r, *w, max, true)).collect()),
                "vel": nums(sample.iter().map(|w| v_base * load_speed_fraction(r, *w, max, false)).collect()),
                "velNatacao": nums(sample.iter().map(|w| v_swim * load_speed_fraction(r, *w, max, true)).collect()),
            })
        }
        "carga" => {
            let catalog = t.sheet_tables().catalog();
            json!({"calculada": num(carried_load(catalog, &a[1])), "atual": num(current_load(catalog, &a[1]))})
        }
        "cargaEscrita" => {
            let sheet = json!({"cargaAtual": a[0], "bolsas": [], "arsenal": [], "equip": {"armaduras": []}});
            num(current_load(t.sheet_tables().catalog(), &sheet))
        }
        "numPeso" => num(weight_from_text(Some(&a[0]))),
        "r1n" => json!(short_number(f(&a[0]))),
        "pecaNova" => {
            let mut sheet = json!({});
            if !a[0].is_null() {
                sheet["arsenal"] = a[0].clone();
            }
            // o uid é sorteado pela fonte: o Rust recebe o da peça que ela deu
            let uid = want["peca"]["uid"].as_str().unwrap_or_default();
            let peca = piece_for(&mut sheet, a[1].as_str().unwrap(), uid);
            json!({"arsenal": sheet["arsenal"], "peca": peca})
        }
        "baseCampo" => {
            let catalog = t.sheet_tables().catalog();
            let id = a[1].as_str().unwrap();
            let (item, fields): (Option<&Value>, Vec<&EditField>) = match a[0].as_str().unwrap() {
                "arma" => (
                    catalog.weapon(id),
                    WEAPON_EDIT_FIELDS.iter().chain(WEAPON_MORE_FIELDS.iter()).collect(),
                ),
                "escudo" => (catalog.shield(id), SHIELD_EDIT_FIELDS.iter().collect()),
                _ => (catalog.armor(id), ARMOR_EDIT_FIELDS.iter().collect()),
            };
            let f = fields.into_iter().find(|f| f.key == a[2]).unwrap();
            edit_field_base(item, f).unwrap_or(Value::Null)
        }
        "escreveCampo" => {
            let catalog = t.sheet_tables().catalog();
            let (kind, fields): (_, Vec<&EditField>) = match a[0].as_str().unwrap() {
                "arma" => (
                    ItemKind::Weapon,
                    WEAPON_EDIT_FIELDS.iter().chain(WEAPON_MORE_FIELDS.iter()).collect(),
                ),
                "escudo" => (ItemKind::Shield, SHIELD_EDIT_FIELDS.iter().collect()),
                _ => (ItemKind::Armor, ARMOR_EDIT_FIELDS.iter().collect()),
            };
            let mut item = catalog.catalog_copy(kind, a[1].as_str().unwrap()).unwrap();
            let f = fields.into_iter().find(|f| f.key == a[2]).unwrap();
            write_edit_field(&mut item, f, Some(a[3].clone()));
            item
        }
        "restaurarItem" => {
            let mut p = a[0].clone();
            t.sheet_tables().catalog().restore_item(&mut p);
            p
        }
        "ehDuasMaos" => json!(t.sheet_tables().catalog().two_handed(&a[0])),
        "marcarUso" => {
            let mut sheet = a[0].clone();
            let role = match a[2].as_str().unwrap() {
                "habil" => HandRole::Main,
                "inabil" => HandRole::Off,
                _ => HandRole::None,
            };
            t.sheet_tables()
                .catalog()
                .mark_hand_use(&mut sheet, a[1].as_str().unwrap(), role);
            sheet["conjuntos"].clone()
        }
        "ataqueDuplo" => {
            let x = &a[0];
            let d = &x["dupla"];
            let dual = DualWield {
                main_bonus: f(&d["habilAp"]),
                off_dice: f(&d["inabilDados"]),
                off_roll_bonus: f(&d["inabilBonus"]),
                off_flat: f(&d["inabilFlat"]),
                off_die: d.get("inabilDado").cloned(),
                off_bonus: f(&d["inabilAp"]),
                off_penalty: f(&d["inabilPen"]),
            };
            let main_die = x.get("dado").filter(|v| !v.is_null()).cloned();
            let r = dual_attack_from(f(&x["dados"]), f(&x["bonus"]), f(&x["flat"]), main_die, &dual);
            // a marcação é da página; a conta e o texto são do motor
            json!(format!(
                "<div class=\"conj-dupla\"><b>Ataque duplo</b> hábil \u{2212}1d6: {} ({}) · inábil \u{2212}{}d6: {} ({}) <span class=\"muted\">· guarda \u{2212}4 até seu turno</span></div>",
                pool_text(r.main.dice, r.main.roll_bonus, r.main.flat),
                dual_damage_text(r.main.damage_die.as_ref(), r.main.damage_bonus),
                centelha_engine::js::number_to_string(dual.off_penalty),
                pool_text(r.off.dice, r.off.roll_bonus, r.off.flat),
                dual_damage_text(r.off.damage_die.as_ref(), r.off.damage_bonus),
            ))
        }
        other => panic!("função sem par no Rust: {other}"),
    }
}

#[test]
fn sheet_editor_matches_typescript() {
    let (t, techniques) = tables();
    let content = repo().join("content");
    let ctx = Ctx {
        t,
        techniques,
        weapons: centelha_data::load_weapons(&content).unwrap(),
        extras: serde_json::from_str(
            &std::fs::read_to_string(content.join("data/armas-extras.json")).unwrap(),
        )
        .unwrap(),
    };
    // `SHEET_EDITOR_REFERENCE` troca a referência por uma cópia (o teste negativo)
    let r: Value = match std::env::var("SHEET_EDITOR_REFERENCE") {
        Ok(p) => serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap(),
        Err(_) => common::read_reference("sheet-editor.json"),
    };
    let cases = r["cases"].as_array().unwrap();
    assert_eq!(cases.len() as u64, r["count"].as_u64().unwrap());
    let mut fields = 0;
    for (i, c) in cases.iter().enumerate() {
        let name = c["fn"].as_str().unwrap();
        let got = run(
            &ctx,
            &r["inputs"],
            name,
            c["args"].as_array().unwrap(),
            &c["out"],
        );
        let mut diffs = Vec::new();
        fields += compare(
            &format!("{i}:{name}{}", c["args"]),
            &got,
            &c["out"],
            &mut diffs,
        );
        // a ordem das chaves da ficha faz parte da paridade (ADR 0004): o
        // `compare` não a olha, então a ficha do `normalize` é conferida à parte
        if name == "normalize" {
            order_diffs(&got["ficha"], &c["out"]["ficha"], "ficha", &mut diffs);
        }
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

/// As chaves de cada objeto na mesma ordem (a da fonte), em toda a ficha.
fn order_diffs(got: &Value, want: &Value, path: &str, diffs: &mut Vec<String>) {
    match (got, want) {
        (Value::Object(g), Value::Object(w)) => {
            let gk: Vec<&String> = g.keys().collect();
            let wk: Vec<&String> = w.keys().collect();
            if gk != wk {
                diffs.push(format!(
                    "{path}: chaves em outra ordem: {gk:?} contra {wk:?}"
                ));
                return;
            }
            for (k, v) in g {
                order_diffs(v, &w[k], &format!("{path}.{k}"), diffs);
            }
        }
        (Value::Array(g), Value::Array(w)) => {
            for (i, (a, b)) in g.iter().zip(w).enumerate() {
                order_diffs(a, b, &format!("{path}[{i}]"), diffs);
            }
        }
        _ => {}
    }
}
