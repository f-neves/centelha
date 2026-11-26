//! Paridade do tempo do combate, função a função, com o TypeScript
//! (tests/parity/combat-functions.json, de tools/parity/timeline.mjs).
//!
//! O teste para no primeiro caso diferente e mostra o caminho de cada campo
//! diferente daquele caso. Nenhuma destas funções sorteia, então não há
//! contagem do gerador a comparar.

mod common;

use centelha_data::{Rules, Weapon};
use centelha_engine::timeline::{
    self, Action, Anatomy, AnatomyInput, FreeTiming, GuardOptions, Interposition, Maneuver, Phase,
    QueueKey, TimeClass, TimeSystem, TimingOverride,
};
use common::{compare, num, opt, repo};
use serde_json::{Map, Value, json};

struct Tables {
    rules: Rules,
    weapons: Vec<Weapon>,
}

fn tables() -> Tables {
    let content = repo().join("content");
    Tables {
        rules: centelha_data::load_rules(&content).unwrap(),
        weapons: centelha_data::load_weapons(&content).unwrap(),
    }
}

fn f(v: &Value) -> f64 {
    match v {
        Value::String(s) if s == "-0" => -0.0,
        v => v.as_f64().unwrap_or_else(|| panic!("não é número: {v}")),
    }
}

fn of(v: Option<&Value>) -> Option<f64> {
    v.filter(|x| !x.is_null()).map(f)
}

fn s(v: &Value) -> Option<&str> {
    v.as_str()
}

fn os(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).map(str::to_owned)
}

fn nums(v: &Value) -> Vec<f64> {
    v.as_array().unwrap().iter().map(f).collect()
}

fn list(v: &[f64]) -> Value {
    Value::Array(v.iter().map(|x| num(*x)).collect())
}

fn system(v: &Value) -> TimeSystem {
    TimeSystem::from_id(v.as_str().unwrap()).unwrap()
}

fn class(v: &Value) -> TimeClass {
    TimeClass::from_id(v.as_str().unwrap()).unwrap()
}

fn anatomy_in(v: &Value) -> Anatomy {
    Anatomy {
        windup: f(&v["preparo"]),
        blows: f(&v["golpes"]),
        recovery: f(&v["recuperacao"]),
        cycle: f(&v["ciclo"]),
        dice_penalties: nums(&v["penDados"]),
        offsets: nums(&v["offs"]),
        warning: os(v.get("aviso")),
    }
}

fn anatomy_out(a: &Anatomy) -> Value {
    let mut m = Map::new();
    m.insert("preparo".into(), num(a.windup));
    m.insert("golpes".into(), num(a.blows));
    m.insert("recuperacao".into(), num(a.recovery));
    m.insert("ciclo".into(), num(a.cycle));
    m.insert("offs".into(), list(&a.offsets));
    m.insert("penDados".into(), list(&a.dice_penalties));
    if let Some(w) = &a.warning {
        m.insert("aviso".into(), json!(w));
    }
    Value::Object(m)
}

const ACTION_KEYS: [&str; 13] = [
    "golpes",
    "livre",
    "desde",
    "tipo",
    "arma",
    "alvo",
    "divida",
    "pressao",
    "aResolver",
    "aid",
    "interpoe",
    "contrape",
    "contrapeDesde",
];

fn action_in(v: &Value) -> Option<Action> {
    let o = v.as_object()?;
    for k in o.keys() {
        assert!(
            ACTION_KEYS.contains(&k.as_str()),
            "campo da ação sem par: {k}"
        );
    }
    Some(Action {
        blows: o.get("golpes").map(nums),
        free_at: of(o.get("livre")),
        since: of(o.get("desde")),
        maneuver: o
            .get("tipo")
            .and_then(Value::as_str)
            .map(|t| Maneuver::from_id(t).unwrap()),
        weapon: os(o.get("arma")),
        target: os(o.get("alvo")),
        debt: of(o.get("divida")),
        pressure: of(o.get("pressao")),
        pending: o.get("aResolver").map(nums),
        aid: os(o.get("aid")),
        interposition: o
            .get("interpoe")
            .filter(|x| !x.is_null())
            .map(|x| Interposition {
                aid: x["aid"].as_str().unwrap().to_owned(),
                blow: f(&x["golpe"]),
                original_target: x["alvoOriginal"].as_str().unwrap().to_owned(),
            }),
        flat_footed: of(o.get("contrape")),
        flat_footed_since: of(o.get("contrapeDesde")),
    })
}

