//! As armaduras e as bolsas da ficha: o que cada clique faz no estado e as
//! contas que o painel mostra (port de `src/lib/ficha-engine.ts`).
//!
//! Cada função muda a ficha como o ouvinte da fonte muda `S`, na mesma ordem.
//! A tela, o diálogo de escolha e o de confirmação ficam com quem chama: aqui
//! entra o que vem depois deles. O enquadramento da foto está em `photo`.

use serde_json::{Map, Value, json};

use super::editor::EditorTables;
use super::equip::ARMOR_EDIT_FIELDS;
use super::numbers::stack_armor;
use super::value::{is_str, num, strict_eq, truthy};
use crate::js;

fn obj(v: &mut Value) -> &mut Map<String, Value> {
    if !v.is_object() {
        *v = Value::Object(Map::new());
    }
    match v {
        Value::Object(m) => m,
        _ => unreachable!(),
    }
}

// ----------------------------------------------------------- as armaduras

/// `pecaArm`: a peça de armadura pelo `uid`.
pub fn armor_piece<'a>(sheet: &'a Value, uid: &str) -> Option<&'a Value> {
    sheet
        .get("equip")?
        .get("armaduras")?
        .as_array()?
        .iter()
        .find(|p| is_str(p.get("uid"), uid))
}

/// `pecaPorChave`: a peça de uma chave do ajuste (`arm:<uid>` é de armadura,
/// qualquer outra é do arsenal).
pub fn piece_by_key<'a>(sheet: &'a Value, key: &str) -> Option<&'a Value> {
    let mut parts = key.split(':');
    let (kind, uid) = (parts.next()?, parts.next().unwrap_or(""));
    if kind == "arm" {
        armor_piece(sheet, uid)
    } else {
        sheet
            .get("arsenal")?
            .as_array()?
            .iter()
            .find(|p| is_str(p.get("uid"), uid))
    }
}

/// `pecasArmadura`: as armaduras vestidas, achatadas.
pub fn worn_armor_pieces(t: &EditorTables, sheet: &Value) -> Vec<Value> {
    t.sheet_tables().catalog().worn_armor(sheet)
}

/// O "−" que a tela escreve na Penalidade: `−N`, ou `0` quando não há.
pub fn penalty_text(penalty: f64) -> String {
    if truthy(Some(&num(penalty))) {
        format!("\u{2212}{}", js::number_to_string(penalty))
    } else {
        "0".to_owned()
    }
}

/// Os números de uma peça de armadura no card (`refreshArm`): a Absorção por
/// categoria, o Nível de Perfuração, a Penalidade e se a peça difere do
/// livro.
#[derive(Debug, Clone, PartialEq)]
pub struct ArmorPieceNumbers {
    pub impact: f64,
    pub cut: f64,
    pub pierce: f64,
    /// `resistPerf || 0`.
    pub pierce_level: f64,
    /// `−N` ou `0` (o `penalty_text`).
    pub penalty: String,
    /// `difereDaOrigem(p, CAMPOS_ARMADURA)`.
    pub adjusted: bool,
}

/// Os números de uma peça de armadura; `None` quando a peça não existe ou não
/// tem item.
pub fn armor_piece_numbers(
    t: &EditorTables,
    sheet: &Value,
    uid: &str,
) -> Option<ArmorPieceNumbers> {
    let piece = armor_piece(sheet, uid)?;
    let cat = t.sheet_tables().catalog();
    let item = cat.piece_item(Some(piece))?;
    let soak = item.get("soak");
    let n = |k: &str| super::value::to_number(soak.and_then(|s| s.get(k)));
    Some(ArmorPieceNumbers {
        impact: n("impacto"),
        cut: n("corte"),
        pierce: n("perfuracao"),
        pierce_level: super::value::num_or0(item.get("resistPerf")),
        penalty: penalty_text(super::value::num_or0(item.get("penalidade"))),
        adjusted: super::arsenal::differs_from_origin(cat, piece, &ARMOR_EDIT_FIELDS),
    })
}

/// A Absorção combinada das peças vestidas (`renderAbsorcao`): vale a maior
/// Absorção de cada categoria, e as Penalidades somam.
#[derive(Debug, Clone, PartialEq)]
pub struct AbsorptionPanel {
    pub impact: f64,
    pub cut: f64,
    pub pierce: f64,
    /// O Nível de Perfuração da maior peça; a tela só o mostra se não é zero.
    pub pierce_level: f64,
    /// `−N` ou `0`.
    pub penalty: String,
    /// O texto de baixo: quantas peças vestidas, ou "Nada vestido.".
    pub note: String,
}

/// O painel de Absorção combinada.
pub fn absorption_panel(t: &EditorTables, sheet: &Value) -> AbsorptionPanel {
    let pieces = worn_armor_pieces(t, sheet);
    let st = stack_armor(&pieces);
    let n = pieces.len();
    let note = if n > 0 {
        let plural = if n > 1 { "s" } else { "" };
        format!(
            "{n} pe\u{e7}a{plural} vestida{plural}: vale a maior Absor\u{e7}\u{e3}o de cada modo, as penalidades somam."
        )
    } else {
        "Nada vestido.".to_owned()
    };
    AbsorptionPanel {
        impact: js::or_zero(st.soak.impact),
        cut: js::or_zero(st.soak.cut),
        pierce: js::or_zero(st.soak.pierce),
        pierce_level: js::or_zero(st.pierce_level),
        penalty: penalty_text(st.penalty),
        note,
    }
}

