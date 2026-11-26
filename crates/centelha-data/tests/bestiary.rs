//! As 324 criaturas de content/ se leem sem erro, e os ids batem com a lista
//! gravada pelo gerador de referências (tests/parity/bestiary-ids.json).

use std::path::{Path, PathBuf};

use centelha_data::bestiary::{Category, Size};
use centelha_data::{DataError, load_bestiary, parse_creature};
use serde::Deserialize;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[derive(Deserialize)]
struct IdList {
    count: usize,
    ids: Vec<String>,
}

#[test]
fn reads_the_whole_bestiary() {
    let creatures = load_bestiary(&repo().join("content")).unwrap_or_else(|e| panic!("{e}"));
    let text = std::fs::read_to_string(repo().join("tests/parity/bestiary-ids.json")).unwrap();
    let expected: IdList = serde_json::from_str(&text).unwrap();
    let ids: Vec<&str> = creatures.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(creatures.len(), expected.count);
    assert_eq!(ids, expected.ids);
}

#[test]
fn rejects_what_the_schema_rejects() {
    let text = std::fs::read_to_string(repo().join("content/data/bestiario/aasimar.json")).unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(parse_creature(&text).is_ok());

    // (ponteiro, valor, caminho do erro, se é erro de forma do JSON)
    let cases: [(&str, serde_json::Value, &str, bool); 6] = [
        ("/ameaca", 7.into(), "ameaca", false),
        ("/centelha", (-1).into(), "centelha", false),
        (
            "/corpo/tamanhoEscolhido",
            "gigante".into(),
            "corpo.tamanhoEscolhido",
            true,
        ),
        ("/perfilCouraca", "pedra".into(), "perfilCouraca", true),
        (
            "/equipamento/arsenal",
            serde_json::json!([{"ref": "x:espada"}]),
            "equipamento.arsenal[0].ref",
            false,
        ),
        ("/ataques/0/arma", "Espada".into(), "ataques[0].arma", false),
    ];
    for (pointer, value, path, shape) in cases {
        let mut bad = json.clone();
        match bad.pointer_mut(pointer) {
            Some(slot) => *slot = value,
            None => {
                let (parent, key) = pointer.rsplit_once('/').unwrap();
                let obj = if parent.is_empty() {
                    &mut bad
                } else {
                    bad.pointer_mut(parent).unwrap()
                };
                obj.as_object_mut().unwrap().insert(key.to_string(), value);
            }
        }
        let err = parse_creature(&bad.to_string()).unwrap_err();
        if shape {
            assert!(matches!(err, DataError::Json { .. }), "{pointer}: {err}");
        } else {
            assert!(matches!(err, DataError::Invalid { .. }), "{pointer}: {err}");
        }
        assert_eq!(err.path(), path, "{pointer}");
    }

    // o número fracionário onde o esquema pede inteiro é erro de forma
    json["atributos"]["forca"] = 2.5.into();
    let err = parse_creature(&json.to_string()).unwrap_err();
    assert!(matches!(err, DataError::Json { .. }), "{err}");
    assert_eq!(err.path(), "atributos.forca");
}

#[test]
fn reads_the_catalogs_the_engine_uses() {
    let content = repo().join("content");
    let rules = centelha_data::load_rules(&content).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(rules.size_accuracy.order.len(), 8);
    let weapons = centelha_data::load_weapons(&content).unwrap_or_else(|e| panic!("{e}"));
    let armors = centelha_data::load_armors(&content).unwrap_or_else(|e| panic!("{e}"));
    let shields = centelha_data::load_shields(&content).unwrap_or_else(|e| panic!("{e}"));
    assert!(!weapons.is_empty() && !armors.is_empty() && !shields.is_empty());
    // toda arma, armadura e escudo citado por uma criatura existe no catálogo
    let creatures = load_bestiary(&content).unwrap();
    let has = |list: &[String], id: &str| list.iter().any(|x| x == id);
    let w: Vec<String> = weapons.iter().map(|x| x.id.clone()).collect();
    let a: Vec<String> = armors.iter().map(|x| x.id.clone()).collect();
    let s: Vec<String> = shields.iter().map(|x| x.id.clone()).collect();
    for c in &creatures {
        for atk in &c.attacks {
            if let Some(id) = &atk.weapon {
                assert!(has(&w, id), "{}: arma {id}", c.id);
            }
        }
        for item in &c.equipment.arsenal {
            let (kind, id) = item.reference.split_once(':').unwrap();
            let list = if kind == "a" { &w } else { &s };
            assert!(has(list, id), "{}: {}", c.id, item.reference);
        }
        for armor in &c.equipment.armors {
            assert!(has(&a, &armor.base), "{}: armadura {}", c.id, armor.base);
        }
    }
}

