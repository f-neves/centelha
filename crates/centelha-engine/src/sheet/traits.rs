//! Mexer nos traços da ficha: o que cada clique faz no estado (port de
//! `src/lib/ficha-engine.ts`).
//!
//! A bolinha (`applyVal`, `setDot`, `bump`), os pisos e os tetos da Raça, a
//! Aparência efetiva, as instâncias nomeadas dos Antecedentes, as
//! especialidades (a de Habilidade e a de defesa), as Técnicas e os Efeitos
//! comprados, a massa e o orçamento. Cada função muda a ficha como o ouvinte
//! da fonte muda `S`, na mesma ordem; o XP gasto, a tela e o salvar ficam com
//! quem chama (`spent_xp` é o `recompute`).

use centelha_data::Technique;
use serde_json::{Map, Value, json};

use super::editor::{
    EditorTables, attribute_floor, clamp_specialties, technique_status, trait_cap,
};
use super::value::{nullish, num, num_or0, strict_eq, to_js_string, to_number, truthy};
use crate::js;

/// A trilha de um traço (o `kind` da fonte).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraitKind {
    /// `attr`: um Atributo.
    Attribute,
    /// `skill`: uma Habilidade.
    Skill,
    /// `skill2`: uma Habilidade Secundária.
    Secondary,
    /// `virtue`: uma Virtude.
    Virtue,
    /// `centelha`.
    Spark,
    /// `willpower`: a Força de Vontade.
    Willpower,
    /// `aparencia`.
    Appearance,
    /// `arte2`: o nível de uma Arte.
    Art,
    /// `ante`: um Antecedente único.
    Background,
    /// `anten`: uma instância de Antecedente nomeado (a chave é `id~uid`).
    NamedBackground,
}

impl TraitKind {
    /// O `kind` como a fonte o escreve.
    pub fn source_name(self) -> &'static str {
        match self {
            TraitKind::Attribute => "attr",
            TraitKind::Skill => "skill",
            TraitKind::Secondary => "skill2",
            TraitKind::Virtue => "virtue",
            TraitKind::Spark => "centelha",
            TraitKind::Willpower => "willpower",
            TraitKind::Appearance => "aparencia",
            TraitKind::Art => "arte2",
            TraitKind::Background => "ante",
            TraitKind::NamedBackground => "anten",
        }
    }

    /// O `kind` da fonte (`attr`, `skill2`, `anten`...), ou `None`.
    pub fn from_source_name(name: &str) -> Option<TraitKind> {
        [
            TraitKind::Attribute,
            TraitKind::Skill,
            TraitKind::Secondary,
            TraitKind::Virtue,
            TraitKind::Spark,
            TraitKind::Willpower,
            TraitKind::Appearance,
            TraitKind::Art,
            TraitKind::Background,
            TraitKind::NamedBackground,
        ]
        .into_iter()
        .find(|k| k.source_name() == name)
    }
}

/// Onde mora a especialidade: nas Habilidades (`p`) ou nas Secundárias (`s`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialtyScope {
    /// As Habilidades (`skills` e `spec` da ficha; `p` na fonte).
    Primary,
    /// As Habilidades Secundárias (`skills2` e `spec2`; `s` na fonte).
    Secondary,
}

impl SpecialtyScope {
    fn skills_key(self) -> &'static str {
        match self {
            SpecialtyScope::Primary => "skills",
            SpecialtyScope::Secondary => "skills2",
        }
    }

    fn spec_key(self) -> &'static str {
        match self {
            SpecialtyScope::Primary => "spec",
            SpecialtyScope::Secondary => "spec2",
        }
    }
}

// ------------------------------------------------------------- a Raça

/// `tetoRacialAttr`: quanto a Raça move o teto do Atributo, com sinal (o
/// `-1` do Anão baixa o teto de 6 para 5).
pub fn racial_cap_bonus(t: &EditorTables, sheet: &Value, key: &str) -> f64 {
    t.racial(sheet, key)
}