/// A ação como o TypeScript a devolve. `interpoe: null` da entrada volta como
/// ausente no Rust; nenhuma saída gravada tem esse caso.
fn action_out(a: &Action) -> Value {
    let mut m = Map::new();
    let mut put = |k: &str, v: Option<Value>| {
        if let Some(v) = v {
            m.insert(k.into(), v);
        }
    };
    put("golpes", a.blows.as_deref().map(list));
    put("livre", a.free_at.map(num));
    put("desde", a.since.map(num));
    put("tipo", a.maneuver.map(|t| json!(t.id())));
    put("arma", a.weapon.as_ref().map(|x| json!(x)));
    put("alvo", a.target.as_ref().map(|x| json!(x)));
    put("divida", a.debt.map(num));
    put("pressao", a.pressure.map(num));
    put("aResolver", a.pending.as_deref().map(list));
    put("aid", a.aid.as_ref().map(|x| json!(x)));
    put(
        "interpoe",
        a.interposition.as_ref().map(
            |i| json!({"aid": i.aid, "golpe": num(i.blow), "alvoOriginal": i.original_target}),
        ),
    );
    put("contrape", a.flat_footed.map(num));
    put("contrapeDesde", a.flat_footed_since.map(num));
    Value::Object(m)
}

fn queue_key(v: &Value) -> QueueKey {
    QueueKey {
        tick: of(v.get("tick")),
        initiative: of(v.get("iniciativa")),
        wits: of(v.get("raciocinio")),
        arrival: os(v.get("chegada")),
        name: os(v.get("nome")),
    }
}

fn sign(o: std::cmp::Ordering) -> Value {
    num(match o {
        std::cmp::Ordering::Less => -1.0,
        std::cmp::Ordering::Equal => 0.0,
        std::cmp::Ordering::Greater => 1.0,
    })
}

/// A fila: os índices da entrada depois da ordenação.
fn queue_out(pieces: &Value) -> Value {
    let keys: Vec<QueueKey> = pieces.as_array().unwrap().iter().map(queue_key).collect();
    Value::Array(
        timeline::sort_queue(&keys)
            .into_iter()
            .map(|i| json!(i))
            .collect(),
    )
}

