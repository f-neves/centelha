//! O equipamento da ficha: o catálogo achatado, a peça e o item inteiro que a
//! ficha guarda (port de `equip.ts`).
//!
//! A ficha guarda cada peça (do arsenal, do slot de mão, de armadura) com uma
//! cópia do item do catálogo em `item`, no envelope aninhado de `armas.json`
//! (`{ origem, nome, tipo, ..., arma: {...} }`). A peça gravada no modelo
//! antigo (`ref` mais `mod`/`ajuste`, e o `armMod` das armaduras) é lida pela
//! conta de antes e vira item inteiro na leitura (`freeze_items`).

use std::collections::HashMap;

use serde_json::{Map, Value, json};

use super::value::{
    coalesce, get, is_object_like, is_str, nullish, num, property_key, set, spread_into, to_number,
    truthy,
};
use crate::js;

/// O tipo de uma peça.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Weapon,
    Shield,
    Armor,
}

impl ItemKind {
    /// O nome do bloco de números no envelope (`arma`, `escudo`, `armadura`).
    pub fn block(self) -> &'static str {
        match self {
            ItemKind::Weapon => "arma",
            ItemKind::Shield => "escudo",
            ItemKind::Armor => "armadura",
        }
    }
}

/// O tipo e a origem de uma peça (`tipoDaPeca`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PieceKind {
    pub kind: ItemKind,
    /// O id do catálogo de onde veio; `None` na peça personalizada.
    pub origin: Option<String>,
    /// A peça personalizada (`c`, `ce`, a armadura `c`).
    pub custom: bool,
}

/// O id da armadura personalizada e o `ref` da arma personalizada.
pub const CUSTOM_ARMOR: &str = "c";
/// O `ref` da arma personalizada.
pub const CUSTOM_WEAPON: &str = "c";
/// O `ref` do escudo personalizado.
pub const CUSTOM_SHIELD: &str = "ce";

/// O catálogo de equipamento como a ficha o lê: o envelope de cada peça e a
/// versão achatada (o bloco de números junto da raiz).
#[derive(Debug, Clone)]
pub struct Catalog {
    /// `ARMA`: as armas achatadas por id (todas, inclusive as só de criatura),
    /// na ordem do JavaScript (`Object.values`).
    weapons: Vec<(String, Value)>,
    weapon_index: HashMap<String, usize>,
    armors: HashMap<String, Value>,
    shields: HashMap<String, Value>,
    /// `BRUTO`: os envelopes como estão no JSON, por tipo e id.
    raw: [HashMap<String, Value>; 3],
}

/// `achata`: o bloco de números junto da raiz, com `notas` repetindo a
/// `descricao`.
fn flatten_catalog(item: &Value) -> Value {
    let block = ["arma", "armadura", "escudo", "municao"]
        .iter()
        .map(|k| get(Some(item), k))
        .find(|b| !nullish(*b))
        .flatten();
    let mut out = Map::new();
    if let Some(Value::Object(m)) = Some(item) {
        for k in super::value::keys(m) {
            if !matches!(k.as_str(), "arma" | "armadura" | "escudo" | "municao") {
                out.insert(k.clone(), m[k.as_str()].clone());
            }
        }
    }
    spread_into(&mut out, block);
    set(&mut out, "notas", get(Some(item), "descricao").cloned());
    Value::Object(out)
}

/// `Object.fromEntries(lista.map((x) => [x.id, x]))`: o id repetido fica com
/// o último valor, na posição do primeiro.
fn by_id(
    list: &[Value],
    f: impl Fn(&Value) -> Value,
) -> (Vec<(String, Value)>, HashMap<String, usize>) {
    let mut order: Vec<(String, Value)> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for x in list {
        let id = property_key(get(Some(x), "id"));
        let v = f(x);
        match index.get(&id) {
            Some(&i) => order[i].1 = v,
            None => {
                index.insert(id.clone(), order.len());
                order.push((id, v));
            }
        }
    }
    (order, index)
}

