//! As Virtudes: `content/data/virtudes.json`.
//!
//! Cada Virtude tem o id, o nome, o que ela resiste ("à crueldade"), a
//! descrição e a descrição de cada nível, com a conduta.

use serde::Deserialize;

use crate::DataError;
use crate::levels::LevelText;

/// Uma Virtude.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Virtue {
    pub id: String,
    #[serde(rename = "nome")]
    pub name: String,
    /// O que a Virtude resiste, como o livro escreve ("à crueldade").
    #[serde(rename = "resiste")]
    pub resists: String,
    #[serde(rename = "descricao")]
    pub description: String,
    #[serde(rename = "niveis", default)]
    pub levels: Vec<LevelText>,
}

/// Lê `virtudes.json` a partir do texto. Id vazio ou repetido dá `Invalid`.
pub fn parse_virtues(json: &str) -> Result<Vec<Virtue>, DataError> {
    let list: Vec<Virtue> = crate::json::parse(json)?;
    crate::json::check_ids(list.iter().map(|v| v.id.as_str()))?;
    Ok(list)
}

/// Lê `<content_dir>/data/virtudes.json` (só no nativo).
#[cfg(not(target_arch = "wasm32"))]
pub fn load_virtues(content_dir: &std::path::Path) -> Result<Vec<Virtue>, DataError> {
    let (name, text) = crate::json::read_data_file(content_dir, "virtudes.json")?;
    parse_virtues(&text).map_err(|e| e.in_file(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_virtue_with_conduct() {
        let json = r#"[{"id":"compaixao","nome":"Compaixão","resiste":"à crueldade","descricao":"x",
            "niveis":[{"nivel":1,"rotulo":"Fria","texto":"Não pesa.","conduta":"Ajuda quando convém."}]}]"#;
        let v = &parse_virtues(json).unwrap()[0];
        assert_eq!(v.resists, "à crueldade");
        assert_eq!(v.levels[0].conduct.as_deref(), Some("Ajuda quando convém."));
    }

    #[test]
    fn empty_id_is_invalid() {
        let json = r#"[{"id":"","nome":"A","resiste":"r","descricao":"x"}]"#;
        assert!(matches!(
            parse_virtues(json),
            Err(DataError::Invalid { .. })
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn reads_the_content_file() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let list = load_virtues(&dir).unwrap();
        assert_eq!(list.len(), 4);
        assert_eq!(list.iter().map(|v| v.levels.len()).sum::<usize>(), 24);
        assert!(
            list.iter()
                .flat_map(|v| &v.levels)
                .all(|l| l.conduct.is_some())
        );
    }
}
