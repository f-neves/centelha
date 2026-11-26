//! As contas da ficha interativa (port de `src/lib/ficha-engine.ts`): a ficha
//! nova, o XP gasto com as parcelas, os tetos e os pisos de cada trilha, a
//! troca de Raça, o recorte das especialidades, o estado das Técnicas e os
//! Efeitos ao alcance.
//!
//! A ficha entra e sai como `serde_json::Value`, na forma de leitura (a que o
//! `for_reading` devolve, depois das migrações da ficha salva). As migrações
//! do `normalize()` da fonte ficam de fora (esperam a função pura das regras).

use std::cmp::Ordering;
use std::collections::HashMap;

use centelha_data::{Art, Effect, Technique};
use serde_json::{Map, Value, json};

use super::SheetTables;
use super::numbers::Movement;
use super::value::{self as jv, num, truthy};
use crate::character::{art_cost, effect_cost, feat_cost, specialty_cost, xp_cost, xp_floor};
use crate::js;

/// As três Secundárias em branco do fim da lista (`LIVRES`).
pub const FREE_SECONDARY: [&str; 3] = ["livre-1", "livre-2", "livre-3"];

/// O orçamento de XP da ficha nova (`fresh()`).
pub const FRESH_BUDGET: f64 = 1500.0;

/// As tabelas da ficha interativa: as da ficha e os catálogos dos traços.
pub struct EditorTables {
    sheet: SheetTables,
    attributes: Vec<String>,
    /// O nome de cada Atributo (`a.nome || a.id`), na ordem de `attributes`.
    attribute_names: Vec<String>,
    skills: Vec<String>,
    secondary: Vec<String>,
    virtues: Vec<String>,
    single_backgrounds: Vec<String>,
    named_backgrounds: Vec<String>,
    techniques: Vec<Technique>,
    arts: Vec<Art>,
    effects: Vec<Effect>,
}

