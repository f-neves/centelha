//! As contas pequenas do personagem que não dependem da ficha inteira: o XP de
//! cada trilha, a Aparência, a Absorção por modo, o portão da Perfuração, o
//! limite da morte, o Tamanho pelo rótulo e o modificador de acerto por
//! Tamanho, o peso que a Arte de força ergue e a jogada da manobra.
//!
//! Port das funções de mesmo assunto de `motor.ts` (`pisoXp`, `precoNivel`,
//! `custoPontos` e os custos nomeados, `aparenciaMod`, `soakNatural`, `soak`,
//! `gatePerfuracaoAbre`, `limiteDaMorte`, `passouDoLimite`, `porteDeRotulo`,
//! `modificadorPorte`, `levantamentoDaArte`, `jogadaDaManobra`). Quem confere é
//! o teste de paridade contra `tests/parity/character.json`.

use centelha_data::Rules;
use centelha_data::bestiary::Size;
use centelha_data::rules::{XpKind, XpTrack};
use unicode_normalization::UnicodeNormalization;

use crate::js;

// ----- XP -----

fn track<'a>(rules: &'a Rules, key: &str) -> Option<&'a XpTrack> {
    rules.xp.track(key)
}

/// `pisoXp`: o nível que já vem pago na trilha. `None` quando a trilha não
/// existe (o TypeScript quebra).
pub fn xp_floor(rules: &Rules, key: &str) -> Option<f64> {
    track(rules, key).map(|t| t.floor)
}

/// `precoNivel`: o preço de um nível isolado, com o piso e a trilha grátis.
pub fn level_price(rules: &Rules, key: &str, level: f64) -> Option<f64> {
    track(rules, key).map(|t| price(t, level))
}

fn price(t: &XpTrack, level: f64) -> f64 {
    if t.kind == XpKind::Free || level <= t.floor {
        0.0
    } else {
        t.base + t.mult * level
    }
}

/// `custoPontos`: o custo para levar o traço de `from` (o piso, quando
/// `None`) até `to`. Na trilha `flat`, só o preço do nível pedido.
pub fn xp_cost(rules: &Rules, key: &str, from: Option<f64>, to: f64) -> Option<f64> {
    let t = track(rules, key)?;
    let from = from.unwrap_or(t.floor);
    Some(match t.kind {
        XpKind::Free => 0.0,
        XpKind::Flat => {
            if to > from {
                price(t, to)
            } else {
                0.0
            }
        }
        XpKind::Accumulated => {
            let mut c = 0.0;
            let mut v = from + 1.0;
            while v <= to {
                c += price(t, v);
                v += 1.0;
            }
            c
        }
    })
}

/// O custo de uma trilha que a leitura das regras garante (`XP_NAMED`).
fn named_cost(rules: &Rules, key: &str, level: f64) -> f64 {
    xp_cost(rules, key, None, level).unwrap_or(0.0)
}

/// `custoTecnica`: a Proeza, pelo preço do nível, sem acumular.
pub fn feat_cost(rules: &Rules, level: f64) -> f64 {
    named_cost(rules, "tecnica", level)
}

/// `custoArte`: acumulado, por Arte.
pub fn art_cost(rules: &Rules, level: f64) -> f64 {
    named_cost(rules, "arte", level)
}

/// `custoEfeito`: o Efeito de uma Arte, pelo preço do nível, sem acumular.
pub fn effect_cost(rules: &Rules, level: f64) -> f64 {
    named_cost(rules, "efeito", level)
}

/// `custoEspecialidade`: acumulado, uma trilha por escopo.
pub fn specialty_cost(rules: &Rules, level: f64, secondary: bool) -> f64 {
    let key = if secondary {
        "especialidadeSecundaria"
    } else {
        "especialidadePrimaria"
    };
    named_cost(rules, key, level)
}

// ----- Aparência, Absorção e Perfuração -----

