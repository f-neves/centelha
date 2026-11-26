//! As regras em número: `content/data/regras.json`.
//!
//! Só as chaves que os derivados da criatura e os números da ficha do jogador
//! leem. O resto do arquivo (as notas e as outras regras) é ignorado por
//! enquanto.

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
    /// `quaseAcerto`: as tabelas do Quase-Acerto.
    #[serde(rename = "quaseAcerto", default)]
    pub near_miss: Option<NearMissRules>,
    /// `xp`: o preço de cada trilha.
    pub xp: XpRules,
    /// `aparencia`: a curva do modificador.
    #[serde(rename = "aparencia")]
    pub appearance: AppearanceRules,
    /// `morte`: o limite da morte abaixo do zero.
    #[serde(rename = "morte", default)]
    pub death: Option<DeathRules>,
    /// `dano`: a Centelha na Absorção e o portão da Perfuração.
    #[serde(rename = "dano", default)]
    pub damage: Option<DamageRules>,
    /// `forca`: a tabela do que a Força ergue.
    #[serde(rename = "forca")]
    pub strength: StrengthRules,
    /// `escalasProeza`: quanto cada Técnica melhora pelo nível.
    #[serde(rename = "escalasProeza", default)]
    pub feat_scales: Option<FeatScales>,
    /// `arcano`: as regras das Artes que a ficha lê.
    #[serde(rename = "arcano", default)]
    pub arcane: Option<ArcaneRules>,
}

/// `arcano`: o que a ficha lê das regras das Artes.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ArcaneRules {
    /// `tetoNivelArte`: o nível máximo de uma Arte.
    #[serde(rename = "tetoNivelArte", default)]
    pub art_level_cap: Option<ArtLevelCap>,
}

/// `arcano.tetoNivelArte`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ArtLevelCap {
    /// `porCentelha`: o teto pelo índice da Centelha (de 0 em diante).
    #[serde(rename = "porCentelha", default)]
    pub by_spark: Vec<f64>,
}

/// `escalasProeza`: as trilhas de melhora das Técnicas.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FeatScales {
    /// `trilhas`: pela chave de melhora (`bonus`, `dano`, `estado`...).
    #[serde(rename = "trilhas", default)]
    pub tracks: BTreeMap<String, FeatScaleTrack>,
}

/// Uma trilha de `escalasProeza.trilhas`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FeatScaleTrack {
    #[serde(rename = "rotulo")]
    pub label: String,
    #[serde(rename = "unidade", default)]
    pub unit: Option<String>,
    /// `valores`: o valor de cada nível, de 1 a 6, como texto.
    #[serde(rename = "valores", default)]
    pub values: Vec<String>,
}

/// Como uma trilha de XP soma os preços dos níveis (`tipo`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum XpKind {
    /// `acum`: soma os preços do piso + 1 até o nível.
    #[serde(rename = "acum")]
    Accumulated,
    /// `flat`: cobra só o preço do nível pedido.
    #[serde(rename = "flat")]
    Flat,
    /// `gratis`: não custa XP.
    #[serde(rename = "gratis")]
    Free,
}

/// Uma trilha de `xp`: o preço de um nível é `base + mult × nível`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct XpTrack {
    #[serde(rename = "tipo")]
    pub kind: XpKind,
    pub base: f64,
    pub mult: f64,
    /// `piso`: o nível que já vem pago.
    #[serde(rename = "piso", default)]
    pub floor: f64,
}

/// `xp`: as trilhas pelo nome (`atributo`, `arte`, `tecnica`...). A chave
/// `modelo`, que é texto, fica de fora.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct XpRules(pub BTreeMap<String, XpTrack>);

impl XpRules {
    pub fn track(&self, key: &str) -> Option<&XpTrack> {
        self.0.get(key)
    }
}

impl<'de> Deserialize<'de> for XpRules {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let raw = BTreeMap::<String, serde_json::Value>::deserialize(d)?;
        let mut tracks = BTreeMap::new();
        for (k, v) in raw {
            if k == "modelo" {
                continue;
            }
            let t = serde_json::from_value::<XpTrack>(v)
                .map_err(|e| D::Error::custom(format!("xp.{k}: {e}")))?;
            tracks.insert(k, t);
        }
        Ok(XpRules(tracks))
    }
}

/// `aparencia`: o modificador por nível, pela chave em texto (`"-1"`, `"6"`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AppearanceRules {
    #[serde(rename = "curva")]
    pub curve: BTreeMap<String, f64>,
}

/// `morte`: o divisor do PV máximo e o lado do arredondamento.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DeathRules {
    #[serde(rename = "limiteDivisor", default)]
    pub limit_divisor: Option<f64>,
    #[serde(rename = "limiteArredonda", default)]
    pub limit_rounding: Option<DeathRounding>,
}

/// `morte.limiteArredonda`: `alto` arredonda para cima; o resto, para baixo.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DeathRounding {
    #[serde(rename = "semCentelha", default)]
    pub without_spark: Option<String>,
    #[serde(rename = "comCentelha", default)]
    pub with_spark: Option<String>,
}

/// `dano`: o que a Centelha soma em cada Absorção e os modos que passam pelo
/// portão da Perfuração.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DamageRules {
    #[serde(rename = "centelhaNoSoak", default)]
    pub spark_in_soak: Option<f64>,
    #[serde(rename = "gatePerfuracao", default)]
    pub pierce_gate: Option<PierceGate>,
}

/// `dano.gatePerfuracao`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PierceGate {
    #[serde(rename = "modos", default)]
    pub modes: Option<Vec<String>>,
}

