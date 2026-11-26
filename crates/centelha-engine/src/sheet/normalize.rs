//! Abrir a ficha: o resto do `normalize()` da ficha interativa (port de
//! `src/lib/ficha-engine.ts`, linhas 303 a 435).
//!
//! Depois do `fichaParaLeitura` (que é o [`for_reading`]), a fonte ainda
//! migra o que as fichas antigas guardavam (Escudos viram Bloqueio, as Armas
//! se fundem, as Secundárias mudam de nome, o `equip.arma` vira conjunto),
//! preenche os padrões de cada trilha, sobe até o piso da Raça o Atributo que
//! ficou abaixo, monta o arsenal a partir das mãos e arruma as bolsas. Aqui o
//! mesmo, na mesma ordem: a ordem das migrações e a das chaves fazem parte da
//! paridade.
//!
//! A fonte sorteia o `uid` das peças novas (`novoUid`, com `Math.random`); aqui
//! ele chega de fora, por uma função, na ordem em que a fonte o pede.

use serde_json::{Map, Value, json};

use super::editor::{EditorTables, clamp_specialties};
use super::equip::Catalog;
use super::format::for_reading;
use super::migrate::{WEAPON_RENAMES, migrate_weapon_refs};
use super::value::{is_str, nullish, num, num_or0, to_js_string, to_number, truthy};
use crate::js;

/// Um Atributo que estava abaixo do piso da Raça e subiu (`SUBIU_PELO_PISO`):
/// o jogador é avisado.
#[derive(Debug, Clone, PartialEq)]
pub struct FloorRaise {
    /// O nome do Atributo (`a.nome || a.id`).
    pub name: String,
    /// O valor que a ficha tinha, como estava.
    pub from: Value,
    /// O piso.
    pub to: f64,
}

/// A ficha aberta.
#[derive(Debug, Clone, PartialEq)]
pub struct Normalized {
    /// A ficha na forma de leitura, com as migrações e os padrões.
    pub sheet: Value,
    /// Os Atributos que subiram até o piso da Raça, na ordem do catálogo.
    pub raised: Vec<FloorRaise>,
    /// A massa foi escolhida pelo jogador (`massaEscolhida`): `corpo.massa` é
    /// número, ou a identidade tem peso escrito. Medida antes dos padrões.
    pub mass_chosen: bool,
}

/// Os Secundários que eram primários: (Habilidade antiga, Secundária nova).
const PRIMARY_TO_SECONDARY: [(&str, &str); 3] = [
    ("lideranca", "lideranca"),
    ("medicina", "cura"),
    ("energia-espiritual", "energia-espiritual"),
];

/// `RENOMES`: as Secundárias renomeadas e fundidas na revisão de
/// nomenclatura. Na fusão fica o maior dos dois níveis.
const SECONDARY_RENAMES: [(&str, &str); 10] = [
    ("culinaria", "gastronomia"),
    ("veterinario", "veterinaria"),
    ("ferraria", "ferreiro"),
    ("escrivania", "escrivao"),
    ("comercio", "comerciante"),
    ("abrir-fechaduras", "abrir-mecanismos"),
    ("ladinagem", "abrir-mecanismos"),
    ("marcenaria", "carpintaria"),
    ("contrabando", "ocultacao"),
    ("falsificacao", "ocultacao"),
];

/// As linhas em branco de cada bolsa (`LINHAS_BOLSA`).
const BAG_ROWS: usize = 6;

