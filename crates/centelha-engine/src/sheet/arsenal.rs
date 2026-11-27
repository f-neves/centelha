//! O arsenal, os conjuntos de mãos e o ajuste de peça: o que cada clique faz
//! no estado (port de `src/lib/ficha-engine.ts`).
//!
//! O arsenal guarda as armas e os escudos que o personagem possui; cada
//! conjunto diz o que vai na mão hábil e na inábil; o ajuste edita o item que
//! a peça guarda (o `escreve`, o `limpa`). Cada função muda a ficha como o
//! ouvinte da fonte muda `S`, na mesma ordem. A tela, o diálogo de escolha e o
//! de confirmação ficam com quem chama: aqui entra o que vem depois deles.

use serde_json::{Map, Value, json};

use super::editor::EditorTables;
use super::equip::{
    ARMOR_EDIT_FIELDS, Catalog, EditField, FieldKind, HandRole, SHIELD_EDIT_FIELDS,
    WEAPON_EDIT_FIELDS, WEAPON_MORE_FIELDS, edit_field_base, write_edit_field,
};
use super::hands::piece_for;
use super::numbers::{HandItem, HandKind, hand_item};
use super::value::{is_js_space, is_str, num, strict_eq, string_to_number, to_js_string, truthy};
use crate::js;

/// A peça personalizada da arma e a do escudo (`REF_ARMA_LIVRE`,
/// `REF_ESCUDO_LIVRE`).
const CUSTOM_WEAPON: &str = "c";
const CUSTOM_SHIELD: &str = "ce";

/// `refLivre`: o `ref` é de uma peça personalizada?
pub fn is_custom_ref(reference: Option<&Value>) -> bool {
    is_str(reference, CUSTOM_WEAPON) || is_str(reference, CUSTOM_SHIELD)
}

/// `ehLivre`: a peça é personalizada?
pub fn is_custom_piece(piece: Option<&Value>) -> bool {
    is_custom_ref(piece.and_then(|p| p.get("ref")))
}

/// `it2H`: o item é uma arma (do catálogo ou personalizada) de duas mãos?
pub fn is_two_handed_item(item: &HandItem) -> bool {
    matches!(item.kind, HandKind::Weapon | HandKind::Custom)
        && item
            .weapon
            .as_ref()
            .and_then(|w| w.get("maos"))
            .is_some_and(|m| strict_eq(Some(m), Some(&json!(2))))
}

/// `improvisado`: o texto da nota da peça personalizada. A tela a põe num
/// `<span class="eq-improv-nota muted">`.
pub fn custom_piece_note(read_only: bool) -> &'static str {
    if read_only {
        "Peça fora do catálogo: nome e números são do jogador."
    } else {
        "Peça fora do catálogo: o nome e os números saem do \u{270e} ajustar."
    }
}

/// `conjAtivo`: o conjunto em uso (o marcado `ativo`, senão o primeiro).
fn active_set(sheet: &Value) -> Option<&Value> {
    let sets = sheet.get("conjuntos").and_then(Value::as_array)?;
    sets.iter()
        .find(|c| truthy(c.get("ativo")))
        .or_else(|| sets.first())
}

/// `papelDaPeca`: como a peça está empunhada no conjunto em uso.
pub fn piece_role(sheet: &Value, uid: &str) -> HandRole {
    let Some(cj) = active_set(sheet) else {
        return HandRole::None;
    };
    let holds = |hand: &str| {
        cj.get(hand)
            .and_then(|h| h.get("uid"))
            .is_some_and(|u| is_str(Some(u), uid))
    };
    if holds("habil") {
        HandRole::Main
    } else if holds("inabil") {
        HandRole::Off
    } else {
        HandRole::None
    }
}

/// `nomePeca`: o nome que o jogador deu à peça, ou o do item.
pub fn piece_name(t: &EditorTables, piece: &Value) -> Option<String> {
    if truthy(piece.get("nome")) {
        return Some(to_js_string(piece.get("nome")));
    }
    hand_item(t.sheet_tables(), Some(piece)).name
}

/// `camposItem`: os campos que o ajuste abre para o item (o escudo tem os
/// dele; as outras peças, os da arma).
pub fn item_fields(kind: HandKind) -> Vec<EditField> {
    if kind == HandKind::Shield {
        SHIELD_EDIT_FIELDS.to_vec()
    } else {
        WEAPON_EDIT_FIELDS
            .iter()
            .chain(WEAPON_MORE_FIELDS.iter())
            .copied()
            .collect()
    }
}

