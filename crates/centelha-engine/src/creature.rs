//! Os derivados da criatura: PV, Defesas, Absorção, peso erguido, Tamanho,
//! faixa de massa, Energia, Iniciativa, Mana, os ataques e o equipamento.
//!
//! Port de `criatura()` e `comForma()` do motor TypeScript, com a mesma
//! aritmética (tudo em `f64`, na mesma ordem de operações). Quem confere é o
//! teste de paridade contra `tests/parity/creatures.json`.

use std::collections::{BTreeMap, HashMap};

use centelha_data::armor::ArmorStats;
use centelha_data::bestiary::{
    Attack, CarapaceProfile, Category, Creature, Form, HpLine, Size, Swarm,
};
use centelha_data::shields::ShieldStats;
use centelha_data::weapons::WeaponStats;
use centelha_data::{Armor, Rules, Shield, Weapon};

use crate::js;

/// As tabelas que os derivados leem: as regras e os catálogos de armas,
/// armaduras e escudos, já lidos pelo `centelha-data`.
#[derive(Debug, Clone)]
pub struct Tables {
    rules: Rules,
    weapons: Vec<Weapon>,
    armors: HashMap<String, ArmorStats>,
    shields: HashMap<String, ShieldStats>,
}

impl Tables {
    /// Monta as tabelas. Como no TypeScript, a arma é procurada pela primeira
    /// peça com o id; armadura e escudo entram só se têm o bloco de números, e
    /// o id repetido fica com a última peça.
    pub fn new(
        rules: Rules,
        weapons: Vec<Weapon>,
        armors: Vec<Armor>,
        shields: Vec<Shield>,
    ) -> Self {
        let armors = armors
            .into_iter()
            .filter_map(|a| a.stats.map(|s| (a.id, s)))
            .collect();
        let shields = shields
            .into_iter()
            .filter_map(|s| s.stats.map(|x| (s.id, x)))
            .collect();
        Tables {
            rules,
            weapons,
            armors,
            shields,
        }
    }

    /// As regras.
    pub fn rules(&self) -> &Rules {
        &self.rules
    }

    fn weapon(&self, id: &str) -> Option<(&Weapon, &WeaponStats)> {
        let w = self.weapons.iter().find(|w| w.id == id)?;
        Some((w, w.stats.as_ref()?))
    }
}

/// Uma Defesa que pode não existir: a criatura sem mente (Inteligência 0) é
/// imune à Mental e à Social.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Defense {
    Value(f64),
    Immune,
}

/// A Absorção por categoria.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Soak {
    pub impact: f64,
    pub cut: f64,
    pub pierce: f64,
}

/// O enxame num quadrado.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwarmState {
    /// O PV do quadrado cheio.
    pub full_hp: f64,
    pub hp: f64,
    pub individuals: f64,
    /// Abaixo deste número de indivíduos o enxame se desfaz.
    pub minimum: f64,
    pub still_swarm: bool,
}

/// Os números de um ataque.
#[derive(Debug, Clone, PartialEq)]
pub struct AttackDerived {
    pub name: String,
    pub accuracy: f64,
    /// A Habilidade da jogada; `None` no ataque de área, que não tem jogada.
    pub skill: Option<String>,
    /// A Velocidade em Ticks.
    pub speed: Option<f64>,
    pub average_damage: Option<f64>,
    /// A soma da jogada de ataque (Atributo + Habilidade + Centelha + Acerto).
    pub roll: Option<f64>,
}

/// As Defesas e a Absorção com o equipamento que a criatura sempre usa.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WithEquipment {
    pub dodge: f64,
    pub block: f64,
    pub soak: Soak,
    pub penalty: f64,
    /// O Nível de Perfuração da armadura, quando ela tem.
    pub pierce_level: Option<f64>,
    /// A Defesa contra o projétil rápido, quando o escudo o bloqueia.
    pub against_shots: Option<f64>,
}

/// Os derivados de uma criatura.
#[derive(Debug, Clone, PartialEq)]
pub struct CreatureDerived {
    pub swarm: Option<SwarmState>,
    pub hp: Option<f64>,
    pub lifted_weight: Option<f64>,
    pub mass_band: Option<f64>,
    pub size: Option<Size>,
    pub second_size: Option<Size>,
    pub dodge: f64,
    pub block: f64,
    pub mental_defense: Defense,
    pub social_defense: Defense,
    /// O bônus da Iniciativa (somado ao 1d6).
    pub initiative: f64,
    pub energy: f64,
    /// `None` em quem não tem Arte.
    pub mana: Option<f64>,
    pub soak: Soak,
    pub attacks: Vec<AttackDerived>,
    pub with_equipment: Option<WithEquipment>,
}

