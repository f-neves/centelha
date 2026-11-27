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

mod arsenal;
mod editor;
mod equip;
mod format;
mod hands;
mod migrate;
mod near_miss;
mod normalize;
mod numbers;
mod pgr;
mod strength;
mod text;
mod traits;
pub(crate) mod value;

use std::collections::HashMap;

use centelha_data::{Ancestry, Rules};
use serde_json::Value;

pub use arsenal::{
    EditTarget, PieceGroup, activate_set, add_set, add_to_arsenal, clean_arsenal,
    custom_piece_note, differs_from_origin, edit_piece_field, edit_target, is_custom_piece,
    is_custom_ref, is_two_handed_item, item_fields, parse_field_input, piece_name, piece_role,
    remove_from_arsenal, remove_set, rename_arsenal_piece, reorder, reset_piece, same_field,
    set_hand_piece, show_field, write_piece_field,
};
pub use editor::{
    EditorCatalogs, EditorTables, FREE_SECONDARY, FRESH_BUDGET, PenalizedMovement, Specialty,
    SpentXp, TechniqueStatus, attribute_floor, change_ancestry, clamp_specialties,
    effects_in_reach, fresh_sheet, half_movement_penalty, mass_chosen, penalized_movement,
    specialty_cap, spent_xp, technique_status, trait_cap,
};
pub use equip::{
    ARMOR_EDIT_FIELDS, Catalog, EditField, FieldKind, HandRole, ItemKind, PieceKind,
    SHIELD_EDIT_FIELDS, WEAPON_EDIT_FIELDS, WEAPON_MORE_FIELDS, edit_field_base, flatten_item,
    piece_kind, write_edit_field,
};
pub use format::{for_reading, for_saving, is_saved_form};
pub use hands::{
    DualAttack, DualAttackHand, dual_attack, dual_attack_from, dual_damage_text, piece_for,
    pool_text,
};
pub use migrate::{WEAPON_RENAMES, migrate_weapon_refs};
pub use near_miss::{
    ArmorNearMiss, AttackOutcome, Graze, NearMissAttacker, NearMissOutcome, NearMissTarget,
    WeaponNearMiss, attack_outcome, encounter_near_miss, grazes, missed_by,
};
pub use normalize::{FloorRaise, Normalized, normalize_sheet};
pub use numbers::{
    ArmorStack, ArtLevel, CombatSummary, DamageVersion, DbSummary, DualWield, HandItem, HandKind,
    HandSet, Initiative, Movement, NearMiss, Passive, SheetNumbers, SheetSummary, Step,
    active_hand_set, average_damage_of_expr, combat_summary, db_summary, hand_item, hand_set,
    movement, player_mass, sheet_numbers, sheet_numbers_read, sheet_summary, sheet_summary_read,
    stack_armor,
};
pub use pgr::{ExtraLine, extra_lines, sheet_anatomy, sheet_flurry_cap};
pub use strength::{
    LoadBand, carried_load, current_load, load_bands, load_speed_fraction, overhead_lift,
    penalized_speed, short_number, throw_cap, throw_range, throw_reserve, throw_table_weights,
    weight_from_text,
};
pub use text::{armor_stats, damage_text, shield_stats, signed_number, weapon_stats};
pub use traits::{
    SpecialtyScope, TraitKind, add_defense_specialty, add_named_background, add_specialty,
    click_dot, close_specialties, effective_appearance, free_secondary_name, is_free_secondary,
    named_instance, open_specialties, racial_appearance_mod, racial_cap_bonus, racial_floor_bonus,
    remove_defense_specialty, remove_named_background, remove_specialty, rename_named_background,
    rename_specialty, set_body_mass, set_budget, set_free_secondary_name, set_identity_field,
    set_specialty_level, set_trait, specialty_count, specialty_skill_level, step_trait,
    toggle_effect, toggle_technique, trait_floor, trait_value,
};

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
