//! A troca de ids de arma numa ficha gravada (port de `migrarRefsDeArma`, de
//! `ficha-migra.ts`): o `ref` `a:<id>` das mãos de cada conjunto e das peças
//! do arsenal, e o `equip.arma` antigo. O ajuste do slot fica como está: é
//! troca de id, não de números.

use serde_json::Value;

/// `RENOMES_ARMA`: os ids de arma que saíram do catálogo e o id que ficou no
/// lugar.
pub const WEAPON_RENAMES: [(&str, &str); 2] =
    [("dardos", "plumbata"), ("glaive-serrilhada", "glaive")];

/// Troca os ids de arma da ficha pela tabela (na repetida, vale a última
/// linha, como o `Map` do TypeScript). Aceita a ficha na forma de leitura;
/// o que não é objeto fica como está.
pub fn migrate_weapon_refs(sheet: &mut Value, table: &[(&str, &str)]) {
    let renamed = |id: &str| -> String {
        table
            .iter()
            .rev()
            .find(|(old, _)| *old == id)
            .map_or(id, |(_, new)| *new)
            .to_owned()
    };
    let fix_ref = |slot: &mut Value| {
        if let Some(Value::String(r)) = slot.get_mut("ref")
            && let Some(id) = r.strip_prefix("a:")
        {
            *r = format!("a:{}", renamed(id));
        }
    };
    let Value::Object(s) = sheet else {
        return;
    };
    if let Some(Value::Object(equip)) = s.get_mut("equip")
        && let Some(Value::String(w)) = equip.get_mut("arma")
    {
        *w = renamed(w);
    }
    if let Some(Value::Array(sets)) = s.get_mut("conjuntos") {
        for cj in sets.iter_mut() {
            for hand in ["habil", "inabil"] {
                if let Some(slot @ Value::Object(_)) = cj.get_mut(hand) {
                    fix_ref(slot);
                }
            }
        }
    }
    if let Some(Value::Array(list)) = s.get_mut("arsenal") {
        for p in list.iter_mut().filter(|p| p.is_object()) {
            fix_ref(p);
        }
    }
}