/// O bônus de Mana por nível da Arte Mana.
const MANA_ART_BONUS: [f64; 7] = [0.0, 1.0, 2.0, 3.0, 5.0, 8.0, 12.0];

/// As Habilidades de tiro e arremesso, que a arma ou o ataque gravam.
const RANGED: [&str; 3] = ["disparo", "atirador", "arremesso"];

// ----- as contas pequenas -----

/// `centelhaNaJogada`: 2 × o menor entre a Centelha e a Habilidade (nunca
/// negativo). Com o `Math.max`/`Math.min` do JavaScript: `NaN` dá `NaN`, e
/// `-0` conta como menor que `0`. Serve à criatura e à ficha.
pub fn spark_in_roll(spark: f64, skill: f64) -> f64 {
    2.0 * js::min(js::max(0.0, spark), js::max(0.0, skill))
}

fn skill_of(c: &Creature, id: &str) -> f64 {
    c.skills
        .get(id)
        .or_else(|| c.secondary_skills.get(id))
        .map_or(0.0, |v| f64::from(*v))
}

fn size_index(size: Size) -> usize {
    Size::ALL.iter().position(|s| *s == size).unwrap_or(0)
}

/// O Tamanho pela área da silhueta, em m².
pub fn size_from_area(rules: &Rules, area: f64) -> Size {
    let order = &rules.size_accuracy.order;
    let i = rules.creature.size.cuts_m2.iter().position(|c| area <= *c);
    match i {
        Some(i) => order[i],
        None => order[order.len() - 1],
    }
}

/// A massa que entra nas contas: a do corpo, ou o volume do incorpóreo lido
/// como carne e osso.
fn mass_for_math(rules: &Rules, c: &Creature) -> Option<f64> {
    let b = &c.body;
    if b.massless {
        return b
            .volume
            .map(|v| v * rules.creature.mass_band.incorporeal_density_kg_m3);
    }
    b.mass
}

/// A faixa de massa das manobras, de 1 a 7.
pub fn mass_band(rules: &Rules, kg: f64) -> f64 {
    let cuts = &rules.creature.mass_band.cuts_kg;
    match cuts.iter().position(|c| kg <= *c) {
        Some(i) => (i + 1) as f64,
        None => (cuts.len() + 1) as f64,
    }
}

/// O PV pela massa, com a escolha da linha na zona de transição.
pub fn hp_from_mass(rules: &Rules, kg: f64, stamina: f64, line: Option<HpLine>) -> f64 {
    let rows = &rules.derived.pv.mass.table;
    let mut i = rows
        .iter()
        .position(|r| r.up_to_kg.is_none_or(|top| kg <= top))
        .unwrap_or(rows.len() - 1);
    let zone = rows
        .iter()
        .position(|r| r.zone_kg.is_some_and(|z| kg >= z[0] && kg <= z[1]));
    if let (Some(z), Some(line)) = (zone, line) {
        i = match line {
            HpLine::Above => z + 1,
            HpLine::Below => z,
        };
    }
    let r = &rows[i];
    r.base + ((stamina * r.stamina_mult) / r.stamina_divisor).floor()
}

/// O PV pelo Tamanho (a tabela `derivados.pv.porte`).
pub fn hp_from_size(rules: &Rules, stamina: f64, size: Size) -> f64 {
    let pv = &rules.derived.pv;
    let (base, mult) = match pv.by_size.as_ref().and_then(|t| t.get(&size)) {
        Some(row) => (row.base, row.stamina_mult),
        None => (pv.base, pv.stamina_mult),
    };
    base + stamina * mult
}

