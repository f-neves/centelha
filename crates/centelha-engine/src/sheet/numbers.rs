//! Os números da ficha do jogador: PV, peso erguido, Defesas, Energia, Mana,
//! Iniciativa, Absorção, o conjunto das mãos, o resumo de combate e o resumo
//! da mesa (port de `numerosPersonagem`, `conjuntoPersonagem`,
//! `combatePersonagem` e do miolo de `motor.ts` que serve ao personagem, e de
//! `resumoFicha` e `resumoParaBanco`, de `mesa-ficha.ts`).

use serde_json::{Map, Value, json};

use super::SheetTables;
use super::equip::{CUSTOM_SHIELD, ItemKind, piece_kind};
use super::format::for_reading;
use super::value::{
    coalesce, get, includes, is_str, nullish, num_or, num_or0, or, property_key, spread,
    spread_into, strict_eq, to_js_string, to_number, truthy,
};
use crate::creature::{Soak, lifted_weight};
use crate::js;

/// O que está numa mão.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandKind {
    /// Mão vazia (ou ocupada pela arma de duas mãos).
    Nothing,
    /// Arma do catálogo.
    Weapon,
    /// Arma personalizada.
    Custom,
    Shield,
}

impl HandKind {
    /// O texto do TypeScript (`nada`, `arma`, `custom`, `escudo`).
    pub fn id(self) -> &'static str {
        match self {
            HandKind::Nothing => "nada",
            HandKind::Weapon => "arma",
            HandKind::Custom => "custom",
            HandKind::Shield => "escudo",
        }
    }

    fn is_weapon(self) -> bool {
        matches!(self, HandKind::Weapon | HandKind::Custom)
    }
}

/// O item de uma mão (`itemDoJogador`).
#[derive(Debug, Clone, PartialEq)]
pub struct HandItem {
    pub kind: HandKind,
    /// O nome que a mão mostra; `None` quando o item não tem nome.
    pub name: Option<String>,
    /// O que a mão soma ao Bloqueio.
    pub defense: f64,
    pub penalty: f64,
    /// A arma achatada (`w`), na arma e na personalizada.
    pub weapon: Option<Value>,
    /// O escudo achatado (`s`).
    pub shield: Option<Value>,
    /// O item do catálogo de onde a peça veio (ou o próprio item).
    pub base: Option<Value>,
    /// O escudo contra o projétil rápido.
    pub vs_fast_projectile: Option<Value>,
}

impl HandItem {
    fn nothing(name: &str) -> Self {
        HandItem {
            kind: HandKind::Nothing,
            name: Some(name.into()),
            defense: 0.0,
            penalty: 0.0,
            weapon: None,
            shield: None,
            base: None,
            vs_fast_projectile: None,
        }
    }
}

/// Uma versão do dano da arma da mão hábil: o rótulo (`2 mãos` na versátil
/// empunhada com as duas) e o fixo somado ao dado.
#[derive(Debug, Clone, PartialEq)]
pub struct DamageVersion {
    pub label: String,
    pub bonus: f64,
}

/// A segunda arma (empunhadura dupla).
#[derive(Debug, Clone, PartialEq)]
pub struct DualWield {
    /// O fixo do dano da mão hábil.
    pub main_bonus: f64,
    pub off_dice: f64,
    pub off_roll_bonus: f64,
    pub off_flat: f64,
    /// O dado da arma da mão inábil, como está no item.
    pub off_die: Option<Value>,
    pub off_bonus: f64,
    pub off_penalty: f64,
}

/// O conjunto das mãos em uso (`conjuntoPersonagem`).
#[derive(Debug, Clone, PartialEq)]
pub struct HandSet {
    pub main_hand: HandItem,
    pub off_hand: HandItem,
    /// A arma do ataque (a da mão hábil, ou os punhos), já rebaixada pelo
    /// requisito de Força.
    pub attack_weapon: Value,
    /// Os dados da jogada de ataque e o +2 da soma ímpar.
    pub dice: f64,
    pub roll_bonus: f64,
    /// O fixo da jogada (acerto da arma + Centelha − Penalidade da armadura).
    pub flat: f64,
    pub ranged: bool,
    pub versions: Vec<DamageVersion>,
    /// A Força que a arma pede e o braço não tem (0 quando cumpre).
    pub strength_missing: f64,
    pub dual: Option<DualWield>,
    /// O que as mãos somam ao Bloqueio.
    pub block: f64,
    /// A Penalidade das mãos (o escudo).
    pub penalty: f64,
    pub armor_penalty: f64,
    /// A Força que a Investida soma mais uma vez no dano.
    pub charge_strength: f64,
}

/// As peças de armadura empilhadas (`empilharArmaduras`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArmorStack {
    pub soak: Soak,
    pub pierce_level: f64,
    pub penalty: f64,
}

/// A Iniciativa: o dado, o que se soma e o texto (`1d6 + 6`).
#[derive(Debug, Clone, PartialEq)]
pub struct Initiative {
    pub die: f64,
    pub bonus: f64,
    pub text: String,
}

/// Os números da ficha (`numerosPersonagem`).
#[derive(Debug, Clone, PartialEq)]
pub struct SheetNumbers {
    pub hp: f64,
    pub lifted_weight: f64,
    pub dodge: f64,
    pub block: f64,
    pub social_defense: f64,
    pub mental_defense: f64,
    pub energy: f64,
    pub mana: f64,
    pub initiative: Initiative,
    pub soak: Soak,
    pub hands: HandSet,
    pub armor: ArmorStack,
}

/// O Quase-Acerto de uma peça: a metade da arma e a da armadura.
#[derive(Debug, Clone, PartialEq)]
pub struct NearMiss {
    pub weapon_bonus: f64,
    pub weapon_damage: f64,
    /// `leve`, `media` ou `pesada`; `None` sem arma nem expressão.
    pub weapon_class: Option<String>,
    pub armor_bonus: f64,
    pub armor_reduction: f64,
    pub armor_classes: Vec<String>,
}

/// O passo real, em metros por Tick, já com a meia Penalidade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Step {
    pub battle: f64,
    pub dash: f64,
    pub run: f64,
}

