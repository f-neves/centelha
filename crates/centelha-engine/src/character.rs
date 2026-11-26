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

/// As letras minúsculas cuja decomposição canônica (NFD) é uma letra das
/// chaves de Tamanho mais marcas de U+0300 a U+036F, pela letra. Gerada pelo
/// `String.prototype.normalize('NFD')` do Node 24 (Unicode 17.0). Uma letra
/// fora daqui não leva a nenhuma chave, com ou sem decomposição: o rótulo que
/// a tem cai no Médio dos dois jeitos.
const UNACCENTED: [(char, &str); 14] = [
    (
        'a',
        "\u{e0}\u{e1}\u{e2}\u{e3}\u{e4}\u{e5}\u{101}\u{103}\u{105}\u{1ce}\u{1df}\u{1e1}\u{1fb}\u{201}\u{203}\u{227}\u{1e01}\u{1ea1}\u{1ea3}\u{1ea5}\u{1ea7}\u{1ea9}\u{1eab}\u{1ead}\u{1eaf}\u{1eb1}\u{1eb3}\u{1eb5}\u{1eb7}",
    ),
    ('c', "\u{e7}\u{107}\u{109}\u{10b}\u{10d}\u{1e09}"),
    ('d', "\u{10f}\u{1e0b}\u{1e0d}\u{1e0f}\u{1e11}\u{1e13}"),
    (
        'e',
        "\u{e8}\u{e9}\u{ea}\u{eb}\u{113}\u{115}\u{117}\u{119}\u{11b}\u{205}\u{207}\u{229}\u{1e15}\u{1e17}\u{1e19}\u{1e1b}\u{1e1d}\u{1eb9}\u{1ebb}\u{1ebd}\u{1ebf}\u{1ec1}\u{1ec3}\u{1ec5}\u{1ec7}",
    ),
    ('g', "\u{11d}\u{11f}\u{121}\u{123}\u{1e7}\u{1f5}\u{1e21}"),
    (
        'i',
        "\u{ec}\u{ed}\u{ee}\u{ef}\u{129}\u{12b}\u{12d}\u{12f}\u{1d0}\u{209}\u{20b}\u{1e2d}\u{1e2f}\u{1ec9}\u{1ecb}",
    ),
    ('l', "\u{13a}\u{13c}\u{13e}\u{1e37}\u{1e39}\u{1e3b}\u{1e3d}"),
    ('m', "\u{1e3f}\u{1e41}\u{1e43}"),
    (
        'n',
        "\u{f1}\u{144}\u{146}\u{148}\u{1f9}\u{1e45}\u{1e47}\u{1e49}\u{1e4b}",
    ),
    (
        'o',
        "\u{f2}\u{f3}\u{f4}\u{f5}\u{f6}\u{14d}\u{14f}\u{151}\u{1a1}\u{1d2}\u{1eb}\u{1ed}\u{20d}\u{20f}\u{22b}\u{22d}\u{22f}\u{231}\u{1e4d}\u{1e4f}\u{1e51}\u{1e53}\u{1ecd}\u{1ecf}\u{1ed1}\u{1ed3}\u{1ed5}\u{1ed7}\u{1ed9}\u{1edb}\u{1edd}\u{1edf}\u{1ee1}\u{1ee3}",
    ),
    ('p', "\u{1e55}\u{1e57}"),
    (
        'r',
        "\u{155}\u{157}\u{159}\u{211}\u{213}\u{1e59}\u{1e5b}\u{1e5d}\u{1e5f}",
    ),
    (
        's',
        "\u{15b}\u{15d}\u{15f}\u{161}\u{219}\u{1e61}\u{1e63}\u{1e65}\u{1e67}\u{1e69}",
    ),
    (
        'u',
        "\u{f9}\u{fa}\u{fb}\u{fc}\u{169}\u{16b}\u{16d}\u{16f}\u{171}\u{173}\u{1b0}\u{1d4}\u{1d6}\u{1d8}\u{1da}\u{1dc}\u{215}\u{217}\u{1e73}\u{1e75}\u{1e77}\u{1e79}\u{1e7b}\u{1ee5}\u{1ee7}\u{1ee9}\u{1eeb}\u{1eed}\u{1eef}\u{1ef1}",
    ),
];

/// A letra sem o acento, como o NFD seguido da remoção das marcas.
fn unaccented(c: char) -> Option<char> {
    if ('\u{300}'..='\u{36f}').contains(&c) {
        return None;
    }
    Some(
        UNACCENTED
            .iter()
            .find(|(_, set)| set.contains(c))
            .map_or(c, |(base, _)| *base),
    )
}

/// `porteDeRotulo`: o Tamanho pelo rótulo do bestiário (`Miúdo`, `Médio`),
/// em minúsculas e sem acento; o rótulo que não se reconhece dá Médio.
pub fn size_from_label(label: Option<&str>) -> Size {
    let k: String = label
        .unwrap_or("")
        .to_lowercase()
        .chars()
        .filter_map(unaccented)
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