/// `pisoRacialAttr`: quantos pontos a Raça já dá de graça, só o positivo.
pub fn racial_floor_bonus(t: &EditorTables, sheet: &Value, key: &str) -> f64 {
    js::max(0.0, t.racial(sheet, key))
}

/// `racApMod`: o que a Raça soma à Aparência.
pub fn racial_appearance_mod(t: &EditorTables, sheet: &Value) -> f64 {
    t.sheet_tables()
        .ancestry_of(sheet.get("raca").unwrap_or(&Value::Null))
        .and_then(|a| a.appearance_mod)
        .map_or(0.0, js::or_zero)
}

/// `apEfetiva`: a Aparência com a Raça, presa entre 0 e 12.
pub fn effective_appearance(t: &EditorTables, sheet: &Value, value: f64) -> f64 {
    js::max(0.0, js::min(12.0, value + racial_appearance_mod(t, sheet)))
}

// ---------------------------------------------------------- piso e valor

/// `pisoDe`: o piso de uma trilha NESTA ficha. O Atributo leva o ponto da
/// Raça (`pisoAttr`); as outras, o da régua de `regras.json`.
pub fn trait_floor(t: &EditorTables, sheet: &Value, kind: TraitKind, key: &str) -> f64 {
    match kind {
        TraitKind::Attribute => attribute_floor(t, sheet, key),
        TraitKind::Skill => t.floor("habilidadePrimaria"),
        TraitKind::Secondary => t.floor("habilidadeSecundaria"),
        TraitKind::Virtue => t.floor("virtude"),
        TraitKind::Spark => t.floor("centelha"),
        TraitKind::Willpower => t.floor("vontade"),
        TraitKind::Appearance => t.floor("aparencia"),
        TraitKind::Art => t.floor("arte"),
        TraitKind::Background | TraitKind::NamedBackground => 0.0,
    }
}

/// `anteInst`: a instância nomeada pela chave `id~uid`, ou `None` se ela
/// sumiu (ficha antiga).
pub fn named_instance<'a>(sheet: &'a Value, key: &str) -> Option<&'a Value> {
    let mut parts = key.split('~');
    let (id, uid) = (parts.next()?, parts.next());
    let list = sheet.get("anteNom")?.get(id)?.as_array()?;
    list.iter()
        .find(|x| strict_eq(x.get("u"), uid.map(|u| json!(u)).as_ref()))
}

fn named_instance_mut<'a>(sheet: &'a mut Value, key: &str) -> Option<&'a mut Value> {
    let mut parts = key.split('~');
    let (id, uid) = (parts.next()?, parts.next());
    let uid = uid.map(|u| json!(u));
    let list = sheet.get_mut("anteNom")?.get_mut(id)?.as_array_mut()?;
    list.iter_mut()
        .find(|x| strict_eq(x.get("u"), uid.as_ref()))
}

/// `valOf`: o valor atual da trilha (`|| 0`: o que falta é 0).
pub fn trait_value(sheet: &Value, kind: TraitKind, key: &str) -> f64 {
    let at = |group: &str| num_or0(sheet.get(group).and_then(|g| g.get(key)));
    match kind {
        TraitKind::Attribute => at("attrs"),
        TraitKind::Skill => at("skills"),
        TraitKind::Secondary => at("skills2"),
        TraitKind::Virtue => at("virtues"),
        TraitKind::Spark => num_or0(sheet.get("centelha")),
        TraitKind::Willpower => num_or0(sheet.get("willpower")),
        TraitKind::Appearance => num_or0(sheet.get("aparencia")),
        TraitKind::Art => at("arte"),
        TraitKind::Background => at("ante"),
        TraitKind::NamedBackground => num_or0(named_instance(sheet, key).and_then(|i| i.get("v"))),
    }
}

// ------------------------------------------------------------ a bolinha