/// Abre a ficha: a leitura, as migrações e os padrões (`normalize`).
///
/// `new_uid` dá o `uid` de cada peça nova, na ordem em que a fonte o pede
/// (uma por instância nomeada sem `u`, uma por armadura sem `uid`, uma por
/// peça do arsenal sem `uid` e uma por arma de mão que não está no arsenal).
pub fn normalize_sheet(
    t: &EditorTables,
    input: &Value,
    new_uid: &mut dyn FnMut() -> String,
) -> Normalized {
    let cat = t.sheet_tables().catalog();
    let mut sheet = for_reading(cat, input);
    let mass_chosen = super::editor::mass_chosen(&sheet);
    if !sheet.is_object() {
        sheet = Value::Object(Map::new());
    }
    let mut raised = Vec::new();

    {
        let s = obj(&mut sheet);
        default_object(s, "corpo");
        default_object(s, "id");
        // virtudes: `valor` virou `bravura`
        if let Some(Value::Object(v)) = s.get_mut("virtues")
            && v.contains_key("valor")
            && !v.contains_key("bravura")
        {
            let valor = v.shift_remove("valor").unwrap_or(Value::Null);
            v.insert("bravura".into(), valor);
        }
        for k in [
            "attrs", "skills", "spec", "skills2", "spec2", "virtues", "tech", "arte", "efeito",
            "secCol", "defSpec",
        ] {
            default_object(s, k);
        }
        let def = obj_at(s, "defSpec");
        for k in ["esquiva", "bloqueio", "social", "mental"] {
            default_with(def, k, || json!([]));
        }

        // Escudos virou Bloqueio
        for group in ["skills", "spec"] {
            let g = obj_at(s, group);
            if !nullish(g.get("escudos")) && nullish(g.get("bloqueio")) {
                let v = g["escudos"].clone();
                g.insert("bloqueio".into(), v);
            }
            g.shift_remove("escudos");
        }
        s.shift_remove("blkPericia");

        merge_weapon_skills(s);
        primary_to_secondary(s);
        secondary_renames(s);

        default_with(s, "willpower", || num(t.floor("vontade")));
        default_with(s, "aparencia", || num(t.floor("aparencia")));
        default_with(s, "centelha", || json!(0));
        default_with(s, "raca", || json!("humano"));
    }
    // a Raça tem de existir
    let raca = sheet.get("raca").cloned().unwrap_or(Value::Null);
    if t.sheet_tables().ancestry_of(&raca).is_none() {
        obj(&mut sheet).insert("raca".into(), json!("humano"));
    }

    // o `+1` racial virou também piso: o Atributo abaixo dele sobe, e avisa
    for (id, name) in t.attributes() {
        let floor = super::editor::attribute_floor(t, &sheet, id);
        let attrs = obj_at(obj(&mut sheet), "attrs");
        let v = attrs.get(id).filter(|v| !v.is_null()).cloned();
        let v = v.unwrap_or_else(|| num(floor));
        if to_number(Some(&v)) >= floor {
            continue;
        }
        raised.push(FloorRaise {
            name: name.to_owned(),
            from: v,
            to: floor,
        });
        attrs.insert(id.to_owned(), num(floor));
    }

    {
        let s = obj(&mut sheet);
        default_with(s, "budget", || json!(1500));
        default_with(s, "derivCol", || json!(true));
        s.shift_remove("modo");
        default_object(s, "ante");
        default_object(s, "anteNom");
        background_renames(s);
        // cada Nomeado é uma lista; a entrada sem `u` ganha um
        for id in t.named_background_ids() {
            let list = match s.get("anteNom").and_then(|a| a.get(id)) {
                Some(Value::Array(a)) => a.clone(),
                _ => Vec::new(),
            };
            let list: Vec<Value> = list
                .iter()
                .map(|x| {
                    let u = match x.get("u") {
                        Some(u) if truthy(Some(u)) => u.clone(),
                        _ => json!(new_uid()),
                    };
                    let n = json!(to_js_string(Some(
                        x.get("n").filter(|n| !n.is_null()).unwrap_or(&json!(""))
                    )));
                    let v = js::or_zero(to_number(x.get("v")));
                    json!({"u": u, "n": n, "v": num(v)})
                })
                .collect();
            obj_at(s, "anteNom").insert(id.clone(), Value::Array(list));
        }
        default_object(s, "equip");
    }
    migrate_weapon_refs(&mut sheet, &WEAPON_RENAMES);

    {
        let s = obj(&mut sheet);
        equip_to_sets(s);
        armor_pieces(cat, s, new_uid);
        build_arsenal(s, new_uid);
    }
    cat.sync_slots(&mut sheet);

    {
        let s = obj(&mut sheet);
        bags(s);
        // vazia quer dizer "use a conta"; qualquer coisa escrita manda nela
        let carga = match s.get("cargaAtual") {
            Some(v) if !v.is_null() => to_js_string(Some(v)),
            _ => String::new(),
        };
        s.insert("cargaAtual".into(), json!(carga));
        for id in t.attributes().map(|(id, _)| id) {
            default_with(obj_at(s, "attrs"), id, || json!(1));
        }
        for id in t.skill_ids() {
            default_with(obj_at(s, "skills"), id, || json!(0));
            clamp_into(s, "skills", "spec", id);
        }
        for id in t.virtue_ids() {
            default_with(obj_at(s, "virtues"), id, || json!(1));
        }
        for id in t.secondary_ids() {
            default_with(obj_at(s, "skills2"), id, || json!(0));
            clamp_into(s, "skills2", "spec2", id);
        }
        default_object(s, "livreNome");
        for id in super::editor::FREE_SECONDARY {
            default_with(obj_at(s, "skills2"), id, || json!(0));
            clamp_into(s, "skills2", "spec2", id);
        }
    }

    Normalized {
        sheet,
        raised,
        mass_chosen,
    }
}