/// `aparenciaMod`: o modificador da Aparência pela curva. `mask` é a
/// Compostura de quem mascara o próprio modificador: cada ponto o leva em
/// direção a zero, sem passar dele (0, `NaN` e `None` não mascaram).
pub fn appearance_mod(rules: &Rules, level: f64, mask: Option<f64>) -> f64 {
    let base = rules
        .appearance
        .curve
        .get(&js::number_to_string(level))
        .copied()
        .unwrap_or(0.0);
    let Some(mask) = mask.filter(|m| !m.is_nan() && *m != 0.0) else {
        return base;
    };
    let reduced = js::max(0.0, base.abs() - js::max(0.0, mask));
    if base < 0.0 { -reduced } else { reduced }
}

/// Um modo de ataque (`Modo`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageMode {
    Cut,
    Piercing,
    Impact,
}

impl DamageMode {
    pub const ALL: [DamageMode; 3] = [DamageMode::Cut, DamageMode::Piercing, DamageMode::Impact];

    /// A chave gravada (`corte`, `perfurante`, `impacto`).
    pub fn id(self) -> &'static str {
        match self {
            DamageMode::Cut => "corte",
            DamageMode::Piercing => "perfurante",
            DamageMode::Impact => "impacto",
        }
    }

    pub fn from_id(id: &str) -> Option<DamageMode> {
        DamageMode::ALL.into_iter().find(|m| m.id() == id)
    }
}

/// `soakNatural`: a Absorção do corpo. Só o Impacto leva o Vigor; `category`
/// é o modo (`corte`, `perfurante`, `impacto`) ou a categoria de Absorção
/// (`impacto`, `corte`, `perfuracao`).
pub fn natural_soak(stamina: f64, category: &str) -> f64 {
    if category == "impacto" { stamina } else { 0.0 }
}

/// `soak`: a Absorção de um modo: a do corpo, mais a Centelha
/// (`dano.centelhaNoSoak` por ponto), mais a da armadura.
pub fn mode_soak(
    rules: &Rules,
    stamina: f64,
    spark: f64,
    mode: DamageMode,
    armor_soak: Option<f64>,
) -> f64 {
    let per_spark = rules
        .damage
        .as_ref()
        .and_then(|d| d.spark_in_soak)
        .unwrap_or(0.0);
    natural_soak(stamina, mode.id()) + spark * per_spark + armor_soak.unwrap_or(0.0)
}

/// `gatePerfuracaoAbre`: o golpe passa pelo portão da Perfuração. Só os modos
/// do portão (o Perfurante) são barrados, quando o Nível de Perfuração da arma
/// é menor que o da armadura.
pub fn pierce_gate_opens(
    rules: &Rules,
    mode: DamageMode,
    weapon_pierce: f64,
    pierce_level: f64,
) -> bool {
    let modes = rules
        .damage
        .as_ref()
        .and_then(|d| d.pierce_gate.as_ref())
        .and_then(|g| g.modes.as_ref());
    let gated = match modes {
        Some(m) => m.iter().any(|m| m == mode.id()),
        None => mode == DamageMode::Piercing,
    };
    !gated || weapon_pierce >= pierce_level
}

// ----- o limite da morte -----

/// `limiteDaMorte`: a Vida em que se morre, abaixo do zero: menos o PV máximo
/// dividido por `morte.limiteDivisor`, arredondado para cima com Centelha e
/// para baixo sem ela (Centelha desconhecida conta como tendo). `None` sem PV
/// máximo positivo: sem PV máximo não há limite.
pub fn death_limit(rules: &Rules, max_hp: Option<f64>, spark: Option<f64>) -> Option<f64> {
    let max_hp = max_hp.filter(|p| *p > 0.0)?;
    let m = rules.death.as_ref();
    let divisor = m
        .and_then(|m| m.limit_divisor)
        .filter(|d| !d.is_nan() && *d != 0.0)
        .unwrap_or(2.0);
    let rounding = m.and_then(|m| m.limit_rounding.as_ref());
    let with_spark = spark.is_none_or(|c| c > 0.0);
    let side = rounding.and_then(|r| {
        if with_spark {
            r.with_spark.as_deref()
        } else {
            r.without_spark.as_deref()
        }
    });
    let x = max_hp / divisor;
    Some(
        -(if side == Some("alto") {
            x.ceil()
        } else {
            x.floor()
        }),
    )
}

