//! O tempo do combate, primeira parte: a régua P/G/R, a ação declarada e a
//! agenda dos golpes, as fases e a escada de Defesa, a entrada pela
//! iniciativa e a fila (port de `combate-tempo.ts`).
//!
//! Tudo é entrada e saída: os números vêm de `regras.json` (bloco `combate` e
//! `derivados.iniciativa`) e do catálogo de armas. Nada aqui sorteia.
//!
//! Os padrões das chaves ausentes são os do TypeScript (`?? 2`, `?? -4`...),
//! escritos junto de cada leitura.

use std::cmp::Ordering;

use centelha_data::Rules;
use centelha_data::Weapon;
use centelha_data::bestiary::AttackClass;
use centelha_data::rules::{FlurryRules, GuardLadder};

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

/// `adiaGolpe`: o golpe sai depois da declaração? Sempre no simultâneo; no
/// `pgr` quando a mesa ligou a chave.
pub fn delays_blow(system: Option<TimeSystem>, delayed_blow: bool) -> bool {
    system == Some(TimeSystem::Simultaneous) || (delayed_blow && system == Some(TimeSystem::Pgr))
}

/// `ehSimultaneo`.
pub fn is_simultaneous(system: Option<TimeSystem>) -> bool {
    system == Some(TimeSystem::Simultaneous)
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

/// O Preparo e o ciclo de uma Arte (`reguaDaArte`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArtTiming {
    pub windup: f64,
    pub cycle: f64,
    pub blow: f64,
    pub recovery: f64,
}

/// `reguaDaArte`: a Arte de grau N sai no último Tick da montagem.
pub fn art_timing(rules: &Rules, level: f64, system: TimeSystem) -> ArtTiming {
    let a = rules.combat.pgr.as_ref().and_then(|p| p.art);
    let get = |f: fn(&centelha_data::rules::ArtTimingRules) -> Option<f64>, d: f64| {
        a.as_ref().and_then(f).unwrap_or(d)
    };
    let cycle = get(|a| a.cycle_base, 3.0) + get(|a| a.cycle_per_level, 1.0) * level;
    let windup = if system.physics() == Physics::Pgr {
        get(|a| a.windup_base, 2.0) + get(|a| a.windup_per_level, 1.0) * level
    } else {
        0.0
    };
    ArtTiming {
        windup,
        cycle,
        blow: 1.0,
        recovery: js::max(0.0, cycle - windup - 1.0),
    }
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

/// O P/G/R escrito à mão pelo mestre (cada parcela opcional).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TimingOverride {
    pub windup: Option<f64>,
    pub blows: Option<f64>,
    pub recovery: Option<f64>,
}

/// `comOverride`: refaz o ciclo e os Ticks dos golpes a partir do P/G/R do
/// mestre. Sem nenhuma parcela, a anatomia volta igual.
pub fn with_override(a: &Anatomy, timing: Option<&TimingOverride>) -> Anatomy {
    let Some(o) =
        timing.filter(|o| o.windup.is_some() || o.blows.is_some() || o.recovery.is_some())
    else {
        return a.clone();
    };
    let windup = js::max(0.0, o.windup.unwrap_or(a.windup));
    let blows = js::max(1.0, o.blows.unwrap_or(a.blows));
    let recovery = js::max(0.0, o.recovery.unwrap_or(a.recovery));
    let len = js::array_length(blows);
    Anatomy {
        windup,
        blows,
        recovery,
        cycle: windup + blows + recovery,
        offsets: (0..len).map(|i| windup + i as f64).collect(),
        dice_penalties: (0..len)
            .map(|i| {
                a.dice_penalties
                    .get(i)
                    .or(a.dice_penalties.last())
                    .copied()
                    .unwrap_or(0.0)
            })
            .collect(),
        warning: a.warning.clone(),
    }
}

/// Quando a ação sem classe resolve (`anatomiaLivre`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreeTiming {
    /// `agora`: no Tick da declaração; o resto do ciclo é Recuperação.
    Now,
    /// `fim`: no último Tick; o caminho todo é Preparo (só no P/G/R).
    End,
}

