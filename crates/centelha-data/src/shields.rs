//! O catálogo de escudos: `content/data/escudos.json`.
//!
//! Cada peça traz o bloco `escudo` (Bloqueio, Penalidade e o bloqueio do
//! projétil rápido), o que a página mostra (as notas, o preço, o peso, o
//! acesso e as tags) e o envelope inteiro, que o motor usa para montar o
//! catálogo da ficha.

use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::DataError;
use crate::catalog_item::{Price, read_item};

/// Uma peça do catálogo de escudos.
#[derive(Debug, Clone, PartialEq)]
pub struct Shield {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    /// O bloco `escudo`; `null` numa peça que não é escudo.
    pub stats: Option<ShieldStats>,
    /// `tipo`: `escudo`.
    pub kind: Option<String>,
    /// `preco`.
    pub price: Option<Price>,
    /// `peso`, em quilos.
    pub weight: Option<f64>,
    /// `acesso`: o quanto a peça é fácil de achar.
    pub access: Option<f64>,
    /// `descricao`: o texto do livro sobre a peça (`notas` no catálogo
    /// achatado).
    pub description: Option<String>,
    envelope: Value,
}

/// Os campos tipados de [`Shield`], como vêm do JSON.
#[derive(Deserialize)]
struct ShieldFields {
    id: String,
    #[serde(rename = "nome")]
    name: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(rename = "escudo", default)]
    stats: Option<ShieldStats>,
    #[serde(rename = "tipo", default)]
    kind: Option<String>,
    #[serde(rename = "preco", default)]
    price: Option<Price>,
    #[serde(rename = "peso", default)]
    weight: Option<f64>,
    #[serde(rename = "acesso", default)]
    access: Option<f64>,
    #[serde(rename = "descricao", default)]
    description: Option<String>,
}

impl<'de> Deserialize<'de> for Shield {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Shield, D::Error> {
        let (f, envelope): (ShieldFields, Value) = read_item(d)?;
        Ok(Shield {
            id: f.id,
            name: f.name,
            tags: f.tags,
            stats: f.stats,
            kind: f.kind,
            price: f.price,
            weight: f.weight,
            access: f.access,
            description: f.description,
            envelope,
        })
    }
}

impl Shield {
    /// A peça inteira como está no JSON (as chaves na ordem do arquivo).
    pub fn envelope(&self) -> &Value {
        &self.envelope
    }
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
