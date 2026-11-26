//! Cliente tipado do Supabase do jogo (o mesmo banco do site atual, R2).
//!
//! Esta primeira parte só lê: `select` em tabelas e views do PostgREST e as
//! funções `stable` que o site chama (`eh_admin` e `grupo_visivel`). O pedido
//! é montado como dado ([`Request`]) e enviado por um [`Transport`]: o `fetch`
//! do navegador no WebAssembly (`FetchTransport`), o `reqwest` no nativo
//! (`ReqwestTransport`), ou respostas gravadas nos testes.
//!
//! Regras:
//! - a URL e a chave `anon` chegam num [`Config`] montado por quem usa o
//!   crate; nenhuma das duas fica no código;
//! - toda leitura de tabela, view ou função pede um [`Session`]; sem sessão,
//!   só se lê o que é [`PublicRelation`] (`migracoes`). Dado de mesa nunca
//!   depende do acesso `anon`:
//!
//! ```compile_fail
//! # use centelha_supabase::{Client, Select, Table, Transport};
//! # async fn f<T: Transport>(c: &Client<T>) {
//! // a mesa não é relação pública: sem sessão, não compila
//! let _ = c.select_public(&Select::<Table>::new()).await;
//! # }
//! ```
//!
//! ```
//! # use centelha_supabase::{Client, Migration, Select, Session, Table, Transport};
//! # async fn f<T: Transport>(c: &Client<T>, s: &Session) {
//! let _ = c.select_public(&Select::<Migration>::new()).await;
//! let _ = c.select(s, &Select::<Table>::new().columns("id, nome")).await;
//! # }
//! ```
//!
//! - admin é só quem a `eh_admin` aprova, e qualquer falha é não admin
//!   ([`Client::is_admin`], R22);
//! - o crate não lê nem grava a sessão do navegador (R26): quem chama
//!   entrega o token pronto.

mod client;
mod config;
mod error;
mod http;
mod query;
mod rows;
mod transport;

pub use client::Client;
pub use config::{Config, Session};
pub use error::Error;
pub use http::{Method, Request, Response, Transport};
pub use query::{Direction, PublicRelation, Relation, Select};
pub use rows::{
    Character, CharacterStatus, MemberRole, Migration, PartyMember, Profile, Table, TableMember,
};
#[cfg(target_arch = "wasm32")]
pub use transport::FetchTransport;
#[cfg(not(target_arch = "wasm32"))]
pub use transport::ReqwestTransport;
