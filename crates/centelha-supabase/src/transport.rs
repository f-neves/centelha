//! Os transportes de verdade: o `fetch` do navegador (pelo `gloo-net`) no
//! WebAssembly, e o `reqwest` no nativo.

use crate::Error;
use crate::http::{Method, Request, Response, Transport};

/// O `fetch` do navegador.
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Copy, Default)]
pub struct FetchTransport;

#[cfg(target_arch = "wasm32")]
impl Transport for FetchTransport {
    async fn send(&self, request: Request) -> Result<Response, Error> {
        use gloo_net::http::{Method as M, RequestBuilder};
        let method = match request.method {
            Method::Get => M::GET,
            Method::Head => M::HEAD,
            Method::Post => M::POST,
        };
        let mut b = RequestBuilder::new(&request.url).method(method);
        for (k, v) in &request.headers {
            b = b.header(k, v);
        }
        let req = match request.body {
            Some(body) => b.body(body),
            None => b.build(),
        }
        .map_err(|e| Error::Transport(e.to_string()))?;
        let r = req
            .send()
            .await
            .map_err(|e| Error::Transport(e.to_string()))?;
        let headers = r
            .headers()
            .entries()
            .map(|(k, v)| (k.to_ascii_lowercase(), v))
            .collect();
        let status = r.status();
        let body = r
            .text()
            .await
            .map_err(|e| Error::Transport(e.to_string()))?;
        Ok(Response {
            status,
            headers,
            body,
        })
    }
}

/// O `reqwest` (nativo, com `rustls`).
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Default)]
pub struct ReqwestTransport {
    client: reqwest::Client,
}

#[cfg(not(target_arch = "wasm32"))]
impl ReqwestTransport {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Transport for ReqwestTransport {
    async fn send(&self, request: Request) -> Result<Response, Error> {
        let method = match request.method {
            Method::Get => reqwest::Method::GET,
            Method::Head => reqwest::Method::HEAD,
            Method::Post => reqwest::Method::POST,
        };
        let mut b = self.client.request(method, &request.url);
        for (k, v) in &request.headers {
            b = b.header(k, v);
        }
        if let Some(body) = request.body {
            b = b.body(body);
        }
        let r = b
            .send()
            .await
            .map_err(|e| Error::Transport(e.without_url().to_string()))?;
        let status = r.status().as_u16();
        let headers = r
            .headers()
            .iter()
            .filter_map(|(k, v)| Some((k.as_str().to_owned(), v.to_str().ok()?.to_owned())))
            .collect();
        let body = r
            .text()
            .await
            .map_err(|e| Error::Transport(e.without_url().to_string()))?;
        Ok(Response {
            status,
            headers,
            body,
        })
    }
}