/// O peso erguido pela R15, em kg, com duas casas.
pub fn lifted_weight(
    rules: &Rules,
    kg: f64,
    strength: f64,
    weightlifting: f64,
    musculature: f64,
) -> f64 {
    let p = &rules.creature.lifted_weight;
    let l = js::log10(kg / p.curve_mass_kg);
    let curve = p.curve_a * js::pow(10.0, p.curve_b * l - p.curve_c * l * l);
    let f_ref = if kg < p.strength_ref_below_kg {
        p.strength_ref + p.strength_ref_slope * js::log10(kg / p.strength_ref_below_kg)
    } else if kg <= p.strength_ref_above_kg {
        p.strength_ref
    } else {
        p.strength_ref + p.strength_ref_slope * js::log10(kg / p.strength_ref_above_kg)
    };
    let f_ref = js::max(0.0, f_ref);
    let e = (p.e_strength * strength
        + p.e_weightlifting * weightlifting
        + p.e_musculature * musculature)
        / p.e_divisor;
    let e_ref = (p.e_ref_strength * f_ref + p.e_ref_sum) / p.e_divisor;
    js::round(js::max(curve, p.floor * kg) * js::pow(p.base, e - e_ref) * 100.0) / 100.0
}

/// A média do dano: cada d6 vale `porDado`, mais o bônus do dado, a Força ×
/// multiplicador e a Centelha. `Some(0)` no golpe sem dano (`dice` nulo);
/// `None` quando o texto do dado não se lê.
pub fn average_damage(
    rules: &Rules,
    dice: Option<&str>,
    strength: f64,
    strength_multiplier: f64,
    spark: f64,
) -> Option<f64> {
    let Some(dice) = dice else {
        return Some(0.0);
    };
    let (count, bonus) = parse_dice(dice)?;
    Some(
        count * rules.creature.average_damage.per_die
            + bonus
            + strength * strength_multiplier
            + spark,
    )
}

/// `^(\d+)d6([+-]\d+)?$`: o número de dados e o bônus.
fn parse_dice(dice: &str) -> Option<(f64, f64)> {
    let (count, rest) = dice.split_once("d6")?;
    let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    if !digits(count) {
        return None;
    }
    let bonus = match rest.as_bytes().first() {
        None => 0.0,
        Some(b'+') if digits(&rest[1..]) => rest[1..].parse::<f64>().ok()?,
        Some(b'-') if digits(&rest[1..]) => -rest[1..].parse::<f64>().ok()?,
        Some(_) => return None,
    };
    Some((count.parse::<f64>().ok()?, bonus))
}

/// Quanto a Couraça soma à Absorção: o nível vezes a fração do perfil, com as
/// metades para cima. Sem perfil, nada.
pub fn carapace_soak(rules: &Rules, level: f64, profile: Option<CarapaceProfile>) -> Soak {
    let profiles = &rules.creature.carapace.profiles;
    let p = match profile {
        None => {
            return Soak {
                impact: 0.0,
                cut: 0.0,
                pierce: 0.0,
            };
        }
        Some(CarapaceProfile::Rigid) => profiles.rigid,
        Some(CarapaceProfile::Leather) => profiles.leather,
        Some(CarapaceProfile::Soft) => profiles.soft,
    };
    Soak {
        impact: (level * p.impact).ceil(),
        cut: (level * p.cut).ceil(),
        pierce: (level * p.pierce).ceil(),
    }
}

/// O enxame no quadrado cheio.
fn swarm_state(rules: &Rules, c: &Creature, swarm: &Swarm) -> SwarmState {
    let full = swarm.individuals_per_square as f64;
    let present = full;
    let full_hp = hp_from_size(
        rules,
        f64::from(c.attributes.stamina),
        swarm.equivalent_size,
    ) + f64::from(c.hp_bonus.unwrap_or(0));
    SwarmState {
        full_hp,
        hp: (full_hp * present / full).floor(),
        individuals: present,
        minimum: (full / 4.0).ceil(),
        still_swarm: present * 4.0 >= full,
    }
}

// ----- ataques -----

/// O ataque com a arma do livro, já com a Força que ela aproveita.
struct WeaponAttack {
    dice: f64,
    bonus: f64,
    strength: f64,
    strength_multiplier: f64,
    accuracy: f64,
    speed: Option<f64>,
}

/// A arma de outro Tamanho: +N d6 acima do Médio, bônus por categoria abaixo.
fn scale_by_size(rules: &Rules, size: Option<Size>) -> (f64, f64) {
    let Some(size) = size.filter(|s| *s != Size::Medium) else {
        return (0.0, 0.0);
    };
    let k = size_index(size) as f64 - size_index(Size::Medium) as f64;
    let e = &rules.combat.scale_by_size;
    if k > 0.0 {
        (k * e.dice_per_size_above, 0.0)
    } else {
        (0.0, -k * e.bonus_per_size_below)
    }
}

