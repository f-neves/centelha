//! O catálogo de armaduras: `content/data/armaduras.json`.
//!
//! Primeira versão: só o bloco `armadura` (Absorção, Nível de Perfuração e
//! Penalidade).

use serde::Deserialize;

use crate::DataError;

/// Uma peça do catálogo de armaduras.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Armor {
    pub id: String,
    #[serde(rename = "nome")]
    pub name: String,
    /// O bloco `armadura`; `null` numa peça que não é armadura.
    #[serde(rename = "armadura", default)]
    pub stats: Option<ArmorStats>,
}

/// O bloco `armadura` de uma peça.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ArmorStats {
    #[serde(default)]
    pub soak: Option<ArmorSoak>,
    /// O Nível de Perfuração que a armadura resiste.
    #[serde(rename = "resistPerf", default)]
    pub pierce_level: Option<f64>,
    #[serde(rename = "penalidade", default)]
    pub penalty: Option<f64>,
}

/// A Absorção da armadura por categoria.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ArmorSoak {
    #[serde(rename = "impacto", default)]
    pub impact: Option<f64>,
    #[serde(rename = "corte", default)]
    pub cut: Option<f64>,
    #[serde(rename = "perfuracao", default)]
    pub pierce: Option<f64>,
}

/// Lê `armaduras.json` a partir do texto.
pub fn parse_armors(json: &str) -> Result<Vec<Armor>, DataError> {
    crate::json::parse(json)
}

/// Lê `<content_dir>/data/armaduras.json` (só no nativo).
#[cfg(not(target_arch = "wasm32"))]
pub fn load_armors(content_dir: &std::path::Path) -> Result<Vec<Armor>, DataError> {
    let (name, text) = crate::json::read_data_file(content_dir, "armaduras.json")?;
    parse_armors(&text).map_err(|e| e.in_file(name))
}
