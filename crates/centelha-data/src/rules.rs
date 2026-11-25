//! As regras em número: `content/data/regras.json`.
//!
//! Primeira versão: só as chaves que os derivados da criatura leem. O resto do
//! arquivo (as notas e as outras regras) é ignorado por enquanto.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::DataError;
use crate::bestiary::Size;

/// O subconjunto de `regras.json` lido hoje.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Rules {
    #[serde(rename = "criatura")]
    pub creature: CreatureRules,
    #[serde(rename = "porteAcerto")]
    pub size_accuracy: SizeAccuracy,
    #[serde(rename = "derivados")]
    pub derived: DerivedRules,
    #[serde(rename = "combate")]
    pub combat: CombatRules,
}

/// `criatura`: as tabelas da criatura.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CreatureRules {
    #[serde(rename = "tamanho")]
    pub size: SizeByArea,
    #[serde(rename = "faixaMassa")]
    pub mass_band: MassBand,
    #[serde(rename = "pesoErguido")]
    pub lifted_weight: LiftedWeight,
    #[serde(rename = "danoMedio")]
    pub average_damage: AverageDamage,
    #[serde(rename = "couraca")]
    pub carapace: CarapaceRules,
}

/// `criatura.tamanho`: o Tamanho pela área da silhueta.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SizeByArea {
    /// Os cortes em m², na ordem dos Tamanhos.
    #[serde(rename = "cortesM2")]
    pub cuts_m2: Vec<f64>,
    /// A folga perto do corte, em fração (0,05).
    #[serde(rename = "folga")]
    pub tolerance: f64,
}

/// `criatura.faixaMassa`: as faixas de massa das manobras.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MassBand {
    #[serde(rename = "cortesKg")]
    pub cuts_kg: Vec<f64>,
    /// A densidade com que o volume do incorpóreo vira massa.
    #[serde(rename = "densidadeIncorporeoKgM3")]
    pub incorporeal_density_kg_m3: f64,
}

/// `criatura.pesoErguido`: as constantes da R15.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct LiftedWeight {
    #[serde(rename = "curvaA")]
    pub curve_a: f64,
    #[serde(rename = "curvaB")]
    pub curve_b: f64,
    #[serde(rename = "curvaC")]
    pub curve_c: f64,
    #[serde(rename = "curvaMassaKg")]
    pub curve_mass_kg: f64,
    #[serde(rename = "piso")]
    pub floor: f64,
    pub base: f64,
    #[serde(rename = "eForca")]
    pub e_strength: f64,
    #[serde(rename = "eHalterofilismo")]
    pub e_weightlifting: f64,
    #[serde(rename = "eMusculatura")]
    pub e_musculature: f64,
    #[serde(rename = "eDivisor")]
    pub e_divisor: f64,
    #[serde(rename = "eRefForca")]
    pub e_ref_strength: f64,
    #[serde(rename = "eRefSoma")]
    pub e_ref_sum: f64,
    #[serde(rename = "forcaRef")]
    pub strength_ref: f64,
    #[serde(rename = "forcaRefInclinacao")]
    pub strength_ref_slope: f64,
    #[serde(rename = "forcaRefAbaixoKg")]
    pub strength_ref_below_kg: f64,
    #[serde(rename = "forcaRefAcimaKg")]
    pub strength_ref_above_kg: f64,
}

/// `criatura.danoMedio`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct AverageDamage {
    /// A média de cada d6.
    #[serde(rename = "porDado")]
    pub per_die: f64,
}

/// `criatura.couraca`: quanto cada perfil soma à Absorção, por nível.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CarapaceRules {
    #[serde(rename = "perfis")]
    pub profiles: CarapaceProfiles,
}

/// Os três perfis de Couraça.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct CarapaceProfiles {
    #[serde(rename = "rigida")]
    pub rigid: SoakFractions,
    #[serde(rename = "couro")]
    pub leather: SoakFractions,
    #[serde(rename = "mole")]
    pub soft: SoakFractions,
}

/// A fração do nível de Couraça em cada categoria de Absorção.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct SoakFractions {
    #[serde(rename = "impacto")]
    pub impact: f64,
    #[serde(rename = "corte")]
    pub cut: f64,
    #[serde(rename = "perfuracao")]
    pub pierce: f64,
}

/// `porteAcerto`: só a ordem dos Tamanhos, por enquanto.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SizeAccuracy {
    #[serde(rename = "ordem")]
    pub order: Vec<Size>,
}

/// `derivados`: as contas dos números da ficha.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DerivedRules {
    pub pv: HpRules,
    #[serde(rename = "defesa")]
    pub defense: DefenseRules,
    #[serde(rename = "defesaMental")]
    pub mental_defense: MentalDefenseRules,
    #[serde(rename = "defesaSocial")]
    pub social_defense: SocialDefenseRules,
    #[serde(rename = "iniciativa")]
    pub initiative: InitiativeRules,
    #[serde(rename = "energia")]
    pub energy: EnergyRules,
    pub mana: ManaRules,
    #[serde(rename = "danoForca")]
    pub strength_damage: StrengthDamage,
}