fn obj(v: &mut Value) -> &mut Map<String, Value> {
    if !v.is_object() {
        *v = Value::Object(Map::new());
    }
    match v {
        Value::Object(m) => m,
        _ => unreachable!(),
    }
}

fn group<'a>(sheet: &'a mut Value, key: &str) -> &'a mut Map<String, Value> {
    let m = obj(sheet);
    let g = m.entry(key.to_owned()).or_insert_with(|| json!({}));
    obj(g)
}

/// `S[spec][key] = clampSpecs(S[spec][key], level)`.
fn clamp_spec(sheet: &mut Value, spec: &str, key: &str, level: f64) {
    let list = group(sheet, spec).get(key).cloned().unwrap_or(Value::Null);
    let clamped: Vec<Value> = clamp_specialties(&list, level)
        .into_iter()
        .map(|e| json!({"s": e.name, "v": num(e.level)}))
        .collect();
    group(sheet, spec).insert(key.to_owned(), Value::Array(clamped));
}

/// `applyVal`: grava o valor da trilha, preso entre o piso e o teto. Na
/// Habilidade e na Secundária recorta as especialidades ao novo nível. O XP
/// gasto (`recompute`) fica com quem chama.
pub fn set_trait(t: &EditorTables, sheet: &mut Value, kind: TraitKind, key: &str, value: f64) {
    let floor = trait_floor(t, sheet, kind, key);
    let cap = trait_cap(t, sheet, kind.source_name(), Some(key));
    let nv = js::max(floor, js::min(value, cap));
    match kind {
        TraitKind::Attribute => {
            group(sheet, "attrs").insert(key.to_owned(), num(nv));
        }
        TraitKind::Skill => {
            group(sheet, "skills").insert(key.to_owned(), num(nv));
            clamp_spec(sheet, "spec", key, nv);
        }
        TraitKind::Secondary => {
            group(sheet, "skills2").insert(key.to_owned(), num(nv));
            clamp_spec(sheet, "spec2", key, nv);
        }
        TraitKind::Virtue => {
            group(sheet, "virtues").insert(key.to_owned(), num(nv));
        }
        TraitKind::Spark => {
            obj(sheet).insert("centelha".into(), num(nv));
        }
        TraitKind::Willpower => {
            obj(sheet).insert("willpower".into(), num(nv));
        }
        TraitKind::Appearance => {
            obj(sheet).insert("aparencia".into(), num(nv));
        }
        TraitKind::Art => {
            group(sheet, "arte").insert(key.to_owned(), num(nv));
        }
        TraitKind::Background => {
            group(sheet, "ante").insert(key.to_owned(), num(nv));
        }
        TraitKind::NamedBackground => {
            if let Some(Value::Object(i)) = named_instance_mut(sheet, key) {
                i.insert("v".into(), num(nv));
            }
        }
    }
}

/// `setDot`: o clique na bolinha `d` (de 1 em diante). Clicar no piso ou
/// abaixo vai ao piso; clicar na bolinha que está acesa desce uma; senão o
/// valor vira `d`. A bolinha acima do teto (`cap` na tela) não responde.
pub fn click_dot(t: &EditorTables, sheet: &mut Value, kind: TraitKind, key: &str, d: f64) {
    if d > trait_cap(t, sheet, kind.source_name(), Some(key)) {
        return;
    }
    let floor = trait_floor(t, sheet, kind, key);
    let value = trait_value(sheet, kind, key);
    let next = if d <= floor {
        floor
    } else if value == d {
        d - 1.0
    } else {
        d
    };
    set_trait(t, sheet, kind, key, next);
}

/// `bump`: as setas do teclado, `delta` de 1 ou -1.
pub fn step_trait(t: &EditorTables, sheet: &mut Value, kind: TraitKind, key: &str, delta: f64) {
    let value = trait_value(sheet, kind, key) + delta;
    set_trait(t, sheet, kind, key, value);
}

// ------------------------------------------------- Antecedentes nomeados

