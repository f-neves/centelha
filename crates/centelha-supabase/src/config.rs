//! A configuração do projeto e a sessão do usuário.
//!
//! Nenhuma das duas mostra o segredo no `Debug`: a chave `anon` e o token de
//! acesso saem como `<oculta>`, para não irem parar num log.

use std::fmt;

/// O endereço do projeto e a chave `anon`, que chegam de fora (no site, das
/// variáveis de build). Nunca no código.
#[derive(Clone, PartialEq, Eq)]
pub struct Config {
    url: String,
    anon_key: String,
}

impl Config {
    /// O endereço do projeto (sem a barra do fim) e a chave `anon`.
    pub fn new(url: impl Into<String>, anon_key: impl Into<String>) -> Config {
        let mut url = url.into();
        while url.ends_with('/') {
            url.pop();
        }
        Config {
            url,
            anon_key: anon_key.into(),
        }
    }

    /// O endereço do projeto.
    pub fn url(&self) -> &str {
        &self.url
    }

    pub(crate) fn anon_key(&self) -> &str {
        &self.anon_key
    }

    /// O endereço do PostgREST para uma tabela, view ou `rpc/<função>`.
    pub(crate) fn rest(&self, path: &str) -> String {
        format!("{}/rest/v1/{path}", self.url)
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("url", &self.url)
            .field("anon_key", &"<oculta>")
            .finish()
    }
}

/// A sessão de um usuário: o token de acesso que o GoTrue deu. O crate não
/// lê nem grava a sessão do navegador (R26); quem chama entrega o token.
#[derive(Clone, PartialEq, Eq)]
pub struct Session {
    access_token: String,
}

impl Session {
    pub fn new(access_token: impl Into<String>) -> Session {
        Session {
            access_token: access_token.into(),
        }
    }

    pub(crate) fn access_token(&self) -> &str {
        &self.access_token
    }
}

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("access_token", &"<oculta>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_hides_the_secrets() {
        let c = Config::new("https://exemplo.invalid/", "chave-de-teste");
        assert_eq!(c.url(), "https://exemplo.invalid");
        let shown = format!("{c:?} {:?}", Session::new("token-de-teste"));
        assert!(!shown.contains("chave-de-teste"), "{shown}");
        assert!(!shown.contains("token-de-teste"), "{shown}");
        assert!(shown.contains("<oculta>"));
    }
}