// ----------------------------------------------------------------- apoios

fn obj(v: &mut Value) -> &mut Map<String, Value> {
    if !v.is_object() {
        *v = Value::Object(Map::new());
    }
    match v {
        Value::Object(m) => m,
        _ => unreachable!(),
    }
}

/// O objeto da chave (o que não é objeto vira `{}`; a fonte falharia).
fn obj_at<'a>(m: &'a mut Map<String, Value>, key: &str) -> &'a mut Map<String, Value> {
    let v = m.entry(key.to_owned()).or_insert_with(|| json!({}));
    obj(v)
}

/// `m[key] ??= make()`: só quando falta ou é `null` (a chave nova entra no
/// fim, a que era `null` fica no lugar).
fn default_with(m: &mut Map<String, Value>, key: &str, make: impl FnOnce() -> Value) {
    if nullish(m.get(key)) {
        m.insert(key.to_owned(), make());
    }
}

fn default_object(m: &mut Map<String, Value>, key: &str) {
    default_with(m, key, || json!({}));
}

/// `(v || []).length` como número: `undefined` (o que não tem tamanho) é
/// `NaN`.
fn length_or_empty(v: Option<&Value>) -> f64 {
    if !truthy(v) {
        return 0.0;
    }
    match v {
        Some(Value::Array(a)) => a.len() as f64,
        Some(Value::String(s)) => s.encode_utf16().count() as f64,
        _ => f64::NAN,
    }
}

fn has_items(v: Option<&Value>) -> bool {
    length_or_empty(v) > 0.0
}

/// `Math.max(a || 0, b || 0)`.
fn max_levels(a: Option<&Value>, b: Option<&Value>) -> f64 {
    js::max(num_or0(a), num_or0(b))
}

// --------------------------------------------------------- as migrações

/// Armas de Uma Mão e de Duas Mãos fundiram em Armas: fica o maior nível, e
/// as especialidades se juntam.
fn merge_weapon_skills(s: &mut Map<String, Value>) {
    let one = "armas-uma-mao";
    let two = "armas-duas-maos";
    let skills = obj_at(s, "skills");
    if nullish(skills.get(one)) && nullish(skills.get(two)) {
        return;
    }
    let level = js::max(
        max_levels(skills.get("armas"), skills.get(one)),
        num_or0(skills.get(two)),
    );
    skills.insert("armas".into(), num(level));
    skills.shift_remove(one);
    skills.shift_remove(two);
    let spec = obj_at(s, "spec");
    let mut all: Vec<Value> = Vec::new();
    for k in ["armas", one, two] {
        if let Some(Value::Array(a)) = spec.get(k).filter(|v| truthy(Some(v))) {
            all.extend(a.iter().cloned());
        }
    }
    if !all.is_empty() {
        spec.insert("armas".into(), Value::Array(all));
    }
    spec.shift_remove(one);
    spec.shift_remove(two);
}

/// Perícias primárias que viraram secundárias.
fn primary_to_secondary(s: &mut Map<String, Value>) {
    for (prim, sec) in PRIMARY_TO_SECONDARY {
        if nullish(obj_at(s, "skills").get(prim)) {
            continue;
        }
        let level = obj_at(s, "skills")[prim].clone();
        let skills2 = obj_at(s, "skills2");
        if nullish(skills2.get(sec)) {
            skills2.insert(sec.into(), level);
        }
        let old = obj_at(s, "spec").get(prim).cloned();
        let spec2 = obj_at(s, "spec2");
        if has_items(old.as_ref()) && !has_items(spec2.get(sec)) {
            spec2.insert(sec.into(), old.unwrap_or(Value::Null));
        }
        obj_at(s, "skills").shift_remove(prim);
        obj_at(s, "spec").shift_remove(prim);
    }
}

