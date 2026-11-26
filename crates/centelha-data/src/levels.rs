//! A descrição de um nível, comum a Atributos, Habilidades, Habilidades
//! Secundárias e Virtudes: o número, o rótulo ("Leigo", "Fraco") e o texto
//! que diz o que esse nível faz na cena.

use serde::Deserialize;

/// Um nível descrito (`niveis[]`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LevelText {
    #[serde(rename = "nivel")]
    pub level: u32,
    #[serde(rename = "rotulo")]
    pub label: String,
    #[serde(rename = "texto")]
    pub text: String,
    /// Só nas Virtudes: como quem tem esse nível age.
    #[serde(rename = "conduta", default)]
    pub conduct: Option<String>,
}
