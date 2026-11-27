//! Leitura e validação do conteúdo e dos dados de `content/`.
//!
//! Os JSON de `content/data` têm as chaves em português (vêm da fonte do
//! conteúdo). Os tipos daqui usam nomes em inglês, pelo `GLOSSARY.md`, e a
//! tradução acontece na leitura, com `serde(rename)`. O JSON nunca é reescrito.
//!
//! A leitura a partir de texto (`parse_*`) funciona em WebAssembly. A leitura a
//! partir de pasta (`load_*`) existe só no nativo.

pub mod ancestry;
pub mod arcane;
pub mod armor;
pub mod arts;
pub mod attributes;
pub mod bestiary;
mod catalog_item;
pub mod chapters;
pub mod effects;
mod error;
pub mod glossary;
mod json;
pub mod levels;
pub mod paths;
pub mod rules;
pub mod shields;
pub mod skills;
pub mod techniques;
pub mod virtues;
pub mod weapons;

#[cfg(not(target_arch = "wasm32"))]
pub use ancestry::load_ancestries;
pub use ancestry::{Ancestry, parse_ancestries};
pub use arcane::{
    DetourNumbers, ImprovisationCost, ManifestationNumbers, Node, SideState, StretchRow,
};
#[cfg(not(target_arch = "wasm32"))]
pub use armor::load_armors;
pub use armor::{Armor, ArmorSoak, ArmorStats, parse_armors};
#[cfg(not(target_arch = "wasm32"))]
pub use arts::load_arts;
pub use arts::{Art, parse_arts};
#[cfg(not(target_arch = "wasm32"))]
pub use attributes::load_attributes;
pub use attributes::{Attribute, parse_attributes};
#[cfg(not(target_arch = "wasm32"))]
pub use bestiary::load_bestiary;
pub use bestiary::{Creature, parse_creature};
pub use catalog_item::Price;
#[cfg(not(target_arch = "wasm32"))]
pub use chapters::load_chapters;
pub use chapters::{Chapter, parse_chapter};
#[cfg(not(target_arch = "wasm32"))]
pub use effects::load_effects;
pub use effects::{Effect, parse_effects};
pub use error::DataError;
#[cfg(not(target_arch = "wasm32"))]
pub use glossary::load_glossary;
pub use glossary::{GlossaryEntry, parse_glossary};
pub use levels::LevelText;
#[cfg(not(target_arch = "wasm32"))]
pub use paths::load_paths;
pub use paths::parse_paths;
#[cfg(not(target_arch = "wasm32"))]
pub use rules::load_rules;
pub use rules::{Rules, parse_rules};
#[cfg(not(target_arch = "wasm32"))]
pub use shields::load_shields;
pub use shields::{FastProjectile, Shield, ShieldStats, parse_shields};
pub use skills::{SecondarySkill, Skill, parse_secondary_skills, parse_skills};
#[cfg(not(target_arch = "wasm32"))]
pub use skills::{load_secondary_skills, load_skills};
#[cfg(not(target_arch = "wasm32"))]
pub use techniques::load_techniques;
pub use techniques::{Technique, parse_techniques};
#[cfg(not(target_arch = "wasm32"))]
pub use virtues::load_virtues;
pub use virtues::{Virtue, parse_virtues};
#[cfg(not(target_arch = "wasm32"))]
pub use weapons::load_weapons;
pub use weapons::{AttackMode, DamageType, RangeBand, Weapon, WeaponStats, parse_weapons};
