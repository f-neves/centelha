//! `with_form` com corpo parcial e com o Tamanho legado da forma sem área: as
//! regras do `comForma()` do TypeScript (a forma muda só o que traz; a área
//! da forma apaga a escolha do Tamanho da principal, a massa apaga a linha de
//! PV, e a massa ou a linha apagam o porquê da linha). Os valores esperados
//! foram conferidos rodando o `comForma()` e o `criatura()` da fonte nos
//! mesmos quatro casos.

mod common;

use centelha_data::bestiary::{HpLine, Size};
use centelha_engine::creature::{Tables, derive, with_form};
use common::repo;
use serde_json::{Value, json};

/// O barghest com Tamanho e linha de PV escolhidos na principal, e a forma
/// `goblin` com o corpo dado.
fn barghest(form_body: Value) -> centelha_data::Creature {
    let text =
        std::fs::read_to_string(repo().join("content/data/bestiario/barghest.json")).unwrap();
    let mut c: Value = serde_json::from_str(&text).unwrap();
    c["corpo"]["tamanhoEscolhido"] = json!("medio");
    c["corpo"]["linhaPV"] = json!("acima");
    c["corpo"]["linhaPVMotivo"] = json!("corpulento");
    c["formas"][0]["corpo"] = form_body;
    centelha_data::parse_creature(&c.to_string()).unwrap_or_else(|e| panic!("{e}"))
}

fn tables() -> Tables {
    let content = repo().join("content");
    Tables::new(
        centelha_data::load_rules(&content).unwrap(),
        centelha_data::load_weapons(&content).unwrap(),
        centelha_data::load_armors(&content).unwrap(),
        centelha_data::load_shields(&content).unwrap(),
    )
}

#[test]
fn partial_body_with_mass_only() {
    let c = barghest(json!({ "massa": 30 }));
    let f = with_form(&c, &c.forms.as_ref().unwrap()[0]);
    assert_eq!(f.id, "goblin");
    assert_ne!(f.name, c.name);
    assert_eq!(f.body.mass, Some(30.0));
    // a área e o Tamanho escolhido ficam os da principal
    assert_eq!(f.body.area, c.body.area);
    assert_eq!(f.body.chosen_size, Some(Size::Medium));
    // a massa da forma apaga a linha de PV e o porquê dela
    assert_eq!(f.body.hp_line, None);
    assert_eq!(f.body.hp_line_reason, None);
    // a forma sem ataques próprios fica com os da principal
    assert_eq!(f.attacks, c.attacks);
    assert_eq!(derive(&tables(), &f).size, Some(Size::Medium));
}

#[test]
fn partial_body_with_area_only() {
    let c = barghest(json!({ "area": 0.25 }));
    let f = with_form(&c, &c.forms.as_ref().unwrap()[0]);
    assert_eq!(f.body.area, Some(0.25));
    assert_eq!(f.body.mass, c.body.mass);
    // a área da forma apaga a escolha do Tamanho; a linha de PV fica
    assert_eq!(f.body.chosen_size, None);
    assert_eq!(f.body.hp_line, Some(HpLine::Above));
    assert_eq!(f.body.hp_line_reason.as_deref(), Some("corpulento"));
    assert_eq!(derive(&tables(), &f).size, Some(Size::Small));
}

#[test]
fn legacy_form_size_without_area() {
    let c = barghest(json!({ "tamanhoEscolhido": "pequeno" }));
    let f = with_form(&c, &c.forms.as_ref().unwrap()[0]);
    // o Tamanho é o da forma, a área continua a da principal
    assert_eq!(f.body.chosen_size, Some(Size::Small));
    assert_eq!(f.body.area, c.body.area);
    assert_eq!(f.body.mass, c.body.mass);
    assert_eq!(f.body.hp_line, Some(HpLine::Above));
    assert_eq!(f.body.hp_line_reason.as_deref(), Some("corpulento"));
    assert_eq!(derive(&tables(), &f).size, Some(Size::Small));
}

#[test]
fn line_without_reason_drops_the_main_reason() {
    let c = barghest(json!({ "linhaPV": "abaixo" }));
    let f = with_form(&c, &c.forms.as_ref().unwrap()[0]);
    assert_eq!(f.body.hp_line, Some(HpLine::Below));
    assert_eq!(f.body.hp_line_reason, None);
    assert_eq!(f.body.chosen_size, Some(Size::Medium));
}