impl Catalog {
    /// Monta o catálogo a partir dos três arquivos (`armas.json`,
    /// `armaduras.json`, `escudos.json`), já lidos como JSON. Cada um precisa
    /// ser uma lista.
    pub fn new(
        weapons: &Value,
        armors: &Value,
        shields: &Value,
    ) -> Result<Self, super::SheetError> {
        let list = |v: &Value, file: &'static str| {
            v.as_array()
                .cloned()
                .ok_or(super::SheetError::CatalogNotList(file))
        };
        let (w, a, s) = (
            list(weapons, "armas.json")?,
            list(armors, "armaduras.json")?,
            list(shields, "escudos.json")?,
        );
        let (weapons_flat, weapon_index) = by_id(&w, flatten_catalog);
        let map = |l: &[Value], f: fn(&Value) -> Value| by_id(l, f).0.into_iter().collect();
        Ok(Catalog {
            weapons: weapons_flat,
            weapon_index,
            armors: map(&a, flatten_catalog),
            shields: map(&s, flatten_catalog),
            raw: [
                map(&w, Value::clone),
                map(&s, Value::clone),
                map(&a, Value::clone),
            ],
        })
    }

    /// Monta o catálogo a partir do texto dos três arquivos.
    pub fn from_json(
        weapons: &str,
        armors: &str,
        shields: &str,
    ) -> Result<Self, super::SheetError> {
        let parse = |t: &str, file: &'static str| {
            serde_json::from_str::<Value>(t).map_err(|e| super::SheetError::CatalogJson {
                file,
                reason: e.to_string(),
            })
        };
        Catalog::new(
            &parse(weapons, "armas.json")?,
            &parse(armors, "armaduras.json")?,
            &parse(shields, "escudos.json")?,
        )
    }

    /// A arma achatada pelo id (`ARMA[id]`).
    pub fn weapon(&self, id: &str) -> Option<&Value> {
        self.weapon_index.get(id).map(|&i| &self.weapons[i].1)
    }

    /// A armadura achatada pelo id (`ARMADURA[id]`).
    pub fn armor(&self, id: &str) -> Option<&Value> {
        self.armors.get(id)
    }

    /// O escudo achatado pelo id (`ESCUDO[id]`).
    pub fn shield(&self, id: &str) -> Option<&Value> {
        self.shields.get(id)
    }

    /// `armaDoCatalogo`: a arma pelo id, ou pela primeira com esse nome.
    pub fn weapon_by_id_or_name(&self, key: &str) -> Option<&Value> {
        if key.is_empty() {
            return None;
        }
        self.weapon(key).filter(|w| truthy(Some(w))).or_else(|| {
            self.weapons
                .iter()
                .map(|(_, w)| w)
                .find(|w| is_str(get(Some(w), "nome"), key))
        })
    }

    /// `armaDoCatalogo` com a chave como valor do JavaScript: nada quando ela
    /// é falsa; a arma pelo id, ou a primeira com esse nome (`===`).
    pub(crate) fn weapon_lookup(&self, key: Option<&Value>) -> Option<&Value> {
        if !truthy(key) {
            return None;
        }
        self.weapon(&property_key(key))
            .filter(|w| truthy(Some(w)))
            .or_else(|| {
                self.weapons
                    .iter()
                    .map(|(_, w)| w)
                    .find(|w| super::value::strict_eq(get(Some(w), "nome"), key))
            })
    }

    /// Os punhos (`ARMA['desarmado']`).
    pub(crate) fn unarmed(&self) -> Option<&Value> {
        self.weapon("desarmado")
    }

    fn raw(&self, kind: ItemKind, id: &str) -> Option<&Value> {
        let i = match kind {
            ItemKind::Weapon => 0,
            ItemKind::Shield => 1,
            ItemKind::Armor => 2,
        };
        self.raw[i].get(id)
    }
}

// ----- as peças personalizadas -----