/// O `[data-ante-add]`: uma instância nova (`{u, n: "", v: 0}`) no fim da
/// lista do Antecedente. O `uid` chega de fora.
pub fn add_named_background(sheet: &mut Value, id: &str, uid: &str) {
    let list = group(sheet, "anteNom")
        .entry(id.to_owned())
        .or_insert(Value::Null);
    if nullish(Some(list)) {
        *list = json!([]);
    }
    if let Value::Array(a) = list {
        a.push(json!({"u": uid, "n": "", "v": 0}));
    }
}

/// O `[data-ante-rm]`: tira a instância (`id~uid`) da lista.
pub fn remove_named_background(sheet: &mut Value, key: &str) {
    let mut parts = key.split('~');
    let (Some(id), uid) = (parts.next(), parts.next()) else {
        return;
    };
    let uid = uid.map(|u| json!(u));
    let kept: Vec<Value> = group(sheet, "anteNom")
        .get(id)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter(|x| !strict_eq(x.get("u"), uid.as_ref()))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    group(sheet, "anteNom").insert(id.to_owned(), Value::Array(kept));
}

/// O `input` do nome da instância (`[data-ante-nome]`): só grava o nome.
pub fn rename_named_background(sheet: &mut Value, key: &str, name: &str) {
    if let Some(Value::Object(i)) = named_instance_mut(sheet, key) {
        i.insert("n".into(), json!(name));
    }
}

// ----------------------------------------------- especialidades de defesa

/// O `[data-defspec-add]` depois do diálogo: a situação e o bônus escritos.
/// O bônus é inteiro de 0 a 6 (`parseInt`, preso); sem situação ou com bônus
/// zero, nada entra.
pub fn add_defense_specialty(sheet: &mut Value, key: &str, situation: &str, bonus: &str) {
    let s = situation.trim_matches(super::value::is_js_space);
    let v = js::max(0.0, js::min(6.0, js::or_zero(parse_int(bonus))));
    if s.is_empty() || v == 0.0 {
        return;
    }
    let list = group(sheet, "defSpec")
        .entry(key.to_owned())
        .or_insert(Value::Null);
    if !truthy(Some(list)) {
        *list = json!([]);
    }
    if let Value::Array(a) = list {
        a.push(json!({"s": s, "v": num(v)}));
    }
}

/// `parseInt(texto, 10)`: o inteiro do começo do texto, `NaN` se não há.
fn parse_int(text: &str) -> f64 {
    let t = text.trim_start_matches(super::value::is_js_space);
    let (neg, rest) = match t.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return f64::NAN;
    }
    let n: f64 = digits.parse().unwrap_or(f64::NAN);
    if neg { -n } else { n }
}

/// O `[data-defspec-del]`: tira a especialidade `idx` da lista da defesa.
pub fn remove_defense_specialty(sheet: &mut Value, key: &str, idx: usize) {
    if let Some(Value::Array(a)) = group(sheet, "defSpec").get_mut(key)
        && idx < a.len()
    {
        a.remove(idx);
    }
}

// ----------------------------------------------- Técnicas e Efeitos

/// A pílula da Técnica: compra ou vende. A Técnica que não está comprada e
/// não vale (Centelha ou pré-requisito) está travada e não responde. Devolve
/// se mudou.
pub fn toggle_technique(sheet: &mut Value, technique: &Technique) -> bool {
    let status = technique_status(sheet, technique);
    if !status.owned && !status.ok {
        return false;
    }
    let id = technique.id.clone();
    let now = !truthy(sheet.get("tech").and_then(|t| t.get(&id)));
    group(sheet, "tech").insert(id, json!(now));
    true
}

/// O Efeito comprado ou não: inverte.
pub fn toggle_effect(sheet: &mut Value, id: &str) {
    let now = !truthy(sheet.get("efeito").and_then(|t| t.get(id)));
    group(sheet, "efeito").insert(id.to_owned(), json!(now));
}