fn weapon_attack(
    t: &Tables,
    id: &str,
    strength: f64,
    two_hands: bool,
    size: Option<Size>,
) -> Option<WeaponAttack> {
    let (item, w0) = t.weapon(id)?;
    let rules = &t.rules;
    let (scale_dice, scale_bonus) = scale_by_size(rules, size);
    // o requisito de Força: quem não o cumpre usa a arma como uma comum
    let below_min = w0.strength_min.is_some_and(|m| m != 0.0 && strength < m);
    let mut mult_field = w0.strength_multiplier;
    let mut bonus = f64::from(w0.damage_bonus.unwrap_or(0));
    if below_min {
        mult_field = Some(1.0);
        bonus = 0.0;
    }
    let mut dice = f64::from(w0.dice);
    if scale_dice != 0.0 || scale_bonus != 0.0 {
        dice += scale_dice;
        bonus += scale_bonus;
    }
    let fm = &rules.derived.strength_damage;
    let has_tag = |tag: &str| item.tags.iter().any(|x| x == tag);
    let mult = if has_tag("distância") {
        mult_field.unwrap_or(1.0)
    } else if w0.hands == Some(2) {
        mult_field.unwrap_or(fm.two_hands)
    } else if has_tag("versátil") && two_hands {
        fm.two_hands
    } else {
        mult_field.unwrap_or(fm.one_hand)
    };
    let capped = match w0.strength_cap {
        Some(cap) => strength.min(cap),
        None => strength,
    };
    let strength = bow_strength(
        rules,
        w0.class == centelha_data::bestiary::AttackClass::Ranged,
        mult,
        capped,
    );
    let speed = match w0.timing {
        Some(p) => Some(f64::from(p.windup) + f64::from(p.blow) + f64::from(p.recovery)),
        None => w0.ticks,
    };
    Some(WeaponAttack {
        dice,
        bonus,
        strength,
        strength_multiplier: mult,
        accuracy: f64::from(w0.accuracy.unwrap_or(0)),
        speed,
    })
}

/// A Força que o arco usa: acima do teto (8) conta como o teto.
fn bow_strength(rules: &Rules, ranged: bool, mult: f64, strength: f64) -> f64 {
    let cap = rules
        .combat
        .ranged
        .and_then(|r| r.maximum)
        .and_then(|m| m.bow_strength_cap)
        .unwrap_or(8.0);
    if ranged && mult > 0.0 {
        strength.min(cap)
    } else {
        strength
    }
}

/// O Atributo do projétil natural: o maior entre Vigor, Destreza e Percepção.
fn natural_projectile_attribute(c: &Creature) -> f64 {
    let a = &c.attributes;
    let (v, d, p) = (a.stamina, a.dexterity, a.perception);
    f64::from(if v >= d && v >= p {
        v
    } else if d >= p {
        d
    } else {
        p
    })
}

/// O Atributo da jogada pela Habilidade.
fn roll_attribute(c: &Creature, skill: &str, natural: bool) -> f64 {
    let a = &c.attributes;
    match skill {
        "disparo" => f64::from(a.stamina),
        "atirador" if natural => natural_projectile_attribute(c),
        "atirador" => f64::from(a.perception),
        _ => f64::from(a.strength.max(a.dexterity)),
    }
}

/// A Habilidade da jogada: a de tiro e arremesso vem da arma ou do ataque; o
/// resto é corpo a corpo, pela maior entre Briga e Armas.
fn roll_skill(t: &Tables, c: &Creature, a: &Attack) -> Option<String> {
    let recorded: Option<&str> = match &a.weapon {
        Some(id) => t.weapon(id).map(|(_, w)| w.skill.as_str()),
        None => a.skill.as_deref(),
    };
    if let Some(r) = recorded.filter(|r| RANGED.contains(r)) {
        return Some(r.to_string());
    }
    use centelha_data::bestiary::AttackClass::{Ranged, Thrown};
    if a.weapon.is_none() && matches!(a.class, Some(Ranged) | Some(Thrown)) {
        return None;
    }
    let get = |id: &str| {
        c.skills
            .get(id)
            .or_else(|| c.secondary_skills.get(id))
            .copied()
            .unwrap_or(0)
    };
    let brawl = get("briga");
    let melee = get("armas");
    // no empate a conta é a mesma; a arma mostra a Habilidade do catálogo dela
    if melee == brawl
        && a.weapon.is_some()
        && let Some(r @ ("armas" | "briga")) = recorded
    {
        return Some(r.to_string());
    }
    Some(if melee > brawl { "armas" } else { "briga" }.to_string())
}