/// O resumo de combate (`combatePersonagem`).
#[derive(Debug, Clone, PartialEq)]
pub struct CombatSummary {
    pub weapon_name: Option<String>,
    /// A jogada de ataque (`3d6+2 +6`).
    pub attack: String,
    /// O dano (`1d6 +6 (C)`).
    pub damage: String,
    pub dodge: f64,
    pub mental_defense: f64,
    pub soak: Soak,
    pub pierce_level: f64,
    /// O Nível de Perfuração do modo principal da arma (`None` quando o modo
    /// não é Perfurante).
    pub weapon_pierce: Option<f64>,
    pub near_miss: NearMiss,
    pub step: Step,
    /// Os Atributos e as Habilidades crus, como estão na ficha recebida.
    pub attributes: Value,
    pub skills: Value,
    pub speed: Option<f64>,
    /// A classe de tempo da arma, como está no item.
    pub class: Option<Value>,
    pub charge_strength: f64,
}

/// Um Valor Passivo do painel do mestre.
#[derive(Debug, Clone, PartialEq)]
pub struct Passive {
    pub id: &'static str,
    pub name: &'static str,
    pub dice: f64,
    pub bonus: f64,
    pub value: f64,
    pub text: String,
}

/// Uma Arte com nível.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtLevel {
    pub id: String,
    pub level: Value,
}

/// O resumo da ficha para a mesa (`resumoFicha`).
#[derive(Debug, Clone, PartialEq)]
pub struct SheetSummary {
    pub name: String,
    pub concept: String,
    pub player: String,
    pub ancestry: String,
    pub spark: f64,
    pub willpower: f64,
    pub appearance: f64,
    pub attributes: Value,
    pub skills: Value,
    pub secondary_skills: Value,
    pub virtues: Value,
    pub hp: f64,
    pub energy: f64,
    pub mana: f64,
    pub dodge: f64,
    pub block: f64,
    pub social_defense: f64,
    pub mental_defense: f64,
    pub soak: Soak,
    pub pierce_level: f64,
    /// A Penalidade física da armadura.
    pub physical_penalty: f64,
    pub initiative: String,
    pub initiative_average: f64,
    pub weapon: String,
    pub attack: String,
    pub damage: String,
    pub passives: Vec<Passive>,
    pub feats: Vec<String>,
    pub arts: Vec<ArtLevel>,
    pub armors: Vec<String>,
    pub near_miss: NearMiss,
}

/// O que vai para `personagens.resumo` (`resumoParaBanco`).
#[derive(Debug, Clone, PartialEq)]
pub struct DbSummary {
    pub hp: f64,
    pub energy: f64,
    pub mana: f64,
    pub weapon: String,
    pub attack: String,
    pub damage: String,
    pub dodge: f64,
    pub block: f64,
    pub social_defense: f64,
    pub mental_defense: f64,
    pub soak: Soak,
    pub pierce_level: f64,
    pub initiative: String,
}

// ----- as contas pequenas -----

/// `x || 0` de um número (`NaN`, `0` e `-0` dão 0).
fn or0(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 { 0.0 } else { x }
}

fn spark_in_roll(spark: f64, skill: f64) -> f64 {
    2.0 * js::min(js::max(0.0, spark), js::max(0.0, skill))
}

/// `defesa`: (Destreza + Habilidade) × mult + 2 × mín(Centelha, Habilidade).
fn defense(t: &SheetTables, dexterity: f64, skill: f64, spark: f64) -> f64 {
    (dexterity + skill) * t.rules.derived.defense.mult + 0.0 + spark_in_roll(spark, skill)
}

/// `empilharArmaduras`: a maior Absorção de cada tipo, o maior Nível de
/// Perfuração e a soma das Penalidades.
pub fn stack_armor(pieces: &[Value]) -> ArmorStack {
    let mut s = ArmorStack {
        soak: Soak {
            impact: 0.0,
            cut: 0.0,
            pierce: 0.0,
        },
        pierce_level: 0.0,
        penalty: 0.0,
    };
    for p in pieces {
        let soak = get(Some(p), "soak");
        s.soak.impact = js::max(s.soak.impact, num_or(get(soak, "impacto"), 0.0));
        s.soak.cut = js::max(s.soak.cut, num_or(get(soak, "corte"), 0.0));
        s.soak.pierce = js::max(s.soak.pierce, num_or(get(soak, "perfuracao"), 0.0));
        s.pierce_level = js::max(s.pierce_level, num_or(get(Some(p), "resistPerf"), 0.0));
        s.penalty += num_or(get(Some(p), "penalidade"), 0.0);
    }
    s
}

/// `deslocamento`: as medidas de movimento (em metros por Tick; o salto
/// vertical em cm), com a fração da Raça antes do arredondamento.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Movement {
    pub dash: f64,
    pub run: f64,
    pub normal: f64,
    pub swim: f64,
    pub vertical_jump: f64,
    pub standing_jump: f64,
    pub running_jump: f64,
}

/// `deslocamento(traits, frac)`. `trait_value` dá o traço pela chave da
/// escada (`forca`, `atletismo`...), `None` quando não há.
pub fn movement(
    t: &SheetTables,
    trait_value: impl Fn(&str) -> Option<f64>,
    fraction: f64,
) -> Movement {
    let calc = |l: &centelha_data::rules::Ladder| {
        let sum = l.0.iter().fold(0.0, |s, (k, v)| {
            if k == "base" {
                s + v
            } else {
                s + trait_value(k).unwrap_or(0.0) * v
            }
        });
        js::round(sum * fraction)
    };
    let m = &t.rules.derived.movement;
    let run = calc(&m.run);
    Movement {
        dash: calc(&m.dash),
        run,
        normal: calc(&m.normal),
        swim: calc(&m.swim),
        vertical_jump: calc(&m.vertical_jump),
        standing_jump: calc(&m.standing_jump),
        running_jump: run + calc(&m.running_jump),
    }
}

/// `atributoDaJogada` (do personagem, sem o ataque natural).
fn roll_attribute(skill: Option<&Value>, at: Option<&Value>) -> f64 {
    if is_str(skill, "disparo") {
        return num_or(get(at, "vigor"), 0.0);
    }
    if is_str(skill, "atirador") {
        return num_or(get(at, "percepcao"), 0.0);
    }
    js::max(
        num_or(get(at, "forca"), 0.0),
        num_or(get(at, "destreza"), 0.0),
    )
}

