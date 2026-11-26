//! O catálogo de escudos: `content/data/escudos.json`.
//!
//! Primeira versão: só o bloco `escudo` (Bloqueio, Penalidade e o bloqueio do
//! projétil rápido).

use serde::Deserialize;

use crate::DataError;

/// Uma peça do catálogo de escudos.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Shield {
    pub id: String,
    #[serde(rename = "nome")]
    pub name: String,
    /// O bloco `escudo`; `null` numa peça que não é escudo.
    #[serde(rename = "escudo", default)]
    pub stats: Option<ShieldStats>,
}

/// O bloco `escudo` de uma peça.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ShieldStats {
    /// O que o escudo soma ao Bloqueio corpo a corpo.
    #[serde(rename = "bloqCaC", default)]
    pub melee_block: Option<f64>,
    #[serde(rename = "penalidade", default)]
    pub penalty: Option<f64>,
    #[serde(rename = "vsProjetilRapido", default)]
    pub vs_fast_projectile: Option<FastProjectile>,
}

/// O escudo contra o projétil rápido.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct FastProjectile {
    #[serde(rename = "bloqueia", default)]
    pub blocks: Option<bool>,
    #[serde(default)]
    pub bonus: Option<f64>,
}

/// Lê `escudos.json` a partir do texto.
pub fn parse_shields(json: &str) -> Result<Vec<Shield>, DataError> {
    let shields: Vec<Shield> = crate::json::parse(json)?;
    for (i, x) in shields.iter().enumerate() {
        if !crate::bestiary::is_id(&x.id) {
            return Err(DataError::invalid(format!("[{i}].id"), "id inválido"));
        }
    }
    Ok(shields)
}

/// Lê `<content_dir>/data/escudos.json` (só no nativo).
#[cfg(not(target_arch = "wasm32"))]
pub fn load_shields(content_dir: &std::path::Path) -> Result<Vec<Shield>, DataError> {
    let (name, text) = crate::json::read_data_file(content_dir, "escudos.json")?;
    parse_shields(&text).map_err(|e| e.in_file(name))
}