fn attack_derived(t: &Tables, c: &Creature, a: &Attack) -> AttackDerived {
    let rules = &t.rules;
    let spark = f64::from(c.spark);
    let strength = f64::from(c.attributes.strength);
    let w = a
        .weapon
        .as_deref()
        .and_then(|id| weapon_attack(t, id, strength, false, a.size));
    let skill = roll_skill(t, c, a);
    let skill_value = skill.as_deref().map_or(0.0, |s| skill_of(c, s));
    let accuracy = match &w {
        Some(w) => w.accuracy,
        None => f64::from(a.accuracy.unwrap_or(0)),
    };
    let speed = match &w {
        Some(w) => w.speed,
        None => a
            .timing
            .map(|p| f64::from(p.windup) + f64::from(p.blow) + f64::from(p.recovery)),
    };
    let average = match &w {
        // o dado da arma passa pelo mesmo texto `NdM+B` do TypeScript
        Some(w) => average_damage(
            rules,
            Some(&weapon_dice_text(w.dice, w.bonus)),
            w.strength,
            w.strength_multiplier,
            spark,
        ),
        None => average_damage(
            rules,
            a.dice.as_deref(),
            strength,
            f64::from(a.strength_multiplier.unwrap_or(0)),
            spark,
        ),
    };
    let roll = skill.as_deref().map(|s| {
        roll_attribute(c, s, w.is_none())
            + skill_value
            + spark_in_roll(spark, skill_value)
            + accuracy
    });
    AttackDerived {
        name: a.name.clone(),
        accuracy,
        skill,
        speed,
        average_damage: average,
        roll,
    }
}

/// O texto do dado da arma, como o TypeScript o escreve (`2d6`, `1d6+2`,
/// `1d6-2`), para o dano médio ler do mesmo jeito.
fn weapon_dice_text(dice: f64, bonus: f64) -> String {
    let n = js_number(dice);
    if bonus == 0.0 {
        format!("{n}d6")
    } else if bonus > 0.0 {
        format!("{n}d6+{}", js_number(bonus))
    } else {
        format!("{n}d6{}", js_number(bonus))
    }
}

/// O número como texto, como o JavaScript escreve um inteiro (sem `.0`).
fn js_number(x: f64) -> String {
    if x.fract() == 0.0 && x.abs() < 1e15 {
        format!("{}", x as i64)
    } else {
        format!("{x}")
    }
}

// ----- a criatura -----

