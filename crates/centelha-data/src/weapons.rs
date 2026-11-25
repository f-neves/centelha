//! O catálogo de armas: `content/data/armas.json`.
//!
//! Primeira versão: só o que o ataque com arma da criatura lê (o bloco `arma`
//! e as `tags`). Preço, peso, descrição, alcance e modos de dano ficam de fora
//! por enquanto.

use serde::Deserialize;

use crate::DataError;
use crate::bestiary::{AttackClass, Timing};

/// Uma peça do catálogo de armas.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Weapon {
    pub id: String,
    #[serde(rename = "nome")]
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// O bloco `arma`; `null` numa peça que não é arma.
    #[serde(rename = "arma", default)]
    pub stats: Option<WeaponStats>,
}

/// O bloco `arma` de uma peça.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WeaponStats {
    #[serde(rename = "classe")]
    pub class: AttackClass,
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