/// `ARMADURA_LIVRE`.
pub(crate) fn custom_armor_base() -> Value {
    json!({
        "id": CUSTOM_ARMOR,
        "nome": "Armadura personalizada",
        "classe": "nenhuma",
        "soak": { "impacto": 0, "corte": 0, "perfuracao": 0 },
        "resistPerf": 0,
        "penalidade": 0,
        "acesso": 10,
        "notas": "Peça fora do catálogo: vale o que estiver no ajuste.",
    })
}

/// `ARMA_LIVRE`.
pub(crate) fn custom_weapon_base() -> Value {
    json!({
        "nome": "Arma personalizada", "dado": 1, "danoBonus": 0, "acerto": -2, "defesaArma": 0,
        "maos": 1, "ticks": 5, "atrib": "forca", "pericia": "briga", "tags": [],
        "tipoDano": "impacto", "forcaMult": 1,
    })
}

/// `ESCUDO_LIVRE`.
pub(crate) fn custom_shield_base() -> Value {
    json!({
        "nome": "Escudo personalizado", "bloqCaC": 0, "penalidade": 0,
        "vsProjetilRapido": { "bloqueia": false, "bonus": 0 },
    })
}

fn custom_base(kind: ItemKind) -> Value {
    match kind {
        ItemKind::Weapon => custom_weapon_base(),
        ItemKind::Shield => custom_shield_base(),
        ItemKind::Armor => custom_armor_base(),
    }
}

// ----- os campos editáveis do modelo antigo -----

/// Um campo numérico editável: a chave e o intervalo.
struct Field {
    key: &'static str,
    min: f64,
    max: f64,
}

const WEAPON_FIELDS: [Field; 5] = [
    Field {
        key: "ticks",
        min: 1.0,
        max: 20.0,
    },
    Field {
        key: "acerto",
        min: -10.0,
        max: 10.0,
    },
    Field {
        key: "dado",
        min: 1.0,
        max: 6.0,
    },
    Field {
        key: "danoBonus",
        min: -10.0,
        max: 10.0,
    },
    Field {
        key: "defesaArma",
        min: -5.0,
        max: 10.0,
    },
];
const SHIELD_FIELDS: [Field; 2] = [
    Field {
        key: "bloqCaC",
        min: -5.0,
        max: 10.0,
    },
    Field {
        key: "penalidade",
        min: 0.0,
        max: 10.0,
    },
];
const ARMOR_FIELDS: [Field; 5] = [
    Field {
        key: "impacto",
        min: 0.0,
        max: 20.0,
    },
    Field {
        key: "corte",
        min: 0.0,
        max: 20.0,
    },
    Field {
        key: "perfuracao",
        min: 0.0,
        max: 20.0,
    },
    Field {
        key: "resistPerf",
        min: 0.0,
        max: 6.0,
    },
    Field {
        key: "penalidade",
        min: 0.0,
        max: 10.0,
    },
];

fn fields(kind: ItemKind) -> &'static [Field] {
    match kind {
        ItemKind::Weapon => &WEAPON_FIELDS,
        ItemKind::Shield => &SHIELD_FIELDS,
        ItemKind::Armor => &ARMOR_FIELDS,
    }
}

fn is_soak(k: &str) -> bool {
    matches!(k, "impacto" | "corte" | "perfuracao")
}

/// `num`: o número de um ajuste, ou nada (vazio, nulo, não finito).
fn adjustment_number(v: Option<&Value>) -> Option<f64> {
    if nullish(v) || is_str(v, "") {
        return None;
    }
    let n = to_number(v);
    n.is_finite().then_some(n)
}

/// `baseCampo` (só os campos numéricos): o valor do catálogo, `?? 0`.
fn base_field<'a>(base: Option<&'a Value>, k: &str) -> Option<&'a Value> {
    let soak = get(base, "soak");
    let v = if truthy(soak) && is_soak(k) {
        get(soak, k)
    } else {
        get(base, k)
    };
    if nullish(v) { Some(zero_ref()) } else { v }
}

