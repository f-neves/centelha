//! Leitura e validação do conteúdo e dos dados de `content/`.
//!
//! Os JSON de `content/data` têm as chaves em português (vêm da fonte do
//! conteúdo). Os tipos daqui usam nomes em inglês, pelo `GLOSSARY.md`, e a
//! tradução acontece na leitura, com `serde(rename)`. O JSON nunca é reescrito.
//!
//! A leitura a partir de texto (`parse_*`) funciona em WebAssembly. A leitura a
//! partir de pasta (`load_*`) existe só no nativo.

pub mod bestiary;
mod error;

#[cfg(not(target_arch = "wasm32"))]
pub use bestiary::load_bestiary;
pub use bestiary::{Creature, parse_creature};
pub use error::DataError;
