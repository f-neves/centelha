//! O valor do JavaScript sobre o `serde_json::Value`, para portar o código da
//! ficha (que trabalha com objetos soltos) com o mesmo comportamento.
//!
//! Convenções:
//! - `Option<&Value>`: `None` é o `undefined` do JavaScript (chave ausente);
//!   `Some(Value::Null)` é o `null`.
//! - Gravar `undefined` numa chave é tirá-la do objeto: o `JSON.stringify` não
//!   escreve chave com `undefined`, e ler a chave dá `undefined` nos dois casos.
//! - A ordem das chaves é a do JavaScript: as chaves que são índice de array
//!   (`"0"`, `"12"`) vêm primeiro, em ordem numérica, e as outras na ordem em
//!   que entraram (`keys`).

use serde_json::{Map, Value};

use crate::js;

/// `undefined` ou `null`.
pub(crate) fn nullish(v: Option<&Value>) -> bool {
    matches!(v, None | Some(Value::Null))
}

/// O valor verdadeiro do JavaScript (`if (v)`).
pub(crate) fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(_) | Value::Object(_)) => true,
    }
}

/// `typeof v === 'object' && v !== null` (inclui o array, como no JavaScript).
pub(crate) fn is_object_like(v: Option<&Value>) -> bool {
    matches!(v, Some(Value::Object(_) | Value::Array(_)))
}

/// `v?.[k]`: a chave de um objeto, o índice de um array; `undefined` no resto.
pub(crate) fn get<'a>(v: Option<&'a Value>, k: &str) -> Option<&'a Value> {
    match v? {
        Value::Object(m) => m.get(k),
        Value::Array(a) => array_index(k).and_then(|i| a.get(i)),
        _ => None,
    }
}

/// `"12"` é índice de array; `"012"`, `"-1"` e `"1.5"` não são.
fn array_index(k: &str) -> Option<usize> {
    if k.is_empty() || (k.len() > 1 && k.starts_with('0')) || !k.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let n: u64 = k.parse().ok()?;
    (n < u64::from(u32::MAX)).then_some(n as usize)
}

/// As chaves de um objeto na ordem do JavaScript (`Object.keys`).
pub(crate) fn keys(m: &Map<String, Value>) -> Vec<&String> {
    let mut idx: Vec<(&String, usize)> = m
        .keys()
        .filter_map(|k| array_index(k).map(|i| (k, i)))
        .collect();
    idx.sort_by_key(|(_, i)| *i);
    let mut out: Vec<&String> = idx.into_iter().map(|(k, _)| k).collect();
    out.extend(m.keys().filter(|k| array_index(k).is_none()));
    out
}

/// `a ?? b`.
pub(crate) fn coalesce<'a>(a: Option<&'a Value>, b: Option<&'a Value>) -> Option<&'a Value> {
    if nullish(a) { b } else { a }
}

/// `a || b`.
pub(crate) fn or<'a>(a: Option<&'a Value>, b: Option<&'a Value>) -> Option<&'a Value> {
    if truthy(a) { a } else { b }
}

/// `Number(v)`.
pub(crate) fn to_number(v: Option<&Value>) -> f64 {
    match v {
        None => f64::NAN,
        Some(Value::Null) => 0.0,
        Some(Value::Bool(b)) => f64::from(u8::from(*b)),
        Some(Value::Number(n)) => n.as_f64().unwrap_or(f64::NAN),
        Some(Value::String(s)) => string_to_number(s),
        Some(a @ Value::Array(_)) => string_to_number(&to_js_string(Some(a))),
        Some(Value::Object(_)) => f64::NAN,
    }
}

/// `v || 0` lido como número.
pub(crate) fn num_or0(v: Option<&Value>) -> f64 {
    if truthy(v) { to_number(v) } else { 0.0 }
}

/// `v ?? d` lido como número.
pub(crate) fn num_or(v: Option<&Value>, d: f64) -> f64 {
    if nullish(v) { d } else { to_number(v) }
}