/// Os derivados de uma criatura (o `criatura()` do TypeScript).
pub fn derive(t: &Tables, c: &Creature) -> CreatureDerived {
    let rules = &t.rules;
    let at = &c.attributes;
    let spark = f64::from(c.spark);
    let body = &c.body;
    let kg = mass_for_math(rules, c);
    let mindless = at.intelligence == 0;
    let dex = f64::from(at.dexterity);
    let defense =
        |skill: f64| (dex + skill) * rules.derived.defense.mult + 0.0 + spark_in_roll(spark, skill);
    let dodge = defense(skill_of(c, "esquiva"));
    let block = defense(skill_of(c, "bloqueio"));

    // Inteligência 1 e o animal usam Sobrevivência no lugar de Sociabilidade
    let social_skill = if c.category == Category::Animal || at.intelligence == 1 {
        skill_of(c, "sobrevivencia")
    } else {
        skill_of(c, "sociabilidade")
    };
    let sd = &rules.derived.social_defense;
    let social = sd
        .traits
        .iter()
        .map(|k| match k.as_str() {
            "compostura" => f64::from(at.composure),
            "sociabilidade" => social_skill,
            _ => 0.0,
        })
        .fold(0.0, |s, v| s + v)
        * sd.mult
        + spark_in_roll(spark, social_skill)
        + 0.0;

    let md = &rules.derived.mental_defense;
    let integrity = skill_of(c, "integridade");
    let mental = integrity * md.mult
        + if md.plus_wits == Some(true) {
            f64::from(at.wits)
        } else {
            0.0
        }
        + if md.plus_willpower == Some(true) {
            f64::from(c.willpower)
        } else {
            0.0
        }
        + spark_in_roll(spark, integrity)
        + 0.0;

    let cs = carapace_soak(rules, f64::from(c.carapace), c.carapace_profile);
    let soak = Soak {
        impact: f64::from(at.stamina) + spark + cs.impact,
        cut: spark + cs.cut,
        pierce: spark + cs.pierce,
    };
    let has_arts = c.arts.values().any(|n| *n > 0);
    let swarm = c.swarm.as_ref().map(|s| swarm_state(rules, c, s));

    let hp = match (&swarm, kg) {
        (Some(s), _) => Some(s.full_hp),
        (None, None) => None,
        (None, Some(kg)) => Some(
            hp_from_mass(rules, kg, f64::from(at.stamina), body.hp_line)
                + f64::from(c.hp_bonus.unwrap_or(0)),
        ),
    };
    let lifted = match kg {
        Some(kg) if !body.cannot_lift => Some(lifted_weight(
            rules,
            kg,
            f64::from(at.strength),
            sec(c, "halterofilismo"),
            sec(c, "musculatura"),
        )),
        _ => None,
    };
    let size = body
        .chosen_size
        .or_else(|| body.area.map(|area| size_from_area(rules, area)));
    let second_size = body.second_size.as_ref().map(|s| {
        s.chosen_size
            .unwrap_or_else(|| size_from_area(rules, s.area))
    });

    // a Iniciativa soma as chaves de `derivados.iniciativa.soma`, procuradas
    // como no TypeScript ({...atributos, ...habilidades, ...habilidadesSec}):
    // a secundária vence a primária, que vence o Atributo
    let initiative = rules
        .derived
        .initiative
        .sum
        .iter()
        .map(|k| {
            c.secondary_skills
                .get(k)
                .or_else(|| c.skills.get(k))
                .copied()
                .or_else(|| at.get(k))
                .map_or(0.0, f64::from)
        })
        .fold(0.0, |s, v| s + v);

    let en = &rules.derived.energy;
    let energy_base = f64::from(at.stamina)
        + f64::from(at.composure)
        + f64::from(at.wits)
        + if en.plus_willpower == Some(true) {
            f64::from(c.willpower)
        } else {
            0.0
        };
    let energy = (energy_base / en.divisor).floor() + spark * en.spark_mult;

    let mana = has_arts.then(|| {
        let m = &rules.derived.mana;
        spark * m.spark_mult
            + if m.plus_willpower == Some(true) {
                f64::from(c.willpower)
            } else {
                0.0
            }
            + MANA_ART_BONUS[0]
    });

    let attacks = c.attacks.iter().map(|a| attack_derived(t, c, a)).collect();

    // o equipamento que a criatura sempre usa: a armadura soma a Absorção e
    // tira a Penalidade da Esquiva e do Bloqueio; o escudo soma ao Bloqueio e
    // tira a Penalidade dele só da Esquiva
    let armor_id = c
        .equipment
        .armors
        .iter()
        .find(|p| p.worn)
        .map(|p| p.base.as_str());
    let shield_id = c
        .equipment
        .arsenal
        .iter()
        .find_map(|p| p.reference.strip_prefix("e:"));
    let with_equipment = (armor_id.is_some_and(|s| !s.is_empty())
        || shield_id.is_some_and(|s| !s.is_empty()))
    .then(|| {
        let arm = armor_id.and_then(|id| t.armors.get(id));
        let esc = shield_id.and_then(|id| t.shields.get(id));
        let pen = arm.and_then(|a| a.penalty).unwrap_or(0.0);
        let pen_s = esc.and_then(|s| s.penalty).unwrap_or(0.0);
        let bl = esc.and_then(|s| s.melee_block).unwrap_or(0.0);
        let armor_soak = |f: fn(&centelha_data::armor::ArmorSoak) -> Option<f64>| {
            arm.and_then(|a| a.soak.as_ref()).and_then(f).unwrap_or(0.0)
        };
        let dodge2 = dodge - pen - pen_s;
        let block2 = block - pen + bl;
        let pierce_level = arm
            .and_then(|a| a.pierce_level)
            .filter(|n| *n != 0.0 && !n.is_nan());
        let against_shots = esc
            .and_then(|s| s.vs_fast_projectile)
            .filter(|v| v.blocks == Some(true))
            .map(|v| js::max(dodge2, block2 + v.bonus.unwrap_or(0.0)));
        WithEquipment {
            dodge: dodge2,
            block: block2,
            soak: Soak {
                impact: soak.impact + armor_soak(|s| s.impact),
                cut: soak.cut + armor_soak(|s| s.cut),
                pierce: soak.pierce + armor_soak(|s| s.pierce),
            },
            penalty: pen,
            pierce_level,
            against_shots,
        }
    });

    CreatureDerived {
        swarm,
        hp,
        lifted_weight: lifted,
        mass_band: kg.map(|kg| mass_band(rules, kg)),
        size,
        second_size,
        dodge,
        block,
        mental_defense: if mindless {
            Defense::Immune
        } else {
            Defense::Value(mental)
        },
        social_defense: if mindless {
            Defense::Immune
        } else {
            Defense::Value(social)
        },
        initiative,
        energy,
        mana,
        soak,
        attacks,
        with_equipment,
    }
}