fn zero_ref() -> &'static Value {
    static Z: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    Z.get_or_init(|| json!(0))
}

/// `valorCampo`: o ajuste do jogador (arredondado e no intervalo), ou o do
/// catálogo.
fn field_value(base: Option<&Value>, adj: Option<&Value>, f: &Field) -> f64 {
    match adjustment_number(get(adj, f.key)) {
        None => {
            let b = base_field(base, f.key);
            if truthy(b) { to_number(b) } else { 0.0 }
        }
        Some(v) => js::max(f.min, js::min(f.max, js::round(v))),
    }
}

// ----- o item inteiro -----

/// `tipoDaPeca`: o tipo e a origem de uma peça, pelo `ref` (arma e escudo) ou
/// pelo `base` (armadura).
pub fn piece_kind(p: Option<&Value>) -> Option<PieceKind> {
    let Some(Value::Object(_) | Value::Array(_)) = p else {
        return None;
    };
    if let Some(Value::String(r)) = get(p, "ref")
        && !r.is_empty()
    {
        let custom = |kind| {
            Some(PieceKind {
                kind,
                origin: None,
                custom: true,
            })
        };
        let from = |kind, o: &str| {
            Some(PieceKind {
                kind,
                origin: Some(o.to_owned()),
                custom: false,
            })
        };
        return if r == CUSTOM_WEAPON {
            custom(ItemKind::Weapon)
        } else if r == CUSTOM_SHIELD {
            custom(ItemKind::Shield)
        } else if let Some(o) = r.strip_prefix("a:") {
            if r == "a:desarmado" {
                None
            } else {
                from(ItemKind::Weapon, o)
            }
        } else if let Some(o) = r.strip_prefix("e:") {
            from(ItemKind::Shield, o)
        } else {
            None
        };
    }
    if let Some(Value::String(b)) = get(p, "base") {
        return Some(if b == CUSTOM_ARMOR {
            PieceKind {
                kind: ItemKind::Armor,
                origin: None,
                custom: true,
            }
        } else {
            PieceKind {
                kind: ItemKind::Armor,
                origin: Some(b.clone()),
                custom: false,
            }
        });
    }
    None
}

/// `semFolego`: o Fôlego não entra no item que a ficha guarda.
pub(super) fn without_breath(item: &mut Value) {
    if let Some(Value::Object(a)) = item.get_mut("arma") {
        a.shift_remove("folego");
    }
}

impl Catalog {
    /// `copiaDoCatalogo`: o envelope do catálogo, com `origem` no lugar do
    /// `id` e sem os blocos vazios.
    pub fn catalog_copy(&self, kind: ItemKind, origin: &str) -> Option<Value> {
        let raw = self.raw(kind, origin)?;
        if !truthy(Some(raw)) {
            return None;
        }
        let mut rest = Map::new();
        if let Value::Object(m) = raw {
            for k in super::value::keys(m) {
                if k != "id" {
                    rest.insert(k.clone(), m[k.as_str()].clone());
                }
            }
        }
        for k in ["arma", "escudo", "armadura", "municao"] {
            if nullish(rest.get(k)) {
                rest.shift_remove(k);
            }
        }
        let mut rest = Value::Object(rest);
        without_breath(&mut rest);
        let mut out = Map::new();
        set(&mut out, "origem", get(Some(raw), "id").cloned());
        spread_into(&mut out, Some(&rest));
        Some(Value::Object(out))
    }
}