/// O espaço do `\s` e do `trim` do JavaScript.
pub(crate) fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{0b}' | '\u{0c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// `Number(texto)`: decimal (com expoente), `0x`/`0o`/`0b`, `Infinity`; vazio
/// dá 0; o resto, `NaN`.
pub(crate) fn string_to_number(s: &str) -> f64 {
    let t = s.trim_matches(is_js_space);
    if t.is_empty() {
        return 0.0;
    }
    for (prefix, radix) in [
        ("0x", 16),
        ("0X", 16),
        ("0o", 8),
        ("0O", 8),
        ("0b", 2),
        ("0B", 2),
    ] {
        if let Some(d) = t.strip_prefix(prefix) {
            if d.is_empty() || !d.chars().all(|c| c.is_digit(radix)) {
                return f64::NAN;
            }
            return d.chars().fold(0.0, |acc, c| {
                acc * f64::from(radix) + f64::from(c.to_digit(radix).unwrap_or(0))
            });
        }
    }
    let (sign, body) = match t.as_bytes()[0] {
        b'+' => (1.0, &t[1..]),
        b'-' => (-1.0, &t[1..]),
        _ => (1.0, t),
    };
    if body == "Infinity" {
        return sign * f64::INFINITY;
    }
    // o formato decimal do JavaScript: dígitos, ponto opcional, expoente opcional
    let b = body.as_bytes();
    let mut i = 0;
    let digits = |i: &mut usize| {
        let s = *i;
        while *i < b.len() && b[*i].is_ascii_digit() {
            *i += 1;
        }
        *i - s
    };
    let int = digits(&mut i);
    let mut frac = 0;
    if i < b.len() && b[i] == b'.' {
        i += 1;
        frac = digits(&mut i);
    }
    if int + frac == 0 {
        return f64::NAN;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        if digits(&mut i) == 0 {
            return f64::NAN;
        }
    }
    if i != b.len() {
        return f64::NAN;
    }
    body.parse::<f64>().map_or(f64::NAN, |x| sign * x)
}

/// `String(v)`.
pub(crate) fn to_js_string(v: Option<&Value>) -> String {
    match v {
        None => "undefined".into(),
        Some(Value::Null) => "null".into(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(a)) => a
            .iter()
            .map(|x| match x {
                Value::Null => String::new(),
                x => to_js_string(Some(x)),
            })
            .collect::<Vec<_>>()
            .join(","),
        Some(Value::Object(_)) => "[object Object]".into(),
    }
}

/// O número como o JSON o grava: inteiro quando é inteiro (e cabe com
/// exatidão), `-0` como `-0.0`, e `NaN` ou infinito como `null` (o que o
/// `JSON.stringify` escreve).
pub(crate) fn num(x: f64) -> Value {
    if x == 0.0 && x.is_sign_negative() {
        return serde_json::Number::from_f64(x).map_or(Value::Null, Value::Number);
    }
    if x.fract() == 0.0 && x.abs() < 9_007_199_254_740_992.0 {
        return Value::from(x as i64);
    }
    serde_json::Number::from_f64(x).map_or(Value::Null, Value::Number)
}

/// Grava `v` na chave `k`; `None` (o `undefined`) tira a chave.
pub(crate) fn set(m: &mut Map<String, Value>, k: &str, v: Option<Value>) {
    match v {
        Some(v) => {
            m.insert(k.to_owned(), v);
        }
        None => {
            m.shift_remove(k);
        }
    }
}

/// `{ ...v }` sobre `m`: objeto copia as chaves; array e texto copiam por
/// índice; o resto não copia nada.
pub(crate) fn spread_into(m: &mut Map<String, Value>, v: Option<&Value>) {
    match v {
        Some(Value::Object(o)) => {
            for k in keys(o) {
                m.insert(k.clone(), o[k.as_str()].clone());
            }
        }
        Some(Value::Array(a)) => {
            for (i, x) in a.iter().enumerate() {
                m.insert(i.to_string(), x.clone());
            }
        }
        Some(Value::String(s)) => {
            for (i, c) in s.chars().enumerate() {
                m.insert(i.to_string(), Value::String(c.to_string()));
            }
        }
        _ => {}
    }
}

/// `{ ...v }`.
pub(crate) fn spread(v: Option<&Value>) -> Map<String, Value> {
    let mut m = Map::new();
    spread_into(&mut m, v);
    m
}

/// `===` entre dois valores (objetos e arrays só são iguais a si mesmos; aqui,
/// por não haver identidade, nunca).
pub(crate) fn strict_eq(a: Option<&Value>, b: Option<&Value>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(Value::Null), Some(Value::Null)) => true,
        (Some(Value::Bool(x)), Some(Value::Bool(y))) => x == y,
        (Some(Value::Number(x)), Some(Value::Number(y))) => x.as_f64() == y.as_f64(),
        (Some(Value::String(x)), Some(Value::String(y))) => x == y,
        _ => false,
    }
}