fn sec(c: &Creature, id: &str) -> f64 {
    c.secondary_skills.get(id).map_or(0.0, |v| f64::from(*v))
}

/// A criatura de uma forma, montada por cima da principal (o `comForma()` do
/// TypeScript): o objeto troca chave por chave (o que a forma não diz vem da
/// principal), a lista troca inteira e o valor troca. Como no TypeScript, o
/// `id` e o `name` passam a ser os da forma.
pub fn with_form(main: &Creature, form: &Form) -> Creature {
    let mut out = main.clone();
    out.id = form.id.clone();
    out.name = form.name.clone();
    if let Some(p) = &form.body {
        let b = &mut out.body;
        if p.mass.is_some() {
            b.mass = p.mass;
        }
        if let Some(v) = p.massless {
            b.massless = v;
        }
        if p.volume.is_some() {
            b.volume = p.volume;
        }
        if p.area.is_some() {
            b.area = p.area;
        }
        if let Some(v) = p.cannot_lift {
            b.cannot_lift = v;
        }
        if p.chosen_size.is_some() {
            b.chosen_size = p.chosen_size;
        }
        if p.hp_line.is_some() {
            b.hp_line = p.hp_line;
        }
        if p.hp_line_reason.is_some() {
            b.hp_line_reason = p.hp_line_reason.clone();
        }
        if p.second_size.is_some() {
            b.second_size = p.second_size.clone();
        }
        // a área e a massa da forma valem por si: a escolha do Tamanho e da
        // linha de PV era da principal
        if p.area.is_some() && p.chosen_size.is_none() {
            b.chosen_size = None;
        }
        if p.mass.is_some() && p.hp_line.is_none() {
            b.hp_line = None;
        }
        // o porquê da linha é da linha
        if (p.mass.is_some() || p.hp_line.is_some()) && p.hp_line_reason.is_none() {
            b.hp_line_reason = None;
        }
    }
    if let Some(n) = form.carapace {
        out.carapace = n;
    }
    if form.carapace_profile.is_some() {
        out.carapace_profile = form.carapace_profile;
    }
    if let Some(p) = &form.attributes {
        let a = &mut out.attributes;
        let set = |slot: &mut i32, v: Option<i32>| {
            if let Some(v) = v {
                *slot = v;
            }
        };
        set(&mut a.strength, p.strength);
        set(&mut a.dexterity, p.dexterity);
        set(&mut a.stamina, p.stamina);
        set(&mut a.perception, p.perception);
        set(&mut a.intelligence, p.intelligence);
        set(&mut a.wits, p.wits);
        set(&mut a.influence, p.influence);
        set(&mut a.insight, p.insight);
        set(&mut a.composure, p.composure);
    }
    if let Some(p) = &form.equipment {
        if let Some(v) = &p.arsenal {
            out.equipment.arsenal = v.clone();
        }
        if let Some(v) = &p.armors {
            out.equipment.armors = v.clone();
        }
        if p.poison.is_some() {
            out.equipment.poison = p.poison.clone();
        }
    }
    if let Some(arts) = &form.arts {
        let merged: &mut BTreeMap<String, i32> = &mut out.arts;
        for (k, v) in arts {
            merged.insert(k.clone(), *v);
        }
    }
    if let Some(attacks) = &form.attacks {
        out.attacks = attacks.clone();
    }
    out
}
