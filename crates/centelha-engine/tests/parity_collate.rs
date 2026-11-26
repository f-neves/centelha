//! Paridade da colação com o `localeCompare` do Node (tests/parity/collate.json,
//! de tools/parity/collate.mjs): o sinal de cada par nas três comparações.

mod common;

use std::cmp::Ordering;

use centelha_engine::collate::{compare, compare_base, compare_numeric_base, covers};
use serde_json::Value;

fn sign(o: Ordering) -> i64 {
    match o {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

#[test]
fn collation_matches_node() {
    let r = common::read_reference("collate.json");
    let pairs = r["pairs"].as_array().unwrap();
    assert_eq!(pairs.len() as u64, r["count"].as_u64().unwrap());
    let mut diffs = Vec::new();
    for p in pairs {
        let (a, b) = (p[0].as_str().unwrap(), p[1].as_str().unwrap());
        let want = |i: usize| p[i].as_i64().unwrap();
        for (i, name, got) in [
            (2, "pt", sign(compare(a, b))),
            (3, "base", sign(compare_base(a, b))),
            (4, "numerico", sign(compare_numeric_base(a, b))),
        ] {
            if got != want(i) {
                diffs.push(format!(
                    "{name}: {a:?} e {b:?}: Rust {got}, Node {}",
                    want(i)
                ));
            }
        }
    }
    println!("colação: {} pares, {} diferenças", pairs.len(), diffs.len());
    assert!(
        diffs.is_empty(),
        "{} diferenças:\n{}",
        diffs.len(),
        diffs[..diffs.len().min(120)].join("\n")
    );
}

/// Os nomes do livro em ordem: os vizinhos da lista ordenada pelo Node saem em
/// ordem também no Rust.
#[test]
fn the_reference_has_the_book_names() {
    let r = common::read_reference("collate.json");
    assert!(r["names"].as_u64().unwrap() > 1000);
    assert!(
        r["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| matches!(p, Value::Array(a) if a.len() == 5))
    );
}

/// Os nomes dos catálogos de `content/` ficam dentro da cobertura da colação:
/// um nome novo com letra que ela não trata (`ø`, `ß`, grego...) faz este teste
/// falhar, com o nome e a letra.
#[test]
fn book_names_stay_inside_the_coverage() {
    let data = common::repo().join("content/data");
    let read = |f: &str| -> Value {
        serde_json::from_str(&std::fs::read_to_string(data.join(f)).unwrap()).unwrap()
    };
    let mut out = Vec::new();
    let mut seen = 0;
    for (file, key) in [
        ("efeitos.json", "nome"),
        ("tecnicas.json", "nome"),
        ("artes.json", "nome"),
        ("caminhos.json", "nome"),
        ("habilidades-secundarias.json", "nome"),
        ("monsters.json", "nome"),
        ("glossario.json", "termo"),
    ] {
        for item in read(file).as_array().unwrap() {
            let name = item[key].as_str().unwrap();
            seen += 1;
            for c in name.chars().filter(|c| !covers(*c)) {
                out.push(format!("{file}: {name:?} tem {c:?} (U+{:04X})", c as u32));
            }
        }
    }
    assert!(seen > 1000, "{seen}");
    assert!(
        out.is_empty(),
        "letras fora da cobertura da colação:\n{}",
        out.join("\n")
    );
}

/// O que fica fora da cobertura, para o teste acima ter o que acusar.
#[test]
fn the_coverage_leaves_out_what_it_does_not_order() {
    for c in ['a', 'Z', '0', ' ', '-', 'á', 'Ç', 'ã', 'Õ', 'ü', 'ñ'] {
        assert!(covers(c), "{c}");
    }
    for c in [
        'ø', 'ß', 'æ', 'ł', 'ı', 'İ', 'Ÿ', 'α', 'Ω', '×', '−', '·', '\u{2019}',
    ] {
        assert!(!covers(c), "{c}");
    }
}