/// `comRequisitoDeForca`: a arma que pede Força que o braço não tem rende o
/// patamar comum (`forcaMult` 1, `danoBonus` 0).
fn with_strength_requirement(w: &Value, strength: f64) -> Value {
    let min = get(Some(w), "forcaMin");
    if !truthy(Some(w)) || !truthy(min) || strength >= to_number(min) {
        return w.clone();
    }
    let mut m = spread(Some(w));
    m.insert("forcaMult".into(), json!(1));
    m.insert("danoBonus".into(), json!(0));
    Value::Object(m)
}

/// `forcaNoArco`: acima de 8 o arco conta 8.
fn bow_strength(t: &SheetTables, w: Option<&Value>, strength: f64) -> f64 {
    let cap = t
        .rules
        .combat
        .ranged
        .and_then(|r| r.maximum)
        .and_then(|m| m.bow_strength_cap)
        .unwrap_or(8.0);
    if is_str(get(w, "classe"), "distancia") && num_or(get(w, "forcaMult"), 0.0) > 0.0 {
        js::min(strength, cap)
    } else {
        strength
    }
}

/// `forcaDaInvestida`.
fn charge_strength(t: &SheetTables, strength: f64, mult: f64, ranged: bool) -> f64 {
    let extra = t
        .rules
        .combat
        .movement
        .and_then(|m| m.charge)
        .and_then(|c| c.extra_strength)
        .unwrap_or(1.0);
    if ranged || mult.is_nan() || mult <= 0.0 {
        0.0
    } else {
        js::max(0.0, strength) * extra
    }
}

fn text_of(v: Option<&Value>) -> Option<String> {
    v.map(|x| match x {
        Value::String(s) => s.clone(),
        other => to_js_string(Some(other)),
    })
}

/// `itemDoJogador`: o item de um slot de mão.
pub fn hand_item(t: &SheetTables, slot: Option<&Value>) -> HandItem {
    let r = or(get(slot, "ref"), None);
    let Some(kind) = piece_kind(slot) else {
        return HandItem::nothing("");
    };
    let Some(it) = t.catalog.piece_item(slot) else {
        return HandItem::nothing("");
    };
    let base = t.catalog.piece_origin(slot).unwrap_or_else(|| it.clone());
    let slot_name = or(get(slot, "nome"), None);
    match kind.kind {
        ItemKind::Shield => {
            let name = if is_str(r, CUSTOM_SHIELD) {
                or(slot_name, get(Some(&it), "nome")).cloned()
            } else {
                get(Some(&it), "nome").cloned()
            };
            let mut s = spread(Some(&it));
            super::value::set(&mut s, "nome", name.clone());
            let s = Value::Object(s);
            let vs = or(get(Some(&s), "vsProjetilRapido"), None)
                .cloned()
                .unwrap_or_else(|| json!({ "bloqueia": false, "bonus": 0 }));
            HandItem {
                kind: HandKind::Shield,
                name: text_of(name.as_ref()),
                defense: num_or0(get(Some(&s), "bloqCaC")),
                penalty: num_or0(get(Some(&s), "penalidade")),
                weapon: None,
                shield: Some(s),
                base: Some(base),
                vs_fast_projectile: Some(vs),
            }
        }
        ItemKind::Weapon => {
            let (kind, w) = if kind.custom {
                let mut w = spread(Some(&it));
                super::value::set(
                    &mut w,
                    "nome",
                    or(slot_name, get(Some(&it), "nome")).cloned(),
                );
                (HandKind::Custom, Value::Object(w))
            } else {
                (HandKind::Weapon, it)
            };
            HandItem {
                kind,
                name: text_of(get(Some(&w), "nome")),
                defense: num_or0(get(Some(&w), "defesaArma")),
                penalty: 0.0,
                shield: None,
                base: Some(base),
                vs_fast_projectile: None,
                weapon: Some(w),
            }
        }
        ItemKind::Armor => HandItem::nothing(""),
    }
}

/// `conjuntoAtivoPersonagem`: o conjunto marcado como ativo, o primeiro, ou o
/// das fichas antigas (`equip.arma`, `equip.escudo`).
pub fn active_hand_set(t: &SheetTables, sheet: &Value) -> Value {
    let s = for_reading(&t.catalog, sheet);
    let sets = get(Some(&s), "conjuntos");
    if let Some(Value::Array(list)) = sets {
        if let Some(c) = list.iter().find(|c| truthy(get(Some(c), "ativo"))) {
            return c.clone();
        }
        if let Some(c) = list.first().filter(|c| !c.is_null()) {
            return c.clone();
        }
    }
    let equip = get(Some(&s), "equip");
    let weapon = coalesce(get(equip, "arma"), None);
    let weapon = if nullish(weapon) {
        "desarmado".to_owned()
    } else {
        to_js_string(weapon)
    };
    let shield = get(equip, "escudo");
    let off = if truthy(shield) && !is_str(shield, "nenhum") {
        format!("e:{}", to_js_string(shield))
    } else {
        "nada".to_owned()
    };
    json!({ "habil": { "ref": format!("a:{weapon}") }, "inabil": { "ref": off } })
}

