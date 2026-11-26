//! As Artes: `content/data/artes.json`.
//!
//! Cada Arte tem a categoria (elemental ou universal), o atributo de
//! conjuração, os níveis (nome, efeito, custo em Mana e exemplos) e o bloco
//! `grid` que a mesa lê.

use serde::Deserialize;

use crate::DataError;

/// A categoria da Arte (`categoria`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ArtCategory {
    #[serde(rename = "elemental")]
    Elemental,
    #[serde(rename = "universal")]
    Universal,
}

impl ArtCategory {
    pub fn id(self) -> &'static str {
        match self {
            ArtCategory::Elemental => "elemental",
            ArtCategory::Universal => "universal",
        }
    }
}

/// O custo de um nível da Arte.
#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize)]
pub struct ArtCost {
    #[serde(default)]
    pub mana: Option<f64>,
}

/// Um nível da Arte (`niveis[]`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ArtLevel {
    #[serde(rename = "nivel")]
    pub level: f64,
    #[serde(rename = "nome")]
    pub name: String,
    #[serde(rename = "efeito")]
    pub effect: String,
    #[serde(rename = "custo", default)]
    pub cost: ArtCost,
    #[serde(rename = "exemplos", default)]
    pub examples: Vec<String>,
}

/// O bloco `grid` da Arte.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ArtGrid {
    /// `elemento`: o elemento no tabuleiro; `null` nas universais.
    #[serde(rename = "elemento", default)]
    pub element: Option<String>,
    #[serde(rename = "cor")]
    pub color: String,
    #[serde(rename = "dadoPorNivel")]
    pub dice_per_level: f64,
    #[serde(rename = "danoBruto", default)]
    pub raw_damage: bool,
}

/// Uma Arte.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Art {
    pub id: String,
    #[serde(rename = "nome")]
    pub name: String,
    #[serde(rename = "categoria")]
    pub category: ArtCategory,
    #[serde(rename = "atributo_conjuracao")]
    pub casting_attribute: String,
    #[serde(rename = "niveis")]
    pub levels: Vec<ArtLevel>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(rename = "pendente", default)]
    pub pending: bool,
    #[serde(default)]
    pub grid: Option<ArtGrid>,
}

/// Lê `artes.json` a partir do texto. Id vazio ou repetido dá `Invalid`.
pub fn parse_arts(json: &str) -> Result<Vec<Art>, DataError> {
    let list: Vec<Art> = crate::json::parse(json)?;
    crate::json::check_ids(list.iter().map(|a| a.id.as_str()))?;
    Ok(list)
}

/// Lê `<content_dir>/data/artes.json` (só no nativo).
#[cfg(not(target_arch = "wasm32"))]
pub fn load_arts(content_dir: &std::path::Path) -> Result<Vec<Art>, DataError> {
    let (name, text) = crate::json::read_data_file(content_dir, "artes.json")?;
    parse_arts(&text).map_err(|e| e.in_file(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: &str = r##"{"id":"fogo","nome":"Fogo","categoria":"elemental","atributo_conjuracao":"inteligencia",
        "niveis":[{"nivel":1,"nome":"Faísca","efeito":"x","custo":{"mana":1},"exemplos":["a"]}],
        "aliases":[],"pendente":false,"grid":{"elemento":null,"cor":"#f00","dadoPorNivel":1,"danoBruto":false}}"##;

    #[test]
    fn reads_an_art() {
        let a = &parse_arts(&format!("[{ONE}]")).unwrap()[0];
        assert_eq!(a.category, ArtCategory::Elemental);
        assert_eq!(a.levels[0].cost.mana, Some(1.0));
        assert_eq!(a.grid.as_ref().unwrap().element, None);
    }

    #[test]
    fn rejects_a_repeated_id() {
        assert!(matches!(
            parse_arts(&format!("[{ONE},{ONE}]")),
            Err(DataError::Invalid { .. })
        ));
    }
}
