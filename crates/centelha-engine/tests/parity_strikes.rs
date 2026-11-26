//! Paridade do lance e dos dados com o TypeScript (tests/parity/strikes.json).
//!
//! - `fixed`: cada lance gravado pela mesa (`scripts/fixtures/lances.jsonl` da
//!   fonte), pelo `resolve_strike` com os dados que caíram (`FixedDice`).
//! - `rolled`: os mesmos lances em ordem, com um Mulberry32 só; além da saída,
//!   quantos números o gerador já entregou ao fim de cada lance. O teste para
//!   no primeiro lance diferente e diz em que número a sequência se separou.
//! - `rolls`: `roll_expr` de expressões reais e de borda, com a contagem.
//!
//! O portão da Perfuração e o piso do raspão (`blow_damage`, `blow_slips`)
//! não têm função no TypeScript (moram no Grid); os casos deles estão no fim,
//! com a conta do Grid.

mod common;

use centelha_engine::dice::roll_expr;
use centelha_engine::rng::Mulberry32;
use centelha_engine::sheet::AttackOutcome;
use centelha_engine::strike::{
    FixedDice, RolledDice, StrikeAttacker, StrikeInput, StrikeOutcome, StrikeTarget, blow_damage,
    blow_slips, resolve_strike,
};
use common::{compare, num, opt, repo};
use serde_json::{Value, json};

fn f(v: &Value) -> f64 {
    match v {
        Value::String(s) if s == "-0" => -0.0,
        v => v.as_f64().unwrap_or_else(|| panic!("não é número: {v}")),
    }
}

fn of(v: Option<&Value>) -> Option<f64> {
    v.filter(|x| !x.is_null()).map(f)
}

fn nums(v: &Value) -> Vec<f64> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|x| if x.is_null() { 0.0 } else { f(x) })
                .collect()
        })
        .unwrap_or_default()
}

fn input(e: &Value) -> StrikeInput {
    let (a, t) = (&e["atacante"], &e["alvo"]);
    StrikeInput {
        attacker: StrikeAttacker {
            attack: a["ataque"].as_str().unwrap().to_owned(),
            damage: a["dano"].as_str().unwrap().to_owned(),
            flat_adjust: f(&a["ajusteFlat"]),
            dice_adjust: f(&a["ajusteDados"]),
            blow_penalty_dice: nums(&a["penDados"]),
            damage_dice: of(a.get("danoDados")),
            damage_flat: of(a.get("danoFlat")),
        },
        target: StrikeTarget {
            base_defense: of(t.get("defesaBase")),
            wound: f(&t["ferimento"]),
            conditions_defense: f(&t["condicoesDefesa"]),
            lost_defense: f(&t["defesaPerdida"]),
            soak: f(&t["soak"]),
            hp: of(t.get("pv")),
        },
        blow_index: f(&e["golpeIndice"]),
        damage_type: e["tipoDano"].as_str().unwrap().to_owned(),
        manual_mod: f(&e["modManual"]),
        graze_margin: f(&e["margemQA"]),
        graze_damage: f(&e["danoQA"]),
    }
}

fn list(v: &[f64]) -> Value {
    Value::Array(v.iter().map(|x| num(*x)).collect())
}

fn outcome(o: &StrikeOutcome) -> Value {
    json!({
        "defesa": opt(o.defense), "total": num(o.total), "soma": num(o.sum),
        "errouPor": opt(o.missed_by), "veredito": o.verdict.map(AttackOutcome::id),
        "danoBruto": num(o.gross_damage), "tipo": o.damage_type, "absorcao": num(o.absorbed),
        "danoLiquido": num(o.net_damage), "pvAntes": opt(o.hp_before), "pvDepois": opt(o.hp_after),
        "rolls": { "acerto": list(&o.attack_rolls), "dano": list(&o.damage_rolls) },
    })
}

fn reference() -> Value {
    common::read_reference("strikes.json")
}

fn report(what: &str, n: usize, fields: usize, diffs: &[String]) {
    println!(
        "{what}: {n} casos, {fields} campos comparados, {} diferenças",
        diffs.len()
    );
    assert!(
        diffs.is_empty(),
        "{} diferenças:\n{}",
        diffs.len(),
        diffs[..diffs.len().min(40)].join("\n")
    );
}

#[test]
fn fixed_strikes_match_typescript() {
    let r = reference();
    let cases = r["fixed"].as_array().unwrap();
    assert_eq!(cases.len() as u64, r["counts"]["fixed"].as_u64().unwrap());
    assert!(cases.len() > 1000);
    let (mut diffs, mut fields) = (Vec::new(), 0);
    for c in cases {
        let e = &c["entrada"];
        let mut dice = FixedDice::new(nums(&c["sorteio"]["acerto"]), nums(&c["sorteio"]["dano"]));
        let got = outcome(&resolve_strike(&input(e), &mut dice));
        fields += compare(e["aid"].as_str().unwrap(), &got, &c["saida"], &mut diffs);
    }
    report("lances com os dados gravados", cases.len(), fields, &diffs);
}