/// `conjuntoPersonagem(S, cj)`: o ataque, o dano e o Bloqueio das mãos.
pub fn hand_set(t: &SheetTables, s: &Value, cj: &Value) -> HandSet {
    let at = coalesce(get(Some(s), "attrs"), None)
        .cloned()
        .unwrap_or_else(|| json!({}));
    let at = Some(&at);
    let skills = get(Some(s), "skills");
    let skills2 = get(Some(s), "skills2");
    let skill_of = |w: Option<&Value>| {
        let k = property_key(get(w, "pericia"));
        num_or0(or(get(skills, &k), get(skills2, &k)))
    };
    let two_handed = |it: &HandItem| {
        it.kind.is_weapon()
            && matches!(get(it.weapon.as_ref(), "maos"), Some(Value::Number(n)) if n.as_f64() == Some(2.0))
    };
    let versatile = |it: &HandItem| {
        it.kind.is_weapon() && includes(or(get(it.weapon.as_ref(), "tags"), None), "versátil")
    };
    let c = num_or0(get(Some(s), "centelha"));
    let strength = num_or0(get(at, "forca"));
    let armor_penalty = stack_armor(&t.catalog.worn_armor(s)).penalty;
    let main = hand_item(t, get(Some(cj), "habil"));
    let off = if two_handed(&main) {
        HandItem::nothing("·")
    } else {
        hand_item(t, get(Some(cj), "inabil"))
    };
    let raw_attack = if main.kind.is_weapon() {
        main.weapon.clone().unwrap_or(Value::Null)
    } else {
        t.catalog.unarmed().cloned().unwrap_or(Value::Null)
    };
    let atk = with_strength_requirement(&raw_attack, strength);
    let a = Some(&atk);
    let skill = skill_of(a);
    let sum = roll_attribute(get(a, "pericia"), at) + skill;
    let dice = (sum / 2.0).floor();
    let roll_bonus = if sum % 2.0 == 1.0 { 2.0 } else { 0.0 };
    let flat = num_or0(get(a, "acerto")) + spark_in_roll(c, skill) - armor_penalty;
    let ranged = includes(or(get(a, "tags"), None), "distância");
    let fm = t.rules.derived.strength_damage;
    let db = num_or0(get(a, "danoBonus"));
    let mult_of = |w: Option<&Value>, d: f64| num_or(get(w, "forcaMult"), d);
    let capped = |w: Option<&Value>| {
        let cap = get(w, "forcaCap");
        let f = if nullish(cap) {
            strength
        } else {
            js::min(strength, to_number(cap))
        };
        bow_strength(t, w, f)
    };
    let cap_f = capped(a);
    let two_hands_atk = matches!(get(a, "maos"), Some(Value::Number(n)) if n.as_f64() == Some(2.0));
    let mut versions = Vec::new();
    if !ranged && versatile(&main) {
        versions.push(if off.kind != HandKind::Nothing {
            DamageVersion {
                label: String::new(),
                bonus: db + strength * mult_of(a, fm.one_hand) + c,
            }
        } else {
            DamageVersion {
                label: "2 mãos".into(),
                bonus: db + strength * fm.two_hands + c,
            }
        });
    } else {
        let mult = if ranged {
            mult_of(a, 1.0)
        } else if two_hands_atk {
            mult_of(a, fm.two_hands)
        } else {
            mult_of(a, fm.one_hand)
        };
        versions.push(DamageVersion {
            label: String::new(),
            bonus: db + cap_f * mult + c,
        });
    }
    let raw_min = get(Some(&raw_attack), "forcaMin");
    let strength_missing = if truthy(raw_min) && strength < to_number(raw_min) {
        to_number(raw_min)
    } else {
        0.0
    };
    let dual = if off.kind.is_weapon() {
        let w = with_strength_requirement(off.weapon.as_ref().unwrap_or(&Value::Null), strength);
        let wi = Some(&w);
        let skill_i = skill_of(wi);
        let sum_i = roll_attribute(get(wi, "pericia"), at) + skill_i;
        let ranged_i = includes(or(get(wi, "tags"), None), "distância");
        let cap_fi = capped(wi);
        let mult_i = if ranged_i {
            mult_of(wi, 1.0)
        } else {
            mult_of(wi, fm.one_hand)
        };
        Some(DualWield {
            main_bonus: versions[0].bonus,
            off_dice: (sum_i / 2.0).floor(),
            off_roll_bonus: if sum_i % 2.0 == 1.0 { 2.0 } else { 0.0 },
            off_flat: num_or0(get(wi, "acerto")) + spark_in_roll(c, skill_i) - armor_penalty,
            off_die: get(wi, "dado").cloned(),
            off_bonus: num_or0(get(wi, "danoBonus")) + cap_fi * mult_i + c,
            off_penalty: 1.0,
        })
    } else {
        None
    };
    let is_body = |slot: Option<&Value>| is_str(get(slot, "ref"), "a:desarmado");
    let main_body = is_body(get(Some(cj), "habil"));
    let off_body = !two_handed(&main) && is_body(get(Some(cj), "inabil"));
    let free_hands = f64::from(u8::from(main_body))
        + f64::from(u8::from(
            (off.kind == HandKind::Nothing || off_body) && !two_handed(&main),
        ));
    let in_hands: Vec<&HandItem> = [(!main_body).then_some(&main), (!off_body).then_some(&off)]
        .into_iter()
        .flatten()
        .filter(|it| it.kind != HandKind::Nothing)
        .collect();
    let hands_def = in_hands.iter().fold(0.0, |s, it| s + it.defense);
    let body_def = free_hands * num_or(get(t.catalog.unarmed(), "defesaArma"), 1.0);
    let block = if in_hands.is_empty() {
        body_def
    } else if free_hands != 0.0 {
        js::max(hands_def, body_def)
    } else {
        hands_def
    };
    let main_mult = if !ranged && versatile(&main) {
        if off.kind != HandKind::Nothing {
            mult_of(a, fm.one_hand)
        } else {
            fm.two_hands
        }
    } else if ranged {
        mult_of(a, 1.0)
    } else if two_hands_atk {
        mult_of(a, fm.two_hands)
    } else {
        mult_of(a, fm.one_hand)
    };
    let charge = charge_strength(
        t,
        if !ranged && versatile(&main) {
            strength
        } else {
            cap_f
        },
        main_mult,
        ranged,
    );
    HandSet {
        penalty: main.penalty + off.penalty,
        main_hand: main,
        off_hand: off,
        attack_weapon: atk,
        dice,
        roll_bonus,
        flat,
        ranged,
        versions,
        strength_missing,
        dual,
        block,
        armor_penalty,
        charge_strength: charge,
    }
}

/// `massaDoJogador`: a massa escrita no corpo, o peso escrito na identidade,
/// ou o meio da faixa da Raça.
pub fn player_mass(t: &SheetTables, s: &Value) -> f64 {
    if let Some(Value::Number(n)) = get(get(Some(s), "corpo"), "massa")
        && let Some(x) = n.as_f64()
        && x.is_finite()
        && x > 0.0
    {
        return x;
    }
    let read = super::value::number_in_text(get(get(Some(s), "id"), "peso")).unwrap_or(f64::NAN);
    let range = t
        .ancestry(get(Some(s), "raca"))
        .and_then(|a| a.mass_kg)
        .unwrap_or([60.0, 80.0]);
    if read > 0.0 {
        read
    } else {
        (range[0] + range[1]) / 2.0
    }
}

