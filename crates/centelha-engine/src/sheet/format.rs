//! A forma gravada da ficha e a ponte para a forma de leitura (port de
//! `ficha-formato.ts`).
//!
//! Há duas formas da mesma ficha:
//! - a de leitura (a dos controles e das contas): `id`, `attrs`, `skills`,
//!   `skills2`, `virtues`, `willpower`, `equip`, `arsenal`, `conjuntos`, com os
//!   slots de mão em `habil`/`inabil` e o ajuste antigo em `mod`;
//! - a gravada (a do banco desde a migração 41): `nome`, `atributos`,
//!   `habilidades`, `habilidadesSec`, `virtudes`, `vontade`, `equipamento`,
//!   com os slots em `maoHabil`/`maoInabil` e o ajuste em `ajuste`.
//!
//! As duas funções aceitam as duas formas e os nomes antigos: `virtues.valor`
//! (hoje `bravura`), as três Secundárias livres (`livre-N` na leitura,
//! `habilidadeSecLivreN` na gravada), a Especialidade `{s, v}` contra
//! `{foco, valor}`, o Antecedente nomeado `{u, n, v}` contra
//! `{uid, nome, valor}`, `mod`/`img`/`enq` contra `ajuste`/`imagem`/
//! `enquadramento`. Toda peça com estatística sai com o item inteiro.

use serde_json::{Map, Value, json};

use super::equip::Catalog;
use super::value::{
    get, is_object_like, keys, nullish, num, number_in_text, or, set, spread, truthy,
};

fn free(i: usize) -> String {
    format!("livre-{i}")
}

fn free_saved(i: usize) -> String {
    format!("habilidadeSecLivre{i}")
}

/// `fichaNova`: a ficha está na forma gravada?
pub fn is_saved_form(f: &Value) -> bool {
    matches!(f, Value::Object(m) if !nullish(m.get("atributos")))
}

/// `Object.fromEntries(Object.entries(obj).map(...))` sobre um objeto.
fn map_entries(obj: Option<&Value>, f: impl Fn(&Value) -> Value) -> Value {
    let mut out = Map::new();
    match obj {
        Some(Value::Object(m)) => {
            for k in keys(m) {
                out.insert(k.clone(), f(&m[k.as_str()]));
            }
        }
        Some(Value::Array(a)) => {
            for (i, x) in a.iter().enumerate() {
                out.insert(i.to_string(), f(x));
            }
        }
        Some(Value::String(s)) => {
            for (i, c) in s.chars().enumerate() {
                out.insert(i.to_string(), f(&json!(c.to_string())));
            }
        }
        _ => {}
    }
    Value::Object(out)
}

/// `a ?? b ?? d`.
fn first_defined(a: Option<&Value>, b: Option<&Value>, d: Value) -> Value {
    if !nullish(a) {
        a.cloned().unwrap_or(d)
    } else if !nullish(b) {
        b.cloned().unwrap_or(d)
    } else {
        d
    }
}

/// `specsNovas` (`to_saved`) e `specsVelhas`: a lista de Especialidades de
/// cada Habilidade, de `{s, v}` para `{foco, valor}` e de volta.
fn specs(obj: Option<&Value>, to_saved: bool) -> Value {
    map_entries(obj, |v| match v {
        Value::Array(list) => Value::Array(
            list.iter()
                .map(|e| {
                    if !is_object_like(Some(e)) && !e.is_null() {
                        return e.clone();
                    }
                    let e = Some(e);
                    if to_saved {
                        json!({
                            "foco": first_defined(get(e, "s"), get(e, "foco"), json!("")),
                            "valor": first_defined(get(e, "v"), get(e, "valor"), json!(0)),
                        })
                    } else {
                        json!({
                            "s": first_defined(get(e, "foco"), get(e, "s"), json!("")),
                            "v": first_defined(get(e, "valor"), get(e, "v"), json!(0)),
                        })
                    }
                })
                .collect(),
        ),
        other => other.clone(),
    })
}

