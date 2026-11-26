//! O cliente contra respostas gravadas (`tests/fixtures/`), sem rede.
//!
//! As fixtures de tabela com sessão são feitas à mão a partir das colunas das
//! migrações, com uuids e nomes inventados. A de `migracoes` é a resposta de
//! verdade do banco à chave `anon` (só o corpo, sem endereço nem chave). A
//! configuração daqui é inventada (`exemplo.invalid`).

use std::cell::RefCell;
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use centelha_supabase::{
    Character, CharacterStatus, Client, Config, Direction, Error, MemberRole, Method, Migration,
    Profile, Request, Response, Select, Session, Table, TableMember, Transport,
};
use serde_json::Value;

/// Roda um futuro que não espera nada (o transporte de fixtures responde na
/// hora).
fn block_on<F: Future>(f: F) -> F::Output {
    let mut f = pin!(f);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

/// Devolve a resposta gravada e guarda o pedido que recebeu.
struct Fixture {
    response: Result<Response, Error>,
    seen: RefCell<Vec<Request>>,
}

impl Transport for Fixture {
    async fn send(&self, request: Request) -> Result<Response, Error> {
        self.seen.borrow_mut().push(request);
        self.response.clone()
    }
}

fn fixture(name: &str) -> Response {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let d: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let body = match d.get("raw") {
        Some(Value::String(s)) => s.clone(),
        _ => d["body"].to_string(),
    };
    let headers = d["headers"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_owned()))
        .collect();
    Response {
        status: d["status"].as_u64().unwrap() as u16,
        headers,
        body,
    }
}

fn client(response: Result<Response, Error>) -> Client<Fixture> {
    Client::new(
        Config::new("https://exemplo.invalid/", "chave-de-teste"),
        Fixture {
            response,
            seen: RefCell::new(Vec::new()),
        },
    )
}

fn session() -> Session {
    Session::new("token-de-teste")
}

/// O único pedido que o cliente mandou.
fn sent(c: &Client<Fixture>) -> Request {
    let seen = c.transport().seen.borrow();
    assert_eq!(seen.len(), 1, "esperava um pedido só");
    seen[0].clone()
}

#[test]
fn select_builds_the_postgrest_request() {
    let c = client(Ok(fixture("mesa_membros.json")));
    let q = Select::<TableMember>::new()
        .columns("papel, mesa:mesas(id, nome)")
        .eq("user_id", "00000000-0000-4000-8000-000000000001")
        .is_in("papel", ["mestre", "jogador"])
        .order("entrou_em", Direction::Descending)
        .order("mesa_id", Direction::Ascending)
        .limit(5);
    let got = block_on(c.select(&session(), &q)).unwrap();
    let r = sent(&c);
    assert_eq!(r.method, Method::Get);
    assert_eq!(
        r.url,
        "https://exemplo.invalid/rest/v1/mesa_membros?select=papel%2Cmesa%3Amesas%28id%2Cnome%29\
         &user_id=eq.00000000-0000-4000-8000-000000000001&papel=in.%28mestre%2Cjogador%29\
         &order=entrou_em.desc%2Cmesa_id.asc&limit=5"
    );
    assert_eq!(r.header("apikey"), Some("chave-de-teste"));
    assert_eq!(r.header("Authorization"), Some("Bearer token-de-teste"));
    assert_eq!(r.header("Accept"), Some("application/json"));
    assert_eq!(r.body, None);
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].role, Some(MemberRole::Player));
    assert_eq!(got[0].table.as_ref().unwrap().name, "Mesa de teste");
    assert_eq!(got[1].role, Some(MemberRole::GameMaster));
}

#[test]
fn in_quotes_values_with_commas() {
    let c = client(Ok(fixture("vazio.json")));
    let q = Select::<Profile>::new().is_in("nome", ["a,b", "c"]);
    block_on(c.select(&session(), &q)).unwrap();
    assert!(sent(&c).url.ends_with("&nome=in.%28%22a%2Cb%22%2Cc%29"));
}

#[test]
fn reads_the_rows() {
    let c = client(Ok(fixture("profiles.json")));
    let p = block_on(c.select(&session(), &Select::<Profile>::new().columns("id, nome"))).unwrap();
    assert_eq!(p[1].name, "Mestre Exemplo");
    assert_eq!(p[0].config, None);

    let c = client(Ok(fixture("mesas.json")));
    let t = block_on(c.select(&session(), &Select::<Table>::new())).unwrap();
    assert!(sent(&c).url.ends_with("/rest/v1/mesas?select=*"));
    assert_eq!(t[0].starting_xp, 30);
    assert_eq!(t[0].reveal["fichaColegas"], "fisico");
    assert!(!t[0].record_strikes);

    let c = client(Ok(fixture("personagens.json")));
    let ch: Vec<Character> = block_on(c.select(&session(), &Select::new())).unwrap();
    assert_eq!(ch[0].status, Some(CharacterStatus::Submitted));
    assert_eq!(ch[0].sheet["attrs"]["forca"], 2);
    assert_eq!(ch[0].image_path, None);
    assert_eq!(ch[0].table_id, None);
}

