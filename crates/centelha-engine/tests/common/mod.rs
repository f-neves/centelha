//! O que os testes de paridade dividem: a leitura das referências e a
//! comparação folha por folha.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use serde_json::Value;

pub fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn read_reference(name: &str) -> Value {
    let text = std::fs::read_to_string(repo().join("tests/parity").join(name)).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// Um número da saída do Rust como o JSON da referência o guarda.
pub fn num(x: f64) -> Value {
    Value::Number(
        serde_json::Number::from_f64(x).unwrap_or_else(|| panic!("número não finito: {x}")),
    )
}

pub fn opt(x: Option<f64>) -> Value {
    x.map_or(Value::Null, num)
}

/// O número da referência: o gerador grava -0 como o texto `"-0"` (o JSON
/// não guarda -0).
fn reference_number(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) if s == "-0" => Some(-0.0),
        _ => None,
    }
}

/// Compara folha por folha; devolve quantas folhas comparou e anota as
/// diferenças. Número por `to_bits()` (0 e -0 são diferentes); texto, booleano
/// e `null` por igualdade; objeto pelas mesmas chaves (a ordem não conta);
/// array pelo mesmo tamanho e item a item.
pub fn compare(path: &str, got: &Value, want: &Value, diffs: &mut Vec<String>) -> usize {
    match (got, want) {
        (Value::Object(g), Value::Object(w)) => {
            let mut n = 0;
            for k in w.keys().chain(g.keys().filter(|k| !w.contains_key(*k))) {
                match (g.get(k), w.get(k)) {
                    (Some(a), Some(b)) => n += compare(&format!("{path}.{k}"), a, b, diffs),
                    (None, Some(b)) => {
                        diffs.push(format!("{path}.{k}: falta no Rust (TypeScript: {b})"))
                    }
                    (Some(a), None) => diffs.push(format!("{path}.{k}: sobra no Rust ({a})")),
                    (None, None) => unreachable!(),
                }
            }
            n
        }
        (Value::Array(g), Value::Array(w)) => {
            if g.len() != w.len() {
                diffs.push(format!(
                    "{path}: {} itens no Rust, {} no TypeScript",
                    g.len(),
                    w.len()
                ));
                return 1;
            }
            g.iter()
                .zip(w)
                .enumerate()
                .map(|(i, (a, b))| compare(&format!("{path}[{i}]"), a, b, diffs))
                .sum()
        }
        (Value::Number(a), b) if reference_number(b).is_some() => {
            let (a, b) = (a.as_f64().unwrap(), reference_number(b).unwrap());
            if a.to_bits() != b.to_bits() {
                diffs.push(format!("{path}: Rust {a:?}, TypeScript {b:?}"));
            }
            1
        }
        (a, b) => {
            if a != b {
                diffs.push(format!("{path}: Rust {a}, TypeScript {b}"));
            }
            1
        }
    }
}

/// O teste do comparador: ele acha cada tipo de diferença.
pub fn check_comparator() {
    use serde_json::json;
    let mut diffs = Vec::new();
    let want = json!({"pv": 37, "zero": "-0", "tamanho": "medio", "mana": null, "ataques": [{"jogada": 7}]});
    let same = json!({"pv": 37.0, "zero": num(-0.0), "tamanho": "medio", "mana": null, "ataques": [{"jogada": 7}]});
    assert_eq!(compare("x", &same, &want, &mut diffs), 5);
    assert!(diffs.is_empty(), "{diffs:?}");
    let cases = [
        json!({"pv": num(f64::from_bits(37f64.to_bits() + 1)), "zero": num(-0.0), "tamanho": "medio", "mana": null, "ataques": [{"jogada": 7}]}),
        json!({"pv": 37, "zero": 0, "tamanho": "medio", "mana": null, "ataques": [{"jogada": 7}]}),
        json!({"pv": 37, "zero": num(-0.0), "tamanho": "grande", "mana": null, "ataques": [{"jogada": 7}]}),
        json!({"pv": 37, "zero": num(-0.0), "tamanho": "medio", "mana": 0, "ataques": [{"jogada": 7}]}),
        json!({"pv": 37, "zero": num(-0.0), "tamanho": "medio", "ataques": [{"jogada": 7}]}),
        json!({"pv": 37, "zero": num(-0.0), "tamanho": "medio", "mana": null, "ataques": [{"jogada": 7}], "enxame": {}}),
        json!({"pv": 37, "zero": num(-0.0), "tamanho": "medio", "mana": null, "ataques": []}),
    ];
    for got in cases {
        let mut diffs = Vec::new();
        compare("x", &got, &want, &mut diffs);
        assert_eq!(diffs.len(), 1, "{got}: {diffs:?}");
    }
    // e o 0 do Rust contra o 0 da referência passa
    let mut diffs = Vec::new();
    compare("x", &json!({"a": 0}), &json!({"a": 0}), &mut diffs);
    assert!(diffs.is_empty());
}