/// `forca`: só a tabela de erguer, por enquanto (`levantamento`, do FAH 3 ao
/// 40, em kg).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StrengthRules {
    #[serde(rename = "levantamento")]
    pub lifting: BTreeMap<String, f64>,
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

/// `porteAcerto`: a ordem dos Tamanhos e o modificador por categoria.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SizeAccuracy {
    #[serde(rename = "ordem")]
    pub order: Vec<Size>,
    #[serde(rename = "reforma")]
    pub reform: SizeAccuracyReform,
}

/// `porteAcerto.reforma`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct SizeAccuracyReform {
    /// `porCategoria`: o modificador por categoria de diferença (3).
    #[serde(rename = "porCategoria")]
    pub per_category: f64,
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
    /// `deslocamento`: as escadas do Deslocamento e dos saltos.
    #[serde(rename = "deslocamento")]
    pub movement: MovementRules,
}

/// `derivados.deslocamento`: cada medida é uma soma de termos (`base` mais
/// traço × peso), arredondada.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MovementRules {
    /// `normal`: o passo de Batalha.
    #[serde(rename = "normal")]
    pub battle: Ladder,
    #[serde(rename = "arranque")]
    pub dash: Ladder,
    #[serde(rename = "corrida")]
    pub run: Ladder,
    #[serde(rename = "natacao")]
    pub swim: Ladder,
    #[serde(rename = "saltoVertical")]
    pub vertical_jump: Ladder,
    #[serde(rename = "saltoHorizontalParado")]
    pub standing_jump: Ladder,
    /// O que o salto correndo soma à Corrida.
    #[serde(rename = "saltoHorizontalCorrendo")]
    pub running_jump: Ladder,
}

/// Os termos de uma escada, na ordem do JSON (a soma é feita nessa ordem,
/// como no TypeScript). A chave `base` é constante; as outras são traços.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ladder(pub Vec<(String, f64)>);

impl<'de> Deserialize<'de> for Ladder {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = Ladder;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("um objeto de números")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut m: A) -> Result<Ladder, A::Error> {
                let mut terms = Vec::new();
                while let Some((k, v)) = m.next_entry::<String, f64>()? {
                    terms.push((k, v));
                }
                Ok(Ladder(terms))
            }
        }
        d.deserialize_map(V)
    }
}

/// `quaseAcerto`: a classe da arma pelo dano médio e o que cada classe de arma
/// e de armadura põe na conta.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct NearMissRules {
    /// Por classe de Quase-Acerto da arma (`leve`, `media`, `pesada`).
    #[serde(rename = "porClasseArma", default)]
    pub by_weapon_class: BTreeMap<String, NearMissWeapon>,
    /// Por classe de armadura (`nenhuma`, `leve`, `media`, `pesada`).
    #[serde(rename = "porClasseArmadura", default)]
    pub by_armor_class: BTreeMap<String, NearMissArmor>,
    #[serde(rename = "classePorDanoMedio", default)]
    pub class_by_average_damage: Option<NearMissThresholds>,
    #[serde(rename = "ignoraSoak", default)]
    pub ignores_soak: Option<bool>,
}

/// O que uma classe de arma põe no Quase-Acerto.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct NearMissWeapon {
    #[serde(default)]
    pub bonus: Option<f64>,
    #[serde(rename = "dano", default)]
    pub damage: Option<f64>,
}

/// O que uma classe de armadura põe no Quase-Acerto.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct NearMissArmor {
    #[serde(default)]
    pub bonus: Option<f64>,
    #[serde(rename = "reducao", default)]
    pub reduction: Option<f64>,
}

/// Os limites do dano médio de cada classe de arma.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct NearMissThresholds {
    /// Até este dano médio, leve (2).
    #[serde(rename = "leveAte", default)]
    pub light_up_to: Option<f64>,
    /// Até este dano médio, média (5,5); acima, pesada.
    #[serde(rename = "mediaAte", default)]
    pub medium_up_to: Option<f64>,
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
    #[serde(rename = "movimento", default)]
    pub movement: Option<CombatMovement>,
}

/// `combate.movimento`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct CombatMovement {
    /// `investida`: a Investida.
    #[serde(rename = "investida", default)]
    pub charge: Option<Charge>,
}

/// `combate.movimento.investida`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct Charge {
    /// Quantas vezes a Força entra a mais no dano (1).
    #[serde(rename = "forcaExtra", default)]
    pub extra_strength: Option<f64>,
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

/// As trilhas de XP que o motor lê pelo nome.
pub const XP_NAMED: [&str; 5] = [
    "tecnica",
    "arte",
    "efeito",
    "especialidadePrimaria",
    "especialidadeSecundaria",
];

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
    // A zona de escolha da linha de PV olha a linha seguinte da tabela; a
    // última não tem seguinte.
    check(
        rules
            .derived
            .pv
            .mass
            .table
            .last()
            .is_some_and(|row| row.zone_kg.is_none()),
        "derivados.pv.massa.conta",
        "a última linha não pode ter zonaKg (a zona olha a linha seguinte)",
    )?;
    // As trilhas que o motor chama pelo nome (os custos de Proeza, Arte,
    // Efeito e Especialidade).
    for k in XP_NAMED {
        check(
            rules.xp.track(k).is_some(),
            &format!("xp.{k}"),
            "a trilha falta",
        )?;
    }
    Ok(rules)
}

/// Lê `<content_dir>/data/regras.json` (só no nativo).
#[cfg(not(target_arch = "wasm32"))]
pub fn load_rules(content_dir: &std::path::Path) -> Result<Rules, DataError> {
    let (name, text) = crate::json::read_data_file(content_dir, "regras.json")?;
    parse_rules(&text).map_err(|e| e.in_file(name))
}
