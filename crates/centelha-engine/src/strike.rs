//! O lance: a conta de um golpe, da jogada ao dano (port de `resolverGolpe`,
//! `defesaEfetiva`, `fonteRolada` e `fonteFixa`, de `lance.ts`).
//!
//! A Defesa do alvo, a jogada de ataque com a penalidade do golpe desta vez,
//! a soma com o modificador manual, o veredito pelo Quase-Acerto e o dano:
//! o acerto rola e desconta a Absorção, com o piso no dano do raspão; o
//! raspão é fixo e ignora a Absorção; o erro não faz nada.
//!
//! O dano do golpe (o piso do raspão e o portão da Perfuração) está numa
//! função só, [`blow_damage`]: no site atual a mesma conta aparece repetida
//! no Grid (`grid.astro`, no lance rolado à mão e no dano que desce a Vida).

use centelha_data::Rules;

use crate::character::{DamageMode, pierce_gate_opens};
use crate::dice::{DiceExpr, roll_expr, sum_total};
use crate::js;
use crate::rng::Mulberry32;
use crate::sheet::{AttackOutcome, attack_outcome, missed_by};

/// De onde vêm os dados do lance (`FonteDeDados`): o resultado de rolar uma
/// expressão com o ajuste de dados e de fixo.
pub trait DiceSource {
    /// Rola a expressão; devolve o total e as faces.
    fn roll(&mut self, expr: &str, extra_dice: f64, extra_flat: f64) -> (f64, Vec<f64>);
}

/// `fonteRolada`: rola pelo Mulberry32.
pub struct RolledDice<'a>(pub &'a mut Mulberry32);

impl DiceSource for RolledDice<'_> {
    fn roll(&mut self, expr: &str, extra_dice: f64, extra_flat: f64) -> (f64, Vec<f64>) {
        let r = roll_expr(expr, extra_dice, extra_flat, self.0);
        (r.total, r.rolls)
    }
}

/// `fonteFixa`: os dados que já caíram, na ordem acerto e dano (a terceira
/// rolagem em diante não tem faces). O fixo sai da expressão, como no
/// `rolarExpr`; o ajuste de dados não muda as faces dadas.
#[derive(Debug, Clone, PartialEq)]
pub struct FixedDice {
    queue: [Vec<f64>; 2],
    next: usize,
}

impl FixedDice {
    pub fn new(attack: Vec<f64>, damage: Vec<f64>) -> Self {
        FixedDice {
            queue: [attack, damage],
            next: 0,
        }
    }
}

impl DiceSource for FixedDice {
    fn roll(&mut self, expr: &str, _extra_dice: f64, extra_flat: f64) -> (f64, Vec<f64>) {
        let rolls = self.queue.get(self.next).cloned().unwrap_or_default();
        self.next += 1;
        let flat = DiceExpr::parse(expr).flat + extra_flat;
        (sum_total(&rolls, flat), rolls)
    }
}

/// O atacante do lance (a parte de `EntradaLance.atacante` que a conta lê).
#[derive(Debug, Clone, PartialEq)]
pub struct StrikeAttacker {
    /// A expressão da jogada de ataque (`ataque`).
    pub attack: String,
    /// A expressão do dano (`dano`).
    pub damage: String,
    /// `ajusteFlat` e `ajusteDados` da jogada.
    pub flat_adjust: f64,
    pub dice_adjust: f64,
    /// `penDados`: a penalidade de dados de cada golpe da ação.
    pub blow_penalty_dice: Vec<f64>,
    /// `danoDados` e `danoFlat`: o ajuste do dano (ausente conta 0).
    pub damage_dice: Option<f64>,
    pub damage_flat: Option<f64>,
}

/// O alvo do lance.
#[derive(Debug, Clone, PartialEq)]
pub struct StrikeTarget {
    /// `defesaBase`; `None` sem Defesa conhecida (não há veredito).
    pub base_defense: Option<f64>,
    /// `ferimento`, `condicoesDefesa` e `defesaPerdida`: o que se soma à Defesa.
    pub wound: f64,
    pub conditions_defense: f64,
    pub lost_defense: f64,
    pub soak: f64,
    pub hp: Option<f64>,
}

/// A entrada do lance (`EntradaLance`, os campos que a conta usa).
#[derive(Debug, Clone, PartialEq)]
pub struct StrikeInput {
    pub attacker: StrikeAttacker,
    pub target: StrikeTarget,
    /// `golpeIndice`: qual golpe da ação (a penalidade sai de
    /// `blow_penalty_dice`, e 0 fora da lista).
    pub blow_index: f64,
    /// `tipoDano`, devolvido como veio.
    pub damage_type: String,
    /// `modManual`: somado ao total da jogada.
    pub manual_mod: f64,
    /// `margemQA` e `danoQA`: a margem e o dano do raspão.
    pub graze_margin: f64,
    pub graze_damage: f64,
}