/// `numerosPersonagem`: os números da ficha. Aceita as duas formas da ficha.
pub fn sheet_numbers(t: &SheetTables, sheet: &Value) -> SheetNumbers {
    let s = for_reading(&t.catalog, sheet);
    let sv = Some(&s);
    let at = coalesce(coalesce(get(sv, "attrs"), get(sv, "atributos")), None)
        .cloned()
        .unwrap_or_else(|| json!({}));
    let hab = coalesce(coalesce(get(sv, "skills"), get(sv, "habilidades")), None)
        .cloned()
        .unwrap_or_else(|| json!({}));
    let sec = coalesce(
        coalesce(get(sv, "skills2"), get(sv, "habilidadesSec")),
        None,
    )
    .cloned()
    .unwrap_or_else(|| json!({}));
    let (a, h) = (Some(&at), Some(&hab));
    let ce = num_or0(get(sv, "centelha"));
    let will = num_or(coalesce(get(sv, "willpower"), get(sv, "vontade")), 0.0);
    let arm = stack_armor(&t.catalog.worn_armor(&s));
    let mut with_traits = spread(sv);
    with_traits.insert("attrs".into(), at.clone());
    with_traits.insert("skills".into(), hab.clone());
    with_traits.insert("skills2".into(), sec.clone());
    let act = hand_set(t, &Value::Object(with_traits), &active_hand_set(t, &s));

    // personagem(): o PV pelo Tamanho da Raça e o peso erguido pela massa
    let ancestry = t.ancestry(get(sv, "raca"));
    let size = ancestry
        .and_then(|r| r.size)
        .unwrap_or(centelha_data::bestiary::Size::Medium);
    let stamina = num_or(get(a, "vigor"), 0.0);
    let vitality = if ancestry.is_some_and(|r| r.adds_stamina_to_hp()) {
        stamina
    } else {
        0.0
    };
    let pv = &t.rules.derived.pv;
    let (hp_base, hp_mult) = match pv.by_size.as_ref().and_then(|m| m.get(&size)) {
        Some(row) => (row.base, row.stamina_mult),
        None => (pv.base, pv.stamina_mult),
    };
    let hp = hp_base + stamina * hp_mult + vitality;
    let lifted = lifted_weight(
        &t.rules,
        player_mass(t, &s),
        num_or(get(a, "forca"), 0.0),
        num_or(get(Some(&sec), "halterofilismo"), 0.0),
        0.0,
    );

    let d = &t.rules.derived;
    let dex = num_or0(get(a, "destreza"));
    let dodge = defense(t, dex, num_or0(get(h, "esquiva")), ce) - arm.penalty - act.penalty;
    let block = defense(t, dex, num_or0(get(h, "bloqueio")), ce) - arm.penalty + act.block;
    let sociability = num_or0(get(h, "sociabilidade"));
    let composure = num_or0(get(a, "compostura"));
    let social = d.social_defense.traits.iter().fold(0.0, |acc, k| {
        acc + match k.as_str() {
            "compostura" => composure,
            "sociabilidade" => sociability,
            _ => 0.0,
        }
    }) * d.social_defense.mult
        + spark_in_roll(ce, sociability)
        + 0.0;
    let integrity = num_or0(get(h, "integridade"));
    let wits = num_or0(get(a, "raciocinio"));
    let mental = integrity * d.mental_defense.mult
        + if d.mental_defense.plus_wits.unwrap_or(false) {
            wits
        } else {
            0.0
        }
        + if d.mental_defense.plus_willpower.unwrap_or(false) {
            will
        } else {
            0.0
        }
        + spark_in_roll(ce, integrity)
        + 0.0;
    let energy_base = num_or0(get(a, "vigor"))
        + composure
        + wits
        + if d.energy.plus_willpower.unwrap_or(false) {
            will
        } else {
            0.0
        };
    let energy = (energy_base / d.energy.divisor).floor() + ce * d.energy.spark_mult;
    let mana_level = js::max(
        0.0,
        js::min(6.0, num_or0(get(get(sv, "arte"), "manipulacao-mana"))),
    );
    const MANA_ART_BONUS: [f64; 7] = [0.0, 1.0, 2.0, 3.0, 5.0, 8.0, 12.0];
    let art = if mana_level.fract() == 0.0 && mana_level >= 0.0 {
        MANA_ART_BONUS
            .get(mana_level as usize)
            .copied()
            .unwrap_or(0.0)
    } else {
        0.0
    };
    let mana = ce * d.mana.spark_mult
        + if d.mana.plus_willpower.unwrap_or(false) {
            will
        } else {
            0.0
        }
        + art;
    let mut traits = spread(a);
    spread_into(&mut traits, h);
    let traits = Value::Object(traits);
    let bonus = d
        .initiative
        .sum
        .iter()
        .fold(0.0, |acc, k| acc + num_or(get(Some(&traits), k), 0.0));
    let initiative = Initiative {
        die: d.initiative.die,
        bonus,
        text: format!("1d6 + {}", js::number_to_string(bonus)),
    };
    let soak = Soak {
        impact: num_or0(get(a, "vigor")) + ce + or0(arm.soak.impact),
        cut: ce + or0(arm.soak.cut),
        pierce: ce + or0(arm.soak.pierce),
    };
    SheetNumbers {
        hp,
        lifted_weight: lifted,
        dodge,
        block,
        social_defense: social,
        mental_defense: mental,
        energy,
        mana,
        initiative,
        soak,
        hands: act,
        armor: arm,
    }
}

const DAMAGE_MODES: [(&str, f64, &str); 3] = [
    ("impacto", 0.0, "(I)"),
    ("corte", 1.0, "(C)"),
    ("perfurante", 2.0, "(P)"),
];

fn mode_order(m: &Value) -> f64 {
    let k = property_key(get(Some(m), "tipo"));
    DAMAGE_MODES
        .iter()
        .find(|(id, _, _)| *id == k)
        .map_or(f64::NAN, |(_, o, _)| *o)
}

/// O `Array.prototype.sort` do V8 para listas curtas (a sequência inicial e
/// a inserção binária do TimSort), com a mesma regra do comparador: `NaN`
/// conta como empate. Mesmo com comparador incoerente o resultado é o do V8.
fn v8_sort(list: &mut [Value], cmp: impl Fn(&Value, &Value) -> f64) {
    let n = list.len();
    if n < 2 {
        return;
    }
    let order = |a: &Value, b: &Value| {
        let v = cmp(a, b);
        if v.is_nan() { 0.0 } else { v }
    };
    // a sequência inicial (crescente, ou estritamente decrescente e invertida)
    let mut run = 2;
    let descending = order(&list[1], &list[0]) < 0.0;
    while run < n {
        let o = order(&list[run], &list[run - 1]);
        if (descending && o >= 0.0) || (!descending && o < 0.0) {
            break;
        }
        run += 1;
    }
    if descending {
        list[..run].reverse();
    }
    // a inserção binária do resto
    for start in run..n {
        let pivot = list[start].clone();
        let (mut left, mut right) = (0, start);
        while left < right {
            let mid = left + ((right - left) >> 1);
            if order(&pivot, &list[mid]) < 0.0 {
                right = mid;
            } else {
                left = mid + 1;
            }
        }
        list[left..=start].rotate_right(1);
    }
}

