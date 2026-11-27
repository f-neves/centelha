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
use centelha_engine::sheet::{
    SpecialtyScope, TraitKind, add_defense_specialty, add_named_background, add_specialty,
    click_dot, close_specialties, effective_appearance, free_secondary_name, is_free_secondary,
    named_instance, open_specialties, racial_appearance_mod, racial_cap_bonus, racial_floor_bonus,
    remove_defense_specialty, remove_named_background, remove_specialty, rename_named_background,
    rename_specialty, set_body_mass, set_budget, set_free_secondary_name, set_identity_field,
    set_specialty_level, set_trait, specialty_count, step_trait, toggle_effect, toggle_technique,
    trait_floor, trait_value,
};
use centelha_engine::sheet::{
    absorption_panel, add_armor, add_bag_row, armor_piece, armor_piece_numbers, bag_weight,
    piece_by_key, piece_weight, remove_armor, remove_bag_row, rename_armor, rename_bag,
    set_bag_cell, set_bag_total, toggle_armor_worn, worn_armor_pieces,
};
use centelha_engine::sheet::{
    activate_set, add_set, add_to_arsenal, clean_arsenal, custom_piece_note, differs_from_origin,
    edit_piece_field, edit_target, is_custom_piece, is_custom_ref, is_two_handed_item, item_fields,
    piece_name, piece_role, remove_from_arsenal, remove_set, rename_arsenal_piece, reorder,
    reset_piece, same_field, set_hand_piece, show_field,
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
        other => run_traits(ctx, inputs, other, a, want)
            .or_else(|| run_arsenal(ctx, inputs, other, a, want))
            .or_else(|| run_gear(ctx, inputs, other, a))
            .unwrap_or_else(|| panic!("função sem par no Rust: {other}")),
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
        // o mesmo nos casos dos traços: o que a operação deixou, em cada objeto
        if run_traits_names().contains(&name) {
            // (a ordem das chaves do envelope do caso é do gerador; a dos valores, da ficha)
            if let Some(o) = c["out"].as_object() {
                for (k, v) in o {
                    order_diffs(&got[k.as_str()], v, k, &mut diffs);
                }
            }
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

// ------------------------------------------------------------ os traços

/// As chaves de topo que mudaram, na ordem da ficha de depois (o `mudou` do
/// gerador: o texto JSON de cada uma, antes e depois).
fn changed_keys(before: &Value, after: &Value) -> Vec<Value> {
    let text = |v: Option<&Value>| v.map(|v| serde_json::to_string(v).unwrap());
    after
        .as_object()
        .unwrap()
        .keys()
        .filter(|k| text(before.get(k.as_str())) != text(after.get(k.as_str())))
        .map(|k| json!(k))
        .collect()
}

/// O que a trilha guarda depois da operação (o `resumo` do gerador).
fn trait_summary(sheet: &Value, kind: TraitKind, key: &str) -> Map<String, Value> {
    let get = |group: &str| {
        sheet
            .get(group)
            .and_then(|g| g.get(key))
            .cloned()
            .unwrap_or(Value::Null)
    };
    let mut out = Map::new();
    match kind {
        TraitKind::NamedBackground => {
            let v = named_instance(sheet, key)
                .and_then(|i| i.get("v"))
                .cloned()
                .unwrap_or(Value::Null);
            out.insert("v".into(), v);
        }
        TraitKind::Spark | TraitKind::Willpower | TraitKind::Appearance => {
            let k = kind.source_name();
            out.insert("v".into(), sheet.get(k).cloned().unwrap_or(Value::Null));
        }
        _ => {
            let group = match kind {
                TraitKind::Attribute => "attrs",
                TraitKind::Skill => "skills",
                TraitKind::Secondary => "skills2",
                TraitKind::Virtue => "virtues",
                TraitKind::Art => "arte",
                _ => "ante",
            };
            out.insert("v".into(), get(group));
            if kind == TraitKind::Skill {
                out.insert("spec".into(), get("spec"));
            }
            if kind == TraitKind::Secondary {
                out.insert("spec2".into(), get("spec2"));
            }
        }
    }
    out
}

/// Uma operação sobre a ficha de entrada: devolve o que ela deixou e as
/// chaves de topo que mudaram.
fn with_changes(
    sheet: Value,
    op: impl FnOnce(&mut Value),
    out: impl FnOnce(&Value) -> Map<String, Value>,
) -> Value {
    let before = sheet.clone();
    let mut after = sheet;
    op(&mut after);
    let mut o = out(&after);
    o.insert("mudou".into(), Value::Array(changed_keys(&before, &after)));
    Value::Object(o)
}

/// Os casos da fatia 6 (mexer nos traços). `None` quando o nome não é daqui.
fn run_traits(ctx: &Ctx, inputs: &Value, name: &str, a: &[Value], want: &Value) -> Option<Value> {
    let t = &ctx.t;
    let sheet = |i: usize| inputs[a[i].as_str().unwrap()]["ficha"].clone();
    let kind = |i: usize| TraitKind::from_source_name(a[i].as_str().unwrap()).unwrap();
    let text = |i: usize| a[i].as_str().unwrap();
    let f = |i: usize| a[i].as_f64().unwrap();
    let scope = |i: usize| match text(i) {
        "p" => SpecialtyScope::Primary,
        _ => SpecialtyScope::Secondary,
    };
    let out = match name {
        "aplicarValor" | "clicarBolinha" | "teclaSeta" => {
            let (k, key) = (kind(1), text(2));
            with_changes(
                sheet(0),
                |s| match name {
                    "aplicarValor" => set_trait(t, s, k, key, f(3)),
                    "clicarBolinha" => click_dot(t, s, k, key, f(3)),
                    _ => match text(3) {
                        "ArrowRight" | "ArrowUp" => step_trait(t, s, k, key, 1.0),
                        "ArrowLeft" | "ArrowDown" => step_trait(t, s, k, key, -1.0),
                        _ => {}
                    },
                },
                |s| trait_summary(s, k, key),
            )
        }
        "pisoDe" => num(trait_floor(t, &sheet(0), kind(1), text(2))),
        "valOf" => num(trait_value(&sheet(0), kind(1), text(2))),
        "tetoRacial" => {
            let s = sheet(0);
            json!({"teto": num(racial_cap_bonus(t, &s, text(1))), "piso": num(racial_floor_bonus(t, &s, text(1)))})
        }
        "racApMod" => num(racial_appearance_mod(t, &sheet(0))),
        "apEfetiva" => num(effective_appearance(t, &sheet(0), f(1))),
        "racApModDaRaca" => num(racial_appearance_mod(t, &json!({"raca": text(0)}))),
        "apEfetivaDaRaca" => num(effective_appearance(t, &json!({"raca": text(0)}), f(1))),
        "ehSecLivre" => json!(is_free_secondary(text(0))),
        "nomeLivre" => {
            let s = json!({"livreNome": {"livre-1": "  Cantoria \u{a0}", "livre-2": "", "livre-3": "Tricô"}});
            json!(free_secondary_name(&s, text(0)))
        }
        "specCount" => num(specialty_count(&a[0])),
        "anteInst" => named_instance(&sheet(0), text(1))
            .cloned()
            .unwrap_or(Value::Null),
        "anteAdd" => {
            let uid = want["uids"][0].as_str().unwrap().to_owned();
            let id = text(1);
            let mut o = with_changes(
                sheet(0),
                |s| add_named_background(s, id, &uid),
                |s| {
                    let mut m = Map::new();
                    m.insert(
                        "lista".into(),
                        s["anteNom"].get(id).cloned().unwrap_or(Value::Null),
                    );
                    m
                },
            );
            o["uids"] = json!([uid]);
            o
        }
        "anteRm" | "anteNome" => {
            let key = text(1);
            let id = key.split('~').next().unwrap().to_owned();
            with_changes(
                sheet(0),
                |s| {
                    if name == "anteRm" {
                        remove_named_background(s, key)
                    } else {
                        rename_named_background(s, key, text(2))
                    }
                },
                |s| {
                    let mut m = Map::new();
                    m.insert(
                        "lista".into(),
                        s["anteNom"].get(&id).cloned().unwrap_or(Value::Null),
                    );
                    m
                },
            )
        }
        "defEspMais" => with_changes(
            sheet(0),
            |s| add_defense_specialty(s, text(1), text(2), text(3)),
            |s| {
                let mut m = Map::new();
                m.insert(
                    "lista".into(),
                    s["defSpec"].get(text(1)).cloned().unwrap_or(Value::Null),
                );
                m
            },
        ),
        "defEspTira" => with_changes(
            sheet(0),
            |s| remove_defense_specialty(s, text(1), a[2].as_u64().unwrap() as usize),
            |s| {
                let mut m = Map::new();
                m.insert(
                    "lista".into(),
                    s["defSpec"].get(text(1)).cloned().unwrap_or(Value::Null),
                );
                m
            },
        ),
        "tecnicaClique" => {
            let tech = ctx.techniques.iter().find(|x| x.id == text(1)).unwrap();
            with_changes(
                sheet(0),
                |s| {
                    toggle_technique(s, tech);
                },
                |s| {
                    let mut m = Map::new();
                    m.insert(
                        "v".into(),
                        s["tech"].get(&tech.id).cloned().unwrap_or(Value::Null),
                    );
                    m
                },
            )
        }
        "efeitoClique" => with_changes(
            sheet(0),
            |s| toggle_effect(s, text(1)),
            |s| {
                let mut m = Map::new();
                m.insert(
                    "v".into(),
                    s["efeito"].get(text(1)).cloned().unwrap_or(Value::Null),
                );
                m
            },
        ),
        "massaCampo" | "identidade" => {
            let mut s = sheet(0);
            let before_flag = if name == "massaCampo" {
                a[2].as_bool().unwrap()
            } else {
                false
            };
            if name == "massaCampo" && before_flag {
                s["corpo"] = json!({"massa": 80});
                s["id"] = json!({"peso": "80 kg"});
            } else if name == "identidade" {
                s["corpo"] = json!({"massa": 55});
            }
            let mut flag = before_flag;
            let mut o = with_changes(
                s,
                |s| {
                    let r = if name == "massaCampo" {
                        let blank = text(1).trim().is_empty();
                        let n = if blank {
                            f64::NAN
                        } else {
                            text(1).parse::<f64>().unwrap_or(f64::NAN)
                        };
                        set_body_mass(s, n, blank)
                    } else {
                        set_identity_field(s, text(1), text(2))
                    };
                    if let Some(v) = r {
                        flag = v;
                    }
                },
                |s| {
                    let mut m = Map::new();
                    m.insert(
                        "corpo".into(),
                        s.get("corpo").cloned().unwrap_or(Value::Null),
                    );
                    m.insert("id".into(), s.get("id").cloned().unwrap_or(Value::Null));
                    m
                },
            );
            // a ordem das chaves do objeto de saída: corpo, id, massaEscolhida, mudou
            let mudou = o["mudou"].take();
            o["massaEscolhida"] = json!(flag);
            o["mudou"] = mudou;
            o
        }
        "orcamento" => with_changes(
            sheet(0),
            |s| set_budget(s, text(1)),
            |s| {
                let mut m = Map::new();
                m.insert(
                    "budget".into(),
                    s.get("budget").cloned().unwrap_or(Value::Null),
                );
                m
            },
        ),
        "livreNome" => with_changes(
            sheet(0),
            |s| set_free_secondary_name(s, text(1), text(2)),
            |s| {
                let mut m = Map::new();
                m.insert(
                    "livreNome".into(),
                    s.get("livreNome").cloned().unwrap_or(Value::Null),
                );
                m
            },
        ),
        "especialidade" => {
            let (sc, key) = (scope(1), text(2));
            let (skills, spec) = if text(1) == "p" {
                ("skills", "spec")
            } else {
                ("skills2", "spec2")
            };
            let mut s = sheet(0);
            s[skills][key] = a[3].clone();
            match &a[4] {
                Value::Null => {
                    s[spec].as_object_mut().unwrap().shift_remove(key);
                }
                list => s[spec][key] = list.clone(),
            }
            let op = a[5].as_array().unwrap();
            let name = op[0].as_str().unwrap();
            let pre = name != "abrir" && name != "fechar";
            let mut opened = Value::Null;
            let mut o = with_changes(
                s,
                |s| {
                    if pre {
                        open_specialties(s, sc, key);
                    }
                    match name {
                        "abrir" => opened = json!(open_specialties(s, sc, key)),
                        "quad" => set_specialty_level(
                            s,
                            sc,
                            key,
                            op[1].as_u64().unwrap() as usize,
                            op[2].as_f64().unwrap(),
                        ),
                        "nome" => rename_specialty(
                            s,
                            sc,
                            key,
                            op[1].as_u64().unwrap() as usize,
                            op[2].as_str().unwrap(),
                        ),
                        "tirar" => remove_specialty(s, sc, key, op[1].as_u64().unwrap() as usize),
                        "mais" => add_specialty(s, sc, key),
                        _ => close_specialties(s, sc, key),
                    }
                },
                |s| {
                    let mut m = Map::new();
                    m.insert(
                        "lista".into(),
                        s[spec].get(key).cloned().unwrap_or(Value::Null),
                    );
                    m
                },
            );
            let mudou = o["mudou"].take();
            o["aberto"] = opened;
            o["mudou"] = mudou;
            o
        }
        _ => return None,
    };
    Some(out)
}

/// Os casos da fatia 6 (os que passam por `run_traits`).
fn run_traits_names() -> &'static [&'static str] {
    &[
        "armaduraAdd",
        "armaduraNome",
        "armaduraVestir",
        "armaduraRm",
        "bolsaNome",
        "bolsaTotal",
        "bolsaCelula",
        "bolsaAdd",
        "bolsaRm",
        "arsenalAdd",
        "arsenalRm",
        "arsenalUso",
        "arsenalNome",
        "conjuntoMao",
        "conjuntoUso",
        "conjuntoRm",
        "conjuntoAdd",
        "ajusteCampo",
        "ajusteReset",
        "limparArsenal",
        "reordenar",
        "aplicarValor",
        "clicarBolinha",
        "teclaSeta",
        "anteAdd",
        "anteRm",
        "anteNome",
        "defEspMais",
        "defEspTira",
        "tecnicaClique",
        "efeitoClique",
        "massaCampo",
        "identidade",
        "orcamento",
        "livreNome",
        "especialidade",
        "anteInst",
        "tetoRacial",
        "pisoDe",
        "valOf",
        "racApMod",
        "apEfetiva",
        "specCount",
    ]
}

// ------------------------------------------------- arsenal, conjuntos, ajuste

/// Os casos da fatia 7. `None` quando o nome não é daqui.
fn run_arsenal(ctx: &Ctx, inputs: &Value, name: &str, a: &[Value], want: &Value) -> Option<Value> {
    let t = &ctx.t;
    let sheet = |i: usize| inputs[a[i].as_str().unwrap()]["ficha"].clone();
    let text = |i: usize| a[i].as_str().unwrap();
    let index = |i: usize| a[i].as_u64().unwrap() as usize;
    // a peça do arsenal pelo uid
    let piece = |s: &Value, uid: &str| {
        s["arsenal"]
            .as_array()
            .and_then(|l| l.iter().find(|p| p["uid"] == uid))
            .cloned()
            .unwrap()
    };
    let cat = t.sheet_tables().catalog();
    let lists = |s: &Value| {
        let mut m = Map::new();
        m.insert(
            "arsenal".into(),
            s.get("arsenal").cloned().unwrap_or(Value::Null),
        );
        m.insert(
            "conjuntos".into(),
            s.get("conjuntos").cloned().unwrap_or(Value::Null),
        );
        m
    };
    let out = match name {
        "ehLivre" => json!(is_custom_piece(Some(&piece(&sheet(0), text(1))))),
        "nomePeca" => match piece_name(t, &piece(&sheet(0), text(1))) {
            Some(n) => json!(n),
            None => Value::Null,
        },
        "papelDaPeca" => json!(match piece_role(&sheet(0), text(1)) {
            centelha_engine::sheet::HandRole::None => "nada",
            centelha_engine::sheet::HandRole::Main => "habil",
            centelha_engine::sheet::HandRole::Off => "inabil",
        }),
        "it2H" => {
            let p = piece(&sheet(0), text(1));
            json!(is_two_handed_item(&centelha_engine::sheet::hand_item(
                t.sheet_tables(),
                Some(&p)
            )))
        }
        "camposItem" => {
            let p = piece(&sheet(0), text(1));
            let kind = centelha_engine::sheet::hand_item(t.sheet_tables(), Some(&p)).kind;
            json!(item_fields(kind).iter().map(|f| f.key).collect::<Vec<_>>())
        }
        "difereDaOrigem" => {
            let p = piece(&sheet(0), text(1));
            let kind = centelha_engine::sheet::hand_item(t.sheet_tables(), Some(&p)).kind;
            json!(differs_from_origin(cat, &p, &item_fields(kind)))
        }
        "alvoMod" => match edit_target(t, &sheet(0), text(1)) {
            Some(target) => {
                json!({"campos": target.fields.iter().map(|f| f.key).collect::<Vec<_>>()})
            }
            None => Value::Null,
        },
        "limparArsenal" => with_changes(
            sheet(0),
            |s| clean_arsenal(t, s),
            |s| {
                let mut m = Map::new();
                m.insert(
                    "arsenal".into(),
                    s.get("arsenal").cloned().unwrap_or(Value::Null),
                );
                m
            },
        ),
        "refLivre" => json!(is_custom_ref(Some(&a[0]))),
        "improvisado" => json!(format!(
            "<span class=\"eq-improv-nota muted\">{}</span>",
            custom_piece_note(a[0].as_bool().unwrap())
        )),
        "mostraCampo" => {
            let f = all_fields().into_iter().find(|f| f.key == text(0)).unwrap();
            // o valor que a fonte devolve, `?? null` (o `undefined` do JSON some)
            show_field(&f, Some(&a[1]))
        }
        "igualCampo" => json!(same_field(Some(&a[0]), Some(&a[1]))),
        "reordenar" => {
            let uids: Vec<String> = a[1]
                .as_array()
                .unwrap()
                .iter()
                .map(|u| u.as_str().unwrap().to_owned())
                .collect();
            Value::Array(reorder(a[0].as_array().unwrap(), &uids))
        }
        "arsenalAdd" => with_changes(
            sheet(0),
            |s| add_to_arsenal(s, text(1), text(2)),
            |s| lists(s),
        ),
        "arsenalRm" => with_changes(
            sheet(0),
            |s| remove_from_arsenal(t, s, text(1)),
            |s| lists(s),
        ),
        "arsenalUso" => with_changes(
            sheet(0),
            |s| {
                let role = match text(2) {
                    "habil" => centelha_engine::sheet::HandRole::Main,
                    "inabil" => centelha_engine::sheet::HandRole::Off,
                    _ => centelha_engine::sheet::HandRole::None,
                };
                cat.mark_hand_use(s, text(1), role);
            },
            |s| lists(s),
        ),
        "arsenalNome" => with_changes(
            sheet(0),
            |s| rename_arsenal_piece(t, s, text(1), text(2)),
            |s| lists(s),
        ),
        "conjuntoMao" => {
            let uid = want["uids"][0].as_str().unwrap_or("").to_owned();
            let mut used = false;
            let mut o = with_changes(
                sheet(0),
                |s| {
                    set_hand_piece(t, s, index(1), text(2), text(3), &uid);
                    used = !uid.is_empty()
                        && s["arsenal"]
                            .as_array()
                            .is_some_and(|l| l.iter().any(|p| p["uid"] == uid.as_str()));
                },
                |s| lists(s),
            );
            o["uids"] = if used { json!([uid]) } else { json!([]) };
            o
        }
        "conjuntoUso" => with_changes(sheet(0), |s| activate_set(s, index(1)), |s| lists(s)),
        "conjuntoRm" => with_changes(sheet(0), |s| remove_set(s, index(1)), |s| lists(s)),
        "conjuntoAdd" => with_changes(sheet(0), add_set, |s| lists(s)),
        "ajusteCampo" => {
            let key = text(1);
            let (piece_key, _) = key.rsplit_once(':').unwrap();
            let (group, uid) = piece_key.split_once(':').unwrap();
            let mut o = with_changes(
                sheet(0),
                |s| edit_piece_field(t, s, key, text(2)),
                |_| Map::new(),
            );
            // o item da peça editada (a fonte o devolve de dentro da ficha de depois)
            let mut after = sheet(0);
            edit_piece_field(t, &mut after, key, text(2));
            let list = if group == "ars" {
                &after["arsenal"]
            } else {
                &after["equip"]["armaduras"]
            };
            let item = list
                .as_array()
                .and_then(|l| l.iter().find(|p| p["uid"] == uid))
                .and_then(|p| p.get("item"))
                .cloned()
                .unwrap_or(Value::Null);
            let mudou = o["mudou"].take();
            o["item"] = item;
            o["mudou"] = mudou;
            o
        }
        "ajusteReset" => {
            let key = text(1);
            let (group, uid) = key.split_once(':').unwrap();
            let mut o = with_changes(sheet(0), |s| reset_piece(t, s, key), |_| Map::new());
            let mut after = sheet(0);
            reset_piece(t, &mut after, key);
            let list = if group == "ars" {
                &after["arsenal"]
            } else {
                &after["equip"]["armaduras"]
            };
            let piece = list
                .as_array()
                .and_then(|l| l.iter().find(|p| p["uid"] == uid))
                .cloned()
                .unwrap_or(Value::Null);
            let mudou = o["mudou"].take();
            o["peca"] = piece;
            o["mudou"] = mudou;
            o
        }
        _ => return None,
    };
    Some(out)
}

/// Todos os campos editáveis (a arma, os da D-259, o escudo e a armadura).
fn all_fields() -> Vec<centelha_engine::sheet::EditField> {
    use centelha_engine::sheet::{
        ARMOR_EDIT_FIELDS, SHIELD_EDIT_FIELDS, WEAPON_EDIT_FIELDS, WEAPON_MORE_FIELDS,
    };
    WEAPON_EDIT_FIELDS
        .iter()
        .chain(WEAPON_MORE_FIELDS.iter())
        .chain(SHIELD_EDIT_FIELDS.iter())
        .chain(ARMOR_EDIT_FIELDS.iter())
        .copied()
        .collect()
}

// ------------------------------------------------------ armaduras e bolsas

/// Os casos da fatia 8. `None` quando o nome não é daqui.
fn run_gear(ctx: &Ctx, inputs: &Value, name: &str, a: &[Value]) -> Option<Value> {
    let t = &ctx.t;
    let sheet = |i: usize| inputs[a[i].as_str().unwrap()]["ficha"].clone();
    let text = |i: usize| a[i].as_str().unwrap();
    let index = |i: usize| a[i].as_u64().unwrap() as usize;
    let cat = t.sheet_tables().catalog();
    let armors = |s: &Value| {
        let mut m = Map::new();
        m.insert(
            "armaduras".into(),
            s.get("equip")
                .and_then(|e| e.get("armaduras"))
                .cloned()
                .unwrap_or(Value::Null),
        );
        m
    };
    let bags = |s: &Value| {
        let mut m = Map::new();
        m.insert(
            "bolsas".into(),
            s.get("bolsas").cloned().unwrap_or(Value::Null),
        );
        m
    };
    let out = match name {
        "pecasArmadura" => Value::Array(worn_armor_pieces(t, &sheet(0))),
        "pesoPeca" => {
            let s = sheet(0);
            let p = piece_by_key(&s, text(1)).unwrap();
            num(piece_weight(cat, p))
        }
        "pesoBolsa" => {
            let s = sheet(0);
            match &a[1] {
                Value::Number(_) => num(bag_weight(&s["bolsas"][index(1)])),
                _ => num(bag_weight(&json!({}))),
            }
        }
        "pecaArm" => armor_piece(&sheet(0), text(1))
            .cloned()
            .unwrap_or(Value::Null),
        "pecaPorChave" => piece_by_key(&sheet(0), text(1))
            .cloned()
            .unwrap_or(Value::Null),
        "numerosDaArmadura" => {
            let n = armor_piece_numbers(t, &sheet(0), text(1)).unwrap();
            json!({
                "impacto": num(n.impact), "corte": num(n.cut), "perfuracao": num(n.pierce),
                "nivel": num(n.pierce_level), "penalidade": n.penalty, "ajustado": n.adjusted,
            })
        }
        "painelAbsorcao" => {
            let p = absorption_panel(t, &sheet(0));
            json!({
                "impacto": num(p.impact), "corte": num(p.cut), "perfuracao": num(p.pierce),
                "nivel": num(p.pierce_level), "penalidade": p.penalty, "nota": p.note,
            })
        }
        "armaduraAdd" => with_changes(sheet(0), |s| add_armor(s, text(1), text(2)), |s| armors(s)),
        "armaduraNome" => with_changes(
            sheet(0),
            |s| rename_armor(s, text(1), text(2)),
            |s| armors(s),
        ),
        "armaduraVestir" => {
            with_changes(sheet(0), |s| toggle_armor_worn(s, text(1)), |s| armors(s))
        }
        "armaduraRm" => with_changes(sheet(0), |s| remove_armor(s, text(1)), |s| armors(s)),
        "bolsaNome" => with_changes(sheet(0), |s| rename_bag(s, index(1), text(2)), |s| bags(s)),
        "bolsaTotal" => with_changes(
            sheet(0),
            |s| set_bag_total(s, index(1), text(2)),
            |s| bags(s),
        ),
        "bolsaCelula" => with_changes(
            sheet(0),
            |s| set_bag_cell(s, index(1), index(2), text(3), text(4)),
            |s| bags(s),
        ),
        "bolsaAdd" => with_changes(sheet(0), |s| add_bag_row(s, index(1)), |s| bags(s)),
        "bolsaRm" => with_changes(
            sheet(0),
            |s| remove_bag_row(s, index(1), index(2)),
            |s| bags(s),
        ),
        _ => return None,
    };
    Some(out)
}
