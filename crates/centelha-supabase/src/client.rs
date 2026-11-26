//! O cliente: monta o pedido, manda pelo transporte e interpreta a resposta.

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::http::{Method, Request, Response, Transport};
use crate::query::{PublicRelation, Relation, Select, Shape, auth_headers};
use crate::rows::PartyMember;
use crate::{Config, Error, Session};

/// O cliente de leitura do Supabase.
#[derive(Debug, Clone)]
pub struct Client<T: Transport> {
    config: Config,
    transport: T,
}

impl<T: Transport> Client<T> {
    pub fn new(config: Config, transport: T) -> Self {
        Client { config, transport }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    async fn send(&self, request: Request) -> Result<Response, Error> {
        self.transport.send(request).await
    }

    /// As linhas da leitura, com a sessão do usuário.
    pub async fn select<R: Relation>(
        &self,
        session: &Session,
        q: &Select<R>,
    ) -> Result<Vec<R>, Error> {
        let r = self
            .send(q.request(&self.config, session, Shape::List))
            .await?;
        rows(&r)
    }

    /// Exatamente uma linha: o erro do PostgREST (`PGRST116`) se não for uma.
    pub async fn single<R: Relation>(&self, session: &Session, q: &Select<R>) -> Result<R, Error> {
        let r = self
            .send(q.request(&self.config, session, Shape::Single))
            .await?;
        decode(&ok(r)?.body)
    }

    /// Nenhuma ou uma linha; mais de uma é erro, como no `maybeSingle`.
    pub async fn maybe_single<R: Relation>(
        &self,
        session: &Session,
        q: &Select<R>,
    ) -> Result<Option<R>, Error> {
        let r = self
            .send(q.request(&self.config, session, Shape::MaybeSingle))
            .await?;
        let mut list: Vec<R> = rows(&r)?;
        match list.len() {
            0 => Ok(None),
            1 => Ok(list.pop()),
            n => Err(Error::Http {
                status: 406,
                code: Some("PGRST116".into()),
                message: format!("JSON object requested, multiple (or no) rows returned ({n})"),
            }),
        }
    }

    /// Quantas linhas a leitura daria (o `Content-Range` do `HEAD`).
    pub async fn count<R: Relation>(&self, session: &Session, q: &Select<R>) -> Result<u64, Error> {
        let r = self
            .send(q.request(&self.config, session, Shape::Count))
            .await?;
        let r = ok(r)?;
        content_range_total(r.header("content-range"))
    }

    /// As linhas de uma relação pública, sem sessão. Só [`PublicRelation`]:
    /// dado de mesa não se lê por aqui.
    pub async fn select_public<R: PublicRelation>(&self, q: &Select<R>) -> Result<Vec<R>, Error> {
        let r = self
            .send(q.request_public(&self.config, Shape::List))
            .await?;
        rows(&r)
    }

    /// `eh_admin`: a sessão é de admin? Toda falha (sem sessão, rede, erro do
    /// banco, sessão vencida, corpo que não é `true`) dá `false`, e nada fica
    /// guardado (R22).
    pub async fn is_admin(&self, session: Option<&Session>) -> bool {
        let Some(session) = session else {
            return false;
        };
        match self
            .send(rpc_request(&self.config, session, "eh_admin", &json!({})))
            .await
        {
            Ok(r) if r.ok() => {
                serde_json::from_str::<Value>(&r.body).is_ok_and(|v| v == Value::Bool(true))
            }
            _ => false,
        }
    }

    /// `grupo_visivel`: os colegas da mesa, como o mestre deixa ver.
    pub async fn visible_party(
        &self,
        session: &Session,
        table_id: &str,
    ) -> Result<Vec<PartyMember>, Error> {
        let body = json!({ "p_mesa": table_id });
        let r = self
            .send(rpc_request(&self.config, session, "grupo_visivel", &body))
            .await?;
        rows(&r)
    }
}

/// O pedido de uma função `stable` (`POST /rest/v1/rpc/<nome>`).
pub(crate) fn rpc_request(config: &Config, session: &Session, name: &str, args: &Value) -> Request {
    let mut headers = auth_headers(config, session.access_token());
    headers.push(("Accept".into(), "application/json".into()));
    headers.push(("Content-Type".into(), "application/json".into()));
    Request {
        method: Method::Post,
        url: config.rest(&format!("rpc/{name}")),
        headers,
        body: Some(args.to_string()),
    }
}

fn ok(r: Response) -> Result<Response, Error> {
    if r.ok() {
        Ok(r)
    } else {
        Err(Error::from_response(r.status, &r.body))
    }
}

fn decode<V: DeserializeOwned>(body: &str) -> Result<V, Error> {
    serde_json::from_str(body).map_err(|e| Error::Decode(e.to_string()))
}

fn rows<V: DeserializeOwned>(r: &Response) -> Result<Vec<V>, Error> {
    if !r.ok() {
        return Err(Error::from_response(r.status, &r.body));
    }
    decode(&r.body)
}

/// O total do `Content-Range` (`0-24/25`, `*/0`).
fn content_range_total(h: Option<&str>) -> Result<u64, Error> {
    let h = h.ok_or_else(|| Error::Decode("sem Content-Range".into()))?;
    h.rsplit_once('/')
        .and_then(|(_, n)| n.parse().ok())
        .ok_or_else(|| Error::Decode(format!("Content-Range sem total: {h}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_total() {
        assert_eq!(content_range_total(Some("0-24/25")), Ok(25));
        assert_eq!(content_range_total(Some("*/0")), Ok(0));
        assert!(content_range_total(Some("0-24/*")).is_err());
        assert!(content_range_total(None).is_err());
    }
}