/// `envelopeLivre`: o envelope da peça personalizada, a partir dos números
/// achatados.
fn custom_envelope(kind: ItemKind, flat: &Value) -> Value {
    let mut block = Map::new();
    if let Value::Object(m) = flat {
        for k in super::value::keys(m) {
            if !matches!(k.as_str(), "nome" | "tags" | "notas" | "id") {
                block.insert(k.clone(), m[k.as_str()].clone());
            }
        }
    }
    let mut out = Map::new();
    out.insert("origem".into(), Value::Null);
    set(&mut out, "nome", get(Some(flat), "nome").cloned());
    out.insert("tipo".into(), json!(kind.block()));
    let tags = get(Some(flat), "tags");
    if truthy(tags) {
        let list = match tags {
            Some(Value::Array(a)) => a.clone(),
            Some(Value::String(s)) => s.chars().map(|c| json!(c.to_string())).collect(),
            _ => Vec::new(),
        };
        out.insert("tags".into(), Value::Array(list));
    }
    out.insert(kind.block().into(), Value::Object(block));
    Value::Object(out)
}

/// `blocoDe`: o bloco de números de um envelope.
fn block_key(item: &Value) -> Option<String> {
    let tipo = property_key(get(Some(item), "tipo"));
    for k in [tipo.as_str(), "arma", "escudo", "armadura"] {
        if !nullish(get(Some(item), k)) {
            return Some(k.to_owned());
        }
    }
    None
}

/// `itemAchatado`: o item no formato dos leitores, com `id` repetindo a origem.
pub fn flatten_item(item: Option<&Value>) -> Option<Value> {
    let item = item.filter(|i| is_object_like(Some(i)))?;
    let block = block_key(item).and_then(|k| get(Some(item), &k).cloned());
    let block = if truthy(block.as_ref()) { block } else { None };
    let mut out = Map::new();
    if let Value::Object(m) = item {
        for k in super::value::keys(m) {
            if !matches!(
                k.as_str(),
                "arma" | "escudo" | "armadura" | "municao" | "origem"
            ) {
                out.insert(k.clone(), m[k.as_str()].clone());
            }
        }
    } else {
        spread_into(&mut out, Some(item));
    }
    spread_into(&mut out, block.as_ref());
    let origin = get(Some(item), "origem");
    if truthy(origin) {
        out.insert("id".into(), origin.cloned().unwrap_or(Value::Null));
    }
    out.insert(
        "origem".into(),
        coalesce(origin, None).cloned().unwrap_or(Value::Null),
    );
    set(&mut out, "notas", get(Some(item), "descricao").cloned());
    Some(Value::Object(out))
}

/// `escreveCampo` para os campos numéricos (a armadura guarda as Absorções em
/// `soak`).
fn write_field(item: &mut Value, k: &str, v: f64) {
    let is_armor = is_str(get(Some(item), "tipo"), "armadura");
    let Some(bk) = block_key(item) else { return };
    let Some(block) = item.get_mut(&bk) else {
        return;
    };
    if !truthy(Some(block)) {
        return;
    }
    let Value::Object(b) = block else { return };
    if is_soak(k) && is_armor {
        let soak = b.entry("soak").or_insert(Value::Null);
        if !truthy(Some(soak)) {
            *soak = Value::Object(Map::new());
        }
        if let Value::Object(s) = soak {
            s.insert(k.into(), num(v));
        }
        return;
    }
    b.insert(k.into(), num(v));
}

/// A chave do ajuste: `mod` na ficha de leitura, `ajuste` na gravada.
#[derive(Debug, Clone, Copy)]
pub(crate) enum AdjustKey {
    Mod,
    Ajuste,
}

impl AdjustKey {
    fn key(self) -> &'static str {
        match self {
            AdjustKey::Mod => "mod",
            AdjustKey::Ajuste => "ajuste",
        }
    }
}