/// As Secundárias renomeadas e fundidas: o maior dos dois níveis.
fn secondary_renames(s: &mut Map<String, Value>) {
    for (old, new) in SECONDARY_RENAMES {
        if nullish(obj_at(s, "skills2").get(old)) {
            continue;
        }
        let skills2 = obj_at(s, "skills2");
        let level = max_levels(skills2.get(new), skills2.get(old));
        skills2.insert(new.into(), num(level));
        let spec2 = obj_at(s, "spec2");
        if has_items(spec2.get(old)) && !has_items(spec2.get(new)) {
            let v = spec2[old].clone();
            spec2.insert(new.into(), v);
        }
        obj_at(s, "skills2").shift_remove(old);
        obj_at(s, "spec2").shift_remove(old);
    }
}

/// A Relíquia virou Artefato (nos Antecedentes únicos e nos nomeados).
fn background_renames(s: &mut Map<String, Value>) {
    for group in ["ante", "anteNom"] {
        let g = obj_at(s, group);
        if !nullish(g.get("reliquia")) && nullish(g.get("artefato")) {
            let v = g["reliquia"].clone();
            g.insert("artefato".into(), v);
        }
        g.shift_remove("reliquia");
    }
}

// ------------------------------------------------------------ o equipamento

/// `normSlot`: uma mão preservando o que o jogador ajustou (o nome, os
/// números do item improvisado, o item e o `mod`).
fn normalize_slot(slot: Option<&Value>, default_ref: &str) -> Value {
    let get = |k: &str| slot.and_then(|s| s.get(k));
    let mut o = Map::new();
    let r = match get("ref") {
        Some(v) if truthy(Some(v)) => to_js_string(Some(v)),
        _ => default_ref.to_owned(),
    };
    o.insert("ref".into(), json!(r));
    if !nullish(get("nome")) {
        o.insert("nome".into(), json!(to_js_string(get("nome"))));
    }
    for k in ["dado", "danoBonus", "acerto"] {
        let n = to_number(get(k));
        if n.is_finite() {
            o.insert(k.into(), num(n));
        }
    }
    if let Some(item) = get("item")
        && truthy(Some(item))
        && (item.is_object() || item.is_array())
    {
        o.insert("item".into(), item.clone());
    }
    if let Some(Value::Object(m)) = get("mod") {
        let mut out = Map::new();
        for (k, v) in m {
            let n = to_number(Some(v));
            if n.is_finite() {
                out.insert(k.clone(), num(n));
            }
        }
        if !out.is_empty() {
            o.insert("mod".into(), Value::Object(out));
        }
    }
    Value::Object(o)
}

/// O conjunto das mãos a partir de `equip.arma` e `equip.escudo`, e cada mão
/// normalizada. Tira `equip.armadura`, `arma` e `escudo`.
fn equip_to_sets(s: &mut Map<String, Value>) {
    let equip = obj_at(s, "equip");
    if !matches!(equip.get("armaduras"), Some(Value::Array(_))) {
        let one = equip
            .get("armadura")
            .filter(|a| truthy(Some(a)) && !is_str(Some(a), "nenhuma"));
        let list = one.map_or_else(Vec::new, |a| vec![a.clone()]);
        equip.insert("armaduras".into(), Value::Array(list));
    }
    equip.shift_remove("armadura");
    let sets_ok = matches!(s.get("conjuntos"), Some(Value::Array(a)) if !a.is_empty());
    if !sets_ok {
        let equip = obj_at(s, "equip");
        let weapon = equip
            .get("arma")
            .filter(|a| truthy(Some(a)) && !is_str(Some(a), "desarmado"))
            .map_or_else(
                || "a:desarmado".to_owned(),
                |a| format!("a:{}", to_js_string(Some(a))),
            );
        let shield = equip
            .get("escudo")
            .filter(|a| truthy(Some(a)) && !is_str(Some(a), "nenhum"))
            .map_or_else(
                || "nada".to_owned(),
                |a| format!("e:{}", to_js_string(Some(a))),
            );
        s.insert(
            "conjuntos".into(),
            json!([{"habil": {"ref": weapon}, "inabil": {"ref": shield}, "ativo": true}]),
        );
    }
    let equip = obj_at(s, "equip");
    equip.shift_remove("arma");
    equip.shift_remove("escudo");
    let sets = match s.get("conjuntos") {
        Some(Value::Array(a)) => a.clone(),
        _ => Vec::new(),
    };
    let mut sets: Vec<Value> = sets
        .iter()
        .map(|c| {
            let mut o = Map::new();
            o.insert(
                "habil".into(),
                normalize_slot(c.get("habil"), "a:desarmado"),
            );
            o.insert("inabil".into(), normalize_slot(c.get("inabil"), "nada"));
            o.insert("ativo".into(), json!(truthy(c.get("ativo"))));
            for k in ["uidH", "uidI"] {
                if let Some(v) = c.get(k) {
                    o.insert(k.into(), v.clone());
                }
            }
            Value::Object(o)
        })
        .collect();
    if !sets.iter().any(|c| truthy(c.get("ativo")))
        && let Some(Value::Object(first)) = sets.first_mut()
    {
        first.insert("ativo".into(), json!(true));
    }
    s.insert("conjuntos".into(), Value::Array(sets));
}

