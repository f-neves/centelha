//! A ficha do jogador: a forma gravada e a de leitura, o equipamento com o
//! item inteiro, e os números que a ficha, a mesa e o Grid mostram.
//!
//! Port de `equip.ts` e `ficha-formato.ts` (um módulo só: no TypeScript os
//! dois se importam), do miolo de `motor.ts` que serve ao personagem
//! (`numerosPersonagem`, `conjuntoPersonagem`, `combatePersonagem`) e de
//! `resumoFicha`/`resumoParaBanco` (`mesa-ficha.ts`). O Quase-Acerto
//! inteiro (`quase-acerto.ts`), os textos curtos do equipamento e a troca de
//! ids de arma da ficha gravada (`migrarRefsDeArma`) vêm junto.
//!
//! A ficha entra como `serde_json::Value`, como o TypeScript a trata: é dado
//! do jogador, gravado no banco em formas de várias épocas, com chaves que o
//! motor não conhece e que têm de voltar intactas na gravação. As regras do
//! JavaScript sobre valor ausente, `null`, falso e texto (`||`, `??`,
//! `Number()`, `String()`) estão em `value`. Os números de saída são tipados.
//!
//! Quem confere é o teste de paridade contra `tests/parity/sheets.json`.

mod editor;
mod equip;
mod format;
mod migrate;
mod near_miss;
mod numbers;
mod text;
pub(crate) mod value;

use std::collections::HashMap;

use centelha_data::{Ancestry, Rules};
use serde_json::Value;

pub use editor::{
    EditorCatalogs, EditorTables, FREE_SECONDARY, FRESH_BUDGET, Specialty, SpentXp,
    TechniqueStatus, attribute_floor, change_ancestry, clamp_specialties, effects_in_reach,
    fresh_sheet, mass_chosen, specialty_cap, spent_xp, technique_status, trait_cap,
};
pub use equip::{Catalog, ItemKind, PieceKind, flatten_item, piece_kind};
pub use format::{for_reading, for_saving, is_saved_form};
pub use migrate::{WEAPON_RENAMES, migrate_weapon_refs};
pub use near_miss::{
    ArmorNearMiss, AttackOutcome, Graze, NearMissAttacker, NearMissOutcome, NearMissTarget,
    WeaponNearMiss, attack_outcome, encounter_near_miss, grazes, missed_by,
};
pub use numbers::{
    ArmorStack, ArtLevel, CombatSummary, DamageVersion, DbSummary, DualWield, HandItem, HandKind,
    HandSet, Initiative, Movement, NearMiss, Passive, SheetNumbers, SheetSummary, Step,
    active_hand_set, average_damage_of_expr, combat_summary, db_summary, hand_item, hand_set,
    movement, player_mass, sheet_numbers, sheet_numbers_read, sheet_summary, sheet_summary_read,
    stack_armor,
};
pub use text::{armor_stats, damage_text, shield_stats, signed_number, weapon_stats};

/// Erro ao montar as tabelas da ficha.
#[derive(Debug, Clone, PartialEq)]
pub enum SheetError {
    /// O arquivo do catálogo não é uma lista.
    CatalogNotList(&'static str),
    /// O texto do catálogo não é JSON.
    CatalogJson { file: &'static str, reason: String },
}

impl std::fmt::Display for SheetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SheetError::CatalogNotList(file) => {
                write!(f, "{file}: o catálogo precisa ser uma lista")
            }
            SheetError::CatalogJson { file, reason } => write!(f, "{file}: {reason}"),
        }
    }
}

impl std::error::Error for SheetError {}

/// O que as contas da ficha leem: as regras, as Raças e o catálogo de
/// equipamento.
#[derive(Debug, Clone)]
pub struct SheetTables {
    rules: Rules,
    ancestries: HashMap<String, Ancestry>,
    catalog: Catalog,
}

impl SheetTables {
    /// Monta as tabelas. A Raça repetida fica com a última (como o
    /// `Object.fromEntries` do TypeScript).
    pub fn new(rules: Rules, ancestries: Vec<Ancestry>, catalog: Catalog) -> Self {
        let ancestries = ancestries.into_iter().map(|a| (a.id.clone(), a)).collect();
        SheetTables {
            rules,
            ancestries,
            catalog,
        }
    }

    pub fn rules(&self) -> &Rules {
        &self.rules
    }

    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    /// A Raça pelo valor de `raca` na ficha (`RACA[S.raca]`).
    pub fn ancestry_of(&self, raca: &Value) -> Option<&Ancestry> {
        self.ancestry(Some(raca))
    }

    fn ancestry(&self, raca: Option<&Value>) -> Option<&Ancestry> {
        self.ancestries.get(&value::property_key(raca))
    }
}