impl Catalog {
    /// `materializa`: o item de uma peça gravada no modelo antigo, o catálogo
    /// de hoje mais o ajuste, com os limites de antes.
    pub(crate) fn materialize(
        &self,
        p: Option<&Value>,
        key: AdjustKey,
        extra: Option<&Value>,
    ) -> Option<Value> {
        let t = piece_kind(p)?;
        let adj = coalesce(extra, get(p, key.key())).cloned();
        let (mut item, base) = if t.custom {
            let base = if t.kind == ItemKind::Weapon {
                let clamp = |v: Option<&Value>, lo: f64, hi: f64, d: f64| {
                    let n = to_number(v);
                    if n.is_finite() {
                        js::max(lo, js::min(hi, js::round(n)))
                    } else {
                        d
                    }
                };
                let mut b = custom_weapon_base();
                if let Value::Object(m) = &mut b {
                    m.insert("dado".into(), num(clamp(get(p, "dado"), 1.0, 2.0, 1.0)));
                    m.insert(
                        "danoBonus".into(),
                        num(clamp(get(p, "danoBonus"), -2.0, 2.0, 0.0)),
                    );
                    m.insert(
                        "acerto".into(),
                        num(clamp(get(p, "acerto"), -6.0, 6.0, -2.0)),
                    );
                }
                b
            } else {
                custom_base(t.kind)
            };
            (custom_envelope(t.kind, &base), base)
        } else {
            let item = self.catalog_copy(t.kind, t.origin.as_deref()?)?;
            let base = flatten_item(Some(&item))?;
            (item, base)
        };
        if is_object_like(adj.as_ref()) {
            for f in fields(t.kind) {
                if adjustment_number(get(adj.as_ref(), f.key)).is_some() {
                    let v = field_value(Some(&base), adj.as_ref(), f);
                    write_field(&mut item, f.key, v);
                }
            }
        }
        Some(item)
    }

    /// `itemDaPeca`: o item em uso de uma peça, achatado (a cópia guardada; na
    /// peça antiga, a conta de antes).
    pub fn piece_item(&self, p: Option<&Value>) -> Option<Value> {
        let item = get(p, "item");
        if truthy(item) && is_object_like(item) {
            return flatten_item(item);
        }
        let item = self.materialize(p, AdjustKey::Mod, None)?;
        flatten_item(Some(&item))
    }

    /// `origemDaPeca`: o item do catálogo de onde a peça veio, achatado.
    pub fn piece_origin(&self, p: Option<&Value>) -> Option<Value> {
        let t = piece_kind(p)?;
        if t.custom {
            return flatten_item(Some(&custom_envelope(t.kind, &custom_base(t.kind))));
        }
        let saved = get(get(p, "item"), "origem");
        let origin = if nullish(saved) {
            t.origin.clone().unwrap_or_default()
        } else {
            property_key(saved)
        };
        let c = self.catalog_copy(t.kind, &origin)?;
        flatten_item(Some(&c))
    }

