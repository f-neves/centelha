//! O tempo do combate, a parte que a ficha usa: a régua P/G/R (o Preparo pela
//! classe e pela Velocidade, a anatomia da ação), a classe de tempo da arma e o
//! teto da rajada (port de `combate-tempo.ts`).
//!
//! Trazido do ramo `backend/tempo` (commit `b50a010`, a primeira parte do port
//! do tempo, que ficou em rascunho quando o Grid saiu do port): só a régua. A
//! agenda, as fases, a escada, a entrada e a fila ficam lá, para a fase 5.
//!
//! Os padrões das chaves ausentes são os do TypeScript (`?? 2`, `?? -4`...),
//! escritos junto de cada leitura.

use centelha_data::Rules;
use centelha_data::Weapon;
use centelha_data::bestiary::AttackClass;
use centelha_data::rules::FlurryRules;

use crate::js;

// ------------------------------------------------------------ a mesa

/// O sistema de tempo da mesa (`Sistema`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSystem {
    /// `normal`: a ação custa a Velocidade e resolve no primeiro Tick.
    Normal,
    /// `pgr`: a mesma Velocidade partida em Preparo, Golpe e Recuperação.
    Pgr,
    /// `simultaneo`: a física do P/G/R, um Tick por vez.
    Simultaneous,
}

impl TimeSystem {
    pub fn id(self) -> &'static str {
        match self {
            TimeSystem::Normal => "normal",
            TimeSystem::Pgr => "pgr",
            TimeSystem::Simultaneous => "simultaneo",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "normal" => Some(TimeSystem::Normal),
            "pgr" => Some(TimeSystem::Pgr),
            "simultaneo" => Some(TimeSystem::Simultaneous),
            _ => None,
        }
    }

    /// `fisicaDe`: o simultâneo usa a régua do P/G/R.
    pub fn physics(self) -> Physics {
        match self {
            TimeSystem::Normal => Physics::Normal,
            TimeSystem::Pgr | TimeSystem::Simultaneous => Physics::Pgr,
        }
    }
}

/// A física de um sistema: que régua de Preparo ele usa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Physics {
    Normal,
    Pgr,
}

impl Physics {
    pub fn id(self) -> &'static str {
        match self {
            Physics::Normal => "normal",
            Physics::Pgr => "pgr",
        }
    }
}

// ------------------------------------------------------------ a régua

/// A classe de tempo de uma arma (`ClasseArma`): as classes do catálogo e a
/// `arte`. As duas hastes do bestiário (`haste-media`, `haste-guerra`) não têm
/// linha nas tabelas do tempo; elas caem nos padrões, como no TypeScript.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeClass {
    Light,
    Medium,
    Polearm,
    Heavy,
    Ranged,
    Thrown,
    Art,
    MediumPolearm,
    WarPolearm,
}

impl TimeClass {
    pub fn id(self) -> &'static str {
        match self {
            TimeClass::Light => "leve",
            TimeClass::Medium => "media",
            TimeClass::Polearm => "haste",
            TimeClass::Heavy => "pesada",
            TimeClass::Ranged => "distancia",
            TimeClass::Thrown => "arremesso",
            TimeClass::Art => "arte",
            TimeClass::MediumPolearm => "haste-media",
            TimeClass::WarPolearm => "haste-guerra",
        }
    }

    /// Todas as ids, inclusive as duas hastes.
    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "leve" => TimeClass::Light,
            "media" => TimeClass::Medium,
            "haste" => TimeClass::Polearm,
            "pesada" => TimeClass::Heavy,
            "distancia" => TimeClass::Ranged,
            "arremesso" => TimeClass::Thrown,
            "arte" => TimeClass::Art,
            "haste-media" => TimeClass::MediumPolearm,
            "haste-guerra" => TimeClass::WarPolearm,
            _ => return None,
        })
    }

    /// As classes que a classe explícita do `classeDeTempo` aceita.
    fn explicit(id: &str) -> Option<Self> {
        match id {
            "haste-media" | "haste-guerra" => None,
            _ => Self::from_id(id),
        }
    }
}

impl From<AttackClass> for TimeClass {
    fn from(c: AttackClass) -> Self {
        match c {
            AttackClass::Light => TimeClass::Light,
            AttackClass::Medium => TimeClass::Medium,
            AttackClass::Heavy => TimeClass::Heavy,
            AttackClass::Polearm => TimeClass::Polearm,
            AttackClass::MediumPolearm => TimeClass::MediumPolearm,
            AttackClass::WarPolearm => TimeClass::WarPolearm,
            AttackClass::Ranged => TimeClass::Ranged,
            AttackClass::Thrown => TimeClass::Thrown,
        }
    }
}