/// `anteNovo` (`to_saved`) e `anteVelho`: os Antecedentes nomeados, de
/// `{u, n, v}` para `{uid, nome, valor}` e de volta.
fn named_backgrounds(obj: Option<&Value>, to_saved: bool) -> Value {
    let (new, old) = if to_saved {
        (["uid", "nome", "valor"], ["u", "n", "v"])
    } else {
        (["u", "n", "v"], ["uid", "nome", "valor"])
    };
    map_entries(obj, |a| match a {
        Value::Array(list) => Value::Array(
            list.iter()
                .map(|e| {
                    let mut m = spread(Some(e));
                    for (n, o) in new.iter().zip(old) {
                        let v = if nullish(get(Some(e), o)) {
                            get(Some(e), n)
                        } else {
                            get(Some(e), o)
                        };
                        set(&mut m, n, v.cloned());
                    }
                    for o in old {
                        set(&mut m, o, None);
                    }
                    Value::Object(m)
                })
                .collect(),
        ),
        other => other.clone(),
    })
}

/// `pecaNova` (`to_saved`) e `pecaVelha`: os nomes dos campos da peça.
fn piece(p: &Value, to_saved: bool) -> Value {
    if !truthy(Some(p)) || !is_object_like(Some(p)) {
        return p.clone();
    }
    let (adj, img, frame) = if to_saved {
        (
            ("mod", "ajuste"),
            ("img", "imagem"),
            ("enq", "enquadramento"),
        )
    } else {
        (
            ("ajuste", "mod"),
            ("imagem", "img"),
            ("enquadramento", "enq"),
        )
    };
    let mut m = spread(Some(p));
    for k in [adj.0, img.0, frame.0] {
        m.shift_remove(k);
    }
    for (from, to) in [adj, img] {
        if let Some(v) = get(Some(p), from) {
            m.insert(to.into(), v.clone());
        }
    }
    let f = get(Some(p), frame.0);
    if truthy(f) {
        let names = if to_saved {
            [("z", "zoom"), ("px", "x"), ("py", "y")]
        } else {
            [("zoom", "z"), ("x", "px"), ("y", "py")]
        };
        let mut e = Map::new();
        for (from, to) in names {
            set(&mut e, to, get(f, from).cloned());
        }
        m.insert(frame.1.into(), Value::Object(e));
    }
    Value::Object(m)
}

/// `conjuntoNovo` (`to_saved`) e `conjuntoVelho`.
fn hand_set(c: &Value, to_saved: bool) -> Value {
    let mut m = spread(Some(c));
    let (from, to, drop): ([&str; 2], [&str; 2], &[&str]) = if to_saved {
        (
            ["habil", "inabil"],
            ["maoHabil", "maoInabil"],
            &["uidH", "uidI"],
        )
    } else {
        (["maoHabil", "maoInabil"], ["habil", "inabil"], &[])
    };
    for k in from.iter().chain(drop) {
        m.shift_remove(*k);
    }
    for (f, t) in from.iter().zip(to) {
        set(&mut m, t, get(Some(c), f).map(|p| piece(p, to_saved)));
    }
    Value::Object(m)
}

/// `(lista || []).map(f)`.
fn map_list(v: Option<&Value>, f: impl Fn(&Value) -> Value) -> Value {
    match or(v, None) {
        Some(Value::Array(a)) => Value::Array(a.iter().map(f).collect()),
        _ => json!([]),
    }
}

/// `virtues.valor` (o nome antigo da Bravura) vira `bravura`; `force` troca
/// mesmo quando já há `bravura` (a gravação faz assim, a leitura não).
fn fix_bravery(v: &mut Map<String, Value>, force: bool) {
    if let Some(val) = v.get("valor").cloned()
        && (force || !v.contains_key("bravura"))
    {
        v.insert("bravura".into(), val);
        v.shift_remove("valor");
    }
}