/// `danoMedioDaExpr`: o dano médio de uma expressão (`2d6 +3`), contando os
/// dados e o fixo. `None` quando não há nada a contar.
pub fn average_damage_of_expr(expr: &str) -> Option<f64> {
    if expr.is_empty() {
        return None;
    }
    let t: Vec<char> = expr.to_lowercase().chars().collect();
    let mut total = 0.0;
    let mut found = false;
    let mut rest = String::new();
    let mut i = 0;
    while i < t.len() {
        match match_dice(&t, i) {
            Some((end, count, faces)) => {
                let n = parse_int(
                    &count
                        .chars()
                        .filter(|c| !super::value::is_js_space(*c))
                        .collect::<String>(),
                );
                let faces = parse_int(&faces).filter(|f| *f != 0.0).unwrap_or(6.0);
                let q = match n {
                    Some(n) if n != 0.0 => n,
                    _ => 1.0,
                };
                total += q * ((faces + 1.0) / 2.0);
                found = true;
                rest.push(' ');
                i = end;
            }
            None => {
                rest.push(t[i]);
                i += 1;
            }
        }
    }
    // o fixo: o que sobra depois de tirar os grupos de dado
    let r: Vec<char> = rest.chars().collect();
    let mut i = 0;
    while i < r.len() {
        if r[i] == '+' || r[i] == '-' {
            let mut j = i + 1;
            while j < r.len() && super::value::is_js_space(r[j]) {
                j += 1;
            }
            let d0 = j;
            while j < r.len() && r[j].is_ascii_digit() {
                j += 1;
            }
            if j > d0 {
                let digits: String = r[d0..j].iter().collect();
                let v = parse_int(&digits).unwrap_or(0.0);
                total += if r[i] == '-' { -v } else { v };
                found = true;
                i = j;
                continue;
            }
        }
        i += 1;
    }
    found.then_some(total)
}

/// `/([+-]?\s*\d*)\s*d\s*(\d+)/` a partir de `i`: o fim, o primeiro grupo e
/// os dígitos das faces.
fn match_dice(t: &[char], i: usize) -> Option<(usize, String, String)> {
    let mut j = i;
    if j < t.len() && (t[j] == '+' || t[j] == '-') {
        j += 1;
    }
    while j < t.len() && super::value::is_js_space(t[j]) {
        j += 1;
    }
    while j < t.len() && t[j].is_ascii_digit() {
        j += 1;
    }
    let group: String = t[i..j].iter().collect();
    while j < t.len() && super::value::is_js_space(t[j]) {
        j += 1;
    }
    if j >= t.len() || t[j] != 'd' {
        return None;
    }
    j += 1;
    while j < t.len() && super::value::is_js_space(t[j]) {
        j += 1;
    }
    let f0 = j;
    while j < t.len() && t[j].is_ascii_digit() {
        j += 1;
    }
    if j == f0 {
        return None;
    }
    Some((j, group, t[f0..j].iter().collect()))
}

/// `parseInt(texto, 10)` de um texto já sem espaços: o sinal e os dígitos do
/// começo; `None` (o `NaN`) sem dígito.
fn parse_int(s: &str) -> Option<f64> {
    let (sign, body) = match s.as_bytes().first() {
        Some(b'+') => (1.0, &s[1..]),
        Some(b'-') => (-1.0, &s[1..]),
        _ => (1.0, s),
    };
    let digits: String = body.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<f64>().ok().map(|x| sign * x)
}

impl SheetTables {
    /// `qaDaPeca`: o Quase-Acerto da arma (pelo catálogo, ou pela expressão
    /// do dano) e o das armaduras.
    pub fn near_miss(&self, weapon: Option<&Value>, damage: &str, armors: &[Value]) -> NearMiss {
        let q = self.rules.near_miss.as_ref();
        let limits = q.and_then(|q| q.class_by_average_damage);
        let light = limits.and_then(|l| l.light_up_to).unwrap_or(2.0);
        let medium = limits.and_then(|l| l.medium_up_to).unwrap_or(5.5);
        let from_catalog = self.catalog.weapon_lookup(weapon).and_then(|w| {
            let dice = to_number(get(Some(w), "dado"));
            let b = to_number(get(Some(w), "danoBonus"));
            let b = if b.is_nan() { 0.0 } else { b };
            (dice.is_finite() && dice > 0.0).then_some(dice * 3.5 + b)
        });
        let avg = from_catalog.or_else(|| average_damage_of_expr(damage));
        let class = avg.map(|m| {
            if m <= light {
                "leve"
            } else if m <= medium {
                "media"
            } else {
                "pesada"
            }
        });
        let w = class.and_then(|c| q.and_then(|q| q.by_weapon_class.get(c)));
        let z = |x: Option<f64>| x.filter(|v| !v.is_nan()).unwrap_or(0.0);
        let mut nm = NearMiss {
            weapon_bonus: z(w.and_then(|w| w.bonus)),
            weapon_damage: z(w.and_then(|w| w.damage)),
            weapon_class: class.map(str::to_owned),
            armor_bonus: 0.0,
            armor_reduction: 0.0,
            armor_classes: Vec::new(),
        };
        for p in armors {
            let c = or(get(Some(p), "classe"), None);
            let c = if truthy(c) {
                property_key(c)
            } else {
                "nenhuma".to_owned()
            };
            let Some(t) = q.and_then(|q| q.by_armor_class.get(&c)) else {
                continue;
            };
            nm.armor_bonus += z(t.bonus);
            nm.armor_reduction = js::max(nm.armor_reduction, z(t.reduction));
            nm.armor_classes.push(c);
        }
        nm
    }
}

fn signed(n: f64) -> String {
    format!(
        "{}{}",
        if n >= 0.0 { "+" } else { "\u{2212}" },
        js::number_to_string(n.abs())
    )
}

