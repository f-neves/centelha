//! O pedido e a resposta como dados, e o envio atrás de um trait.
//!
//! O crate monta o [`Request`] e interpreta o [`Response`]; quem envia é um
//! [`Transport`]: o `fetch` do navegador no WebAssembly, o `reqwest` no
//! nativo, ou respostas gravadas nos testes.

use std::fmt;
use std::future::Future;

use crate::Error;

/// O método HTTP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Head,
    Post,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Head => "HEAD",
            Method::Post => "POST",
        }
    }
}

/// Um pedido pronto: o endereço inteiro (com a consulta), os cabeçalhos e o
/// corpo. O `Debug` não mostra os cabeçalhos `apikey` e `Authorization`.
#[derive(Clone, PartialEq, Eq)]
pub struct Request {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

impl Request {
    /// O valor de um cabeçalho (o nome sem diferença de caixa).
    pub fn header(&self, name: &str) -> Option<&str> {
        find_header(&self.headers, name)
    }
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let headers: Vec<(&str, &str)> = self
            .headers
            .iter()
            .map(|(k, v)| {
                let secret =
                    k.eq_ignore_ascii_case("apikey") || k.eq_ignore_ascii_case("authorization");
                (k.as_str(), if secret { "<oculta>" } else { v.as_str() })
            })
            .collect();
        f.debug_struct("Request")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("headers", &headers)
            .field("body", &self.body)
            .finish()
    }
}

/// A resposta: a situação, os cabeçalhos (nomes em minúsculas) e o corpo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Response {
    /// O valor de um cabeçalho (o nome sem diferença de caixa).
    pub fn header(&self, name: &str) -> Option<&str> {
        find_header(&self.headers, name)
    }

    /// A situação é de sucesso (2xx)?
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

fn find_header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

/// Quem envia o pedido e devolve a resposta.
pub trait Transport {
    fn send(&self, request: Request) -> impl Future<Output = Result<Response, Error>>;
}

/// A codificação de um valor da consulta, como o `URLSearchParams` do
/// navegador: letras, dígitos e `*-._` ficam, o espaço vira `+`, o resto vira
/// `%XX` dos bytes em UTF-8.
pub(crate) fn form_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'*' | b'-' | b'.' | b'_' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_like_url_search_params() {
        assert_eq!(form_encode("in.(a,b)"), "in.%28a%2Cb%29");
        assert_eq!(form_encode("eq.Ana Lú"), "eq.Ana+L%C3%BA");
        assert_eq!(form_encode("id,nome"), "id%2Cnome");
    }

    #[test]
    fn debug_hides_the_auth_headers() {
        let r = Request {
            method: Method::Get,
            url: "https://exemplo.invalid/rest/v1/mesas".into(),
            headers: vec![
                ("apikey".into(), "chave-de-teste".into()),
                ("Authorization".into(), "Bearer token-de-teste".into()),
                ("Accept".into(), "application/json".into()),
            ],
            body: None,
        };
        let shown = format!("{r:?}");
        assert!(
            !shown.contains("chave-de-teste") && !shown.contains("token-de-teste"),
            "{shown}"
        );
        assert!(shown.contains("application/json"));
        assert_eq!(r.header("authorization"), Some("Bearer token-de-teste"));
    }
}
