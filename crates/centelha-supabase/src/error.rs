//! Os erros do cliente.

use std::fmt;

use serde::Deserialize;

/// O que pode dar errado numa leitura.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// O pedido não chegou ou a resposta não voltou (rede, navegador).
    Transport(String),
    /// O servidor respondeu com erro. `code` e `message` vêm do corpo do
    /// PostgREST quando ele é o JSON de erro (`PGRST116`, `42501`...); senão
    /// `code` é `None` e `message` é o corpo.
    Http {
        status: u16,
        code: Option<String>,
        message: String,
    },
    /// A resposta veio, mas não no formato esperado.
    Decode(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Transport(m) => write!(f, "falha no envio: {m}"),
            Error::Http {
                status,
                code: Some(c),
                message,
            } => write!(f, "HTTP {status} ({c}): {message}"),
            Error::Http {
                status,
                code: None,
                message,
            } => write!(f, "HTTP {status}: {message}"),
            Error::Decode(m) => write!(f, "resposta fora do formato: {m}"),
        }
    }
}

impl std::error::Error for Error {}

/// O corpo de erro do PostgREST.
#[derive(Deserialize)]
struct PostgrestError {
    code: Option<String>,
    message: Option<String>,
}

impl Error {
    /// O erro de uma resposta com situação de falha.
    pub(crate) fn from_response(status: u16, body: &str) -> Error {
        match serde_json::from_str::<PostgrestError>(body) {
            Ok(e) => Error::Http {
                status,
                code: e.code,
                message: e.message.unwrap_or_default(),
            },
            Err(_) => Error::Http {
                status,
                code: None,
                message: body.to_owned(),
            },
        }
    }
}
