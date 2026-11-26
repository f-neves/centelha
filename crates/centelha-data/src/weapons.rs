//! O catálogo de armas: `content/data/armas.json`.
//!
//! Cada peça traz o que as páginas e as contas leem (o bloco `arma`, as
//! `tags`, o preço, o peso, as notas e o `soCriatura`) e o envelope inteiro,
//! que o motor usa para montar o catálogo da ficha.

use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::DataError;
use crate::bestiary::{AttackClass, Timing};
use crate::catalog_item::{Price, read_item};

/// O tipo de dano de um modo de ataque (`tipo` e `tipoDano`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum DamageType {
    #[serde(rename = "corte")]
    Cut,
    #[serde(rename = "perfurante")]
    Pierce,
    #[serde(rename = "impacto")]
    Impact,
}

/// Um modo de ataque da arma (`modos[]`): o tipo de dano, o Nível de
/// Perfuração e se é um dos principais.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct AttackMode {
    #[serde(rename = "tipo")]
    pub kind: DamageType,
    /// `perf`: o Nível de Perfuração do modo, quando é perfurante.
    #[serde(rename = "perf", default)]
    pub pierce_level: Option<f64>,
    /// `principal`: o modo comum da arma; os outros pagam o secundário.
    #[serde(rename = "principal", default)]
    pub primary: bool,
}

/// A faixa de alcance de uma arma de longe (`alcance`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum RangeBand {
    #[serde(rename = "curto")]
    Short,
    #[serde(rename = "medio")]
    Medium,
    #[serde(rename = "longo")]
    Long,
}

/// Uma peça do catálogo de armas.
///
/// Os campos tipados são os que as páginas e as contas leem. O envelope
/// ([`Weapon::envelope`]) guarda a peça inteira, na ordem das chaves do
/// arquivo, e é por ele que o motor monta o catálogo da ficha.
#[derive(Debug, Clone, PartialEq)]
pub struct Weapon {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    /// O bloco `arma`; `null` numa peça que não é arma.
    pub stats: Option<WeaponStats>,
    /// `tipo`: `arma` (as armas de hoje).
    pub kind: Option<String>,
    /// `preco`.
    pub price: Option<Price>,
    /// `peso`, em quilos.
    pub weight: Option<f64>,
    /// `descricao`: o texto do livro sobre a peça (`notas` no catálogo
    /// achatado).
    pub description: Option<String>,
    /// `soCriatura`: a arma é só de criatura e não entra na loja.
    pub creature_only: bool,
    envelope: Value,
}

/// Os campos tipados de [`Weapon`], como vêm do JSON.
#[derive(Deserialize)]
struct WeaponFields {
    id: String,
    #[serde(rename = "nome")]
    name: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(rename = "arma", default)]
    stats: Option<WeaponStats>,
    #[serde(rename = "tipo", default)]
    kind: Option<String>,
    #[serde(rename = "preco", default)]
    price: Option<Price>,
    #[serde(rename = "peso", default)]
    weight: Option<f64>,
    #[serde(rename = "descricao", default)]
    description: Option<String>,
    #[serde(rename = "soCriatura", default)]
    creature_only: bool,
}

impl<'de> Deserialize<'de> for Weapon {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Weapon, D::Error> {
        let (f, envelope): (WeaponFields, Value) = read_item(d)?;
        Ok(Weapon {
            id: f.id,
            name: f.name,
            tags: f.tags,
            stats: f.stats,
            kind: f.kind,
            price: f.price,
            weight: f.weight,
            description: f.description,
            creature_only: f.creature_only,
            envelope,
        })
    }
}

impl Weapon {
    /// A peça inteira como está no JSON (as chaves na ordem do arquivo).
    pub fn envelope(&self) -> &Value {
        &self.envelope
    }
}

