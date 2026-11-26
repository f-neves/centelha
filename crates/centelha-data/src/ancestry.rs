//! As Raças: `content/data/racas.json`.
//!
//! Primeira versão: os campos que os números da ficha do jogador leem (o
//! Tamanho, a faixa de massa, a fração do deslocamento, os bônus condicionais e
//! os Atributos raciais), mais o id e o nome. A prosa é ignorada.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::DataError;
use crate::bestiary::Size;

/// Uma Raça.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Ancestry {
    pub id: String,
    #[serde(rename = "nome")]
    pub name: String,
    /// O Tamanho da Raça (o PV do jogador sai da linha dele).
    #[serde(rename = "porte", default)]
    pub size: Option<Size>,
    /// O que a Raça soma (ou tira) de cada Atributo, pela chave dos dados.
    #[serde(rename = "atributos", default)]
    pub attributes: BTreeMap<String, f64>,
    #[serde(rename = "aparenciaMod", default)]
    pub appearance_mod: Option<f64>,
    /// A faixa de massa da Raça, em kg (a massa padrão é o meio dela).
    #[serde(rename = "massaKg", default)]
    pub mass_kg: Option<[f64; 2]>,
    /// A fração do deslocamento (a baixa estatura anda dois terços).
    #[serde(rename = "deslocamentoFrac", default)]
    pub movement_fraction: Option<f64>,
    #[serde(rename = "bonusCondicional", default)]
    pub conditional_bonuses: Vec<ConditionalBonus>,
}

/// Um bônus da Raça que vale numa situação (ou sempre, como a Vitalidade).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ConditionalBonus {
    /// O bônus como está nos dados (`+1d6`, `+2`, `vigor`).
    #[serde(default)]
    pub bonus: Option<String>,
    /// O número da ficha em que o bônus entra (`pv`), quando entra num.
    #[serde(rename = "campo", default)]
    pub field: Option<String>,
    #[serde(rename = "escopo", default)]
    pub scope: Option<String>,
    #[serde(rename = "nome", default)]
    pub name: Option<String>,
}

impl Ancestry {
    /// A Raça soma o Vigor ao PV (a Vitalidade do orc e do meio-orc)?
    pub fn adds_stamina_to_hp(&self) -> bool {
        self.conditional_bonuses
            .iter()
            .any(|b| b.field.as_deref() == Some("pv") && b.bonus.as_deref() == Some("vigor"))
    }
}

/// Lê `racas.json` a partir do texto.
pub fn parse_ancestries(json: &str) -> Result<Vec<Ancestry>, DataError> {
    let list: Vec<Ancestry> = crate::json::parse(json)?;
    for (i, a) in list.iter().enumerate() {
        if !crate::bestiary::is_id(&a.id) {
            return Err(DataError::invalid(format!("[{i}].id"), "id inválido"));
        }
    }
    Ok(list)
}

/// Lê `<content_dir>/data/racas.json` (só no nativo).
#[cfg(not(target_arch = "wasm32"))]
pub fn load_ancestries(content_dir: &std::path::Path) -> Result<Vec<Ancestry>, DataError> {
    let (name, text) = crate::json::read_data_file(content_dir, "racas.json")?;
    parse_ancestries(&text).map_err(|e| e.in_file(name))
}
