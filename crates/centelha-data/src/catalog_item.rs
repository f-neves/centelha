//! O que os três catálogos de equipamento (armas, armaduras e escudos) têm em
//! comum: o preço, o envelope cru e a leitura que guarda os dois.
//!
//! Cada peça é lida duas vezes do mesmo JSON: nos campos tipados (o que as
//! páginas e as contas leem) e como o envelope inteiro, com as chaves na ordem
//! do arquivo. O motor monta o catálogo da ficha pelo envelope (a ordem das
//! chaves faz parte da paridade, ADR 0004), então o que o tipo não conhece não
//! se perde, e mudar um campo tipado não muda o catálogo do motor.

use serde::Deserialize;
use serde::de::{self, DeserializeOwned, Deserializer};
use serde_json::Value;

/// O preço de uma peça (`preco`), em moedas.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct Price {
    /// `pc`: as peças de cobre.
    #[serde(rename = "pc", default)]
    pub copper: Option<f64>,
}

/// Lê o valor inteiro e os campos tipados dele. Um erro nos campos diz o
/// caminho dentro da peça (`arma.dado`, por exemplo).
pub(crate) fn read_item<'de, D, T>(d: D) -> Result<(T, Value), D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    let value = Value::deserialize(d)?;
    let fields: T = serde_path_to_error::deserialize(value.clone())
        .map_err(|e| de::Error::custom(format!("{}: {}", e.path(), e.inner())))?;
    Ok((fields, value))
}