impl FreeTiming {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "agora" => Some(FreeTiming::Now),
            "fim" => Some(FreeTiming::End),
            _ => None,
        }
    }
}

/// `anatomiaLivre`: a ação que a régua não previu.
pub fn free_anatomy(speed: f64, when: FreeTiming, system: TimeSystem) -> Anatomy {
    let speed = js::max(1.0, or_default(Some(js::round(speed)), 1.0));
    let windup = if system.physics() == Physics::Pgr && when == FreeTiming::End {
        speed - 1.0
    } else {
        0.0
    };
    Anatomy {
        windup,
        blows: 1.0,
        recovery: js::max(0.0, speed - windup - 1.0),
        cycle: speed,
        offsets: vec![windup],
        dice_penalties: vec![0.0],
        warning: None,
    }
}

/// `penDadosDaRegua`: a penalidade de dados do golpe de índice dado (fora da
/// lista, 0).
pub fn ruler_dice_penalty(a: &Anatomy, index: f64) -> f64 {
    if index >= 0.0 && index.fract() == 0.0 {
        a.dice_penalties.get(index as usize).copied().unwrap_or(0.0)
    } else {
        0.0
    }
}

// ------------------------------------------------------------ a entrada

/// Onde uma peça entra na linha de Ticks (`ticksDeEntrada`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Entry {
    pub tick: f64,
    /// O contrapé: os dados a menos (negativo).
    pub dice_penalty: f64,
    /// Quantos pontos atrás da maior iniciativa.
    pub behind: f64,
}

/// `ticksDeEntrada`: quem tirou a maior entra no primeiro Tick; os outros, um
/// Tick por degrau de atraso (arredondado para cima), com o contrapé por
/// degrau. Na mesma ordem da entrada.
pub fn entry_ticks(rules: &Rules, initiatives: &[f64]) -> Vec<Entry> {
    if initiatives.is_empty() {
        return Vec::new();
    }
    let i = &rules.derived.initiative;
    let first = i.first_tick.unwrap_or(1.0);
    let gap = js::max(1.0, i.gap_per_step.unwrap_or(6.0));
    let per_step = i.flat_footed_dice_per_step.unwrap_or(-1.0);
    let top = initiatives
        .iter()
        .fold(f64::NEG_INFINITY, |m, &v| js::max(m, js::or_zero(v)));
    initiatives
        .iter()
        .map(|&v| {
            let behind = js::max(0.0, top - js::or_zero(v));
            let step = (behind / gap).ceil();
            Entry {
                tick: first + step,
                dice_penalty: per_step * step,
                behind,
            }
        })
        .collect()
}

/// `faseDeQuemVaiAgir`: a fase presumida de quem age no Tick do alvo e ainda
/// não declarou (golpeia no Tick dele mais o Preparo).
pub fn phase_of_pending_actor(target_tick: f64, target_windup: f64, blow_tick: f64) -> Phase {
    let blow = target_tick + js::max(0.0, target_windup);
    if blow_tick < target_tick || blow_tick > blow {
        Phase::Free
    } else if blow_tick == blow {
        Phase::Blow
    } else {
        Phase::Windup
    }
}

fn decay(rules: &Rules) -> f64 {
    rules
        .derived
        .initiative
        .flat_footed_decay_per_tick
        .unwrap_or(1.0)
}

fn flat_footed_value(a: Option<&Action>) -> f64 {
    js::or_zero(a.and_then(|a| a.flat_footed).unwrap_or(0.0))
}

fn flat_footed_since(a: Option<&Action>) -> f64 {
    js::or_zero(a.and_then(|a| a.flat_footed_since).unwrap_or(0.0))
}

/// `contrapeEm`: o contrapé que resta no Tick (sobe um degrau por Tick).
pub fn flat_footed_at(rules: &Rules, action: Option<&Action>, tick: f64) -> f64 {
    let n = flat_footed_value(action);
    if n == 0.0 {
        return 0.0;
    }
    let since = flat_footed_since(action);
    js::min(0.0, n + js::max(0.0, tick - since) * decay(rules))
}