/// `combatePersonagem`: o resumo de combate que a mesa e o Grid leem.
///
/// Como no TypeScript, o passo, os Atributos e as Habilidades saem da ficha
/// recebida (`attrs`, `skills`, `skills2`), e não da ficha lida: numa ficha na
/// forma gravada eles saem vazios.
pub fn combat_summary(t: &SheetTables, sheet: &Value) -> CombatSummary {
    let n = sheet_numbers(t, sheet);
    let a = &n.hands;
    let w = Some(&a.attack_weapon);
    let attack = format!(
        "{}d6{}{}",
        js::number_to_string(a.dice),
        if or0(a.roll_bonus) != 0.0 { "+2" } else { "" },
        if or0(a.flat) != 0.0 {
            format!(" {}", signed(a.flat))
        } else {
            String::new()
        }
    );
    let mut modes: Vec<Value> = match get(w, "modos") {
        Some(Value::Array(m)) => m.clone(),
        _ => {
            let mut m = Map::new();
            super::value::set(&mut m, "tipo", get(w, "tipoDano").cloned());
            m.insert("principal".into(), json!(true));
            vec![Value::Object(m)]
        }
    };
    v8_sort(&mut modes, |x, y| mode_order(x) - mode_order(y));
    let chosen = get(w, "fichaModo");
    let main = modes
        .iter()
        .find(|m| strict_eq(get(Some(m), "tipo"), chosen))
        .or_else(|| modes.iter().find(|m| truthy(get(Some(m), "principal"))))
        .or_else(|| modes.first());
    let main_type = get(main, "tipo");
    let sigla = DAMAGE_MODES
        .iter()
        .find(|(id, _, _)| is_str(main_type, id))
        .map_or("", |(_, _, s)| s);
    let ap = a.versions[0].bonus;
    let damage = format!(
        "{}d6{}{}",
        to_js_string(get(w, "dado")),
        if or0(ap) != 0.0 {
            format!(" {}", signed(ap))
        } else {
            String::new()
        },
        if sigla.is_empty() {
            String::new()
        } else {
            format!(" {sigla}")
        }
    );
    let raw = Some(sheet);
    let at = or(get(raw, "attrs"), None)
        .cloned()
        .unwrap_or_else(|| json!({}));
    let hab = or(get(raw, "skills"), None)
        .cloned()
        .unwrap_or_else(|| json!({}));
    let sec = or(get(raw, "skills2"), None)
        .cloned()
        .unwrap_or_else(|| json!({}));
    let (atv, habv, secv) = (Some(&at), Some(&hab), Some(&sec));
    let athletics = num_or0(or(get(habv, "atletismo"), get(secv, "atletismo")));
    let strength = num_or0(get(atv, "forca"));
    let dexterity = num_or0(get(atv, "destreza"));
    let spark = num_or0(get(raw, "centelha"));
    let fraction = t
        .ancestry(get(raw, "raca"))
        .and_then(|r| r.movement_fraction)
        .unwrap_or(1.0);
    let dz = movement(
        t,
        |k| match k {
            "forca" => Some(strength),
            "destreza" => Some(dexterity),
            "atletismo" => Some(athletics),
            "centelha" => Some(spark),
            _ => None,
        },
        fraction,
    );
    let pen = ((a.armor_penalty + a.penalty) / 2.0).floor();
    let step = Step {
        battle: js::max(0.0, dz.normal - pen),
        dash: js::max(0.0, dz.dash - pen),
        run: js::max(0.0, dz.run - pen),
    };
    let weapon_pierce = is_str(main_type, "perfurante").then(|| num_or(get(main, "perf"), 0.0));
    let origin = get(w, "origem");
    let qa_weapon = if truthy(origin) {
        Some(json!(""))
    } else {
        or(get(w, "id"), get(w, "nome")).cloned()
    };
    let qa_expr = if truthy(origin) && to_number(get(w, "dado")) > 0.0 {
        let b = get(w, "danoBonus");
        let bonus = if truthy(b) {
            let x = to_number(b);
            format!(
                "{}{}",
                if x > 0.0 { "+" } else { "-" },
                js::number_to_string(x.abs())
            )
        } else {
            String::new()
        };
        format!("{}d6{bonus}", to_js_string(get(w, "dado")))
    } else {
        damage.clone()
    };
    let near_miss = t.near_miss(qa_weapon.as_ref(), &qa_expr, &t.catalog.worn_armor(sheet));
    let ticks = to_number(get(w, "ticks"));
    let mut skills = spread(secv);
    spread_into(&mut skills, habv);
    CombatSummary {
        weapon_name: text_of(get(w, "nome")),
        attack,
        damage,
        dodge: n.dodge,
        mental_defense: n.mental_defense,
        soak: n.soak,
        pierce_level: or0(n.armor.pierce_level),
        weapon_pierce,
        near_miss,
        step,
        attributes: at,
        skills: Value::Object(skills),
        speed: ticks.is_finite().then_some(ticks),
        class: Some(
            coalesce(get(w, "classe"), None)
                .cloned()
                .unwrap_or(Value::Null),
        ),
        charge_strength: a.charge_strength,
    }
}

fn pool(attribute: f64, skill: f64) -> (f64, f64) {
    let sum = attribute + skill;
    (
        (sum / 2.0).floor(),
        if sum % 2.0 == 1.0 { 2.0 } else { 0.0 },
    )
}