fn run(t: &Tables, name: &str, a: &[Value]) -> Value {
    let r = &t.rules;
    let w = &t.weapons[..];
    let act = |v: &Value| action_in(v);
    match name {
        "fisicaDe" => json!(system(&a[0]).physics().id()),
        "armaDoCatalogo" => json!(timeline::catalog_weapon(w, s(&a[0])).map(|x| x.id.clone())),
        "classeDaArma" => json!(timeline::weapon_class(w, s(&a[0])).id()),
        "velocidadeDaArma" => num(timeline::weapon_speed(
            w,
            s(&a[0]),
            of(Some(&a[1])).unwrap_or(5.0),
        )),
        "classeDeTempo" => {
            json!(timeline::time_class(w, s(&a[0]), of(Some(&a[1])), s(&a[2])).id())
        }
        "preparoDe" => num(timeline::windup_of(
            r,
            class(&a[0]),
            f(&a[1]),
            system(&a[2]),
        )),
        "cicloExtraDaDupla" => num(timeline::dual_wield_extra_cycle(
            r,
            class(&a[0]),
            system(&a[1]),
        )),
        "tetoDaRajada" => num(timeline::flurry_cap(r, class(&a[0]))),
        "reguaDaArte" => {
            let x = timeline::art_timing(r, f(&a[0]), system(&a[1]));
            json!({"preparo": num(x.windup), "ciclo": num(x.cycle), "golpe": num(x.blow),
                   "recuperacao": num(x.recovery)})
        }
        "anatomia" => {
            let o = &a[0];
            let input = AnatomyInput {
                class: class(&o["classe"]),
                speed: of(o.get("velocidade")),
                system: system(&o["sistema"]),
                maneuver: o
                    .get("manobra")
                    .and_then(Value::as_str)
                    .map(|m| Maneuver::from_id(m).unwrap()),
                blows: of(o.get("golpes")),
            };
            anatomy_out(&timeline::anatomy(r, &input))
        }
        "comOverride" => {
            let o = (!a[1].is_null()).then(|| TimingOverride {
                windup: of(a[1].get("preparo")),
                blows: of(a[1].get("golpes")),
                recovery: of(a[1].get("recuperacao")),
            });
            anatomy_out(&timeline::with_override(&anatomy_in(&a[0]), o.as_ref()))
        }
        "anatomiaLivre" => anatomy_out(&timeline::free_anatomy(
            f(&a[0]),
            FreeTiming::from_id(a[1].as_str().unwrap()).unwrap(),
            system(&a[2]),
        )),
        "ticksDeEntrada" => Value::Array(
            timeline::entry_ticks(r, &nums(&a[0]))
                .iter()
                .map(|e| json!({"tick": num(e.tick), "penDados": num(e.dice_penalty), "atras": num(e.behind)}))
                .collect(),
        ),
        "contrapeDe" => match timeline::carried_flat_footed(act(&a[0]).as_ref()) {
            Some(x) => json!({"contrape": num(x.value), "contrapeDesde": num(x.since)}),
            None => json!({}),
        },
        "contrapeAcaba" => opt(timeline::flat_footed_ends(r, act(&a[0]).as_ref())),
        "contrapeEm" => num(timeline::flat_footed_at(r, act(&a[0]).as_ref(), f(&a[1]))),
        "faseDeQuemVaiAgir" => json!(
            timeline::phase_of_pending_actor(f(&a[0]), f(&a[1]), f(&a[2])).id()
        ),
        "ordemDaFila" => sign(timeline::queue_order(&queue_key(&a[0]), &queue_key(&a[1]))),
        "fila" => queue_out(&a[0]),
        "acaoVazia" => json!(timeline::action_is_empty(act(&a[0]).as_ref())),
        "temGesto" => json!(timeline::has_gesture(act(&a[0]).as_ref())),
        "declarar" => action_out(&timeline::declare(f(&a[0]), &anatomy_in(&a[1]))),
        "penDadosDaRegua" => num(timeline::ruler_dice_penalty(&anatomy_in(&a[0]), f(&a[1]))),
        "golpesNoAr" => list(timeline::pending_blows(act(&a[0]).as_ref())),
        "proximoGolpe" => opt(timeline::next_blow(act(&a[0]).as_ref())),
        "golpeDaAgenda" => json!(timeline::scheduled_blow_index(act(&a[0]).as_ref(), f(&a[1]))),
        "golpeDevido" => opt(timeline::due_blow(act(&a[0]).as_ref(), f(&a[1]))),
        "faseEm" => json!(timeline::phase_at(act(&a[0]).as_ref(), f(&a[1])).id()),
        "golpesDados" => num(timeline::blows_dealt(act(&a[0]).as_ref(), f(&a[1]))),
        "podeSerInterrompido" => json!(timeline::can_be_interrupted(act(&a[0]).as_ref(), f(&a[1]))),
        "agendar" => action_out(&timeline::schedule(&act(&a[0]).unwrap(), f(&a[1]))),
        "golpeResolvido" => action_out(&timeline::blow_resolved(&act(&a[0]).unwrap(), f(&a[1]))),
        "defesaPerdida" => {
            let g = &a[2];
            let opts = GuardOptions {
                off_hand_guard: g.get("segura").and_then(Value::as_bool).unwrap_or(false),
                phase: g
                    .get("fase")
                    .and_then(Value::as_str)
                    .map(|p| Phase::from_id(p).unwrap()),
            };
            let x = timeline::lost_defense(r, act(&a[0]).as_ref(), f(&a[1]), opts);
            json!({"fase": x.phase.id(), "acao": num(x.action), "pressao": num(x.pressure), "total": num(x.total)})
        }
        "adiaGolpe" | "ehSimultaneo" => {
            let c = &a[0];
            let sys = c
                .get("sistema")
                .and_then(Value::as_str)
                .map(|x| TimeSystem::from_id(x).unwrap());
            if name == "adiaGolpe" {
                let delayed = c.get("golpeAdiado").and_then(Value::as_bool).unwrap_or(false);
                json!(timeline::delays_blow(sys, delayed))
            } else {
                json!(timeline::is_simultaneous(sys))
            }
        }
        other => panic!("função sem par no Rust: {other}"),
    }
}

