//! Os Atributos: `content/data/atributos.json`.
//!
//! Cada Atributo tem o id (a chave em `attrs` da ficha), o nome, o grupo
//! (`fisico`, `social` ou `mental`), a descrição e a descrição de cada nível.
//! A ordem do arquivo é a da ficha.

use serde::Deserialize;

use crate::DataError;
use crate::levels::LevelText;

/// Um Atributo.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Attribute {
    pub id: String,
    #[serde(rename = "nome")]
    pub name: String,
    /// O grupo como está no arquivo (`fisico`, `social`, `mental`).
    #[serde(rename = "grupo")]
    pub group: String,
    #[serde(rename = "descricao")]
    pub description: String,
    #[serde(rename = "niveis", default)]
    pub levels: Vec<LevelText>,
}

/// Lê `atributos.json` a partir do texto. Id vazio ou repetido dá `Invalid`.
pub fn parse_attributes(json: &str) -> Result<Vec<Attribute>, DataError> {
    let list: Vec<Attribute> = crate::json::parse(json)?;
    crate::json::check_ids(list.iter().map(|a| a.id.as_str()))?;
    Ok(list)
}

/// Lê `<content_dir>/data/atributos.json` (só no nativo).
#[cfg(not(target_arch = "wasm32"))]
pub fn load_attributes(content_dir: &std::path::Path) -> Result<Vec<Attribute>, DataError> {
    let (name, text) = crate::json::read_data_file(content_dir, "atributos.json")?;
    parse_attributes(&text).map_err(|e| e.in_file(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_an_attribute() {
        let json = r#"[{"id":"forca","nome":"Força","grupo":"fisico","descricao":"O corpo.",
            "niveis":[{"nivel":1,"rotulo":"Fraco","texto":"Perde."}],"novo":1}]"#;
        let a = &parse_attributes(json).unwrap()[0];
        assert_eq!(
            (a.id.as_str(), a.name.as_str(), a.group.as_str()),
            ("forca", "Força", "fisico")
        );
        assert_eq!(a.levels[0].level, 1);
        assert_eq!(a.levels[0].label, "Fraco");
        assert_eq!(a.levels[0].conduct, None);
    }

    #[test]
    fn repeated_id_is_invalid() {
        let json = r#"[{"id":"a","nome":"A","grupo":"g","descricao":"x"},
            {"id":"a","nome":"B","grupo":"g","descricao":"y"}]"#;
        assert!(matches!(
            parse_attributes(json),
            Err(DataError::Invalid { .. })
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn reads_the_content_file() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let list = load_attributes(&dir).unwrap();
        assert_eq!(list.len(), 9);
        assert_eq!(list.iter().map(|a| a.levels.len()).sum::<usize>(), 54);
        assert_eq!(list[0].id, "forca");
    }
}