/// `armaDoCatalogo`: a arma pelo id, senão pelo nome (a primeira com o nome).
/// Texto vazio não acha nada. Com id repetido, vale a última peça (o índice
/// por id do TypeScript é um objeto montado em ordem); hoje os ids do
/// catálogo são únicos.
pub fn catalog_weapon<'a>(weapons: &'a [Weapon], id_or_name: Option<&str>) -> Option<&'a Weapon> {
    let key = id_or_name.filter(|s| !s.is_empty())?;
    weapons
        .iter()
        .rev()
        .find(|w| w.id == key)
        .or_else(|| weapons.iter().find(|w| w.name == key))
}

/// `classeDaArma`: a classe do catálogo; `leve` sem arma.
pub fn weapon_class(weapons: &[Weapon], id_or_name: Option<&str>) -> TimeClass {
    catalog_weapon(weapons, id_or_name)
        .and_then(|w| w.stats.as_ref())
        .map_or(TimeClass::Light, |s| s.class.into())
}

/// `classeDeTempo`: a classe explícita, senão a do catálogo, senão pela
/// Velocidade (5 ou menos leve, 6 média, o resto pesada; sem Velocidade, 5).
pub fn time_class(
    weapons: &[Weapon],
    weapon: Option<&str>,
    speed: Option<f64>,
    explicit: Option<&str>,
) -> TimeClass {
    if let Some(c) = explicit.and_then(TimeClass::explicit) {
        return c;
    }
    if catalog_weapon(weapons, weapon).is_some() {
        return weapon_class(weapons, weapon);
    }
    let v = speed.unwrap_or(5.0);
    if v <= 5.0 {
        TimeClass::Light
    } else if v == 6.0 {
        TimeClass::Medium
    } else {
        TimeClass::Heavy
    }
}

/// `velocidadeDaArma`: os Ticks do catálogo, senão o padrão (o TypeScript usa
/// 5 quando quem chama não diz).
pub fn weapon_speed(weapons: &[Weapon], id_or_name: Option<&str>, default: f64) -> f64 {
    catalog_weapon(weapons, id_or_name)
        .and_then(|w| w.stats.as_ref())
        .and_then(|s| s.ticks)
        .unwrap_or(default)
}

/// `preparoDe`: o Preparo pela tabela `combate.pgr.preparo`, entre 0 e a
/// Velocidade menos 1; zero no sistema normal e na classe sem linha.
pub fn windup_of(rules: &Rules, class: TimeClass, speed: f64, system: TimeSystem) -> f64 {
    if system.physics() != Physics::Pgr {
        return 0.0;
    }
    let Some(r) = rules
        .combat
        .pgr
        .as_ref()
        .and_then(|p| p.windup.as_ref())
        .and_then(|t| t.get(class.id()))
    else {
        return 0.0;
    };
    let p = r
        .fixed
        .unwrap_or_else(|| speed + js::or_zero(r.from_speed.unwrap_or(0.0)));
    js::max(0.0, js::min(p, js::max(0.0, speed - 1.0)))
}

fn flurry(rules: &Rules) -> Option<&FlurryRules> {
    rules.combat.flurry.as_ref()
}

/// `tetoDaRajada`: o teto de golpes da rajada pela classe (sem linha, 1).
pub fn flurry_cap(rules: &Rules, class: TimeClass) -> f64 {
    flurry(rules)
        .and_then(|f| f.cap.as_ref())
        .and_then(|t| t.get(class.id()).copied().flatten())
        .unwrap_or(1.0)
}

/// `cicloExtraDaDupla`: os Ticks que o ciclo cresce na empunhadura dupla.
pub fn dual_wield_extra_cycle(rules: &Rules, class: TimeClass, system: TimeSystem) -> f64 {
    let d = rules.combat.dual_wield.as_ref();
    if system.physics() != Physics::Pgr {
        return d.and_then(|d| d.extra_cycle_normal).unwrap_or(0.0);
    }
    d.and_then(|d| d.extra_cycle.as_ref())
        .and_then(|t| t.get(class.id()).copied().flatten())
        .unwrap_or(0.0)
}

/// A manobra da ação (`Manobra`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Maneuver {
    /// `simples`: um golpe.
    Single,
    /// `segura`: duas armas, golpe com uma só (a outra alivia a escada).
    OffHandGuard,
    /// `dupla`: um golpe com cada mão.
    DualWield,
    /// `rajada`: golpes seguidos.
    Flurry,
}