fn reference() -> Value {
    common::read_reference("combat-functions.json")
}

#[test]
fn timeline_matches_typescript() {
    let t = tables();
    let r = reference();
    let cases = r["cases"].as_array().unwrap();
    assert_eq!(cases.len() as u64, r["count"].as_u64().unwrap());
    let functions = r["counts"].as_object().unwrap().len();
    assert!(functions >= 35, "faltam funções na referência: {functions}");
    let mut fields = 0;
    for (i, c) in cases.iter().enumerate() {
        let name = c["fn"].as_str().unwrap();
        let got = run(&t, name, c["args"].as_array().unwrap());
        let mut diffs = Vec::new();
        fields += compare(
            &format!("{i}:{name}{}", c["args"]),
            &got,
            &c["out"],
            &mut diffs,
        );
        assert!(
            diffs.is_empty(),
            "primeiro caso diferente: {i} ({name}; estas funções não sorteiam, o gerador não entra):\n{}",
            diffs.join("\n")
        );
    }
    println!(
        "tempo do combate: {} casos em {functions} funções, {fields} campos comparados, 0 diferenças",
        cases.len()
    );
}

/// A ordem das peças na entrada conta: trocar duas peças diferentes muda a
/// fila, e a comparação acusa. E no empate inteiro a fila guarda a ordem de
/// entrada (o `sort` do JavaScript é estável).
#[test]
fn swapping_two_pieces_changes_the_queue() {
    let r = reference();
    let (mut caught, mut ties) = (0, 0);
    for c in r["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["fn"] == "fila")
    {
        let pieces = c["args"][0].as_array().unwrap();
        let pairs: Vec<(usize, usize)> = (0..pieces.len())
            .flat_map(|x| (x + 1..pieces.len()).map(move |y| (x, y)))
            .collect();
        if let Some(&(x, y)) = pairs.iter().find(|&&(x, y)| pieces[x] != pieces[y]) {
            let mut swapped = pieces.clone();
            swapped.swap(x, y);
            let mut diffs = Vec::new();
            compare(
                "fila",
                &queue_out(&Value::Array(swapped)),
                &c["out"],
                &mut diffs,
            );
            assert!(!diffs.is_empty(), "a troca de {x} e {y} não mudou a fila");
            caught += 1;
        }
        let order: Vec<u64> = c["out"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect();
        for &(x, y) in pairs.iter().filter(|&&(x, y)| pieces[x] == pieces[y]) {
            let at = |i: usize| order.iter().position(|&v| v == i as u64).unwrap();
            assert!(
                at(x) < at(y),
                "o empate inteiro de {x} e {y} trocou de ordem"
            );
            ties += 1;
        }
    }
    assert!(
        caught >= 70 && ties >= 10,
        "trocas {caught}, empates {ties}"
    );
}

/// Os ids do catálogo são únicos (o índice por id da fonte fica com a última
/// peça de um id repetido; o Rust também, mas hoje não há repetido).
#[test]
fn catalog_ids_are_unique() {
    let t = tables();
    let mut ids: Vec<&str> = t.weapons.iter().map(|w| w.id.as_str()).collect();
    ids.sort_unstable();
    let n = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), n);
}

/// Os textos das peças da fila gravadas: nenhum com maiúscula ou
/// acento, onde a ordem por ponto de código e a do `localeCompare` coincidem.
#[test]
fn queue_texts_are_plain() {
    let r = reference();
    for c in r["cases"].as_array().unwrap() {
        let pieces: Vec<&Value> = match c["fn"].as_str().unwrap() {
            "ordemDaFila" => vec![&c["args"][0], &c["args"][1]],
            "fila" => c["args"][0].as_array().unwrap().iter().collect(),
            _ => continue,
        };
        for p in pieces {
            for k in ["chegada", "nome"] {
                if let Some(x) = p.get(k).and_then(Value::as_str) {
                    assert!(
                        x.chars()
                            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-'),
                        "{x}"
                    );
                }
            }
        }
    }
}

#[test]
fn the_comparison_catches_differences() {
    common::check_comparator();
}