/// O bloco `arma` de uma peça.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WeaponStats {
    #[serde(rename = "classe")]
    pub class: AttackClass,
    /// O Atributo da jogada (`atrib`).
    #[serde(rename = "atrib", default)]
    pub attribute: Option<String>,
    /// A Habilidade da jogada (`armas`, `briga`, `atirador`, `arremesso`).
    #[serde(rename = "pericia")]
    pub skill: String,
    /// Quantos d6 de dano.
    #[serde(rename = "dado")]
    pub dice: i32,
    #[serde(rename = "danoBonus", default)]
    pub damage_bonus: Option<i32>,
    #[serde(rename = "acerto", default)]
    pub accuracy: Option<i32>,
    /// `defesaArma`: o que a arma soma à Defesa de quem a empunha.
    #[serde(rename = "defesaArma", default)]
    pub defense: Option<f64>,
    #[serde(rename = "maos", default)]
    pub hands: Option<i32>,
    /// A Velocidade em Ticks, quando a arma não diz as três parcelas.
    #[serde(default)]
    pub ticks: Option<f64>,
    #[serde(rename = "pgr", default)]
    pub timing: Option<Timing>,
    #[serde(rename = "forcaMult", default)]
    pub strength_multiplier: Option<f64>,
    /// A Força máxima que a arma aproveita.
    #[serde(rename = "forcaCap", default)]
    pub strength_cap: Option<f64>,
    /// A Força que a arma pede para render inteira.
    #[serde(rename = "forcaMin", default)]
    pub strength_min: Option<f64>,
    /// `tipoDano`: o tipo do modo principal.
    #[serde(rename = "tipoDano", default)]
    pub damage_type: Option<DamageType>,
    /// `pen`: o Nível de Perfuração da arma.
    #[serde(rename = "pen", default)]
    pub pierce_level: Option<f64>,
    /// `modos`: os modos de ataque, na ordem do arquivo.
    #[serde(rename = "modos", default)]
    pub modes: Vec<AttackMode>,
    /// `fichaModo`: o modo que a ficha mostra, quando não é o primeiro.
    #[serde(rename = "fichaModo", default)]
    pub sheet_mode: Option<DamageType>,
    /// `alcance`: a faixa de uma arma de longe.
    #[serde(rename = "alcance", default)]
    pub range_band: Option<RangeBand>,
    /// `distMax`: a distância máxima, em metros.
    #[serde(rename = "distMax", default)]
    pub max_distance: Option<f64>,
    /// `efetiva`: a distância efetiva (sem penalidade de mira), em metros.
    #[serde(rename = "efetiva", default)]
    pub effective: Option<f64>,
    /// `alcanceLivreFrac`: a fração do alcance sem penalidade.
    #[serde(rename = "alcanceLivreFrac", default)]
    pub free_range_fraction: Option<f64>,
    /// `semDano`: a arma não causa dano (a rede).
    #[serde(rename = "semDano", default)]
    pub no_damage: bool,
    /// `alcancePelaMassa`: o alcance sai da massa que o ataque dá.
    #[serde(rename = "alcancePelaMassa", default)]
    pub range_by_mass: bool,
}

/// Lê `armas.json` a partir do texto.
pub fn parse_weapons(json: &str) -> Result<Vec<Weapon>, DataError> {
    let weapons: Vec<Weapon> = crate::json::parse(json)?;
    for (i, w) in weapons.iter().enumerate() {
        if !crate::bestiary::is_id(&w.id) {
            return Err(DataError::invalid(format!("[{i}].id"), "id inválido"));
        }
    }
    Ok(weapons)
}

/// Lê `<content_dir>/data/armas.json` (só no nativo).
#[cfg(not(target_arch = "wasm32"))]
pub fn load_weapons(content_dir: &std::path::Path) -> Result<Vec<Weapon>, DataError> {
    let (name, text) = crate::json::read_data_file(content_dir, "armas.json")?;
    parse_weapons(&text).map_err(|e| e.in_file(name))
}
