//! O glossário do livro: `content/data/glossario.json`.
//!
//! Cada termo tem um `id` (a âncora na página do glossário), o nome, os
//! sinônimos e a definição. A ordem do arquivo é a da fonte; quem mostra a
//! lista decide como ordenar.

use serde::Deserialize;

use crate::DataError;

/// Um termo do glossário.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GlossaryEntry {
    pub id: String,
    #[serde(rename = "termo")]
    pub term: String,
    /// Os sinônimos (ajudam a busca); podem faltar.
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(rename = "definicao")]
    pub definition: String,
}

/// Lê `glossario.json` a partir do texto. Id vazio ou repetido dá `Invalid`.
pub fn parse_glossary(json: &str) -> Result<Vec<GlossaryEntry>, DataError> {
    let entries: Vec<GlossaryEntry> = crate::json::parse(json)?;
    crate::json::check_ids(entries.iter().map(|e| e.id.as_str()))?;
    Ok(entries)
}

/// Lê `<content_dir>/data/glossario.json` (só no nativo).
#[cfg(not(target_arch = "wasm32"))]
pub fn load_glossary(content_dir: &std::path::Path) -> Result<Vec<GlossaryEntry>, DataError> {
    let (name, text) = crate::json::read_data_file(content_dir, "glossario.json")?;
    parse_glossary(&text).map_err(|e| e.in_file(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_an_entry() {
        let json = r#"[{"id":"pool","termo":"Pool","aliases":["parada"],"definicao":"Os dados."}]"#;
        let entries = parse_glossary(json).unwrap();
        assert_eq!(
            entries,
            [GlossaryEntry {
                id: "pool".into(),
                term: "Pool".into(),
                aliases: vec!["parada".into()],
                definition: "Os dados.".into(),
            }]
        );
    }

    #[test]
    fn aliases_may_be_missing() {
        let json = r#"[{"id":"pool","termo":"Pool","definicao":"Os dados."}]"#;
        assert!(parse_glossary(json).unwrap()[0].aliases.is_empty());
    }

    fn invalid_reason(json: &str) -> String {
        match parse_glossary(json) {
            Err(DataError::Invalid { reason, .. }) => reason,
            other => panic!("esperava Invalid, veio {other:?}"),
        }
    }

    #[test]
    fn empty_or_repeated_id_is_invalid() {
        let empty = r#"[{"id":" ","termo":"A","definicao":"x"}]"#;
        assert_eq!(invalid_reason(empty), "id vazio");
        let repeated =
            r#"[{"id":"a","termo":"A","definicao":"x"},{"id":"a","termo":"B","definicao":"y"}]"#;
        assert_eq!(invalid_reason(repeated), "id repetido: a");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn loads_the_glossary() {
        let content = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let entries = load_glossary(&content).unwrap();
        assert!(entries.len() > 20);
        assert!(
            entries
                .iter()
                .all(|e| !e.term.is_empty() && !e.definition.is_empty())
        );
    }
}