/// O contrapé carregado para a ação nova (`contrapeDe`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlatFooted {
    pub value: f64,
    pub since: f64,
}

/// `contrapeDe`: o contrapé atravessa a declaração (é do relógio, não da
/// ação). `None` sem contrapé.
pub fn carried_flat_footed(action: Option<&Action>) -> Option<FlatFooted> {
    if flat_footed_value(action) == 0.0 {
        return None;
    }
    Some(FlatFooted {
        value: action.and_then(|a| a.flat_footed).unwrap_or(0.0),
        since: flat_footed_since(action),
    })
}

/// `contrapeAcaba`: o Tick em que o contrapé zera.
pub fn flat_footed_ends(rules: &Rules, action: Option<&Action>) -> Option<f64> {
    let n = flat_footed_value(action);
    if n == 0.0 {
        return None;
    }
    Some(flat_footed_since(action) + (n.abs() / decay(rules)).ceil())
}

// ------------------------------------------------------------ a fila

/// Como uma peça entra na fila (`NaOrdem`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QueueKey {
    pub tick: Option<f64>,
    pub initiative: Option<f64>,
    /// Raciocínio; ausente vale -1 (fica atrás de quem tem 0).
    pub wits: Option<f64>,
    /// O critério de estabilidade: na mesa e no laço, o id da peça.
    pub arrival: Option<String>,
    pub name: Option<String>,
}

fn by_difference(d: f64) -> Option<Ordering> {
    if d.is_nan() || d == 0.0 {
        None
    } else if d < 0.0 {
        Some(Ordering::Less)
    } else {
        Some(Ordering::Greater)
    }
}

fn text(s: &Option<String>) -> &str {
    s.as_deref().unwrap_or("")
}

/// `ordemDaFila`: Tick (menor primeiro), iniciativa (maior), Raciocínio
/// (maior), chegada e nome.
///
/// A chegada e o nome são comparados por ponto de código, e não pela colação
/// do `localeCompare`. As duas ordens coincidem nos ids da mesa e do laço
/// (letras minúsculas sem acento, dígitos e hífen); com maiúscula ou acento
/// podem diferir (`"Ana"` e `"ana"`, `"é"` e `"f"`).
pub fn queue_order(a: &QueueKey, b: &QueueKey) -> Ordering {
    by_difference(a.tick.unwrap_or(0.0) - b.tick.unwrap_or(0.0))
        .or_else(|| by_difference(b.initiative.unwrap_or(0.0) - a.initiative.unwrap_or(0.0)))
        .or_else(|| by_difference(b.wits.unwrap_or(-1.0) - a.wits.unwrap_or(-1.0)))
        .unwrap_or_else(|| {
            text(&a.arrival)
                .cmp(text(&b.arrival))
                .then_with(|| text(&a.name).cmp(text(&b.name)))
        })
}

/// A fila ordenada: os índices das peças depois da ordenação estável pelo
/// [`queue_order`], como o `sort` do JavaScript. No empate inteiro vale a
/// ordem de entrada.
pub fn sort_queue(keys: &[QueueKey]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..keys.len()).collect();
    order.sort_by(|&a, &b| queue_order(&keys[a], &keys[b]));
    order
}

// ------------------------------------------------------------ a ação

/// A fase de alguém num Tick (`Fase`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// `livre`
    Free,
    /// `preparo`
    Windup,
    /// `golpe`
    Blow,
    /// `recuperacao`
    Recovery,
}

impl Phase {
    pub fn id(self) -> &'static str {
        match self {
            Phase::Free => "livre",
            Phase::Windup => "preparo",
            Phase::Blow => "golpe",
            Phase::Recovery => "recuperacao",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "livre" => Some(Phase::Free),
            "preparo" => Some(Phase::Windup),
            "golpe" => Some(Phase::Blow),
            "recuperacao" => Some(Phase::Recovery),
            _ => None,
        }
    }
}

