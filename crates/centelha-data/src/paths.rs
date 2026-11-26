//! Os Caminhos (as Proezas): `content/data/caminhos.json`.
//!
//! Cada Caminho tem a trilha (corpo, voz ou mente), o atributo e a
//! Habilidade âncora. As Técnicas apontam para o Caminho pelo `id`.

use serde::Deserialize;

use crate::DataError;

/// A trilha do Caminho (`trilha`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Track {
    #[serde(rename = "corpo")]
    Body,
    #[serde(rename = "voz")]
    Voice,
    #[serde(rename = "mente")]
    Mind,
}

impl Track {
    pub fn id(self) -> &'static str {
        match self {
            Track::Body => "corpo",
            Track::Voice => "voz",
            Track::Mind => "mente",
        }
    }
}

/// Um Caminho.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Path {
    pub id: String,
    #[serde(rename = "nome")]
    pub name: String,
    #[serde(rename = "trilha")]
    pub track: Track,
    #[serde(rename = "atributo")]
    pub attribute: String,
    /// `habilidade_ancora`: o nome da Habilidade (texto do livro).
    #[serde(rename = "habilidade_ancora")]
    pub anchor_skill: String,
    #[serde(rename = "descricao")]
    pub description: String,
    #[serde(rename = "nota", default)]
    pub note: Option<String>,
}

/// Lê `caminhos.json` a partir do texto. Id vazio ou repetido dá `Invalid`.
pub fn parse_paths(json: &str) -> Result<Vec<Path>, DataError> {
    let list: Vec<Path> = crate::json::parse(json)?;
    crate::json::check_ids(list.iter().map(|p| p.id.as_str()))?;
    Ok(list)
}

/// Lê `<content_dir>/data/caminhos.json` (só no nativo).
#[cfg(not(target_arch = "wasm32"))]
pub fn load_paths(content_dir: &std::path::Path) -> Result<Vec<Path>, DataError> {
    let (name, text) = crate::json::read_data_file(content_dir, "caminhos.json")?;
    parse_paths(&text).map_err(|e| e.in_file(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: &str = r#"{"id":"vento","nome":"Vento","trilha":"corpo","atributo":"destreza",
        "habilidade_ancora":"Esquiva","descricao":"x","outro":true}"#;

    #[test]
    fn reads_a_path() {
        let p = &parse_paths(&format!("[{ONE}]")).unwrap()[0];
        assert_eq!((p.track, p.note.as_deref()), (Track::Body, None));
    }

    #[test]
    fn rejects_a_repeated_id() {
        assert!(matches!(
            parse_paths(&format!("[{ONE},{ONE}]")),
            Err(DataError::Invalid { .. })
        ));
    }
}
