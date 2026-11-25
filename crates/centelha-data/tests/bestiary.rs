//! As 324 criaturas de content/ se leem sem erro, e os ids batem com a lista
//! gravada pelo gerador de referências (tests/parity/bestiary-ids.json).

use std::path::{Path, PathBuf};

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