/// A interposição que a ação sustenta (`interpoe`): o golpe declarado (a ação
/// do agressor e o Tick absoluto) que ela cobre.
#[derive(Debug, Clone, PartialEq)]
pub struct Interposition {
    pub aid: String,
    pub blow: f64,
    pub original_target: String,
}

/// A ação declarada (`Acao`), como mora na coluna `acao` dos combatentes.
///
/// `blows` são os Ticks absolutos dos golpes; `free_at`, o Tick em que o
/// ciclo fecha. Os campos ausentes no JSON são `None`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Action {
    /// `golpes`
    pub blows: Option<Vec<f64>>,
    /// `livre`
    pub free_at: Option<f64>,
    /// `desde`: o Tick da declaração.
    pub since: Option<f64>,
    /// `tipo`
    pub maneuver: Option<Maneuver>,
    /// `arma`
    pub weapon: Option<String>,
    /// `alvo`
    pub target: Option<String>,
    /// `divida`: Ticks já empurrados para o futuro.
    pub debt: Option<f64>,
    /// `pressao`: ataques recebidos neste ciclo.
    pub pressure: Option<f64>,
    /// `aResolver`: os golpes ainda no ar (só com o golpe adiado).
    pub pending: Option<Vec<f64>>,
    /// `aid`: o identificador da ação.
    pub aid: Option<String>,
    /// `interpoe`
    pub interposition: Option<Interposition>,
    /// `contrape`: os dados do contrapé (negativo).
    pub flat_footed: Option<f64>,
    /// `contrapeDesde`
    pub flat_footed_since: Option<f64>,
}

impl Action {
    /// Os Ticks dos golpes (vazio sem agenda).
    pub fn blow_ticks(&self) -> &[f64] {
        self.blows.as_deref().unwrap_or(&[])
    }
}

/// `acaoVazia`: sem golpe, sem Pressão e sem interposição.
pub fn action_is_empty(action: Option<&Action>) -> bool {
    match action {
        None => true,
        Some(a) => {
            a.blow_ticks().is_empty()
                && !a.pressure.is_some_and(|p| p > 0.0)
                && a.interposition.is_none()
        }
    }
}

/// `temGesto`: tem agenda de golpes.
pub fn has_gesture(action: Option<&Action>) -> bool {
    action.is_some_and(|a| !a.blow_ticks().is_empty())
}

/// `declarar`: a ação a partir do Tick da declaração (dívida e Pressão
/// zeradas). Os outros campos ficam para quem chama.
pub fn declare(tick: f64, a: &Anatomy) -> Action {
    Action {
        blows: Some(a.offsets.iter().map(|o| tick + o).collect()),
        free_at: Some(tick + a.cycle),
        since: Some(tick),
        debt: Some(0.0),
        pressure: Some(0.0),
        ..Action::default()
    }
}

/// `golpeDaAgenda`: o índice do golpe que cai no Tick (0 quando não acha).
pub fn scheduled_blow_index(action: Option<&Action>, tick: f64) -> usize {
    action
        .map(Action::blow_ticks)
        .and_then(|g| g.iter().position(|&x| x == tick))
        .unwrap_or(0)
}

/// `agendar`: marca os golpes futuros como ainda no ar. Sem golpe futuro, a
/// ação volta igual.
pub fn schedule(action: &Action, tick: f64) -> Action {
    let in_air: Vec<f64> = action
        .blow_ticks()
        .iter()
        .copied()
        .filter(|&g| g > tick)
        .collect();
    let mut a = action.clone();
    if !in_air.is_empty() {
        a.pending = Some(in_air);
    }
    a
}

/// `golpesNoAr`: os Ticks de golpe que a ação ainda deve resolver.
pub fn pending_blows(action: Option<&Action>) -> &[f64] {
    action.and_then(|a| a.pending.as_deref()).unwrap_or(&[])
}

fn smallest(xs: impl Iterator<Item = f64>) -> Option<f64> {
    let mut any = false;
    let m = xs.fold(f64::INFINITY, |m, x| {
        any = true;
        js::min(m, x)
    });
    any.then_some(m)
}

/// `proximoGolpe`: o primeiro golpe no ar.
pub fn next_blow(action: Option<&Action>) -> Option<f64> {
    smallest(pending_blows(action).iter().copied())
}