const SAVED_KEYS: [&str; 19] = [
    "nome",
    "conceito",
    "jogador",
    "titulo",
    "idade",
    "atributos",
    "habilidades",
    "especialidades",
    "habilidadesSec",
    "especialidadesSec",
    "virtudes",
    "vontade",
    "proezas",
    "artes",
    "efeitos",
    "antecedentes",
    "antecedentesNomeados",
    "xpTotal",
    "equipamento",
];

const READING_KEYS: [&str; 21] = [
    "id",
    "attrs",
    "skills",
    "spec",
    "skills2",
    "spec2",
    "livreNome",
    "virtues",
    "willpower",
    "tech",
    "arte",
    "efeito",
    "ante",
    "anteNom",
    "budget",
    "equip",
    "arsenal",
    "conjuntos",
    "defSpec",
    "derivCol",
    "secCol",
];

/// `fichaParaLeitura`: a ficha na forma de leitura, com o item inteiro em
/// toda peça. Aceita as duas formas.
pub fn for_reading(cat: &Catalog, input: &Value) -> Value {
    if !is_saved_form(input) {
        let mut v = input.clone();
        if let Some(Value::Object(virtues)) = v.get_mut("virtues") {
            fix_bravery(virtues, false);
        }
        cat.freeze_items(&mut v, false);
        return v;
    }
    let f = input;
    let Value::Object(fm) = f else {
        return f.clone();
    };
    let e = or(fm.get("equipamento"), None)
        .cloned()
        .unwrap_or_else(|| json!({}));
    let e = Some(&e);

    let mut id = Map::new();
    for k in ["nome", "conceito", "jogador", "titulo", "idade"] {
        set(&mut id, k, fm.get(k).cloned());
    }
    let body = fm.get("corpo");
    for (k, field, unit) in [("altura", "altura", "m"), ("massa", "peso", "kg")] {
        if let Some(Value::Number(n)) = get(body, k)
            && n.as_f64().is_some_and(f64::is_finite)
        {
            let x = n.as_f64().unwrap_or(0.0);
            id.insert(
                field.into(),
                json!(format!("{} {unit}", crate::js::number_to_string(x))),
            );
        }
    }

    let mut hs = spread(or(fm.get("habilidadesSec"), None));
    let mut free_names = Map::new();
    for i in 1..=3 {
        let v = hs.get(&free_saved(i)).cloned();
        if truthy(v.as_ref()) {
            set(&mut hs, &free(i), get(v.as_ref(), "valor").cloned());
            set(&mut free_names, &free(i), get(v.as_ref(), "nome").cloned());
        }
        hs.shift_remove(&free_saved(i));
    }
    let mut sp2 = spread(or(fm.get("especialidadesSec"), None));
    for i in 1..=3 {
        if truthy(sp2.get(&free_saved(i))) {
            let v = sp2[&free_saved(i)].clone();
            sp2.insert(free(i), v);
            sp2.shift_remove(&free_saved(i));
        }
    }
    let mut virtues = spread(or(fm.get("virtudes"), None));
    fix_bravery(&mut virtues, false);

    let mut out = spread(Some(f));
    for k in SAVED_KEYS {
        out.shift_remove(k);
    }
    let flags = |list: Option<&Value>| {
        let mut m = Map::new();
        if let Some(Value::Array(a)) = or(list, None) {
            for x in a {
                m.insert(super::value::property_key(Some(x)), json!(true));
            }
        }
        Value::Object(m)
    };
    let mut equip = spread(e);
    equip.insert(
        "armaduras".into(),
        map_list(get(e, "armaduras"), |p| piece(p, false)),
    );
    let fields: [(&str, Option<Value>); 18] = [
        ("id", Some(Value::Object(id))),
        ("attrs", fm.get("atributos").cloned()),
        ("skills", fm.get("habilidades").cloned()),
        ("spec", Some(specs(fm.get("especialidades"), false))),
        ("skills2", Some(Value::Object(hs))),
        ("spec2", Some(specs(Some(&Value::Object(sp2)), false))),
        ("livreNome", Some(Value::Object(free_names))),
        ("virtues", Some(Value::Object(virtues))),
        ("willpower", fm.get("vontade").cloned()),
        ("tech", Some(flags(fm.get("proezas")))),
        (
            "arte",
            Some(
                or(fm.get("artes"), None)
                    .cloned()
                    .unwrap_or_else(|| json!({})),
            ),
        ),
        ("efeito", Some(flags(fm.get("efeitos")))),
        (
            "ante",
            Some(
                or(fm.get("antecedentes"), None)
                    .cloned()
                    .unwrap_or_else(|| json!({})),
            ),
        ),
        (
            "anteNom",
            Some(named_backgrounds(fm.get("antecedentesNomeados"), false)),
        ),
        ("budget", fm.get("xpTotal").cloned()),
        ("equip", Some(Value::Object(equip))),
        (
            "arsenal",
            Some(map_list(get(e, "arsenal"), |p| piece(p, false))),
        ),
        (
            "conjuntos",
            Some(map_list(get(e, "conjuntos"), |c| hand_set(c, false))),
        ),
    ];
    for (k, v) in fields {
        set(&mut out, k, v);
    }
    let mut out = Value::Object(out);
    cat.freeze_items(&mut out, false);
    breath_in_leftovers(&mut out);
    out
}