/// `igualCampo`: dois valores de campo são iguais pelo texto JSON (a ordem
/// das chaves conta; o que falta vale `null`; `-0` e `0` valem o mesmo).
pub fn same_field(a: Option<&Value>, b: Option<&Value>) -> bool {
    let text = |v: Option<&Value>| {
        let mut v = v.cloned().unwrap_or(Value::Null);
        positive_zero(&mut v);
        serde_json::to_string(&v).unwrap_or_default()
    };
    text(a) == text(b)
}

/// O `-0` vira `0` em toda a árvore: o `JSON.stringify` da fonte escreve os dois
/// como `0`, e o `serde_json` escreveria `-0.0`.
fn positive_zero(v: &mut Value) {
    match v {
        Value::Number(n) if n.as_f64().is_some_and(|x| x == 0.0 && x.is_sign_negative()) => {
            *v = json!(0);
        }
        Value::Array(a) => a.iter_mut().for_each(positive_zero),
        Value::Object(m) => m.values_mut().for_each(positive_zero),
        _ => {}
    }
}

/// `mostraCampo`: o valor do campo como a tela o escreve (a lista, separada
/// por vírgula; o que falta, vazio).
pub fn show_field(field: &EditField, value: Option<&Value>) -> Value {
    if field.kind == FieldKind::List {
        return match value {
            Some(Value::Array(a)) => json!(
                a.iter()
                    .map(|x| match x {
                        Value::Null => String::new(),
                        other => to_js_string(Some(other)),
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            _ => json!(""),
        };
    }
    match value {
        Some(v) if !v.is_null() => v.clone(),
        _ => json!(""),
    }
}

/// `difereDaOrigem`: algum campo da peça difere do item do livro de onde ela
/// veio?
pub fn differs_from_origin(cat: &Catalog, piece: &Value, fields: &[EditField]) -> bool {
    let (Some(item), Some(origin)) = (cat.piece_item(Some(piece)), cat.piece_origin(Some(piece)))
    else {
        return false;
    };
    fields.iter().any(|f| {
        let (a, b) = (
            edit_field_base(Some(&item), f),
            edit_field_base(Some(&origin), f),
        );
        !same_field(a.as_ref(), b.as_ref())
    })
}

/// `reordenar`: a lista na ordem dos `uid` que vieram da tela; o que a tela
/// não mostrava (peça filtrada, por exemplo) fica no fim, não some.
pub fn reorder(list: &[Value], uids: &[String]) -> Vec<Value> {
    let by_uid = |u: &str| list.iter().rev().find(|p| is_str(p.get("uid"), u));
    let mut out: Vec<Value> = uids.iter().filter_map(|u| by_uid(u).cloned()).collect();
    for p in list {
        let shown = uids.iter().any(|u| is_str(p.get("uid"), u));
        if !shown {
            out.push(p.clone());
        }
    }
    out
}

fn obj(v: &mut Value) -> &mut Map<String, Value> {
    if !v.is_object() {
        *v = Value::Object(Map::new());
    }
    match v {
        Value::Object(m) => m,
        _ => unreachable!(),
    }
}

fn empty_hand(hand: &str) -> Value {
    if hand == "habil" {
        json!({"ref": "a:desarmado"})
    } else {
        json!({"ref": "nada"})
    }
}

/// `limparArsenal`: descarta as peças que ninguém usa e que não guardam nada
/// do jogador (sem imagem, sem ajuste, sem nome de peça personalizada).
pub fn clean_arsenal(t: &EditorTables, sheet: &mut Value) {
    let used: Vec<Value> = sheet
        .get("conjuntos")
        .and_then(Value::as_array)
        .map(|sets| {
            sets.iter()
                .flat_map(|cj| ["habil", "inabil"].map(|h| cj.get(h).and_then(|s| s.get("uid"))))
                .flatten()
                .filter(|u| truthy(Some(u)))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let cat = t.sheet_tables().catalog();
    let list: Vec<Value> = sheet
        .get("arsenal")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let kept: Vec<Value> = list
        .into_iter()
        .filter(|p| {
            used.iter().any(|u| strict_eq(Some(u), p.get("uid")))
                || truthy(p.get("img"))
                || truthy(p.get("mod"))
                || differs_from_origin(
                    cat,
                    p,
                    &item_fields(hand_item(t.sheet_tables(), Some(p)).kind),
                )
                || (is_custom_piece(Some(p)) && truthy(p.get("nome")))
        })
        .collect();
    obj(sheet).insert("arsenal".into(), Value::Array(kept));
}

// ----------------------------------------------------------------- arsenal

/// O item escolhido para entrar no arsenal (depois do diálogo de escolha): a
/// arma personalizada nasce com nome vazio, dado 1, dano 0 e acerto -2; o
/// escudo personalizado, com nome vazio; os do catálogo, só com o `ref`. Entra
/// no fim, sem procurar peça igual (o `pecaPara` procura; este botão não). O
/// `uid` vem de fora.
pub fn add_to_arsenal(sheet: &mut Value, reference: &str, uid: &str) {
    let piece = match reference {
        CUSTOM_WEAPON => {
            json!({"uid": uid, "ref": reference, "nome": "", "dado": 1, "danoBonus": 0, "acerto": -2})
        }
        CUSTOM_SHIELD => json!({"uid": uid, "ref": reference, "nome": ""}),
        _ => json!({"uid": uid, "ref": reference}),
    };
    let arsenal = obj(sheet)
        .entry("arsenal".to_owned())
        .or_insert(Value::Null);
    if !truthy(Some(arsenal)) {
        *arsenal = json!([]);
    }
    if let Value::Array(a) = arsenal {
        a.push(piece);
    }
}

/// O descarte de uma peça do arsenal (depois da confirmação): a mão que a
/// segurava fica livre, a peça sai e as mãos se sincronizam. Sem a peça,
/// nada muda.
pub fn remove_from_arsenal(t: &EditorTables, sheet: &mut Value, uid: &str) {
    let exists = sheet
        .get("arsenal")
        .and_then(Value::as_array)
        .is_some_and(|a| a.iter().any(|p| is_str(p.get("uid"), uid)));
    if !exists {
        return;
    }
    let cat = t.sheet_tables().catalog();
    cat.mark_hand_use(sheet, uid, HandRole::None);
    let kept: Vec<Value> = sheet
        .get("arsenal")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter(|p| !strict_eq(p.get("uid"), Some(&json!(uid))))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    obj(sheet).insert("arsenal".into(), Value::Array(kept));
    cat.sync_slots(sheet);
}

/// O campo de nome da peça do arsenal: grava o nome e sincroniza as mãos.
pub fn rename_arsenal_piece(t: &EditorTables, sheet: &mut Value, uid: &str, name: &str) {
    let Some(Value::Array(a)) = sheet.get_mut("arsenal") else {
        return;
    };
    let Some(Value::Object(p)) = a.iter_mut().find(|p| is_str(p.get("uid"), uid)) else {
        return;
    };
    p.insert("nome".into(), json!(name));
    t.sheet_tables().catalog().sync_slots(sheet);
}

// --------------------------------------------------------------- conjuntos

/// O seletor de uma mão (`select[data-conj-sel]`): `value` é uma referência
/// do catálogo (`a:espada-longa`, `c`, `ce`) ou `nada`. A peça de um objeto só:
/// se já estava na outra mão do conjunto, sai de lá; a arma de duas mãos
/// sobe para a hábil e libera a inábil. Depois, o arsenal perde o que ninguém
/// usa. `uid` é o da peça nova, se ela for criada (o `pecaPara`).
pub fn set_hand_piece(
    t: &EditorTables,
    sheet: &mut Value,
    set_index: usize,
    hand: &str,
    value: &str,
    uid: &str,
) {
    let cat = t.sheet_tables().catalog();
    let has_set = sheet
        .get("conjuntos")
        .and_then(Value::as_array)
        .is_some_and(|s| set_index < s.len());
    if !has_set || (hand != "habil" && hand != "inabil") {
        return;
    }
    if value == "nada" {
        sheet["conjuntos"][set_index][hand] = empty_hand(hand);
    } else {
        let piece = piece_for(sheet, value, uid);
        let (p_uid, p_ref) = (
            piece.get("uid").cloned().unwrap_or(Value::Null),
            piece.get("ref").cloned().unwrap_or(Value::Null),
        );
        let other = if hand == "habil" { "inabil" } else { "habil" };
        let cj = &mut sheet["conjuntos"][set_index];
        let in_other = cj
            .get(other)
            .and_then(|o| o.get("uid"))
            .is_some_and(|u| strict_eq(Some(u), Some(&p_uid)));
        if in_other {
            cj[other] = empty_hand(other);
        }
        cj[hand] = json!({"uid": p_uid, "ref": p_ref});
        cat.freeze_items(sheet, false);
        // a peça de agora, já com o item (o `congelarItens` mexe na ficha toda)
        let now = sheet
            .get("arsenal")
            .and_then(Value::as_array)
            .and_then(|a| a.iter().find(|p| strict_eq(p.get("uid"), Some(&p_uid))))
            .cloned();
        if now.as_ref().is_some_and(|p| cat.two_handed(p)) {
            let cj = &mut sheet["conjuntos"][set_index];
            if hand == "inabil" {
                cj["habil"] = json!({"uid": p_uid, "ref": p_ref});
            }
            cj["inabil"] = json!({"ref": "nada"});
        }
    }
    clean_arsenal(t, sheet);
    cat.sync_slots(sheet);
}

/// O botão "em uso" de um conjunto: só ele fica ativo.
pub fn activate_set(sheet: &mut Value, index: usize) {
    if let Some(Value::Array(sets)) = sheet.get_mut("conjuntos") {
        for (j, c) in sets.iter_mut().enumerate() {
            obj(c).insert("ativo".into(), json!(j == index));
        }
    }
}

/// O botão de remover um conjunto: se nenhum ficar em uso, o primeiro fica.
pub fn remove_set(sheet: &mut Value, index: usize) {
    let Some(Value::Array(sets)) = sheet.get_mut("conjuntos") else {
        return;
    };
    if index < sets.len() {
        sets.remove(index);
    }
    if !sets.iter().any(|c| truthy(c.get("ativo")))
        && let Some(first) = sets.first_mut()
    {
        obj(first).insert("ativo".into(), json!(true));
    }
}

/// O botão de adicionar um conjunto: mãos livres, fora de uso.
pub fn add_set(sheet: &mut Value) {
    let sets = obj(sheet)
        .entry("conjuntos".to_owned())
        .or_insert(Value::Null);
    if !truthy(Some(sets)) {
        *sets = json!([]);
    }
    if let Value::Array(a) = sets {
        a.push(json!({"habil": {"ref": "a:desarmado"}, "inabil": {"ref": "nada"}, "ativo": false}));
    }
}

// ------------------------------------------------------------ o ajuste

/// De onde vem a peça que o ajuste edita: do arsenal (`ars`) ou das
/// armaduras (`arm`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceGroup {
    Arsenal,
    Armor,
}

impl PieceGroup {
    /// O prefixo da chave do ajuste (`ars:<uid>`, `arm:<uid>`).
    pub fn prefix(self) -> &'static str {
        match self {
            PieceGroup::Arsenal => "ars",
            PieceGroup::Armor => "arm",
        }
    }
}

/// A peça do ajuste: onde mora e que campos abre.
#[derive(Debug, Clone, PartialEq)]
pub struct EditTarget {
    pub group: PieceGroup,
    pub uid: String,
    pub fields: Vec<EditField>,
}

fn piece_index(sheet: &Value, group: PieceGroup, uid: &str) -> Option<usize> {
    let list = match group {
        PieceGroup::Arsenal => sheet.get("arsenal"),
        PieceGroup::Armor => sheet.get("equip").and_then(|e| e.get("armaduras")),
    }?;
    list.as_array()?
        .iter()
        .position(|p| is_str(p.get("uid"), uid))
}

fn piece_mut<'a>(sheet: &'a mut Value, group: PieceGroup, uid: &str) -> Option<&'a mut Value> {
    let i = piece_index(sheet, group, uid)?;
    match group {
        PieceGroup::Arsenal => sheet.get_mut("arsenal")?.get_mut(i),
        PieceGroup::Armor => sheet.get_mut("equip")?.get_mut("armaduras")?.get_mut(i),
    }
}

/// `alvoMod`: a peça de uma chave do ajuste (`ars:<uid>` ou `arm:<uid>`) e os
/// campos dela. `None` quando a peça não existe, não tem item ou é "nada".
pub fn edit_target(t: &EditorTables, sheet: &Value, key: &str) -> Option<EditTarget> {
    let mut parts = key.split(':');
    let prefix = parts.next().unwrap_or("");
    let uid = parts.next().unwrap_or("");
    match prefix {
        "arm" => {
            let i = piece_index(sheet, PieceGroup::Armor, uid)?;
            let piece = &sheet["equip"]["armaduras"][i];
            t.sheet_tables().catalog().piece_item(Some(piece))?;
            Some(EditTarget {
                group: PieceGroup::Armor,
                uid: uid.to_owned(),
                fields: ARMOR_EDIT_FIELDS.to_vec(),
            })
        }
        "ars" => {
            let i = piece_index(sheet, PieceGroup::Arsenal, uid)?;
            let kind = hand_item(t.sheet_tables(), Some(&sheet["arsenal"][i])).kind;
            if kind == HandKind::Nothing {
                return None;
            }
            Some(EditTarget {
                group: PieceGroup::Arsenal,
                uid: uid.to_owned(),
                fields: item_fields(kind),
            })
        }
        _ => None,
    }
}

/// O texto de um campo do ajuste como valor (`aoEditar`): `Some(None)` apaga
/// o campo, `None` não grava nada (campo obrigatório vazio, ou número que não
/// é número). A lista se separa por vírgula; o número usa a vírgula como
/// ponto, arredonda ao passo do campo e fica entre o mínimo e o máximo.
pub fn parse_field_input(field: &EditField, raw: &str) -> Option<Option<Value>> {
    let text = raw.trim_matches(is_js_space);
    match field.kind {
        FieldKind::List => Some(Some(Value::Array(
            text.split(',')
                .map(|t| t.trim_matches(is_js_space))
                .filter(|t| !t.is_empty())
                .map(|t| json!(t))
                .collect(),
        ))),
        FieldKind::Choice => {
            if text.is_empty() {
                if field.optional { Some(None) } else { None }
            } else {
                Some(Some(json!(text)))
            }
        }
        FieldKind::Number => {
            if text.is_empty() {
                return if field.optional { Some(None) } else { None };
            }
            let n = string_to_number(&text.replacen(',', ".", 1));
            if !n.is_finite() {
                return None;
            }
            let step = field.step.unwrap_or(1.0);
            let v = js::max(field.min, js::min(field.max, js::round(n / step) * step));
            Some(Some(num(v)))
        }
    }
}

/// O `aoEditar` do painel de ajuste: `key` é `<ars|arm>:<uid>:<campo>` e
/// `raw` o texto do campo. Grava o valor no item da peça (que ganha item
/// inteiro antes, se ainda não tinha) e sincroniza as mãos.
pub fn edit_piece_field(t: &EditorTables, sheet: &mut Value, key: &str, raw: &str) {
    let Some((piece_key, field_key)) = key.rsplit_once(':') else {
        return;
    };
    let Some(target) = edit_target(t, sheet, piece_key) else {
        return;
    };
    let Some(field) = target.fields.iter().find(|f| f.key == field_key) else {
        return;
    };
    let Some(value) = parse_field_input(field, raw) else {
        return;
    };
    let cat = t.sheet_tables().catalog();
    write_piece_field(cat, sheet, &target, field, value);
}

/// O `escreve` do alvo: escreve no item da peça (dando item inteiro a todas
/// as peças antes, se ela não o tinha) e sincroniza as mãos.
pub fn write_piece_field(
    cat: &Catalog,
    sheet: &mut Value,
    target: &EditTarget,
    field: &EditField,
    value: Option<Value>,
) {
    let has_item = |sheet: &Value| {
        piece_index(sheet, target.group, &target.uid).is_some_and(|i| {
            let list = match target.group {
                PieceGroup::Arsenal => &sheet["arsenal"],
                PieceGroup::Armor => &sheet["equip"]["armaduras"],
            };
            truthy(list[i].get("item"))
        })
    };
    if !has_item(sheet) {
        cat.freeze_items(sheet, false);
    }
    if !has_item(sheet) {
        return;
    }
    if let Some(piece) = piece_mut(sheet, target.group, &target.uid)
        && let Some(item) = piece.get_mut("item")
    {
        write_edit_field(item, field, value);
    }
    cat.sync_slots(sheet);
}

/// O botão de restaurar do painel de ajuste (`limpa`): tira o `mod`, devolve
/// o item ao do livro (a peça personalizada volta ao padrão) e sincroniza.
pub fn reset_piece(t: &EditorTables, sheet: &mut Value, key: &str) {
    let Some(target) = edit_target(t, sheet, key) else {
        return;
    };
    let cat = t.sheet_tables().catalog();
    if let Some(piece) = piece_mut(sheet, target.group, &target.uid) {
        if let Value::Object(m) = piece {
            m.shift_remove("mod");
        }
        cat.restore_item(piece);
    }
    cat.sync_slots(sheet);
}