fn ids(v: &Value, keep: impl Fn(&Value) -> bool) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter(|x| keep(x))
                .filter_map(|x| x.get("id").and_then(Value::as_str).map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// O nome de cada item (`nome || id`), na ordem do arquivo.
fn names(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| {
                    let id = x.get("id").and_then(Value::as_str)?;
                    let name = x
                        .get("nome")
                        .and_then(Value::as_str)
                        .filter(|n| !n.is_empty());
                    Some(name.unwrap_or(id).to_owned())
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Os catálogos crus dos traços, para [`EditorTables::new`]: o
/// `atributos.json`, o `habilidades.json`, o `habilidades-secundarias.json`, o
/// `virtudes.json` e o `antecedentes.json`, e as Técnicas, as Artes e os Efeitos
/// já lidos. O id de cada Secundária é o da ficha (o `slug` do nome, na
/// fonte; hoje os dois coincidem).
pub struct EditorCatalogs<'a> {
    pub attributes: &'a Value,
    pub skills: &'a Value,
    pub secondary: &'a Value,
    pub virtues: &'a Value,
    pub backgrounds: &'a Value,
    pub techniques: Vec<Technique>,
    pub arts: Vec<Art>,
    pub effects: Vec<Effect>,
}

impl EditorTables {
    pub fn new(sheet: SheetTables, c: EditorCatalogs<'_>) -> Self {
        EditorTables {
            sheet,
            attributes: ids(c.attributes, |_| true),
            attribute_names: names(c.attributes),
            skills: ids(c.skills, |h| !truthy(h.get("secundaria"))),
            secondary: ids(c.secondary, |_| true),
            virtues: ids(c.virtues, |_| true),
            single_backgrounds: ids(c.backgrounds, |a| jv::is_str(a.get("formato"), "unico")),
            named_backgrounds: ids(c.backgrounds, |a| jv::is_str(a.get("formato"), "nomeado")),
            techniques: c.techniques,
            arts: c.arts,
            effects: c.effects,
        }
    }

    pub fn sheet_tables(&self) -> &SheetTables {
        &self.sheet
    }

    /// Os ids dos Atributos e os nomes (`a.nome || a.id`), na ordem do arquivo.
    pub(super) fn attributes(&self) -> impl Iterator<Item = (&str, &str)> {
        self.attributes
            .iter()
            .zip(&self.attribute_names)
            .map(|(a, n)| (a.as_str(), n.as_str()))
    }

    /// Os ids das Habilidades, das Secundárias, das Virtudes e dos
    /// Antecedentes nomeados.
    pub(super) fn skill_ids(&self) -> &[String] {
        &self.skills
    }

    pub(super) fn secondary_ids(&self) -> &[String] {
        &self.secondary
    }

    pub(super) fn virtue_ids(&self) -> &[String] {
        &self.virtues
    }

    pub(super) fn named_background_ids(&self) -> &[String] {
        &self.named_backgrounds
    }

    pub(super) fn floor(&self, key: &str) -> f64 {
        xp_floor(self.sheet.rules(), key).unwrap_or(0.0)
    }

    fn cost(&self, key: &str, from: Option<f64>, to: f64) -> f64 {
        xp_cost(self.sheet.rules(), key, from, to).unwrap_or(0.0)
    }

    /// `RACA[S.raca]?.atributos?.[key] || 0`.
    fn racial(&self, sheet: &Value, key: &str) -> f64 {
        self.sheet
            .ancestry_of(sheet.get("raca").unwrap_or(&Value::Null))
            .and_then(|a| a.attributes.get(key).copied())
            .map_or(0.0, or0)
    }
}

// ------------------------------------------------------------ a ficha nova

/// `fresh()`: a ficha nova, no piso de cada trilha (custo zero), Raça humana,
/// orçamento de 1500 XP. As chaves na ordem da fonte.
pub fn fresh_sheet(t: &EditorTables) -> Value {
    let named = |keys: &mut dyn Iterator<Item = &str>, v: &Value| -> Value {
        Value::Object(keys.map(|k| (k.to_owned(), v.clone())).collect())
    };
    let human = json!({"raca": "humano"});
    let rows = || {
        Value::Array(
            (0..6)
                .map(|_| json!({"item": "", "peso": "", "preco": ""}))
                .collect(),
        )
    };
    let secondary = || t.secondary.iter().map(String::as_str).chain(FREE_SECONDARY);
    let mut s = Map::new();
    s.insert("id".into(), json!({}));
    s.insert("corpo".into(), json!({}));
    s.insert(
        "attrs".into(),
        Value::Object(
            t.attributes
                .iter()
                .map(|a| (a.clone(), num(attribute_floor(t, &human, a))))
                .collect(),
        ),
    );
    s.insert(
        "skills".into(),
        named(
            &mut t.skills.iter().map(String::as_str),
            &num(t.floor("habilidadePrimaria")),
        ),
    );
    s.insert(
        "spec".into(),
        named(&mut t.skills.iter().map(String::as_str), &json!([])),
    );
    s.insert("skills2".into(), named(&mut secondary(), &json!(0)));
    s.insert("spec2".into(), named(&mut secondary(), &json!([])));
    s.insert(
        "virtues".into(),
        named(
            &mut t.virtues.iter().map(String::as_str),
            &num(t.floor("virtude")),
        ),
    );
    s.insert("willpower".into(), num(t.floor("vontade")));
    s.insert("aparencia".into(), num(t.floor("aparencia")));
    s.insert("centelha".into(), json!(0));
    s.insert("raca".into(), json!("humano"));
    for k in ["tech", "arte", "efeito", "ante", "anteNom"] {
        s.insert(k.into(), json!({}));
    }
    s.insert("budget".into(), num(FRESH_BUDGET));
    s.insert("equip".into(), json!({"armaduras": []}));
    s.insert("arsenal".into(), json!([]));
    s.insert(
        "conjuntos".into(),
        json!([{"habil": {"ref": "a:desarmado"}, "inabil": {"ref": "nada"}, "ativo": true}]),
    );
    s.insert(
        "bolsas".into(),
        json!([
            {"nome": "Mochila", "itens": rows()},
            {"nome": "Equipamentos", "itens": rows()},
            {"nome": "Baú", "itens": rows()},
        ]),
    );
    s.insert(
        "defSpec".into(),
        json!({"esquiva": [], "bloqueio": [], "social": [], "mental": []}),
    );
    s.insert("livreNome".into(), json!({}));
    Value::Object(s)
}

// ------------------------------------------------------------ o XP gasto

/// O XP gasto, com as parcelas do `recompute()`, na ordem da soma.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpentXp {
    pub attributes: f64,
    pub skills: f64,
    pub specialties: f64,
    pub virtues: f64,
    pub willpower: f64,
    pub appearance: f64,
    pub spark: f64,
    pub secondary: f64,
    pub techniques: f64,
    pub arts: f64,
    pub effects: f64,
    pub backgrounds: f64,
    pub ancestry: f64,
    pub total: f64,
}

/// `x || 0` com um número: `NaN` e `-0` viram `0`.
fn or0(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 { 0.0 } else { x }
}

/// `specCostSum`: a soma das especialidades nomeadas (`v` de cada uma).
fn specialties_cost(t: &EditorTables, list: Option<&Value>, secondary: bool) -> f64 {
    jv_list(list).iter().fold(0.0, |a, e| {
        a + specialty_cost(t.sheet.rules(), jv::num_or0(e.get("v")), secondary)
    })
}

fn jv_list(v: Option<&Value>) -> &[Value] {
    v.and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

/// O XP gasto na ficha lida (o total do `recompute()` e as parcelas).
pub fn spent_xp(t: &EditorTables, sheet: &Value) -> SpentXp {
    let get = |k: &str| sheet.get(k);
    let at = |o: &str, k: &str| jv::get(get(o), k);
    let mut x = SpentXp {
        attributes: 0.0,
        skills: 0.0,
        specialties: 0.0,
        virtues: 0.0,
        willpower: 0.0,
        appearance: 0.0,
        spark: 0.0,
        secondary: 0.0,
        techniques: 0.0,
        arts: 0.0,
        effects: 0.0,
        backgrounds: 0.0,
        ancestry: 0.0,
        total: 0.0,
    };
    for a in &t.attributes {
        let floor = attribute_floor(t, sheet, a);
        let v = jv::num_or(at("attrs", a), floor);
        x.attributes += t.cost("atributo", Some(floor), v);
    }
    for h in &t.skills {
        x.skills += t.cost("habilidadePrimaria", None, jv::num_or0(at("skills", h)));
        x.specialties += specialties_cost(t, at("spec", h), false);
    }
    for k in ["esquiva", "bloqueio", "social", "mental"] {
        x.specialties += specialties_cost(t, at("defSpec", k), false);
    }
    for v in &t.virtues {
        x.virtues += t.cost("virtude", None, jv::num_or(at("virtues", v), 1.0));
    }
    x.willpower = t.cost("vontade", None, jv::num_or(get("willpower"), 0.0));
    x.appearance = t.cost("aparencia", None, jv::num_or(get("aparencia"), 0.0));
    x.spark = t.cost("centelha", None, jv::num_or0(get("centelha")));
    for k in t.secondary.iter().map(String::as_str).chain(FREE_SECONDARY) {
        x.secondary += t.cost("habilidadeSecundaria", None, jv::num_or0(at("skills2", k)))
            + specialties_cost(t, at("spec2", k), true);
    }
    let level: HashMap<&str, f64> = t
        .techniques
        .iter()
        .map(|q| (q.id.as_str(), q.level))
        .collect();
    if let Some(m) = get("tech").and_then(Value::as_object) {
        for id in jv::keys(m) {
            let lvl = level.get(id.as_str()).copied().unwrap_or(0.0);
            if truthy(m.get(id.as_str())) && lvl != 0.0 && !lvl.is_nan() {
                x.techniques += feat_cost(t.sheet.rules(), lvl);
            }
        }
    }
    for a in &t.arts {
        x.arts += art_cost(t.sheet.rules(), jv::num_or0(at("arte", &a.id)));
    }
    for e in &t.effects {
        if truthy(at("efeito", &e.id)) {
            x.effects += effect_cost(t.sheet.rules(), e.level);
        }
    }
    for a in &t.single_backgrounds {
        x.backgrounds += t.cost("antecedente", None, jv::num_or0(at("ante", a)));
    }
    for a in &t.named_backgrounds {
        for i in jv_list(at("anteNom", a)) {
            x.backgrounds += t.cost("antecedente", None, jv::num_or0(i.get("v")));
        }
    }
    x.ancestry = t
        .sheet
        .ancestry_of(get("raca").unwrap_or(&Value::Null))
        .and_then(|r| r.cost)
        .map_or(0.0, or0);
    x.total = x.attributes
        + x.skills
        + x.specialties
        + x.virtues
        + x.willpower
        + x.appearance
        + x.spark
        + x.secondary
        + x.techniques
        + x.arts
        + x.effects
        + x.backgrounds
        + x.ancestry;
    x
}

// ------------------------------------------------------------ tetos e pisos

/// `pisoAttr`: o piso de um Atributo nesta ficha, o da régua mais o que a
/// Raça dá de graça (só o positivo).
pub fn attribute_floor(t: &EditorTables, sheet: &Value, key: &str) -> f64 {
    t.floor("atributo") + js::max(0.0, t.racial(sheet, key))
}

/// `capFor`: o teto de uma trilha da ficha. `kind` é o da fonte: `attr`
/// (com a Raça, com sinal), `skill`, `skill2`, `virtue`, `centelha`, `ante`,
/// `anten` (6), `willpower` e `aparencia` (12), e `arte2` (a Arte pela
/// Centelha, `arcano.tetoNivelArte`). Outro valor dá 6.
pub fn trait_cap(t: &EditorTables, sheet: &Value, kind: &str, key: Option<&str>) -> f64 {
    if kind == "arte2" {
        let tab: &[f64] = t
            .sheet
            .rules()
            .arcane
            .as_ref()
            .and_then(|a| a.art_level_cap.as_ref())
            .map_or(&[], |c| c.by_spark.as_slice());
        let spark = jv::num_or0(sheet.get("centelha"));
        let i = js::max(0.0, js::min(tab.len() as f64 - 1.0, spark));
        return if i >= 0.0 && i.fract() == 0.0 {
            tab.get(i as usize).copied().unwrap_or(6.0)
        } else {
            6.0
        };
    }
    let racial = if kind == "attr" {
        key.map_or(0.0, |k| t.racial(sheet, k))
    } else {
        0.0
    };
    let base = match kind {
        "willpower" | "aparencia" => 12.0,
        _ => 6.0,
    };
    base + racial
}

/// A massa foi escrita pelo jogador? (`massaEscolhida`, a conta do
/// `normalize` ao abrir a ficha: `corpo.massa` é número, ou a identidade tem
/// peso.) É o `mass_chosen` de [`change_ancestry`].
///
/// A ficha tem de estar na forma de leitura (a do [`super::for_reading`]): o
/// `normalize` roda o `fichaParaLeitura` antes desta conta, e a leitura passa
/// o `id.peso` numérico para `corpo.massa` (`ficha-formato.ts:94-97`). Numa
/// ficha gravada, a mesma massa pode estar só no `id.peso`.
pub fn mass_chosen(sheet: &Value) -> bool {
    jv::get(sheet.get("corpo"), "massa").is_some_and(Value::is_number)
        || truthy(jv::get(sheet.get("id"), "peso"))
}

/// A troca de Raça (o `change` do seletor): a Raça nova; a massa sai quando o
/// jogador não a escreveu (`mass_chosen`, o que a ficha tinha ao abrir); e
/// cada Atributo vai para dentro do piso e do teto novos.
pub fn change_ancestry(t: &EditorTables, sheet: &mut Value, ancestry: &str, mass_chosen: bool) {
    let Some(m) = sheet.as_object_mut() else {
        return;
    };
    m.insert("raca".into(), json!(ancestry));
    if !mass_chosen && let Some(Value::Object(body)) = m.get_mut("corpo") {
        body.shift_remove("massa");
    }
    let view = sheet.clone();
    let floor = t.floor("atributo");
    let mut attrs = jv::spread(view.get("attrs"));
    for a in &t.attributes {
        let v = jv::num_or(attrs.get(a.as_str()), floor);
        let clamped = js::max(
            attribute_floor(t, &view, a),
            js::min(v, trait_cap(t, &view, "attr", Some(a))),
        );
        attrs.insert(a.clone(), num(clamped));
    }
    if let Some(m) = sheet.as_object_mut() {
        m.insert("attrs".into(), Value::Object(attrs));
    }
}

// ------------------------------------------------------------ especialidades

/// Uma especialidade nomeada: o nome que o jogador escreveu e o nível.
#[derive(Debug, Clone, PartialEq)]
pub struct Specialty {
    pub name: String,
    pub level: f64,
}

/// O teto das especialidades de uma Habilidade: até metade do nível delas, e
/// cada uma até a metade do nível.
pub fn specialty_cap(skill_level: f64) -> f64 {
    (or0(skill_level) / 2.0).floor()
}

/// `clampSpecs`: as especialidades na forma nomeada (`[{s, v}]`), com o
/// formato antigo convertido (número solto, lista de nomes por nível) e os
/// tetos aplicados.
pub fn clamp_specialties(list: &Value, skill_level: f64) -> Vec<Specialty> {
    let cap = specialty_cap(skill_level);
    let mut out: Vec<Specialty> = Vec::new();
    match list {
        Value::Array(a) if a.first().is_some_and(Value::is_string) => {
            let mut counts: Vec<(String, f64)> = Vec::new();
            for s in a {
                let k = if s.is_null() {
                    String::new()
                } else {
                    jv::to_js_string(Some(s))
                };
                match counts.iter_mut().find(|(n, _)| *n == k) {
                    Some((_, c)) => *c += 1.0,
                    None => counts.push((k, 1.0)),
                }
            }
            out = counts
                .into_iter()
                .map(|(name, level)| Specialty { name, level })
                .collect();
        }
        Value::Array(a) => {
            out = a
                .iter()
                .map(|e| match e {
                    Value::Object(o) => Specialty {
                        name: if jv::nullish(o.get("s")) {
                            String::new()
                        } else {
                            jv::to_js_string(o.get("s"))
                        },
                        level: if truthy(o.get("v")) {
                            jv::to_number(o.get("v"))
                        } else {
                            0.0
                        },
                    },
                    Value::Array(_) => Specialty {
                        name: String::new(),
                        level: 0.0,
                    },
                    _ => Specialty {
                        name: String::new(),
                        level: 0.0,
                    },
                })
                .collect();
        }
        Value::Number(n) if n.as_f64().is_some_and(|x| x > 0.0) => {
            out.push(Specialty {
                name: String::new(),
                level: n.as_f64().unwrap_or(0.0),
            });
        }
        _ => {}
    }
    let kept: Vec<Specialty> = out
        .into_iter()
        .map(|e| Specialty {
            name: e.name,
            level: js::max(0.0, js::min(cap, e.level)),
        })
        .filter(|e| e.level > 0.0)
        .collect();
    let end = if cap >= 0.0 { cap as usize } else { 0 };
    kept.into_iter().take(end).collect()
}

// ------------------------------------------------------------ Técnicas e Efeitos

/// O estado de uma Técnica na ficha: comprada, a Centelha que basta, os
/// pré-requisitos diretos que faltam e se ela vale.
#[derive(Debug, Clone, PartialEq)]
pub struct TechniqueStatus {
    pub owned: bool,
    pub spark_ok: bool,
    pub missing: Vec<String>,
    pub ok: bool,
}

/// O estado de uma Técnica (`renderCaminhos`): vale com a Centelha no nível
/// dela e todos os pré-requisitos diretos comprados.
pub fn technique_status(sheet: &Value, technique: &Technique) -> TechniqueStatus {
    let owned = truthy(jv::get(sheet.get("tech"), &technique.id));
    let spark_ok = jv::num_or0(sheet.get("centelha")) >= technique.level;
    let missing: Vec<String> = technique
        .prereq
        .iter()
        .filter(|p| !truthy(jv::get(sheet.get("tech"), p)))
        .cloned()
        .collect();
    let ok = spark_ok && missing.is_empty();
    TechniqueStatus {
        owned,
        spark_ok,
        missing,
        ok,
    }
}

/// `efeitosAoAlcance`: os Efeitos que cabem numa Arte em que a ficha tem o
/// nível do Efeito, por nível e nome (a colação `pt`, [`crate::collate`]).
pub fn effects_in_reach<'a>(t: &'a EditorTables, sheet: &Value) -> Vec<&'a Effect> {
    let mut list: Vec<&Effect> = t
        .effects
        .iter()
        .filter(|e| {
            e.arts
                .iter()
                .any(|a| jv::num_or0(jv::get(sheet.get("arte"), &a.id)) >= e.level)
        })
        .collect();
    list.sort_by(|a, b| {
        a.level
            .partial_cmp(&b.level)
            .unwrap_or(Ordering::Equal)
            .then_with(|| crate::collate::compare(&a.name, &b.name))
    });
    list
}