    /// `congelarItens`: dá item inteiro a toda peça que ainda não tem.
    /// `saved`: a ficha no formato gravado (`equipamento`, `maoHabil`,
    /// `ajuste`); senão, no de leitura (`arsenal`, `habil`, `mod`, `armMod`).
    pub(crate) fn freeze_items(&self, s: &mut Value, saved: bool) {
        let (Value::Object(_) | Value::Array(_)) = s else {
            return;
        };
        let key = if saved {
            AdjustKey::Ajuste
        } else {
            AdjustKey::Mod
        };
        let fix = |p: &mut Value, extra: Option<&Value>| {
            if !truthy(Some(p)) || !is_object_like(Some(p)) {
                return;
            }
            let item = get(Some(p), "item");
            if truthy(item) && is_object_like(item) {
                if let Some(i) = p.get_mut("item") {
                    without_breath(i);
                }
                return;
            }
            let Some(item) = self.materialize(Some(p), key, extra) else {
                return;
            };
            if let Value::Object(m) = p {
                m.insert("item".into(), item);
                m.shift_remove(key.key());
                if is_str(m.get("ref"), CUSTOM_WEAPON) {
                    for k in ["dado", "danoBonus", "acerto"] {
                        m.shift_remove(k);
                    }
                }
            }
        };
        let hands: [&str; 2] = if saved {
            ["maoHabil", "maoInabil"]
        } else {
            ["habil", "inabil"]
        };
        let eq_key = if saved { "equipamento" } else { "equip" };

        // o arsenal e os conjuntos
        let lists: [(&str, bool); 2] = [("arsenal", false), ("conjuntos", true)];
        for (name, is_sets) in lists {
            let list = if saved {
                s.get_mut(eq_key).and_then(|e| e.get_mut(name))
            } else {
                s.get_mut(name)
            };
            let Some(Value::Array(list)) = list else {
                continue;
            };
            for p in list.iter_mut() {
                if is_sets {
                    for h in hands {
                        if let Some(slot) = p.get_mut(h) {
                            fix(slot, None);
                        }
                    }
                } else {
                    fix(p, None);
                }
            }
        }

        // as armaduras
        let Some(eq) = s.get_mut(eq_key) else { return };
        if !truthy(Some(eq)) {
            return;
        }
        let Some(Value::Array(armors)) = eq.get("armaduras").cloned() else {
            return;
        };
        let arm_mod = eq.get("armMod").cloned();
        let mods = if !saved && truthy(arm_mod.as_ref()) && is_object_like(arm_mod.as_ref()) {
            arm_mod.clone()
        } else {
            None
        };
        let new_list: Vec<Value> = armors
            .into_iter()
            .map(|mut p| {
                if let Value::String(id) = &p {
                    let mut o = json!({ "base": id, "vestida": true });
                    fix(&mut o, get(mods.as_ref(), id));
                    if o.get("item").is_some() { o } else { p }
                } else {
                    fix(&mut p, None);
                    p
                }
            })
            .collect();
        let any_string = new_list.iter().any(Value::is_string);
        if let Value::Object(e) = eq {
            e.insert("armaduras".into(), Value::Array(new_list));
            if !saved && truthy(arm_mod.as_ref()) && !any_string {
                e.shift_remove("armMod");
            }
        }
    }

    /// `armadurasDe`: as peças de armadura vestidas, cada uma pelo item dela
    /// (achatado). Aceita os formatos antigos.
    pub fn worn_armor(&self, sheet: &Value) -> Vec<Value> {
        self.worn_armor_read(&super::format::for_reading(self, sheet))
    }

    /// `armadurasDe` da ficha já lida (`for_reading`), sem ler de novo.
    pub fn worn_armor_read(&self, s: &Value) -> Vec<Value> {
        let equip = get(Some(s), "equip");
        let list = super::value::or(get(equip, "armaduras"), None);
        let mods = get(equip, "armMod");
        let Some(Value::Array(list)) = list else {
            return Vec::new();
        };
        list.iter()
            .filter_map(|p| {
                if let Value::String(id) = p {
                    let it = self.materialize(
                        Some(&json!({ "base": id })),
                        AdjustKey::Mod,
                        get(mods, id),
                    )?;
                    return flatten_item(Some(&it));
                }
                if !truthy(Some(p)) || matches!(get(Some(p), "vestida"), Some(Value::Bool(false))) {
                    return None;
                }
                let mut a = self.piece_item(Some(p))?;
                let name = get(Some(p), "nome");
                if truthy(name)
                    && let Value::Object(m) = &mut a
                {
                    m.insert("nome".into(), name.cloned().unwrap_or(Value::Null));
                }
                Some(a)
            })
            .filter(|a| truthy(Some(a)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn piece_kinds() {
        let k = |v: Value| piece_kind(Some(&v));
        assert_eq!(k(json!({"ref": "a:desarmado"})), None);
        assert_eq!(k(json!({"ref": "nada"})), None);
        assert_eq!(
            k(json!({"ref": "e:broquel"})),
            Some(PieceKind {
                kind: ItemKind::Shield,
                origin: Some("broquel".into()),
                custom: false
            })
        );
        assert_eq!(
            k(json!({"ref": "", "base": "c"})),
            Some(PieceKind {
                kind: ItemKind::Armor,
                origin: None,
                custom: true
            })
        );
        assert_eq!(k(json!("gambeson")), None);
    }
}
