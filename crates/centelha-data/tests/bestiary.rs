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

    let cases: [(&str, serde_json::Value, &str); 5] = [
        ("/ameaca", 7.into(), "ameaca"),
        ("/centelha", (-1).into(), "centelha"),
        (
            "/corpo/tamanhoEscolhido",
            "gigante".into(),
            "corpo.tamanhoEscolhido",
        ),
        ("/perfilCouraca", "pedra".into(), "perfilCouraca"),
        (
            "/equipamento/arsenal",
            serde_json::json!([{"ref": "x:espada"}]),
            "equipamento.arsenal[0].ref",
        ),
    ];
    for (pointer, value, path) in cases {
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
        assert!(matches!(err, DataError::Invalid { .. }), "{pointer}: {err}");
        assert_eq!(err.path(), path, "{pointer}");
    }

    // o número fracionário onde o esquema pede inteiro é erro de forma
    json["atributos"]["forca"] = 2.5.into();
    let err = parse_creature(&json.to_string()).unwrap_err();
    assert!(matches!(err, DataError::Json { .. }), "{err}");
    assert_eq!(err.path(), "atributos.forca");
}
