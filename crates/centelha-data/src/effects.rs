//! Os Efeitos das Artes: `content/data/efeitos.json`.
//!
//! Cada Efeito tem o nível, as Artes que o comportam (com o sabor de cada
//! uma), os parâmetros da conjuração, o texto e o bloco `grid` que a mesa lê.
//! As listas abertas do `grid` (forma, âncora, gatilho, alvo) ficam como
//! texto.

use serde::Deserialize;

use crate::DataError;

/// Uma Arte que comporta o Efeito (`artes[]`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct EffectArt {
    /// O id da Arte em `artes.json`.
    pub id: String,
    #[serde(rename = "sabor")]
    pub flavor: String,
}

/// Como o parâmetro entra na conta (`tipo`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ParameterKind {
    /// `padrao`: a régua comum do parâmetro.
    #[serde(rename = "padrao")]
    Standard,
    /// `substitui`: troca outro parâmetro (`replaces`).
    #[serde(rename = "substitui")]
    Replaces,
    /// `fixo`: um valor só.
    #[serde(rename = "fixo")]
    Fixed,
}

impl ParameterKind {
    pub fn id(self) -> &'static str {
        match self {
            ParameterKind::Standard => "padrao",
            ParameterKind::Replaces => "substitui",
            ParameterKind::Fixed => "fixo",
        }
    }
}

/// Um parâmetro da conjuração (`parametros[]`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct EffectParameter {
    #[serde(rename = "nome")]
    pub name: String,
    #[serde(rename = "tipo")]
    pub kind: ParameterKind,
    /// `substitui`: o parâmetro trocado (`area`...).
    #[serde(rename = "substitui", default)]
    pub replaces: Option<String>,
    #[serde(rename = "unidade", default)]
    pub unit: Option<String>,
    /// `escala`: os valores por nível, como texto.
    #[serde(rename = "escala", default)]
    pub scale: Option<Vec<String>>,
    #[serde(rename = "nota", default)]
    pub note: Option<String>,
    /// `regua`: a régua da Duração (`breve`, `longa`).
    #[serde(rename = "regua", default)]
    pub ruler: Option<String>,
    #[serde(rename = "valor", default)]
    pub value: Option<String>,
    #[serde(rename = "pontos", default)]
    pub points: Option<f64>,
    #[serde(rename = "porNivel", default)]
    pub per_level: Option<bool>,
    #[serde(rename = "custaMana", default)]
    pub mana_cost: Option<f64>,
}

/// O bloco `grid` do Efeito.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct EffectGrid {
    #[serde(rename = "forma")]
    pub shape: String,
    #[serde(rename = "ancora")]
    pub anchor: String,
    #[serde(rename = "gatilho")]
    pub trigger: String,
    #[serde(rename = "alvo")]
    pub target: String,
    #[serde(rename = "persiste", default)]
    pub persists: bool,
    #[serde(rename = "materia", default)]
    pub matter: Option<String>,
    #[serde(rename = "condicao", default)]
    pub condition: Option<String>,
    #[serde(rename = "condicaoAparente", default)]
    pub apparent_condition: Option<String>,
    #[serde(rename = "pegaItem", default)]
    pub takes_item: bool,
    #[serde(rename = "arenaInteira", default)]
    pub whole_arena: bool,
    #[serde(rename = "dissipa", default)]
    pub dispels: bool,
    #[serde(rename = "fere", default)]
    pub harms: bool,
    #[serde(rename = "cura", default)]
    pub heals: bool,
    #[serde(rename = "teste", default)]
    pub test: bool,
}

/// Um Efeito.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Effect {
    pub id: String,
    #[serde(rename = "nome")]
    pub name: String,
    #[serde(rename = "nivel")]
    pub level: f64,
    #[serde(rename = "escalonavel", default)]
    pub scalable: bool,
    #[serde(rename = "artes", default)]
    pub arts: Vec<EffectArt>,
    #[serde(rename = "parametros", default)]
    pub parameters: Vec<EffectParameter>,
    /// `efeito`: o texto do Efeito.
    #[serde(rename = "efeito")]
    pub text: String,
    #[serde(default)]
    pub grid: Option<EffectGrid>,
    #[serde(rename = "notas", default)]
    pub notes: Option<String>,
    /// `acaoLivre`: conjura sem gastar a ação.
    #[serde(rename = "acaoLivre", default)]
    pub free_action: bool,
}

/// Lê `efeitos.json` a partir do texto. Id vazio ou repetido dá `Invalid`.
pub fn parse_effects(json: &str) -> Result<Vec<Effect>, DataError> {
    let list: Vec<Effect> = crate::json::parse(json)?;
    crate::json::check_ids(list.iter().map(|e| e.id.as_str()))?;
    Ok(list)
}

/// Lê `<content_dir>/data/efeitos.json` (só no nativo).
#[cfg(not(target_arch = "wasm32"))]
pub fn load_effects(content_dir: &std::path::Path) -> Result<Vec<Effect>, DataError> {
    let (name, text) = crate::json::read_data_file(content_dir, "efeitos.json")?;
    parse_effects(&text).map_err(|e| e.in_file(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: &str = r#"{"id":"neblina","nome":"Neblina","nivel":1,"escalonavel":true,
        "artes":[{"id":"agua","sabor":"x"}],
        "parametros":[{"nome":"Volume","tipo":"substitui","substitui":"area","escala":["1","2"]}],
        "efeito":"x","grid":{"forma":"zona","ancora":"ponto","gatilho":"imediato","alvo":"varios",
        "materia":null,"condicao":"cego","fere":true}}"#;

    #[test]
    fn reads_an_effect() {
        let e = &parse_effects(&format!("[{ONE}]")).unwrap()[0];
        assert_eq!(e.parameters[0].kind, ParameterKind::Replaces);
        assert_eq!(e.parameters[0].replaces.as_deref(), Some("area"));
        let g = e.grid.as_ref().unwrap();
        assert!(g.harms && !g.heals && !e.free_action);
        assert_eq!(g.condition.as_deref(), Some("cego"));
    }

    #[test]
    fn rejects_an_empty_id() {
        let empty = ONE.replace(r#""id":"neblina""#, r#""id":"""#);
        assert!(matches!(
            parse_effects(&format!("[{empty}]")),
            Err(DataError::Invalid { .. })
        ));
    }
}
