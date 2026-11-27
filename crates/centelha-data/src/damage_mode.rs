//! O modo de ataque: corte, perfuração ou impacto.
//!
//! É o `tipo` de cada entrada de `modos[]` e o `tipoDano` das armas do
//! catálogo, e o modo que o golpe, a Absorção e o motor da criatura usam. Um
//! tipo só para o workspace: o motor o reexporta em `character::DamageMode`.

use serde::Deserialize;

/// Um modo de ataque (`Modo`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum DamageMode {
    #[serde(rename = "corte")]
    Cut,
    #[serde(rename = "perfurante")]
    Pierce,
    #[serde(rename = "impacto")]
    Impact,
}

impl DamageMode {
    /// Os três modos, na ordem de sempre.
    pub const ALL: [DamageMode; 3] = [DamageMode::Cut, DamageMode::Pierce, DamageMode::Impact];

    /// A chave gravada (`corte`, `perfurante`, `impacto`).
    pub fn id(self) -> &'static str {
        match self {
            DamageMode::Cut => "corte",
            DamageMode::Pierce => "perfurante",
            DamageMode::Impact => "impacto",
        }
    }

    /// O modo de uma chave gravada, ou `None`.
    pub fn from_id(id: &str) -> Option<DamageMode> {
        DamageMode::ALL.into_iter().find(|m| m.id() == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_go_and_come_back() {
        for m in DamageMode::ALL {
            assert_eq!(DamageMode::from_id(m.id()), Some(m));
        }
        assert_eq!(DamageMode::from_id("fogo"), None);
        assert_eq!(DamageMode::from_id("perfuracao"), None);
    }

    #[test]
    fn reads_from_the_json_keys() {
        let modes: Vec<DamageMode> =
            serde_json::from_str(r#"["corte","perfurante","impacto"]"#).unwrap();
        assert_eq!(modes, DamageMode::ALL);
        assert!(serde_json::from_str::<DamageMode>(r#""fogo""#).is_err());
    }
}