/// `passouDoLimite`: a Vida chegou ao limite da morte ou abaixo dele. Sem
/// limite ou sem Vida, não passou.
pub fn past_death_limit(
    rules: &Rules,
    life: Option<f64>,
    max_hp: Option<f64>,
    spark: Option<f64>,
) -> bool {
    match (death_limit(rules, max_hp, spark), life) {
        (Some(limit), Some(life)) => life <= limit,
        _ => false,
    }
}

// ----- Tamanho -----

/// `porteDeRotulo`: o Tamanho pelo rótulo do bestiário (`Miúdo`, `Médio`),
/// em minúsculas e sem acento; o rótulo que não se reconhece dá Médio.
pub fn size_from_label(label: Option<&str>) -> Size {
    let k: String = label
        .unwrap_or("")
        .to_lowercase()
        .nfd()
        .filter(|c| !('\u{300}'..='\u{36f}').contains(c))
        .collect();
    Size::ALL
        .into_iter()
        .find(|s| s.id() == k)
        .unwrap_or(Size::Medium)
}

/// `modificadorPorte`: o modificador de acerto pela diferença de Tamanho,
/// `porteAcerto.reforma.porCategoria` por categoria. Corpo a corpo só o menor
/// ganha; à distância vale nos dois sentidos.
pub fn size_modifier(rules: &Rules, attacker: Size, target: Size, ranged: bool) -> f64 {
    let t = &rules.size_accuracy;
    let (Some(ia), Some(ib)) = (
        t.order.iter().position(|s| *s == attacker),
        t.order.iter().position(|s| *s == target),
    ) else {
        return 0.0;
    };
    let diff = ib as f64 - ia as f64;
    if diff <= 0.0 && !ranged {
        return 0.0;
    }
    diff * t.reform.per_category
}

// ----- Arte de força e manobras -----

/// `levantamentoDaArte`: o peso que a Arte de força ergue, em kg, pela tabela
/// `forca.levantamento` no FAH `nível × 7 − 2` (preso entre 3 e 40). `None`
/// quando o FAH não está na tabela (nível `NaN`).
pub fn art_lifting(rules: &Rules, level: f64) -> Option<f64> {
    let fah = js::max(3.0, js::min(40.0, js::round(level * 7.0 - 2.0)));
    rules
        .strength
        .lifting
        .get(&js::number_to_string(fah))
        .copied()
}

/// `jogadaDaManobra`: o texto da jogada do agarrão e da derrubada da
/// criatura (o maior entre Força e Destreza, mais Briga). `None` em outra
/// regra. O traço ausente entra como 0.
pub fn maneuver_roll(
    rule: Option<&str>,
    strength: f64,
    dexterity: f64,
    brawl: f64,
    spark: f64,
) -> Option<String> {
    if !matches!(rule, Some("agarrar" | "derrubar")) {
        return None;
    }
    let (name, value) = if strength >= dexterity {
        ("Força", strength)
    } else {
        ("Destreza", dexterity)
    };
    let sum = value + brawl;
    let odd = sum % 2.0;
    let bonus = if odd != 0.0 && !odd.is_nan() {
        2.0
    } else {
        0.0
    } + 2.0 * js::min(spark, brawl);
    let bonus_text = if bonus != 0.0 && !bonus.is_nan() {
        format!(" +{}", js::number_to_string(bonus))
    } else {
        String::new()
    };
    Some(format!(
        "Nesta criatura: {}d6{bonus_text} ({name} {} + Briga {}).",
        js::number_to_string((sum / 2.0).floor()),
        js::number_to_string(value),
        js::number_to_string(brawl),
    ))
}

#[cfg(test)]
mod tests {
    use crate::creature::spark_in_roll;

    /// O `centelhaNaJogada` com o `Math.max`/`Math.min` do JavaScript: `NaN`
    /// passa adiante e `-0` vira `0` (o `f64::max` daria `0` e `-0`).
    #[test]
    fn spark_in_roll_follows_javascript() {
        assert!(spark_in_roll(f64::NAN, 2.0).is_nan());
        assert!(spark_in_roll(2.0, f64::NAN).is_nan());
        assert_eq!(spark_in_roll(-0.0, 3.0).to_bits(), 0.0f64.to_bits());
        assert_eq!(spark_in_roll(3.0, 1.0), 2.0);
        assert_eq!(spark_in_roll(-2.0, 4.0), 0.0);
    }
}
