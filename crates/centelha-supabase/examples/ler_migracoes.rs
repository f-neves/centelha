//! A conferência de ponta a ponta, fora do CI: lê `migracoes` do banco com a
//! chave `anon`, sem sessão, pelo transporte nativo.
//!
//! O endereço e a chave vêm das variáveis `CENTELHA_SUPABASE_URL` e
//! `CENTELHA_SUPABASE_ANON_KEY` e não são mostrados. Com um caminho como
//! argumento, grava a resposta como fixture (só a situação, o
//! `content-range` e o corpo).

use centelha_supabase::{
    Client, Config, Direction, Error, Migration, Request, ReqwestTransport, Response, Select,
    Transport,
};

/// O transporte nativo, guardando a última resposta.
struct Recording {
    inner: ReqwestTransport,
    last: std::cell::RefCell<Option<Response>>,
}

impl Transport for Recording {
    async fn send(&self, request: Request) -> Result<Response, Error> {
        let r = self.inner.send(request).await?;
        *self.last.borrow_mut() = Some(r.clone());
        Ok(r)
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let var = |k: &str| std::env::var(k).unwrap_or_else(|_| panic!("falta a variável {k}"));
    let config = Config::new(
        var("CENTELHA_SUPABASE_URL"),
        var("CENTELHA_SUPABASE_ANON_KEY"),
    );
    let client = Client::new(
        config,
        Recording {
            inner: ReqwestTransport::new(),
            last: Default::default(),
        },
    );
    let q = Select::<Migration>::new().order("numero", Direction::Ascending);
    match client.select_public(&q).await {
        Ok(list) => {
            let last = list.last().map_or(0, |m| m.number);
            println!("{} migrações lidas, a última é a {last}", list.len());
        }
        Err(e) => {
            println!("erro: {e}");
            std::process::exit(1);
        }
    }
    if let Some(path) = std::env::args().nth(1) {
        let r = client.transport().last.borrow().clone().unwrap();
        let body: serde_json::Value = serde_json::from_str(&r.body).unwrap();
        let mut headers = serde_json::Map::new();
        if let Some(cr) = r.header("content-range") {
            headers.insert("content-range".into(), cr.into());
        }
        let out = serde_json::json!({ "status": r.status, "headers": headers, "body": body });
        std::fs::write(&path, serde_json::to_string_pretty(&out).unwrap() + "\n").unwrap();
        println!("gravado em {path}");
    }
}