/// `resumoFicha`: tudo o que o mestre vê de um personagem.
pub fn sheet_summary(t: &SheetTables, sheet: &Value) -> SheetSummary {
    let s = for_reading(&t.catalog, sheet);
    let sv = Some(&s);
    let attrs = get(sv, "attrs");
    let skills = get(sv, "skills");
    let attr = |k: &str| num_or0(get(attrs, k));
    let skill = |k: &str| num_or0(get(skills, k));
    let c = num_or0(get(sv, "centelha"));
    let will = num_or0(get(sv, "willpower"));
    let nums = sheet_numbers(t, &s);
    let arm = stack_armor(&t.catalog.worn_armor(&s));
    let combat = combat_summary(t, &s);
    let passive = |id: &'static str, name: &'static str, a: f64, h: f64| {
        let (dice, bonus) = pool(a, h);
        Passive {
            id,
            name,
            dice,
            bonus,
            value: (a + h) * 2.0 + spark_in_roll(c, h),
            text: format!(
                "{}d6{}",
                js::number_to_string(dice),
                if bonus != 0.0 {
                    format!(" + {}", js::number_to_string(bonus))
                } else {
                    String::new()
                }
            ),
        }
    };
    let passives = vec![
        passive(
            "prontidao",
            "Prontidão",
            js::max(attr("percepcao"), attr("raciocinio")),
            skill("prontidao"),
        ),
        passive(
            "investigacao",
            "Investigação",
            attr("percepcao"),
            skill("investigacao"),
        ),
        passive("empatia", "Empatia", attr("perspicacia"), skill("empatia")),
        passive(
            "furtividade",
            "Furtividade",
            attr("destreza"),
            skill("furtividade"),
        ),
        passive(
            "sobrevivencia",
            "Sobrevivência",
            attr("percepcao"),
            skill("sobrevivencia"),
        ),
        passive(
            "ocultismo",
            "Ocultismo",
            attr("inteligencia"),
            skill("ocultismo"),
        ),
        passive(
            "conhecimentos",
            "Conhecimentos",
            attr("inteligencia"),
            skill("conhecimentos"),
        ),
        passive("atletismo", "Atletismo", attr("forca"), skill("atletismo")),
        passive(
            "resistencia",
            "Resistência",
            attr("vigor"),
            skill("resistencia"),
        ),
        passive("integridade", "Integridade", will, skill("integridade")),
    ];
    let text_or_empty = |v: Option<&Value>| {
        if truthy(v) {
            text_of(v).unwrap_or_default()
        } else {
            String::new()
        }
    };
    let obj_or_empty = |v: Option<&Value>| or(v, None).cloned().unwrap_or_else(|| json!({}));
    let id = get(sv, "id");
    let feats = match or(get(sv, "tech"), None) {
        Some(Value::Object(m)) => super::value::keys(m)
            .into_iter()
            .filter(|k| truthy(m.get(k.as_str())))
            .cloned()
            .collect(),
        _ => Vec::new(),
    };
    let arts = match or(get(sv, "arte"), None) {
        Some(Value::Object(m)) => super::value::keys(m)
            .into_iter()
            .filter(|k| to_number(m.get(k.as_str())) > 0.0)
            .map(|k| ArtLevel {
                id: k.clone(),
                level: m[k.as_str()].clone(),
            })
            .collect(),
        _ => Vec::new(),
    };
    let armors = match or(get(get(sv, "equip"), "armaduras"), None) {
        Some(Value::Array(list)) => list
            .iter()
            .filter(|p| !matches!(get(Some(p), "vestida"), Some(Value::Bool(false))))
            .map(|p| text_or_empty(or(get(Some(p), "nome"), get(Some(p), "base"))))
            .collect(),
        _ => Vec::new(),
    };
    SheetSummary {
        name: text_or_empty(get(id, "nome")),
        concept: text_or_empty(get(id, "conceito")),
        player: text_or_empty(get(id, "jogador")),
        ancestry: text_or_empty(get(sv, "raca")),
        spark: c,
        willpower: will,
        appearance: num_or0(get(sv, "aparencia")),
        attributes: obj_or_empty(attrs),
        skills: obj_or_empty(skills),
        secondary_skills: obj_or_empty(get(sv, "skills2")),
        virtues: obj_or_empty(get(sv, "virtues")),
        hp: nums.hp,
        energy: nums.energy,
        mana: nums.mana,
        dodge: nums.dodge,
        block: nums.block,
        social_defense: nums.social_defense,
        mental_defense: nums.mental_defense,
        soak: nums.soak,
        pierce_level: or0(arm.pierce_level),
        physical_penalty: or0(arm.penalty),
        initiative: nums.initiative.text.clone(),
        initiative_average: 3.5 + nums.initiative.bonus,
        weapon: combat
            .weapon_name
            .clone()
            .filter(|n| !n.is_empty())
            .unwrap_or_default(),
        attack: combat.attack.clone(),
        damage: combat.damage.clone(),
        passives,
        feats,
        arts,
        armors,
        near_miss: combat.near_miss,
    }
}

/// `resumoParaBanco`: o recorte do resumo que vai para o banco.
pub fn db_summary(r: &SheetSummary) -> DbSummary {
    DbSummary {
        hp: r.hp,
        energy: r.energy,
        mana: r.mana,
        weapon: r.weapon.clone(),
        attack: r.attack.clone(),
        damage: r.damage.clone(),
        dodge: r.dodge,
        block: r.block,
        social_defense: r.social_defense,
        mental_defense: r.mental_defense,
        soak: r.soak,
        pierce_level: r.pierce_level,
        initiative: r.initiative.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn average_damage_of_expressions() {
        assert_eq!(average_damage_of_expr("2d6 +3"), Some(10.0));
        assert_eq!(average_damage_of_expr("1d6 +6 (C)"), Some(9.5));
        // o sinal de menos tipográfico não é lido como fixo
        assert_eq!(average_damage_of_expr("1d6 \u{2212}1 (C)"), Some(3.5));
        assert_eq!(average_damage_of_expr("-d6"), Some(3.5));
        assert_eq!(average_damage_of_expr("d0+1"), Some(4.5));
        assert_eq!(average_damage_of_expr("4"), None);
        assert_eq!(average_damage_of_expr("+ 4"), Some(4.0));
        assert_eq!(average_damage_of_expr(""), None);
    }

    #[test]
    fn sort_like_v8() {
        // o "x" empata com todos (comparador incoerente): os resultados são os
        // que o Node dá
        let cases: [([&str; 4], [&str; 4]); 3] = [
            (
                ["perfurante", "x", "impacto", "corte"],
                ["perfurante", "x", "impacto", "corte"],
            ),
            (
                ["corte", "x", "perfurante", "impacto"],
                ["corte", "x", "impacto", "perfurante"],
            ),
            (
                ["perfurante", "corte", "x", "impacto"],
                ["impacto", "corte", "perfurante", "x"],
            ),
        ];
        for (input, want) in cases {
            let mut v: Vec<Value> = input.iter().map(|t| json!({ "tipo": t })).collect();
            v8_sort(&mut v, |a, b| mode_order(a) - mode_order(b));
            let got: Vec<&str> = v.iter().map(|m| m["tipo"].as_str().unwrap()).collect();
            assert_eq!(got, want, "{input:?}");
        }
    }
}