/// `baseArmadura(id)`: a armadura do catálogo, ou a personalizada (`c`).
fn has_armor_base(cat: &Catalog, base: Option<&Value>) -> bool {
    match base {
        Some(Value::String(b)) => {
            b == "c" || (!b.is_empty() && cat.armor(b).is_some_and(|a| truthy(Some(a))))
        }
        _ => false,
    }
}

/// As peças de armadura: o texto simples vira peça com `uid`, o `armMod` antigo
/// entra em cada uma, e a peça sem item e sem base no catálogo sai.
fn armor_pieces(cat: &Catalog, s: &mut Map<String, Value>, new_uid: &mut dyn FnMut() -> String) {
    let equip = obj_at(s, "equip");
    let old_mods = match equip.get("armMod") {
        Some(m @ Value::Object(_)) => m.clone(),
        _ => json!({}),
    };
    let list = match equip.get("armaduras") {
        Some(Value::Array(a)) => a.clone(),
        _ => Vec::new(),
    };
    let mut out = Vec::new();
    for p in &list {
        let mut o = Map::new();
        if let Value::String(base) = p {
            o.insert("uid".into(), json!(new_uid()));
            o.insert("base".into(), json!(base));
            if let Some(m) = old_mods.get(base.as_str())
                && truthy(Some(m))
            {
                o.insert("mod".into(), m.clone());
            }
            o.insert("vestida".into(), json!(true));
        } else {
            let g = |k: &str| p.get(k);
            let uid = match g("uid") {
                Some(u) if truthy(Some(u)) => u.clone(),
                _ => json!(new_uid()),
            };
            o.insert("uid".into(), uid);
            if let Some(b) = g("base") {
                o.insert("base".into(), b.clone());
            }
            for k in ["nome", "mod", "item", "img", "enq"] {
                if let Some(v) = g(k).filter(|v| truthy(Some(v))) {
                    o.insert(k.into(), v.clone());
                }
            }
            o.insert(
                "vestida".into(),
                json!(!matches!(g("vestida"), Some(Value::Bool(false)))),
            );
        }
        let has_item = truthy(o.get("item"));
        if has_item || has_armor_base(cat, o.get("base")) {
            out.push(Value::Object(o));
        }
    }
    equip.insert("armaduras".into(), Value::Array(out));
    equip.shift_remove("armMod");
}

/// `assinaturaPeca`: duas peças são a mesma coisa se têm o mesmo item, os
/// mesmos ajustes e o mesmo nome.
fn piece_signature(p: Option<&Value>) -> String {
    let get = |k: &str| p.and_then(|p| p.get(k));
    let text = |v: Option<&Value>| match v {
        Some(v) if !v.is_null() => to_js_string(Some(v)),
        _ => String::new(),
    };
    let json_or_null = |v: Option<&Value>| match v {
        Some(v) if truthy(Some(v)) => serde_json::to_string(v).unwrap_or_default(),
        _ => "null".to_owned(),
    };
    let name = match get("nome") {
        Some(v) if truthy(Some(v)) => to_js_string(Some(v)),
        _ => String::new(),
    };
    [
        text(get("ref")),
        name,
        json_or_null(get("mod")),
        json_or_null(get("item")),
        text(get("dado")),
        text(get("danoBonus")),
        text(get("acerto")),
    ]
    .join("|")
}

