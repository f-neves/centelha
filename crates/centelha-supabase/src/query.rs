//! A leitura de uma tabela ou view do PostgREST (`Relation`): as colunas,
//! os filtros, a ordem e o limite, e o pedido que sai deles.

use std::marker::PhantomData;

use serde::de::DeserializeOwned;

use crate::http::{Method, Request, form_encode};
use crate::{Config, Session};

/// Uma tabela ou view do PostgREST e o tipo da linha dela.
pub trait Relation: DeserializeOwned {
    /// O nome no banco.
    const NAME: &'static str;
}

/// Tabela que o banco abre à chave `anon`. Só ela se lê sem sessão; dado de
/// mesa nunca (a leitura de mesa pede um [`Session`] no tipo).
pub trait PublicRelation: Relation {}

/// A ordem de uma coluna.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Ascending,
    Descending,
}

/// Como a resposta volta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shape {
    /// A lista de linhas.
    List,
    /// Exatamente uma linha (`single`): o PostgREST dá erro se não for uma.
    Single,
    /// Nenhuma ou uma (`maybe_single`).
    MaybeSingle,
    /// Só a contagem, sem as linhas (`count`, pelo `HEAD`).
    Count,
}

/// A leitura de uma relação, como o `from(...).select(...)` do site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Select<R: Relation> {
    columns: Option<String>,
    filters: Vec<(String, String)>,
    order: Vec<String>,
    limit: Option<u64>,
    relation: PhantomData<fn() -> R>,
}

impl<R: Relation> Default for Select<R> {
    fn default() -> Self {
        Select::new()
    }
}

impl<R: Relation> Select<R> {
    /// Todas as colunas (`*`).
    pub fn new() -> Self {
        Select {
            columns: None,
            filters: Vec::new(),
            order: Vec::new(),
            limit: None,
            relation: PhantomData,
        }
    }

    /// As colunas, como o PostgREST as escreve (`id, nome`, ou com a relação
    /// embutida, `papel, mesa:mesas(id, nome)`). Os espaços fora de aspas
    /// saem, como no `supabase-js`.
    pub fn columns(mut self, columns: &str) -> Self {
        let mut out = String::new();
        let mut quoted = false;
        for c in columns.chars() {
            if c == '"' {
                quoted = !quoted;
            }
            if c.is_whitespace() && !quoted {
                continue;
            }
            out.push(c);
        }
        self.columns = Some(out);
        self
    }

    /// `coluna = valor`.
    pub fn eq(mut self, column: &str, value: impl ToString) -> Self {
        self.filters
            .push((column.to_owned(), format!("eq.{}", value.to_string())));
        self
    }

    /// `coluna` em uma lista. O valor com vírgula ou parêntese vai entre
    /// aspas, como no `supabase-js`.
    pub fn is_in<V: ToString>(mut self, column: &str, values: impl IntoIterator<Item = V>) -> Self {
        let list: Vec<String> = values
            .into_iter()
            .map(|v| {
                let s = v.to_string();
                if s.contains([',', '(', ')']) {
                    format!("\"{s}\"")
                } else {
                    s
                }
            })
            .collect();
        self.filters
            .push((column.to_owned(), format!("in.({})", list.join(","))));
        self
    }

    /// Mais uma coluna na ordem.
    pub fn order(mut self, column: &str, direction: Direction) -> Self {
        let d = match direction {
            Direction::Ascending => "asc",
            Direction::Descending => "desc",
        };
        self.order.push(format!("{column}.{d}"));
        self
    }

    /// No máximo `n` linhas.
    pub fn limit(mut self, n: u64) -> Self {
        self.limit = Some(n);
        self
    }

    /// A consulta (`select=...&coluna=eq...&order=...&limit=...`).
    fn query(&self) -> String {
        let mut parts = vec![format!(
            "select={}",
            form_encode(self.columns.as_deref().unwrap_or("*"))
        )];
        for (k, v) in &self.filters {
            parts.push(format!("{}={}", form_encode(k), form_encode(v)));
        }
        if !self.order.is_empty() {
            parts.push(format!("order={}", form_encode(&self.order.join(","))));
        }
        if let Some(n) = self.limit {
            parts.push(format!("limit={n}"));
        }
        parts.join("&")
    }

    /// O pedido com a sessão do usuário.
    pub(crate) fn request(&self, config: &Config, session: &Session, shape: Shape) -> Request {
        build(
            config,
            &self.query(),
            R::NAME,
            session.access_token(),
            shape,
        )
    }
}

impl<R: PublicRelation> Select<R> {
    /// O pedido sem sessão (só com a chave `anon`), só para relação pública.
    pub(crate) fn request_public(&self, config: &Config, shape: Shape) -> Request {
        build(config, &self.query(), R::NAME, config.anon_key(), shape)
    }
}

fn build(config: &Config, query: &str, name: &str, bearer: &str, shape: Shape) -> Request {
    let mut headers = auth_headers(config, bearer);
    let accept = match shape {
        Shape::Single => "application/vnd.pgrst.object+json",
        _ => "application/json",
    };
    headers.push(("Accept".into(), accept.into()));
    let method = if shape == Shape::Count {
        headers.push(("Prefer".into(), "count=exact".into()));
        Method::Head
    } else {
        Method::Get
    };
    Request {
        method,
        url: format!("{}?{query}", config.rest(name)),
        headers,
        body: None,
    }
}

/// Os cabeçalhos de toda chamada: a chave `anon` e o portador (o token da
/// sessão, ou a própria chave `anon` sem sessão).
pub(crate) fn auth_headers(config: &Config, bearer: &str) -> Vec<(String, String)> {
    vec![
        ("apikey".into(), config.anon_key().into()),
        ("Authorization".into(), format!("Bearer {bearer}")),
    ]
}
