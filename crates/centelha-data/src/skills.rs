//! As Habilidades (`content/data/habilidades.json`) e as Habilidades
//! Secundárias (`content/data/habilidades-secundarias.json`).
//!
//! A Habilidade tem o id (a chave em `skills` da ficha), o nome, o grupo
//! (`combate`, `fisica`, `social`, `saber`), os Atributos que costumam ir com
//! ela, a descrição e a descrição de cada nível. A Secundária (a chave em
//! `skills2`) tem o mesmo, sem os Atributos; uma delas (`escapismo`) não
//! descreve os níveis. A ordem dos arquivos é a da ficha.

use serde::Deserialize;

use crate::DataError;
use crate::levels::LevelText;

/// Uma Habilidade.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Skill {
    pub id: String,
    #[serde(rename = "nome")]
    pub name: String,
    /// O grupo como está no arquivo (`combate`, `fisica`, `social`, `saber`).
    #[serde(rename = "grupo")]
    pub group: String,
    /// Os ids dos Atributos que costumam somar com ela.
    #[serde(rename = "atributos", default)]
    pub attributes: Vec<String>,
    #[serde(rename = "descricao")]
    pub description: String,
    #[serde(rename = "niveis", default)]
    pub levels: Vec<LevelText>,
}

/// Uma Habilidade Secundária.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SecondarySkill {
    pub id: String,
    #[serde(rename = "nome")]
    pub name: String,
    /// O grupo como está no arquivo (`corpo`, `sociais`, `conhecimento`,
    /// `oficio`, `expressao`, `subterfugio`, `interior`).
    #[serde(rename = "grupo")]
    pub group: String,
    #[serde(rename = "descricao")]
    pub description: String,
    /// Vazio quando o arquivo não descreve os níveis.
    #[serde(rename = "niveis", default)]
    pub levels: Vec<LevelText>,
}

/// Lê `habilidades.json` a partir do texto. Id vazio ou repetido dá `Invalid`.
pub fn parse_skills(json: &str) -> Result<Vec<Skill>, DataError> {
    let list: Vec<Skill> = crate::json::parse(json)?;
    crate::json::check_ids(list.iter().map(|s| s.id.as_str()))?;
    Ok(list)
}

/// Lê `habilidades-secundarias.json` a partir do texto. Id vazio ou repetido
/// dá `Invalid`.
pub fn parse_secondary_skills(json: &str) -> Result<Vec<SecondarySkill>, DataError> {
    let list: Vec<SecondarySkill> = crate::json::parse(json)?;
    crate::json::check_ids(list.iter().map(|s| s.id.as_str()))?;
    Ok(list)
}

/// Lê `<content_dir>/data/habilidades.json` (só no nativo).
#[cfg(not(target_arch = "wasm32"))]
pub fn load_skills(content_dir: &std::path::Path) -> Result<Vec<Skill>, DataError> {
    let (name, text) = crate::json::read_data_file(content_dir, "habilidades.json")?;
    parse_skills(&text).map_err(|e| e.in_file(name))
}

/// Lê `<content_dir>/data/habilidades-secundarias.json` (só no nativo).
#[cfg(not(target_arch = "wasm32"))]
pub fn load_secondary_skills(
    content_dir: &std::path::Path,
) -> Result<Vec<SecondarySkill>, DataError> {
    let (name, text) = crate::json::read_data_file(content_dir, "habilidades-secundarias.json")?;
    parse_secondary_skills(&text).map_err(|e| e.in_file(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_skill() {
        let json = r#"[{"id":"briga","nome":"Briga","grupo":"combate","atributos":["forca","destreza"],
            "descricao":"A luta.","niveis":[{"nivel":0,"rotulo":"Leigo","texto":"Soco largo."}]}]"#;
        let s = &parse_skills(json).unwrap()[0];
        assert_eq!(s.attributes, ["forca", "destreza"]);
        assert_eq!(s.levels[0].level, 0);
    }

    #[test]
    fn secondary_levels_may_be_missing() {
        let json = r#"[{"id":"escapismo","nome":"Escapismo","grupo":"corpo","descricao":"x"}]"#;
        assert!(parse_secondary_skills(json).unwrap()[0].levels.is_empty());
    }

    #[test]
    fn repeated_id_is_invalid() {
        let json = r#"[{"id":"a","nome":"A","grupo":"g","descricao":"x"},
            {"id":"a","nome":"B","grupo":"g","descricao":"y"}]"#;
        assert!(matches!(
            parse_secondary_skills(json),
            Err(DataError::Invalid { .. })
        ));
        assert!(matches!(parse_skills(json), Err(DataError::Invalid { .. })));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn reads_the_content_files() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let skills = load_skills(&dir).unwrap();
        assert_eq!(skills.len(), 24);
        assert_eq!(skills.iter().map(|s| s.levels.len()).sum::<usize>(), 168);
        assert_eq!(skills[0].id, "briga");
        let secondary = load_secondary_skills(&dir).unwrap();
        assert_eq!(secondary.len(), 67);
        assert_eq!(secondary.iter().map(|s| s.levels.len()).sum::<usize>(), 462);
        let no_levels: Vec<&str> = secondary
            .iter()
            .filter(|s| s.levels.is_empty())
            .map(|s| s.id.as_str())
            .collect();
        assert_eq!(no_levels, ["escapismo"]);
    }
}
