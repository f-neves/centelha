//! As linhas das tabelas que o cliente lê, com os nomes das colunas em
//! português traduzidos na leitura.
//!
//! O site pede quase sempre só algumas colunas (`select('id, nome')`), então
//! todo campo fora a chave tem valor padrão quando não vem. As datas ficam
//! como o texto ISO que o PostgREST devolve.

use serde::Deserialize;
use serde_json::Value;

use crate::query::{PublicRelation, Relation};

/// `profiles`: o perfil de cada conta.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Profile {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "nome", default)]
    pub name: String,
    #[serde(rename = "criado_em", default)]
    pub created_at: String,
    /// As preferências do site guardadas na conta.
    #[serde(default)]
    pub config: Option<Value>,
}

/// `mesas`: a mesa (a sessão de jogo).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Table {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "nome", default)]
    pub name: String,
    #[serde(rename = "descricao", default)]
    pub description: String,
    #[serde(rename = "mestre_id", default)]
    pub game_master_id: String,
    #[serde(rename = "codigo_convite", default)]
    pub invite_code: String,
    #[serde(rename = "criado_em", default)]
    pub created_at: String,
    #[serde(rename = "xp_inicial", default)]
    pub starting_xp: i64,
    /// O que o mestre revela aos jogadores (`fichaColegas`, `vidaColegas`...).
    #[serde(rename = "revelar", default)]
    pub reveal: Value,
    #[serde(rename = "combate", default)]
    pub combat: Value,
    #[serde(rename = "gravar_lances", default)]
    pub record_strikes: bool,
}

/// O papel de quem está na mesa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum MemberRole {
    #[serde(rename = "mestre")]
    GameMaster,
    #[serde(rename = "jogador")]
    Player,
}

/// `mesa_membros`: quem está em qual mesa.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TableMember {
    #[serde(rename = "mesa_id", default)]
    pub table_id: String,
    #[serde(default)]
    pub user_id: String,
    #[serde(rename = "papel", default)]
    pub role: Option<MemberRole>,
    #[serde(rename = "entrou_em", default)]
    pub joined_at: String,
    /// A mesa embutida (`mesa:mesas(id, nome)` ou `mesas(nome)`).
    #[serde(rename = "mesa", alias = "mesas", default)]
    pub table: Option<Table>,
}

/// Em que pé está a ficha na mesa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum CharacterStatus {
    #[serde(rename = "rascunho")]
    Draft,
    #[serde(rename = "enviado")]
    Submitted,
    #[serde(rename = "aprovado")]
    Approved,
}

/// `personagens`: o personagem, com a ficha como JSON (ADR 0004).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Character {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "dono_id", default)]
    pub owner_id: String,
    #[serde(rename = "mesa_id", default)]
    pub table_id: Option<String>,
    #[serde(rename = "nome", default)]
    pub name: String,
    #[serde(rename = "conceito", default)]
    pub concept: String,
    /// A ficha como o site atual a grava.
    #[serde(rename = "ficha", default)]
    pub sheet: Value,
    #[serde(rename = "descricao", default)]
    pub description: String,
    #[serde(rename = "historia", default)]
    pub history: String,
    #[serde(default)]
    pub background: String,
    #[serde(rename = "anotacoes", default)]
    pub notes: String,
    #[serde(rename = "imagem_path", default)]
    pub image_path: Option<String>,
    #[serde(rename = "imagem_pos", default)]
    pub image_position: Option<Value>,
    #[serde(rename = "atualizado_em", default)]
    pub updated_at: String,
    #[serde(default)]
    pub status: Option<CharacterStatus>,
    #[serde(rename = "revisao_nota", default)]
    pub review_note: String,
    #[serde(rename = "aprovado_em", default)]
    pub approved_at: Option<String>,
    #[serde(rename = "aprovado_por", default)]
    pub approved_by: Option<String>,
    /// A vaga (o personagem que o mestre abriu para alguém reivindicar).
    #[serde(rename = "vaga", default)]
    pub slot: Option<String>,
    #[serde(rename = "codigo_vaga", default)]
    pub slot_code: Option<String>,
    /// O resumo que a mesa mostra (Vida, Energia...).
    #[serde(rename = "resumo", default)]
    pub summary: Value,
    /// A mesa embutida (`mesa:mesas(nome)`).
    #[serde(rename = "mesa", alias = "mesas", default)]
    pub table: Option<Table>,
}

/// `migracoes`: as migrações aplicadas no banco. A única tabela que o banco
/// abre à chave `anon`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Migration {
    #[serde(rename = "numero", default)]
    pub number: i64,
    #[serde(rename = "arquivo", default)]
    pub file: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    /// Aplicada à mão, fora do roteiro.
    #[serde(rename = "a_mao", default)]
    pub by_hand: bool,
    #[serde(rename = "aplicada_em", default)]
    pub applied_at: String,
}

/// Uma linha da `grupo_visivel`: um colega de mesa como o mestre deixa ver.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PartyMember {
    pub id: String,
    #[serde(rename = "nome", default)]
    pub name: String,
    #[serde(rename = "conceito", default)]
    pub concept: String,
    #[serde(rename = "dono_id", default)]
    pub owner_id: String,
    #[serde(rename = "imagem_path", default)]
    pub image_path: Option<String>,
    /// O quanto da ficha se vê: `tudo`, `fisico` ou outro (nada).
    #[serde(rename = "nivel", default)]
    pub level: String,
    /// A ficha inteira (`tudo`), o físico (`fisico`) ou `{}`.
    #[serde(rename = "dados", default)]
    pub data: Value,
    /// O resumo liberado (Vida, Energia, situação).
    #[serde(default)]
    pub extra: Value,
}

impl Relation for Profile {
    const NAME: &'static str = "profiles";
}
impl Relation for Table {
    const NAME: &'static str = "mesas";
}
impl Relation for TableMember {
    const NAME: &'static str = "mesa_membros";
}
impl Relation for Character {
    const NAME: &'static str = "personagens";
}
impl Relation for Migration {
    const NAME: &'static str = "migracoes";
}
impl PublicRelation for Migration {}
