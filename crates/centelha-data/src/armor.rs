//! O catálogo de armaduras: `content/data/armaduras.json`.
//!
//! Cada peça traz o bloco `armadura` (a classe, a Absorção, o Nível de
//! Perfuração e a Penalidade), o que a página mostra (as notas, o preço, o
//! peso, o acesso e as tags) e o envelope inteiro, que o motor usa para montar
//! o catálogo da ficha.

use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::DataError;
use crate::catalog_item::{Price, read_item};

/// Uma peça do catálogo de armaduras.
#[derive(Debug, Clone, PartialEq)]
pub struct Armor {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    /// O bloco `armadura`; `null` numa peça que não é armadura.
    pub stats: Option<ArmorStats>,
    /// `tipo`: `armadura`.
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

/// Os campos tipados de [`Armor`], como vêm do JSON.
#[derive(Deserialize)]
struct ArmorFields {
    id: String,
    #[serde(rename = "nome")]
    name: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(rename = "armadura", default)]
    stats: Option<ArmorStats>,
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

impl<'de> Deserialize<'de> for Armor {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Armor, D::Error> {
        let (f, envelope): (ArmorFields, Value) = read_item(d)?;
        Ok(Armor {
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

impl Armor {
    /// A peça inteira como está no JSON (as chaves na ordem do arquivo).
    ///
    /// O `Catalog::from_data` do motor monta o catálogo da ficha por este
    /// envelope, e não pelos campos tipados: quem lê a peça para mostrar usa
    /// os campos tipados, e mudar um campo tipado não muda o catálogo do
    /// motor.
    pub fn envelope(&self) -> &Value {
        &self.envelope
    }
}

/// O bloco `armadura` de uma peça.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ArmorStats {
    /// `classe`: `nenhuma`, `leve`, `media`, `pesada`...
    #[serde(rename = "classe", default)]
    pub class: Option<String>,
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
    let armors: Vec<Armor> = crate::json::parse(json)?;
    for (i, x) in armors.iter().enumerate() {
        if !crate::bestiary::is_id(&x.id) {
            return Err(DataError::invalid(format!("[{i}].id"), "id inválido"));
        }
    }
    Ok(armors)
}

/// Lê `<content_dir>/data/armaduras.json` (só no nativo).
#[cfg(not(target_arch = "wasm32"))]
pub fn load_armors(content_dir: &std::path::Path) -> Result<Vec<Armor>, DataError> {
    let (name, text) = crate::json::read_data_file(content_dir, "armaduras.json")?;
    parse_armors(&text).map_err(|e| e.in_file(name))
}