// -------------------------------------------- massa, identidade, orçamento

/// O campo da massa (`corpo-massa`): `value_as_number` é o `valueAsNumber` do
/// campo e `blank` diz se o texto está vazio. Campo vazio devolve a conta
/// (a massa sai e o padrão da Raça volta); número finito e positivo grava a
/// massa e o peso em texto. Devolve o `massaEscolhida` de agora.
pub fn set_body_mass(sheet: &mut Value, value_as_number: f64, blank: bool) -> Option<bool> {
    if blank {
        if let Some(Value::Object(c)) = sheet.get_mut("corpo") {
            c.shift_remove("massa");
        }
        if let Some(Value::Object(i)) = sheet.get_mut("id") {
            i.shift_remove("peso");
        }
        return Some(false);
    }
    if value_as_number.is_finite() && value_as_number > 0.0 {
        let m = num(value_as_number);
        let nn = |sheet: &mut Value, k: &str| {
            let g = obj(sheet).entry(k.to_owned()).or_insert(Value::Null);
            if nullish(Some(g)) {
                *g = json!({});
            }
        };
        nn(sheet, "corpo");
        group(sheet, "corpo").insert("massa".into(), m);
        nn(sheet, "id");
        group(sheet, "id").insert(
            "peso".into(),
            json!(format!("{} kg", js::number_to_string(value_as_number))),
        );
        return Some(true);
    }
    None
}

/// Um campo de texto da identidade (`.idrow .txt`). Em `peso`, o primeiro
/// número do texto (vírgula vale ponto) vira a massa; o campo vazio tira a
/// massa. Devolve o novo `massaEscolhida`, ou `None` se nada mudou nele.
pub fn set_identity_field(sheet: &mut Value, field: &str, value: &str) -> Option<bool> {
    group(sheet, "id").insert(field.to_owned(), json!(value));
    if field != "peso" {
        return None;
    }
    let text = value.replacen(',', ".", 1);
    let mass = super::value::first_number(&text).unwrap_or(f64::NAN);
    if mass > 0.0 {
        let g = obj(sheet).entry("corpo".to_owned()).or_insert(Value::Null);
        if nullish(Some(g)) {
            *g = json!({});
        }
        group(sheet, "corpo").insert("massa".into(), num(mass));
        Some(true)
    } else if value.trim_matches(super::value::is_js_space).is_empty() {
        if let Some(Value::Object(c)) = sheet.get_mut("corpo") {
            c.shift_remove("massa");
        }
        Some(false)
    } else {
        None
    }
}

/// O orçamento de XP (`xpBudget`): `+valor || 0`.
pub fn set_budget(sheet: &mut Value, value: &str) {
    let n = js::or_zero(super::value::string_to_number(value));
    obj(sheet).insert("budget".into(), num(n));
}

/// O nome de uma Secundária livre (`data-livre`).
pub fn set_free_secondary_name(sheet: &mut Value, key: &str, name: &str) {
    group(sheet, "livreNome").insert(key.to_owned(), json!(name));
}

/// `ehSecLivre`: a chave é de uma Secundária em branco?
pub fn is_free_secondary(key: &str) -> bool {
    super::editor::FREE_SECONDARY.contains(&key)
}

/// `nomeLivre`: o nome que o jogador deu à Secundária livre, sem as bordas.
pub fn free_secondary_name(sheet: &Value, key: &str) -> String {
    let name = sheet.get("livreNome").and_then(|n| n.get(key));
    if truthy(name) {
        to_js_string(name)
            .trim_matches(super::value::is_js_space)
            .to_owned()
    } else {
        String::new()
    }
}

// ----------------------------------------------------- as especialidades

/// `specCount`: a soma dos níveis das especialidades.
pub fn specialty_count(list: &Value) -> f64 {
    list.as_array().map_or(0.0, |a| {
        a.iter().fold(0.0, |acc, e| acc + num_or0(e.get("v")))
    })
}