/// O resultado do lance (`SaidaLance`).
#[derive(Debug, Clone, PartialEq)]
pub struct StrikeOutcome {
    pub defense: Option<f64>,
    pub total: f64,
    /// O total mais o modificador manual.
    pub sum: f64,
    pub missed_by: Option<f64>,
    pub verdict: Option<AttackOutcome>,
    pub gross_damage: f64,
    pub damage_type: String,
    pub absorbed: f64,
    pub net_damage: f64,
    pub hp_before: Option<f64>,
    pub hp_after: Option<f64>,
    pub attack_rolls: Vec<f64>,
    pub damage_rolls: Vec<f64>,
}

/// O dano de um golpe já decidido.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlowDamage {
    pub gross: f64,
    pub absorbed: f64,
    pub net: f64,
}

/// `defesaEfetiva`: a Defesa base mais o ferimento, as condições e a Defesa
/// perdida; `None` sem Defesa base.
pub fn effective_defense(target: &StrikeTarget) -> Option<f64> {
    target
        .base_defense
        .map(|d| d + target.wound + target.conditions_defense + target.lost_defense)
}

/// O golpe resvala no portão da Perfuração (`resvalaGate` do Grid): com o
/// portão ligado na cena e a arma em modo Perfurante (`weapon_pierce` só
/// existe nesse modo), quando o Nível de Perfuração da arma não vence o do
/// alvo (sem o do alvo, 0).
pub fn blow_slips(
    rules: &Rules,
    gate_on: bool,
    weapon_pierce: Option<f64>,
    pierce_level: Option<f64>,
) -> bool {
    gate_on
        && weapon_pierce.is_some_and(|p| {
            !pierce_gate_opens(rules, DamageMode::Pierce, p, pierce_level.unwrap_or(0.0))
        })
}

/// O dano do golpe pelo veredito: no acerto, o rolado (nunca abaixo de 0)
/// menos a Absorção, e nunca menos que o dano do raspão; no raspão, o dano
/// fixo do raspão, sem Absorção; no erro (ou sem veredito), nada. O golpe que
/// resvala no portão da Perfuração não faz dano nenhum, nem o do raspão (é o
/// que o Grid aplica na Vida).
pub fn blow_damage(
    verdict: Option<AttackOutcome>,
    rolled: f64,
    graze_damage: f64,
    soak: f64,
    slips: bool,
) -> BlowDamage {
    let none = BlowDamage {
        gross: 0.0,
        absorbed: 0.0,
        net: 0.0,
    };
    if slips {
        return none;
    }
    match verdict {
        Some(AttackOutcome::Hit) => {
            let gross = js::max(0.0, rolled);
            BlowDamage {
                gross,
                absorbed: js::min(gross, soak),
                net: js::max(js::max(graze_damage, gross - soak), 0.0),
            }
        }
        Some(AttackOutcome::Graze) => {
            let gross = js::max(0.0, graze_damage);
            BlowDamage {
                gross,
                absorbed: 0.0,
                net: gross,
            }
        }
        Some(AttackOutcome::Miss) | None => none,
    }
}

/// A penalidade de dados do golpe desta vez (`penDados[golpeIndice] ?? 0`).
fn blow_penalty(a: &StrikeAttacker, index: f64) -> f64 {
    let i = index as usize;
    if index >= 0.0 && index.fract() == 0.0 && (i as f64) == index {
        a.blow_penalty_dice.get(i).copied().unwrap_or(0.0)
    } else {
        0.0
    }
}

/// `resolverGolpe`: o lance inteiro. O dano só é rolado no acerto.
pub fn resolve_strike(input: &StrikeInput, source: &mut impl DiceSource) -> StrikeOutcome {
    let a = &input.attacker;
    let t = &input.target;
    let defense = effective_defense(t);
    let pen = blow_penalty(a, input.blow_index);
    let (total, attack_rolls) = source.roll(&a.attack, a.dice_adjust + pen, a.flat_adjust);
    let sum = total + input.manual_mod;
    let missed = defense.map(|d| missed_by(sum, d));
    let verdict = defense.map(|d| attack_outcome(sum, d, input.graze_margin));
    let (rolled, damage_rolls) = if verdict == Some(AttackOutcome::Hit) {
        let or0 = |x: Option<f64>| x.filter(|v| *v != 0.0 && !v.is_nan()).unwrap_or(0.0);
        source.roll(&a.damage, or0(a.damage_dice), or0(a.damage_flat))
    } else {
        (0.0, Vec::new())
    };
    let dmg = blow_damage(verdict, rolled, input.graze_damage, t.soak, false);
    StrikeOutcome {
        defense,
        total,
        sum,
        missed_by: missed,
        verdict,
        gross_damage: dmg.gross,
        damage_type: input.damage_type.clone(),
        absorbed: dmg.absorbed,
        net_damage: dmg.net,
        hp_before: t.hp,
        hp_after: t.hp.map(|hp| js::max(0.0, hp - dmg.net)),
        attack_rolls,
        damage_rolls,
    }
}