/// A armadura escolhida para entrar (depois do diálogo): a personalizada (`c`)
/// ou a do catálogo, já vestida. O `uid` vem de fora.
pub fn add_armor(sheet: &mut Value, base: &str, uid: &str) {
    let equip = obj(sheet).entry("equip".to_owned()).or_insert(Value::Null);
    if !truthy(Some(equip)) {
        *equip = json!({});
    }
    let list = obj(equip)
        .entry("armaduras".to_owned())
        .or_insert(Value::Null);
    if !truthy(Some(list)) {
        *list = json!([]);
    }
    if let Value::Array(a) = list {
        a.push(json!({"uid": uid, "base": base, "vestida": true}));
    }
}

/// O campo de nome da peça: sem texto, a peça perde o nome (volta ao do livro).
/// A chave `nome`, quando falta, entra onde a fonte a deixa (depois de `base`).
pub fn rename_armor(sheet: &mut Value, uid: &str, name: &str) {
    let Some(Value::Array(a)) = sheet.get_mut("equip").and_then(|e| e.get_mut("armaduras")) else {
        return;
    };
    let Some(Value::Object(p)) = a.iter_mut().find(|p| is_str(p.get("uid"), uid)) else {
        return;
    };
    let name = name.trim_matches(super::value::is_js_space);
    if name.is_empty() {
        p.shift_remove("nome");
    } else if p.contains_key("nome") {
        p.insert("nome".into(), json!(name));
    } else {
        // a fonte monta cada peça de armadura com `nome` logo depois de `base`
        // (vazio, o `undefined` some do JSON), então o nome escrito depois cai
        // ali, e não no fim
        let pos = |k: &str| p.keys().position(|x| x == k);
        let at = pos("base").or_else(|| pos("uid")).map_or(0, |i| i + 1);
        p.shift_insert(at, "nome".into(), json!(name));
    }
}

/// O botão "Vestir": inverte `vestida`.
pub fn toggle_armor_worn(sheet: &mut Value, uid: &str) {
    let Some(Value::Array(a)) = sheet.get_mut("equip").and_then(|e| e.get_mut("armaduras")) else {
        return;
    };
    if let Some(Value::Object(p)) = a.iter_mut().find(|p| is_str(p.get("uid"), uid)) {
        let now = !truthy(p.get("vestida"));
        p.insert("vestida".into(), json!(now));
    }
}

/// O descarte de uma armadura (depois da confirmação). Sem a peça, nada muda.
pub fn remove_armor(sheet: &mut Value, uid: &str) {
    if armor_piece(sheet, uid).is_none() {
        return;
    }
    if let Some(Value::Array(a)) = sheet.get_mut("equip").and_then(|e| e.get_mut("armaduras")) {
        a.retain(|x| !strict_eq(x.get("uid"), Some(&json!(uid))));
    }
}

// ----------------------------------------------------------------- bolsas

fn blank_row() -> Value {
    json!({"item": "", "peso": "", "preco": ""})
}

fn bag_mut(sheet: &mut Value, index: usize) -> Option<&mut Map<String, Value>> {
    match sheet.get_mut("bolsas")?.get_mut(index)? {
        Value::Object(m) => Some(m),
        _ => None,
    }
}

/// O nome da bolsa.
pub fn rename_bag(sheet: &mut Value, index: usize, name: &str) {
    if let Some(b) = bag_mut(sheet, index) {
        b.insert("nome".into(), json!(name));
    }
}

/// O peso total escrito no lugar da soma das linhas (`pesoTotal`).
pub fn set_bag_total(sheet: &mut Value, index: usize, text: &str) {
    if let Some(b) = bag_mut(sheet, index) {
        b.insert("pesoTotal".into(), json!(text));
    }
}

/// Uma célula da tabela da bolsa (`item`, `peso` ou `preco`). Sem a linha,
/// nada muda.
pub fn set_bag_cell(sheet: &mut Value, bag: usize, row: usize, field: &str, text: &str) {
    if let Some(b) = bag_mut(sheet, bag)
        && let Some(Value::Array(rows)) = b.get_mut("itens")
        && let Some(Value::Object(r)) = rows.get_mut(row)
    {
        r.insert(field.to_owned(), json!(text));
    }
}

/// O botão de adicionar uma linha à bolsa.
pub fn add_bag_row(sheet: &mut Value, index: usize) {
    if let Some(b) = bag_mut(sheet, index) {
        let rows = b.entry("itens".to_owned()).or_insert(Value::Null);
        if !truthy(Some(rows)) {
            *rows = json!([]);
        }
        if let Value::Array(a) = rows {
            a.push(blank_row());
        }
    }
}

/// O botão de remover uma linha: a tabela nunca fica sem nenhuma.
pub fn remove_bag_row(sheet: &mut Value, bag: usize, row: usize) {
    if let Some(b) = bag_mut(sheet, bag)
        && let Some(Value::Array(rows)) = b.get_mut("itens")
    {
        if row < rows.len() {
            rows.remove(row);
        }
        if rows.is_empty() {
            rows.push(blank_row());
        }
    }
}