// ------------------------------------------------------------ o deslocamento

/// A metade da penalidade física (armadura e escudo) que o deslocamento
/// paga, arredondada para baixo.
pub fn half_movement_penalty(physical_penalty: f64) -> f64 {
    (physical_penalty / 2.0).floor()
}

/// O deslocamento da ficha com a penalidade: cada medida perde a metade da
/// penalidade física (o salto vertical, em centímetros, perde dez vezes isso),
/// sem ficar abaixo de zero; a natação não perde nada.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PenalizedMovement {
    pub half_penalty: f64,
    pub battle: f64,
    pub dash: f64,
    pub run: f64,
    pub swim: f64,
    pub vertical_jump: f64,
    pub standing_jump: f64,
    pub running_jump: f64,
}

/// As medidas do `renderDerived` (e a Vel. de Corrida da seção de força): a
/// conta é uma só, sobre o [`Movement`] e a penalidade física.
pub fn penalized_movement(m: &Movement, physical_penalty: f64) -> PenalizedMovement {
    let half = half_movement_penalty(physical_penalty);
    let mp = |v: f64| js::max(0.0, v - half);
    PenalizedMovement {
        half_penalty: half,
        battle: mp(m.battle),
        dash: mp(m.dash),
        run: mp(m.run),
        swim: m.swim,
        vertical_jump: js::max(0.0, m.vertical_jump - half * 10.0),
        standing_jump: mp(m.standing_jump),
        running_jump: mp(m.running_jump),
    }
}