#[test]
fn single_asks_for_an_object() {
    let c = client(Ok(fixture("personagem-unico.json")));
    let q = Select::<Character>::new().eq("id", "00000000-0000-4000-8000-000000000020");
    let ch = block_on(c.single(&session(), &q)).unwrap();
    assert_eq!(
        sent(&c).header("Accept"),
        Some("application/vnd.pgrst.object+json")
    );
    assert_eq!(
        ch.table_id.as_deref(),
        Some("00000000-0000-4000-8000-000000000010")
    );

    let c = client(Ok(fixture("nenhuma-linha.json")));
    let e = block_on(c.single(&session(), &q)).unwrap_err();
    assert_eq!(
        e,
        Error::Http {
            status: 406,
            code: Some("PGRST116".into()),
            message: "JSON object requested, multiple (or no) rows returned".into()
        }
    );
}

#[test]
fn maybe_single_takes_none_or_one() {
    let c = client(Ok(fixture("vazio.json")));
    let q = Select::<Profile>::new();
    assert_eq!(block_on(c.maybe_single(&session(), &q)), Ok(None));
    assert_eq!(sent(&c).header("Accept"), Some("application/json"));

    let c = client(Ok(fixture("profiles.json")));
    let e = block_on(c.maybe_single(&session(), &q)).unwrap_err();
    assert!(matches!(e, Error::Http { status: 406, code: Some(ref k), .. } if k == "PGRST116"));
}

#[test]
fn count_reads_the_content_range() {
    let c = client(Ok(fixture("contagem.json")));
    let q = Select::<Character>::new().eq("mesa_id", "00000000-0000-4000-8000-000000000010");
    assert_eq!(block_on(c.count(&session(), &q)), Ok(7));
    let r = sent(&c);
    assert_eq!(r.method, Method::Head);
    assert_eq!(r.header("Prefer"), Some("count=exact"));
}

#[test]
fn errors_keep_the_postgrest_code() {
    let c = client(Ok(fixture("sem-permissao.json")));
    let e = block_on(c.select(&session(), &Select::<Table>::new())).unwrap_err();
    assert_eq!(
        e,
        Error::Http {
            status: 401,
            code: Some("PGRST301".into()),
            message: "JWT expired".into()
        }
    );
    let c = client(Ok(fixture("erro-do-banco.json")));
    let e = block_on(c.select(&session(), &Select::<Table>::new())).unwrap_err();
    assert_eq!(
        e,
        Error::Http {
            status: 500,
            code: None,
            message: "Internal Server Error".into()
        }
    );
    // corpo fora do formato (um booleano no lugar da lista)
    let c = client(Ok(fixture("eh_admin-sim.json")));
    let e = block_on(c.select(&session(), &Select::<Migration>::new())).unwrap_err();
    assert!(matches!(e, Error::Decode(_)), "{e:?}");
}

#[test]
fn the_public_read_goes_with_the_anon_key() {
    let c = client(Ok(fixture("migracoes.json")));
    let q = Select::<Migration>::new().order("numero", Direction::Ascending);
    let m = block_on(c.select_public(&q)).unwrap();
    let r = sent(&c);
    assert_eq!(r.header("Authorization"), Some("Bearer chave-de-teste"));
    assert!(
        r.url
            .ends_with("/rest/v1/migracoes?select=*&order=numero.asc")
    );
    // a resposta de verdade: as migrações aplicadas, em ordem
    assert!(m.len() >= 40, "{}", m.len());
    assert!(m.windows(2).all(|w| w[0].number < w[1].number));
    assert_eq!(m[0].number, 1);
}

#[test]
fn is_admin_is_true_only_for_a_true_answer() {
    let c = client(Ok(fixture("eh_admin-sim.json")));
    assert!(block_on(c.is_admin(Some(&session()))));
    let r = sent(&c);
    assert_eq!(r.method, Method::Post);
    assert_eq!(r.url, "https://exemplo.invalid/rest/v1/rpc/eh_admin");
    assert_eq!(r.body.as_deref(), Some("{}"));
    assert_eq!(r.header("Content-Type"), Some("application/json"));
    assert_eq!(r.header("Authorization"), Some("Bearer token-de-teste"));

    for f in [
        "eh_admin-nao.json",
        "eh_admin-nulo.json",
        "eh_admin-texto.json",
        "sem-permissao.json",
        "erro-do-banco.json",
        "vazio.json",
    ] {
        assert!(
            !block_on(client(Ok(fixture(f))).is_admin(Some(&session()))),
            "{f}"
        );
    }
    let down = client(Err(Error::Transport("sem rede".into())));
    assert!(!block_on(down.is_admin(Some(&session()))));
    // sem sessão, nem pergunta
    let c = client(Ok(fixture("eh_admin-sim.json")));
    assert!(!block_on(c.is_admin(None)));
    assert!(c.transport().seen.borrow().is_empty());
}

#[test]
fn visible_party_calls_grupo_visivel() {
    let c = client(Ok(fixture("grupo_visivel.json")));
    let g = block_on(c.visible_party(&session(), "00000000-0000-4000-8000-000000000010")).unwrap();
    let r = sent(&c);
    assert_eq!(r.url, "https://exemplo.invalid/rest/v1/rpc/grupo_visivel");
    assert_eq!(
        r.body.as_deref(),
        Some(r#"{"p_mesa":"00000000-0000-4000-8000-000000000010"}"#)
    );
    assert_eq!(g[0].level, "fisico");
    assert_eq!(g[0].data["forca"], "3");
    assert_eq!(g[0].extra["pv"], 7);
}