/// `derivados.pv`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HpRules {
    pub base: f64,
    #[serde(rename = "vigorMult")]
    pub stamina_mult: f64,
    /// A linha por Tamanho (sem ela, vale `base` e `stamina_mult`).
    #[serde(rename = "porte", default)]
    pub by_size: Option<BTreeMap<Size, HpLineRule>>,
    #[serde(rename = "massa")]
    pub mass: HpByMass,
}

/// Uma linha de `derivados.pv.porte`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct HpLineRule {
    pub base: f64,
    #[serde(rename = "vigorMult")]
    pub stamina_mult: f64,
}

/// `derivados.pv.massa`: a Vida pela massa, da criatura.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HpByMass {
    #[serde(rename = "conta")]
    pub table: Vec<HpMassRow>,
}

/// Uma linha da tabela de PV pela massa.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct HpMassRow {
    /// Até esta massa, inclusive; `null` na última linha.
    #[serde(rename = "ateKg")]
    pub up_to_kg: Option<f64>,
    pub base: f64,
    #[serde(rename = "vigorMult")]
    pub stamina_mult: f64,
    #[serde(rename = "vigorDivisor")]
    pub stamina_divisor: f64,
    /// A zona de transição do topo da linha, em kg.
    #[serde(rename = "zonaKg")]
    pub zone_kg: Option<[f64; 2]>,
}

/// `derivados.defesa` (Esquiva e Bloqueio).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct DefenseRules {
    pub mult: f64,
}

/// `derivados.defesaMental`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct MentalDefenseRules {
    pub mult: f64,
    #[serde(rename = "maisRaciocinio", default)]
    pub plus_wits: Option<bool>,
    #[serde(rename = "maisVontade", default)]
    pub plus_willpower: Option<bool>,
}

/// `derivados.defesaSocial`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SocialDefenseRules {
    pub mult: f64,
    /// Os termos somados (`compostura`, `sociabilidade`).
    #[serde(rename = "tracos")]
    pub traits: Vec<String>,
}

/// `derivados.iniciativa`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct InitiativeRules {
    #[serde(rename = "dado")]
    pub die: f64,
    /// As chaves somadas ao dado (`raciocinio`, `prontidao`).
    #[serde(rename = "soma")]
    pub sum: Vec<String>,
}

/// `derivados.energia`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct EnergyRules {
    pub divisor: f64,
    #[serde(rename = "centelhaMult")]
    pub spark_mult: f64,
    #[serde(rename = "maisVontade", default)]
    pub plus_willpower: Option<bool>,
}

/// `derivados.mana`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ManaRules {
    #[serde(rename = "centelhaMult")]
    pub spark_mult: f64,
    #[serde(rename = "maisVontade", default)]
    pub plus_willpower: Option<bool>,
}

/// `derivados.danoForca`: a Força no dano por número de mãos.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct StrengthDamage {
    #[serde(rename = "umaMao")]
    pub one_hand: f64,
    #[serde(rename = "duasMaos")]
    pub two_hands: f64,
}

/// `combate`: o que os derivados da criatura leem.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CombatRules {
    #[serde(rename = "escalaPorTamanho")]
    pub scale_by_size: ScaleBySize,
    #[serde(rename = "distancia", default)]
    pub ranged: Option<RangedRules>,
}

/// `combate.escalaPorTamanho`: a arma de outro Tamanho.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ScaleBySize {
    #[serde(rename = "dadosPorCategoriaAcima")]
    pub dice_per_size_above: f64,
    #[serde(rename = "bonusPorCategoriaAbaixo")]
    pub bonus_per_size_below: f64,
}

/// `combate.distancia`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct RangedRules {
    #[serde(rename = "maxima", default)]
    pub maximum: Option<RangedMaximum>,
}

/// `combate.distancia.maxima`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct RangedMaximum {
    /// A Força acima da qual o arco conta como esta (8).
    #[serde(rename = "forcaAcimaDeContaComo", default)]
    pub bow_strength_cap: Option<f64>,
}

/// Lê `regras.json` a partir do texto.
pub fn parse_rules(json: &str) -> Result<Rules, DataError> {
    let rules: Rules = crate::json::parse(json)?;
    let check = |ok: bool, path: &str, reason: &str| {
        if ok {
            Ok(())
        } else {
            Err(DataError::invalid(path, reason))
        }
    };
    check(
        rules.size_accuracy.order.len() == rules.creature.size.cuts_m2.len() + 1,
        "porteAcerto.ordem",
        "a ordem tem um Tamanho a mais que os cortes de área",
    )?;
    check(
        !rules.derived.pv.mass.table.is_empty()
            && rules
                .derived
                .pv
                .mass
                .table
                .last()
                .is_some_and(|row| row.up_to_kg.is_none()),
        "derivados.pv.massa.conta",
        "a última linha não tem limite (ateKg nulo)",
    )?;
    Ok(rules)
}

/// Lê `<content_dir>/data/regras.json` (só no nativo).
#[cfg(not(target_arch = "wasm32"))]
pub fn load_rules(content_dir: &std::path::Path) -> Result<Rules, DataError> {
    let (name, text) = crate::json::read_data_file(content_dir, "regras.json")?;
    parse_rules(&text).map_err(|e| e.in_file(name))
}
