//! As Técnicas: `content/data/tecnicas.json`.
//!
//! Cada Técnica pertence a um Caminho (`path`, o id em `caminhos.json`), tem
//! um nível de 1 a 6, a trilha de melhora (`effect`, a chave de
//! `regras.json` → `escalasProeza.trilhas`) e os pré-requisitos (ids de outras
//! Técnicas). A ordem do arquivo é a da fonte.

use serde::Deserialize;

use crate::DataError;

/// Como a Técnica se usa (`tipo`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum TechniqueKind {
    /// `passiva`: vale sempre, sem custo.
    #[serde(rename = "passiva")]
    Passive,
    /// `ativa`: custa na hora de usar.
    #[serde(rename = "ativa")]
    Active,
    /// `reflexiva`: a reação, fora da vez.
    #[serde(rename = "reflexiva")]
    Reflexive,
}

impl TechniqueKind {
    pub fn id(self) -> &'static str {
        match self {
            TechniqueKind::Passive => "passiva",
            TechniqueKind::Active => "ativa",
            TechniqueKind::Reflexive => "reflexiva",
        }
    }
}

/// O custo de usar a Técnica (`custo`); vazio na passiva.
#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize)]
pub struct TechniqueCost {
    #[serde(rename = "energia", default)]
    pub energy: Option<f64>,
    #[serde(rename = "vontade", default)]
    pub willpower: Option<f64>,
}

/// Uma Técnica.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Technique {
    pub id: String,
    #[serde(rename = "nome")]
    pub name: String,
    /// `caminho`: o id do Caminho.
    #[serde(rename = "caminho")]
    pub path: String,
    #[serde(rename = "atributo")]
    pub attribute: String,
    #[serde(rename = "tipo")]
    pub kind: TechniqueKind,
    #[serde(rename = "custo", default)]
    pub cost: TechniqueCost,
    /// Os ids das Técnicas pedidas antes.
    #[serde(default)]
    pub prereq: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(rename = "texto")]
    pub text: String,
    /// `pendente`: rascunho.
    #[serde(rename = "pendente", default)]
    pub pending: bool,
    #[serde(rename = "nivel")]
    pub level: f64,
    /// `efeito`: a trilha de melhora (`estado`, `bonus`, `dano`...).
    #[serde(rename = "efeito")]
    pub effect: String,
    /// `modulo`: o módulo do sistema; `folego` saiu do jogo.
    #[serde(rename = "modulo", default)]
    pub module: Option<String>,
}

/// Lê `tecnicas.json` a partir do texto. Id vazio ou repetido dá `Invalid`.
pub fn parse_techniques(json: &str) -> Result<Vec<Technique>, DataError> {
    let list: Vec<Technique> = crate::json::parse(json)?;
    crate::json::check_ids(list.iter().map(|t| t.id.as_str()))?;
    Ok(list)
}

/// Lê `<content_dir>/data/tecnicas.json` (só no nativo).
#[cfg(not(target_arch = "wasm32"))]
pub fn load_techniques(content_dir: &std::path::Path) -> Result<Vec<Technique>, DataError> {
    let (name, text) = crate::json::read_data_file(content_dir, "tecnicas.json")?;
    parse_techniques(&text).map_err(|e| e.in_file(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: &str = r#"{"id":"passo-veloz","nome":"Passo Veloz","caminho":"vento","atributo":"destreza",
        "tipo":"ativa","custo":{"energia":2},"prereq":["a"],"aliases":[],"texto":"x","pendente":false,
        "nivel":1,"efeito":"estado","novo":1}"#;

    #[test]
    fn reads_a_technique_and_ignores_unknown_fields() {
        let t = &parse_techniques(&format!("[{ONE}]")).unwrap()[0];
        assert_eq!(t.kind, TechniqueKind::Active);
        assert_eq!(t.cost.energy, Some(2.0));
        assert_eq!(t.cost.willpower, None);
        assert_eq!((t.level, t.module.as_deref()), (1.0, None));
    }

    #[test]
    fn rejects_an_empty_or_repeated_id() {
        let empty = ONE.replace(r#""id":"passo-veloz""#, r#""id":" ""#);
        assert!(matches!(
            parse_techniques(&format!("[{empty}]")),
            Err(DataError::Invalid { .. })
        ));
        assert!(matches!(
            parse_techniques(&format!("[{ONE},{ONE}]")),
            Err(DataError::Invalid { .. })
        ));
    }
}