/// No TypeScript, o `equip` da leitura é `{ ...equipamento }`: o `arsenal` e
/// os `conjuntos` gravados ficam lá dentro, e a peça deles divide o objeto
/// `item` com a peça lida (a cópia de `pecaVelha` é rasa). O `semFolego` da
/// leitura apaga o Fôlego do item da peça lida e, pelo mesmo objeto, do item
/// que ficou no `equip`. Aqui as cópias são separadas, e esta função faz a
/// mesma remoção nas que ficaram.
fn breath_in_leftovers(out: &mut Value) {
    let Some(Value::Object(equip)) = out.get_mut("equip") else {
        return;
    };
    let fix = |p: &mut Value| {
        if let Value::Object(m) = p
            && let Some(item @ Value::Object(_)) = m.get_mut("item")
        {
            super::equip::without_breath(item);
        }
    };
    if let Some(Value::Array(list)) = equip.get_mut("arsenal") {
        list.iter_mut().for_each(fix);
    }
    if let Some(Value::Array(sets)) = equip.get_mut("conjuntos") {
        for cj in sets.iter_mut() {
            for hand in ["maoHabil", "maoInabil"] {
                if let Some(p) = cj.get_mut(hand) {
                    fix(p);
                }
            }
        }
    }
}

/// `fichaParaGravar`: a ficha na forma gravada (a do banco), com o item
/// inteiro em toda peça. Aceita as duas formas.
pub fn for_saving(cat: &Catalog, input: &Value) -> Value {
    if is_saved_form(input) {
        let mut v = input.clone();
        cat.freeze_items(&mut v, true);
        return v;
    }
    if !truthy(Some(input)) || !is_object_like(Some(input)) {
        return input.clone();
    }
    let mut entry = input.clone();
    cat.freeze_items(&mut entry, false);
    let ent = Some(&entry);
    let id = or(get(ent, "id"), None)
        .cloned()
        .unwrap_or_else(|| json!({}));
    let id = Some(&id);

    let mut hs = spread(or(get(ent, "skills2"), None));
    let mut sp2 = spread(or(get(ent, "spec2"), None));
    for i in 1..=3 {
        let name = or(get(get(ent, "livreNome"), &free(i)), None)
            .cloned()
            .unwrap_or_else(|| json!(""));
        let value = or(hs.get(&free(i)), None)
            .cloned()
            .unwrap_or_else(|| json!(0));
        hs.insert(free_saved(i), json!({ "nome": name, "valor": value }));
        hs.shift_remove(&free(i));
        if truthy(sp2.get(&free(i))) {
            let v = sp2[&free(i)].clone();
            sp2.insert(free_saved(i), v);
            sp2.shift_remove(&free(i));
        }
    }
    let mut virtues = spread(or(get(ent, "virtues"), None));
    fix_bravery(&mut virtues, true);

    let mut body = spread(or(get(ent, "corpo"), None));
    if !body.contains_key("altura")
        && let Some(h) = number_in_text(get(id, "altura"))
    {
        body.insert("altura".into(), num(h));
    }
    let mass = body.get("massa");
    let mass_ok = matches!(mass, Some(Value::Number(n)) if n.as_f64().is_some_and(|x| x.is_finite() && x > 0.0));
    if !mass_ok {
        body.shift_remove("massa");
        if let Some(w) = number_in_text(get(id, "peso"))
            && w > 0.0
        {
            body.insert("massa".into(), num(w));
        }
    }

    let mut out = spread(ent);
    for k in READING_KEYS {
        out.shift_remove(k);
    }
    let text_or_empty = |k: &str| or(get(id, k), None).cloned().unwrap_or_else(|| json!(""));
    let truthy_keys = |v: Option<&Value>| match or(v, None) {
        Some(Value::Object(m)) => Value::Array(
            keys(m)
                .into_iter()
                .filter(|k| truthy(m.get(k.as_str())))
                .map(|k| json!(k))
                .collect(),
        ),
        _ => json!([]),
    };
    let e = or(get(ent, "equip"), None)
        .cloned()
        .unwrap_or_else(|| json!({}));
    let mut equip = spread(Some(&e));
    equip.insert(
        "armaduras".into(),
        map_list(get(Some(&e), "armaduras"), |p| piece(p, true)),
    );
    equip.insert(
        "arsenal".into(),
        map_list(get(ent, "arsenal"), |p| piece(p, true)),
    );
    equip.insert(
        "conjuntos".into(),
        map_list(get(ent, "conjuntos"), |c| hand_set(c, true)),
    );
    let obj_or_empty = |k: &str| or(get(ent, k), None).cloned().unwrap_or_else(|| json!({}));
    let fields: Vec<(&str, Option<Value>)> = vec![
        ("nome", Some(text_or_empty("nome"))),
        ("conceito", Some(text_or_empty("conceito"))),
        ("jogador", Some(text_or_empty("jogador"))),
        ("titulo", Some(text_or_empty("titulo"))),
        ("idade", Some(text_or_empty("idade"))),
        ("corpo", Some(Value::Object(body))),
        ("atributos", Some(obj_or_empty("attrs"))),
        ("habilidades", Some(obj_or_empty("skills"))),
        ("especialidades", Some(specs(get(ent, "spec"), true))),
        ("habilidadesSec", Some(Value::Object(hs))),
        (
            "especialidadesSec",
            Some(specs(Some(&Value::Object(sp2)), true)),
        ),
        ("virtudes", Some(Value::Object(virtues))),
        (
            "vontade",
            Some(
                or(get(ent, "willpower"), None)
                    .cloned()
                    .unwrap_or_else(|| json!(0)),
            ),
        ),
        ("proezas", Some(truthy_keys(get(ent, "tech")))),
        ("artes", Some(obj_or_empty("arte"))),
        ("efeitos", Some(truthy_keys(get(ent, "efeito")))),
        ("antecedentes", Some(obj_or_empty("ante"))),
        (
            "antecedentesNomeados",
            Some(named_backgrounds(get(ent, "anteNom"), true)),
        ),
        ("xpTotal", get(ent, "budget").cloned()),
        ("equipamento", Some(Value::Object(equip))),
    ];
    for (k, v) in fields {
        set(&mut out, k, v);
    }
    Value::Object(out)
}
