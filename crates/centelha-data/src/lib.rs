//! Leitura e validação do conteúdo e dos dados de `content/`.
//!
//! Os JSON de `content/data` têm as chaves em português (vêm da fonte do
//! conteúdo). Os tipos daqui usam nomes em inglês, pelo `GLOSSARY.md`, e a
//! tradução acontece na leitura, com `serde(rename)`. O JSON nunca é reescrito.
//!
//! A leitura a partir de texto (`parse_*`) funciona em WebAssembly. A leitura a
//! partir de pasta (`load_*`) existe só no nativo.

pub mod ancestry;
pub mod armor;
pub mod bestiary;
pub mod chapters;
mod error;
pub mod glossary;
mod json;
pub mod rules;
pub mod shields;
pub mod weapons;

#[cfg(not(target_arch = "wasm32"))]
pub use ancestry::load_ancestries;
pub use ancestry::{Ancestry, parse_ancestries};
#[cfg(not(target_arch = "wasm32"))]
pub use armor::load_armors;
pub use armor::{Armor, parse_armors};
#[cfg(not(target_arch = "wasm32"))]
pub use bestiary::load_bestiary;
pub use bestiary::{Creature, parse_creature};
#[cfg(not(target_arch = "wasm32"))]
pub use chapters::load_chapters;
pub use chapters::{Chapter, parse_chapter};
pub use error::DataError;
#[cfg(not(target_arch = "wasm32"))]
pub use glossary::load_glossary;
pub use glossary::{GlossaryEntry, parse_glossary};
#[cfg(not(target_arch = "wasm32"))]
pub use rules::load_rules;
pub use rules::{Rules, parse_rules};
#[cfg(not(target_arch = "wasm32"))]
pub use shields::load_shields;
pub use shields::{Shield, parse_shields};
#[cfg(not(target_arch = "wasm32"))]
pub use weapons::load_weapons;
pub use weapons::{Weapon, parse_weapons};