/// `specSkill`: o nível da Habilidade (ou da Secundária) da especialidade.
pub fn specialty_skill_level(sheet: &Value, scope: SpecialtyScope, key: &str) -> f64 {
    num_or0(sheet.get(scope.skills_key()).and_then(|g| g.get(key)))
}

fn spec_list<'a>(sheet: &'a mut Value, scope: SpecialtyScope, key: &str) -> &'a mut Vec<Value> {
    let g = group(sheet, scope.spec_key());
    let v = g.entry(key.to_owned()).or_insert(Value::Null);
    if !truthy(Some(v)) {
        *v = json!([]);
    }
    if !v.is_array() {
        *v = json!([]);
    }
    match v {
        Value::Array(a) => a,
        _ => unreachable!(),
    }
}

/// Abrir o modal das especialidades (`openSpecPop`): sem metade do nível a
/// Habilidade não abre (`false`); abrindo, a lista nasce vazia se faltava e
/// ganha uma linha em branco se ficou sem nenhuma.
pub fn open_specialties(sheet: &mut Value, scope: SpecialtyScope, key: &str) -> bool {
    let cap = (specialty_skill_level(sheet, scope, key) / 2.0).floor();
    if cap <= 0.0 || cap.is_nan() {
        return false;
    }
    let list = spec_list(sheet, scope, key);
    if list.is_empty() {
        list.push(json!({"s": "", "v": 0}));
    }
    true
}

/// O quadradinho `d` da especialidade `i`: clicar no que está aceso desce um,
/// senão o nível vira `d` (até o teto). O quadradinho acima do teto não
/// responde.
pub fn set_specialty_level(sheet: &mut Value, scope: SpecialtyScope, key: &str, i: usize, d: f64) {
    let cap = (specialty_skill_level(sheet, scope, key) / 2.0).floor();
    if d > cap {
        return;
    }
    let Some(Value::Object(e)) = spec_list(sheet, scope, key).get_mut(i) else {
        return;
    };
    let now = if strict_eq(e.get("v"), Some(&num(d))) {
        d - 1.0
    } else {
        js::min(cap, d)
    };
    e.insert("v".into(), num(now));
}

/// O campo de nome da especialidade `i`.
pub fn rename_specialty(sheet: &mut Value, scope: SpecialtyScope, key: &str, i: usize, name: &str) {
    if let Some(Value::Object(e)) = spec_list(sheet, scope, key).get_mut(i) {
        e.insert("s".into(), json!(name));
    }
}

/// O botão de remover a especialidade `i`; a lista que fica vazia ganha de
/// novo a linha em branco.
pub fn remove_specialty(sheet: &mut Value, scope: SpecialtyScope, key: &str, i: usize) {
    let list = spec_list(sheet, scope, key);
    if i < list.len() {
        list.remove(i);
    }
    if list.is_empty() {
        list.push(json!({"s": "", "v": 0}));
    }
}

/// O botão "+ Nova Especialidade": só entra se ainda cabe (menos que metade
/// do nível).
pub fn add_specialty(sheet: &mut Value, scope: SpecialtyScope, key: &str) {
    let cap = (specialty_skill_level(sheet, scope, key) / 2.0).floor();
    let list = spec_list(sheet, scope, key);
    if (list.len() as f64) < cap {
        list.push(json!({"s": "", "v": 0}));
    }
}

/// Fechar o modal (`closeSpecPop`): as especialidades de nível zero saem.
pub fn close_specialties(sheet: &mut Value, scope: SpecialtyScope, key: &str) {
    let g = group(sheet, scope.spec_key());
    if let Some(Value::Array(a)) = g.get(key) {
        let kept: Vec<Value> = a
            .iter()
            .filter(|e| to_number(e.get("v")) > 0.0)
            .cloned()
            .collect();
        g.insert(key.to_owned(), Value::Array(kept));
    }
}