/// `golpeDevido`: o golpe no ar que já venceu no Tick (`<=`).
pub fn due_blow(action: Option<&Action>, tick: f64) -> Option<f64> {
    smallest(pending_blows(action).iter().copied().filter(|&g| g <= tick))
}

/// `golpeResolvido`: tira do ar o golpe do Tick; a agenda fica.
pub fn blow_resolved(action: &Action, tick: f64) -> Action {
    let rest: Vec<f64> = pending_blows(Some(action))
        .iter()
        .copied()
        .filter(|&g| g != tick)
        .collect();
    let mut a = action.clone();
    a.pending = (!rest.is_empty()).then_some(rest);
    a
}

/// `faseEm`: livre, preparo, golpe ou recuperação no Tick.
pub fn phase_at(action: Option<&Action>, tick: f64) -> Phase {
    let Some(a) = action.filter(|a| !action_is_empty(Some(a)) && has_gesture(Some(a))) else {
        return Phase::Free;
    };
    if a.free_at.is_some_and(|l| tick >= l) {
        return Phase::Free;
    }
    let g = a.blow_ticks();
    // `includes` acha o NaN; `-0` e `0` são o mesmo Tick
    if g.iter()
        .any(|&x| x == tick || (x.is_nan() && tick.is_nan()))
    {
        return Phase::Blow;
    }
    let last = g.iter().fold(f64::NEG_INFINITY, |m, &x| js::max(m, x));
    if tick < last {
        Phase::Windup
    } else {
        Phase::Recovery
    }
}

/// `golpesDados`: quantos golpes já saíram até o Tick (inclusive).
pub fn blows_dealt(action: Option<&Action>, tick: f64) -> f64 {
    match action {
        Some(a) if !action_is_empty(action) => {
            a.blow_ticks().iter().filter(|&&g| g <= tick).count() as f64
        }
        _ => 0.0,
    }
}

/// As opções da escada: a segunda arma na mão (`segura`) e a fase forçada de
/// quem ainda não declarou ([`phase_of_pending_actor`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GuardOptions {
    pub off_hand_guard: bool,
    pub phase: Option<Phase>,
}

/// A Defesa perdida num Tick (`defesaPerdida`): a da fase, a da Pressão e a
/// soma (números negativos ou zero).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LostDefense {
    pub phase: Phase,
    pub action: f64,
    pub pressure: f64,
    pub total: f64,
}

/// `defesaPerdida`: Preparo -2, Golpe -4 (+2 com a segunda arma), Recuperação
/// -2 por golpe dado, mais -2 por ataque de Pressão.
pub fn lost_defense(
    rules: &Rules,
    action: Option<&Action>,
    tick: f64,
    opts: GuardOptions,
) -> LostDefense {
    let e: GuardLadder = rules.combat.ladder.unwrap_or(GuardLadder {
        windup: None,
        blow: None,
        recovery_per_blow: None,
        pressure_per_attack: None,
        second_hand_relief: None,
    });
    let phase = opts.phase.unwrap_or_else(|| phase_at(action, tick));
    let pressure = js::or_zero(action.and_then(|a| a.pressure).unwrap_or(0.0))
        * e.pressure_per_attack.unwrap_or(-2.0);
    let lost = match phase {
        Phase::Windup => e.windup.unwrap_or(-2.0),
        Phase::Blow => {
            e.blow.unwrap_or(-4.0)
                + if opts.off_hand_guard {
                    e.second_hand_relief.unwrap_or(2.0)
                } else {
                    0.0
                }
        }
        Phase::Recovery => e.recovery_per_blow.unwrap_or(-2.0) * blows_dealt(action, tick),
        Phase::Free => 0.0,
    };
    LostDefense {
        phase,
        action: lost,
        pressure,
        total: lost + pressure,
    }
}

/// `podeSerInterrompido`: só no Preparo.
pub fn can_be_interrupted(action: Option<&Action>, tick: f64) -> bool {
    phase_at(action, tick) == Phase::Windup
}