#[test]
fn accepts_the_legacy_form_size_without_area() {
    // a forma com Tamanho próprio e sem área própria: o esquema de entrada da
    // fonte recusa, a leitura dela aceita, e aqui também
    let text =
        std::fs::read_to_string(repo().join("content/data/bestiario/barghest.json")).unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&text).unwrap();
    json["formas"][0]["corpo"] = serde_json::json!({ "tamanhoEscolhido": "pequeno" });
    let c = parse_creature(&json.to_string()).unwrap_or_else(|e| panic!("{e}"));
    let body = c.forms.as_ref().unwrap()[0].body.as_ref().unwrap();
    assert_eq!(body.chosen_size, Some(Size::Small));
    assert_eq!(body.area, None);
}

#[test]
fn keeps_an_unknown_category() {
    let text = std::fs::read_to_string(repo().join("content/data/bestiario/aasimar.json")).unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&text).unwrap();
    json["categoria"] = "categoria-nova".into();
    let c = parse_creature(&json.to_string()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(c.category, Category::Other("categoria-nova".into()));
    assert_eq!(c.category.id(), "categoria-nova");
    json["categoria"] = "morto-vivo".into();
    let c = parse_creature(&json.to_string()).unwrap();
    assert_eq!(c.category, Category::Undead);
    assert_eq!(c.category.id(), "morto-vivo");
}

#[test]
fn rejects_a_zone_on_the_last_mass_row() {
    let text = std::fs::read_to_string(repo().join("content/data/regras.json")).unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&text).unwrap();
    let rows = json["derivados"]["pv"]["massa"]["conta"]
        .as_array_mut()
        .unwrap();
    rows.last_mut().unwrap()["zonaKg"] = serde_json::json!([100000, 120000]);
    let err = centelha_data::parse_rules(&json.to_string()).unwrap_err();
    assert!(matches!(err, DataError::Invalid { .. }), "{err}");
    assert_eq!(err.path(), "derivados.pv.massa.conta");
}

#[test]
fn checks_the_ids_of_armors_and_shields() {
    let bad = r#"[{"id": "Placa Velha", "nome": "x", "armadura": null, "escudo": null}]"#;
    for err in [
        centelha_data::parse_armors(bad).unwrap_err(),
        centelha_data::parse_shields(bad).unwrap_err(),
        centelha_data::parse_ancestries(bad).unwrap_err(),
    ] {
        assert!(matches!(err, DataError::Invalid { .. }), "{err}");
        assert_eq!(err.path(), "[0].id");
    }
}

#[test]
fn reads_the_ancestries() {
    let list =
        centelha_data::load_ancestries(&repo().join("content")).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(list.len(), 8);
    let get = |id: &str| list.iter().find(|a| a.id == id).unwrap();
    assert_eq!(get("humano").size, Some(Size::Medium));
    assert_eq!(get("humano").mass_kg, Some([60.0, 80.0]));
    assert_eq!(get("gnomo").size, Some(Size::Small));
    assert!(get("orc").adds_stamina_to_hp());
    assert!(!get("elfo").adds_stamina_to_hp());
    assert_eq!(get("anao").attributes.get("perspicacia"), Some(&-1.0));
}