/// `v === texto`.
pub(crate) fn is_str(v: Option<&Value>, s: &str) -> bool {
    matches!(v, Some(Value::String(x)) if x == s)
}

/// `(v || []).includes(x)`: o array procura o elemento; o texto procura o
/// pedaço.
pub(crate) fn includes(v: Option<&Value>, x: &str) -> bool {
    match v {
        Some(Value::Array(a)) => a.iter().any(|e| is_str(Some(e), x)),
        Some(Value::String(s)) if truthy(v) => s.contains(x),
        _ => false,
    }
}

/// A chave de propriedade de um valor (`obj[v]`): o texto dele.
pub(crate) fn property_key(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        other => to_js_string(other),
    }
}

/// O primeiro número de um texto (`/\d+(?:\.\d+)?/`), como número.
pub(crate) fn first_number(s: &str) -> Option<f64> {
    let b = s.as_bytes();
    let start = b.iter().position(u8::is_ascii_digit)?;
    let mut end = start;
    while end < b.len() && b[end].is_ascii_digit() {
        end += 1;
    }
    if end + 1 < b.len() && b[end] == b'.' && b[end + 1].is_ascii_digit() {
        end += 1;
        while end < b.len() && b[end].is_ascii_digit() {
            end += 1;
        }
    }
    s[start..end].parse().ok()
}

/// `String(v ?? '').replace(',', '.')` e o primeiro número dele.
pub(crate) fn number_in_text(v: Option<&Value>) -> Option<f64> {
    let s = if nullish(v) {
        String::new()
    } else {
        to_js_string(v)
    };
    first_number(&s.replacen(',', ".", 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn numbers_like_javascript() {
        for (s, want) in [
            ("", 0.0),
            ("  12 ", 12.0),
            ("1e3", 1000.0),
            (".5", 0.5),
            ("5.", 5.0),
            ("0x1f", 31.0),
            ("-Infinity", f64::NEG_INFINITY),
        ] {
            assert_eq!(string_to_number(s), want, "{s:?}");
        }
        for s in ["abc", "1,5", "e3", ".", "0x", "1e", "--1"] {
            assert!(string_to_number(s).is_nan(), "{s:?}");
        }
        assert_eq!(to_number(Some(&json!([7]))), 7.0);
        assert!(to_number(Some(&json!({}))).is_nan());
        assert!(to_number(None).is_nan());
        assert_eq!(to_number(Some(&Value::Null)), 0.0);
    }

    #[test]
    fn text_like_javascript() {
        assert_eq!(to_js_string(Some(&json!([1, null, "a"]))), "1,,a");
        assert_eq!(to_js_string(Some(&json!(1.5))), "1.5");
        assert_eq!(to_js_string(Some(&json!({"a": 1}))), "[object Object]");
        assert_eq!(number_in_text(Some(&json!("78,5 kg"))), Some(78.5));
        assert_eq!(number_in_text(Some(&json!("cerca de 80."))), Some(80.0));
        assert_eq!(number_in_text(Some(&json!("pesado"))), None);
        assert_eq!(number_in_text(None), None);
    }

    #[test]
    fn keys_in_javascript_order() {
        let v = json!({"b": 1, "10": 2, "a": 3, "2": 4, "01": 5});
        let m = v.as_object().unwrap();
        let k: Vec<&str> = keys(m).into_iter().map(String::as_str).collect();
        assert_eq!(k, ["2", "10", "b", "a", "01"]);
    }

    #[test]
    fn truth_and_includes() {
        assert!(!truthy(Some(&json!(0))));
        assert!(!truthy(Some(&json!(""))));
        assert!(truthy(Some(&json!([]))));
        assert!(includes(Some(&json!(["versátil"])), "versátil"));
        assert!(includes(Some(&json!("muito versátil")), "versátil"));
        assert!(!includes(Some(&json!({"versátil": true})), "versátil"));
        assert_eq!(num(3.0), json!(3));
        assert!(num(-0.0).as_f64().unwrap().is_sign_negative());
    }
}