impl Maneuver {
    pub fn id(self) -> &'static str {
        match self {
            Maneuver::Single => "simples",
            Maneuver::OffHandGuard => "segura",
            Maneuver::DualWield => "dupla",
            Maneuver::Flurry => "rajada",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "simples" => Some(Maneuver::Single),
            "segura" => Some(Maneuver::OffHandGuard),
            "dupla" => Some(Maneuver::DualWield),
            "rajada" => Some(Maneuver::Flurry),
            _ => None,
        }
    }
}

/// A ação desmontada (`Anatomia`): Preparo, golpes, Recuperação, ciclo, a
/// penalidade de dados de cada golpe e em que Tick cada um sai, contado da
/// declaração (0 = já).
#[derive(Debug, Clone, PartialEq)]
pub struct Anatomy {
    pub windup: f64,
    pub blows: f64,
    pub recovery: f64,
    pub cycle: f64,
    pub dice_penalties: Vec<f64>,
    pub offsets: Vec<f64>,
    pub warning: Option<String>,
}

/// A entrada da `anatomia`. `speed` e `blows` ausentes (ou 0) valem o padrão
/// do TypeScript (`|| 1`, `|| 2`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnatomyInput {
    pub class: TimeClass,
    pub speed: Option<f64>,
    pub system: TimeSystem,
    pub maneuver: Option<Maneuver>,
    pub blows: Option<f64>,
}

/// `x || d` com um número opcional.
fn or_default(x: Option<f64>, d: f64) -> f64 {
    match x {
        Some(v) if !v.is_nan() && v != 0.0 => v,
        _ => d,
    }
}

/// `anatomia`: a régua limpa (sem o P/G/R do mestre, que é [`with_override`]).
pub fn anatomy(rules: &Rules, input: &AnatomyInput) -> Anatomy {
    let class = input.class;
    let speed = js::max(1.0, or_default(input.speed, 1.0));
    let windup = windup_of(rules, class, speed, input.system);
    match input.maneuver.unwrap_or(Maneuver::Single) {
        Maneuver::Flurry => {
            let f = flurry(rules);
            let cap = flurry_cap(rules, class);
            let wanted = or_default(input.blows, 2.0);
            let mut n = js::max(1.0, js::min(wanted, cap));
            let mut warning = (wanted > cap).then(|| {
                format!(
                    "A rajada da arma {} para em {} golpes.",
                    class.id(),
                    js::number_to_string(cap)
                )
            });
            if f.and_then(|f| f.melee_only).unwrap_or(false)
                && matches!(
                    class,
                    TimeClass::Ranged | TimeClass::Thrown | TimeClass::Art
                )
            {
                n = 1.0;
                warning = Some("A rajada é só corpo a corpo.".to_owned());
            }
            let extras = n - 1.0;
            let pen = f.and_then(|f| f.dice_per_extra_blow).unwrap_or(-1.0);
            let cycle = speed + f.and_then(|f| f.speed_per_extra_blow).unwrap_or(2.0) * extras;
            let len = js::array_length(n);
            let accumulates = f.and_then(|f| f.penalty_accumulates).unwrap_or(false);
            Anatomy {
                windup,
                blows: n,
                recovery: js::max(0.0, cycle - windup - n),
                cycle,
                offsets: (0..len).map(|i| windup + i as f64).collect(),
                warning,
                dice_penalties: (0..len)
                    .map(|i| {
                        if accumulates {
                            pen * i as f64
                        } else if i > 0 {
                            pen
                        } else {
                            0.0
                        }
                    })
                    .collect(),
            }
        }
        Maneuver::DualWield => {
            let d = rules.combat.dual_wield.as_ref();
            let cycle = speed + dual_wield_extra_cycle(rules, class, input.system);
            let pen = d.and_then(|d| d.dice_penalty).unwrap_or(-1.0);
            let both = d.and_then(|d| d.penalty_both_hands).unwrap_or(false);
            Anatomy {
                windup,
                blows: 2.0,
                recovery: js::max(0.0, cycle - windup - 2.0),
                cycle,
                offsets: vec![windup, windup + 1.0],
                dice_penalties: vec![pen, if both { pen } else { 0.0 }],
                warning: None,
            }
        }
        // a simples e a segura têm a mesma anatomia; muda a escada
        Maneuver::Single | Maneuver::OffHandGuard => Anatomy {
            windup,
            blows: 1.0,
            recovery: js::max(0.0, speed - windup - 1.0),
            cycle: speed,
            offsets: vec![windup],
            dice_penalties: vec![0.0],
            warning: None,
        },
    }
}