/// O arsenal: as peças que já tinham `ref` (cada uma com `uid`), mais as que
/// estavam nas mãos e não estavam nele.
fn build_arsenal(s: &mut Map<String, Value>, new_uid: &mut dyn FnMut() -> String) {
    let mut arsenal: Vec<Value> = match s.get("arsenal") {
        Some(Value::Array(a)) => a
            .iter()
            .filter(|p| truthy(Some(p)) && truthy(p.get("ref")))
            .map(|p| {
                let mut o = match p {
                    Value::Object(m) => m.clone(),
                    _ => Map::new(),
                };
                let uid = match o.get("uid") {
                    Some(u) if truthy(Some(u)) => u.clone(),
                    _ => json!(new_uid()),
                };
                o.insert("uid".into(), uid);
                Value::Object(o)
            })
            .collect(),
        _ => Vec::new(),
    };
    if let Some(Value::Array(sets)) = s.get_mut("conjuntos") {
        for cj in sets.iter_mut() {
            for hand in ["habil", "inabil"] {
                let Some(slot) = cj.get_mut(hand) else {
                    continue;
                };
                if !truthy(Some(slot))
                    || is_str(slot.get("ref"), "nada")
                    || is_str(slot.get("ref"), "a:desarmado")
                {
                    continue;
                }
                let sig = piece_signature(Some(slot));
                let found = arsenal
                    .iter()
                    .find(|p| piece_signature(Some(p)) == sig)
                    .map(|p| p.get("uid").cloned());
                if let Some(uid) = found {
                    if let (Value::Object(m), Some(uid)) = (&mut *slot, uid) {
                        m.insert("uid".into(), uid);
                    }
                    continue;
                }
                let mut new = Map::new();
                let uid = json!(new_uid());
                new.insert("uid".into(), uid.clone());
                for k in ["ref", "nome", "mod", "item", "dado", "danoBonus", "acerto"] {
                    if let Some(v) = slot.get(k) {
                        new.insert(k.into(), v.clone());
                    }
                }
                arsenal.push(Value::Object(new));
                if let Value::Object(m) = slot {
                    m.insert("uid".into(), uid);
                }
            }
        }
    }
    s.insert("arsenal".into(), Value::Array(arsenal));
}

// ----------------------------------------------------------------- bolsas

fn blank_row() -> Value {
    json!({"item": "", "peso": "", "preco": ""})
}

/// `normBolsa`: uma bolsa, e o formato antigo (um `texto` corrido) vira tabela.
fn normalize_bag(b: &Value, default_name: &str) -> Value {
    let text = |v: Option<&Value>, d: &str| match v {
        Some(v) if !v.is_null() => to_js_string(Some(v)),
        _ => d.to_owned(),
    };
    let mut name = text(b.get("nome"), default_name);
    if name == "Carroça" {
        name = "Baú".to_owned();
    }
    let mut rows: Vec<Value> = match b.get("itens") {
        Some(Value::Array(a)) => a
            .iter()
            .map(|l| {
                json!({
                    "item": text(l.get("item"), ""),
                    "peso": text(l.get("peso"), ""),
                    "preco": text(l.get("preco"), ""),
                })
            })
            .collect(),
        _ => text(b.get("texto"), "")
            .split('\n')
            .map(|l| l.trim_matches(super::value::is_js_space))
            .filter(|l| !l.is_empty())
            .map(|l| json!({"item": l, "peso": "", "preco": ""}))
            .collect(),
    };
    while rows.len() < BAG_ROWS {
        rows.push(blank_row());
    }
    json!({"nome": name, "itens": rows, "pesoTotal": text(b.get("pesoTotal"), "")})
}

/// As bolsas: as três de sempre, ou as que a ficha tem, normalizadas.
fn bags(s: &mut Map<String, Value>) {
    let names = ["Mochila", "Equipamentos", "Baú"];
    let list = match s.get("bolsas") {
        Some(Value::Array(a)) if !a.is_empty() => a
            .iter()
            .enumerate()
            .map(|(i, b)| normalize_bag(b, names.get(i).copied().unwrap_or("Bolsa")))
            .collect(),
        _ => names
            .iter()
            .map(|n| json!({"nome": n, "itens": (0..BAG_ROWS).map(|_| blank_row()).collect::<Vec<_>>()}))
            .collect(),
    };
    s.insert("bolsas".into(), Value::Array(list));
}

// ------------------------------------------------------- as especialidades

/// `S[spec][id] = clampSpecs(S[spec][id], S[skills][id] || 0)`.
fn clamp_into(s: &mut Map<String, Value>, skills: &str, spec: &str, id: &str) {
    let level = num_or0(obj_at(s, skills).get(id));
    let list = obj_at(s, spec).get(id).cloned().unwrap_or(Value::Null);
    let clamped: Vec<Value> = clamp_specialties(&list, level)
        .into_iter()
        .map(|e| json!({"s": e.name, "v": num(e.level)}))
        .collect();
    obj_at(s, spec).insert(id.to_owned(), Value::Array(clamped));
}