#[test]
fn rolled_strikes_match_typescript() {
    let r = reference();
    let fixed = r["fixed"].as_array().unwrap();
    let rolled = r["rolled"].as_array().unwrap();
    assert_eq!(fixed.len(), rolled.len());
    let mut rng = Mulberry32::new(u32::try_from(r["rolledSeed"].as_u64().unwrap()).unwrap());
    let mut fields = 0;
    for (i, (c, want)) in fixed.iter().zip(rolled).enumerate() {
        let before = rng.draws();
        let got = outcome(&resolve_strike(
            &input(&c["entrada"]),
            &mut RolledDice(&mut rng),
        ));
        let mut diffs = Vec::new();
        fields += compare(
            c["entrada"]["aid"].as_str().unwrap(),
            &got,
            &want["saida"],
            &mut diffs,
        );
        let want_draws = want["draws"].as_u64().unwrap();
        assert!(
            diffs.is_empty() && rng.draws() == want_draws,
            "primeiro lance diferente: {i} (o gerador tinha entregado {before} números antes dele; \
             ao fim, Rust {} e TypeScript {want_draws}):\n{}",
            rng.draws(),
            diffs.join("\n")
        );
    }
    report("lances rolados", rolled.len(), fields, &[]);
}

#[test]
fn rolls_match_typescript() {
    let r = reference();
    let cases = r["rolls"].as_array().unwrap();
    let mut rng = Mulberry32::new(u32::try_from(r["rollsSeed"].as_u64().unwrap()).unwrap());
    let mut fields = 0;
    for (i, c) in cases.iter().enumerate() {
        let before = rng.draws();
        let roll = roll_expr(
            c["expr"].as_str().unwrap(),
            f(&c["extraDados"]),
            f(&c["extraFlat"]),
            &mut rng,
        );
        let got = json!({
            "dados": num(roll.dice), "flat": num(roll.flat), "rolls": list(&roll.rolls),
            "total": num(roll.total), "draws": rng.draws(),
        });
        let want = json!({
            "dados": c["dados"], "flat": c["flat"], "rolls": c["rolls"], "total": c["total"], "draws": c["draws"],
        });
        let mut diffs = Vec::new();
        fields += compare(&format!("{i}:{}", c["expr"]), &got, &want, &mut diffs);
        assert!(
            diffs.is_empty(),
            "primeira rolagem diferente: {i} (o gerador tinha entregado {before} números antes dela):\n{}",
            diffs.join("\n")
        );
    }
    report("rolagens", cases.len(), fields, &[]);
}

/// O portão e o piso, pela conta do Grid (`grid.astro`, `resvalaGate`,
/// `contaDoLance` e o `piso` passado ao `aplicarDano`).
#[test]
fn gate_and_graze_floor() {
    let content = repo().join("content");
    let rules = centelha_data::load_rules(&content).unwrap();
    // o portão só vale ligado, com a arma em modo Perfurante, e quando a
    // Perfuração da arma não vence a do alvo (sem a do alvo, 0)
    assert!(blow_slips(&rules, true, Some(1.0), Some(2.0)));
    assert!(!blow_slips(&rules, true, Some(2.0), Some(2.0)));
    assert!(!blow_slips(&rules, false, Some(0.0), Some(3.0)));
    assert!(!blow_slips(&rules, true, None, Some(3.0)));
    assert!(!blow_slips(&rules, true, Some(0.0), None));
    // o acerto nunca dói menos que o raspão; o raspão ignora a Absorção
    let hit = blow_damage(Some(AttackOutcome::Hit), 6.0, 3.0, 5.0, false);
    assert_eq!((hit.gross, hit.absorbed, hit.net), (6.0, 5.0, 3.0));
    let hit = blow_damage(Some(AttackOutcome::Hit), 12.0, 3.0, 5.0, false);
    assert_eq!((hit.gross, hit.absorbed, hit.net), (12.0, 5.0, 7.0));
    let graze = blow_damage(Some(AttackOutcome::Graze), 12.0, 3.0, 5.0, false);
    assert_eq!((graze.gross, graze.absorbed, graze.net), (3.0, 0.0, 3.0));
    let miss = blow_damage(Some(AttackOutcome::Miss), 12.0, 3.0, 5.0, false);
    assert_eq!(miss.net, 0.0);
    // resvalou: nada, nem o piso do raspão
    for v in [AttackOutcome::Hit, AttackOutcome::Graze] {
        let d = blow_damage(Some(v), 12.0, 3.0, 5.0, true);
        assert_eq!((d.gross, d.absorbed, d.net), (0.0, 0.0, 0.0));
    }
}

#[test]
fn the_comparison_catches_differences() {
    common::check_comparator();
}
