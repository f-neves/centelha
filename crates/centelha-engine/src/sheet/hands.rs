//! As mãos na ficha interativa: a peça nova do arsenal, o texto do pool e o
//! ataque duplo da empunhadura dupla (port de `ficha-engine.ts`: o
//! `pecaPara`, o `poolTxt` e o `duplaConj`).

use serde_json::{Value, json};

use super::numbers::{DualWield, HandSet};
use super::text::signed_number;
use super::value::{get, is_str, to_js_string, truthy};

/// `pecaPara`: a peça do arsenal para o `ref` escolhido. A personalizada é
/// sempre nova: a arma (`c`) nasce com nome vazio, dado 1, dano 0 e acerto
/// -2; o escudo (`ce`), com nome vazio. A do catálogo reaproveita a peça do
/// arsenal com o mesmo `ref`, ou nasce só com ele. A peça nova entra no fim
/// do arsenal; o `uid` dela é de quem chama (a fonte sorteia um).
pub fn piece_for(s: &mut Value, reference: &str, uid: &str) -> Value {
    let Value::Object(m) = s else {
        return Value::Null;
    };
    let arsenal = m.entry("arsenal").or_insert(Value::Null);
    if !truthy(Some(arsenal)) {
        *arsenal = json!([]);
    }
    let Value::Array(list) = arsenal else {
        return Value::Null;
    };
    let new = match reference {
        "c" => {
            json!({"uid": uid, "ref": reference, "nome": "", "dado": 1, "danoBonus": 0, "acerto": -2})
        }
        "ce" => json!({"uid": uid, "ref": reference, "nome": ""}),
        _ => {
            if let Some(p) = list.iter().find(|x| is_str(get(Some(x), "ref"), reference)) {
                return p.clone();
            }
            json!({"uid": uid, "ref": reference})
        }
    };
    list.push(new.clone());
    new
}

/// `poolTxt`: o pool de uma jogada. Sem dado nenhum, o livro escreve "nenhum
/// dado, total fixo 2" (ou 0); o fixo vai com o sinal.
pub fn pool_text(dice: f64, roll_bonus: f64, flat: f64) -> String {
    let head = if dice > 0.0 {
        format!(
            "{}d6{}",
            crate::js::number_to_string(dice),
            if roll_bonus != 0.0 && !roll_bonus.is_nan() {
                "+2"
            } else {
                ""
            }
        )
    } else if roll_bonus != 0.0 && !roll_bonus.is_nan() {
        "nenhum dado, total fixo 2".to_owned()
    } else {
        "nada, total fixo 0".to_owned()
    };
    if flat != 0.0 && !flat.is_nan() {
        format!("{head} {}", signed_number(flat))
    } else {
        head
    }
}

/// Uma jogada de ataque: os dados (nunca abaixo de zero), o +2 da soma ímpar
/// e o fixo; e o dano: o dado da arma e o fixo dele.
#[derive(Debug, Clone, PartialEq)]
pub struct DualAttackHand {
    pub dice: f64,
    pub roll_bonus: f64,
    pub flat: f64,
    pub damage_die: Option<Value>,
    pub damage_bonus: f64,
}

/// O ataque duplo (`duplaConj`): a mão hábil a -1d6 e a inábil a -N d6, cada
/// uma com o seu dano.
#[derive(Debug, Clone, PartialEq)]
pub struct DualAttack {
    pub main: DualAttackHand,
    pub off: DualAttackHand,
}

/// O ataque duplo a partir dos números do conjunto (os dados, o +2 e o fixo
/// da jogada da mão hábil, o dado da arma dela e a segunda arma).
pub fn dual_attack_from(
    dice: f64,
    roll_bonus: f64,
    flat: f64,
    main_die: Option<Value>,
    dual: &DualWield,
) -> DualAttack {
    let p0 = |n: f64| crate::js::max(0.0, n);
    DualAttack {
        main: DualAttackHand {
            dice: p0(dice - 1.0),
            roll_bonus,
            flat,
            damage_die: main_die,
            damage_bonus: dual.main_bonus,
        },
        off: DualAttackHand {
            dice: p0(dual.off_dice - dual.off_penalty),
            roll_bonus: dual.off_roll_bonus,
            flat: dual.off_flat,
            damage_die: dual.off_die.clone(),
            damage_bonus: dual.off_bonus,
        },
    }
}

/// O ataque duplo do conjunto, quando a mão inábil também é arma.
pub fn dual_attack(set: &HandSet) -> Option<DualAttack> {
    let dual = set.dual.as_ref()?;
    Some(dual_attack_from(
        set.dice,
        set.roll_bonus,
        set.flat,
        set.attack_weapon.get("dado").cloned(),
        dual,
    ))
}

/// O dano de uma mão no ataque duplo: o dado da arma em d6 e o fixo com o
/// sinal (sem o fixo quando é zero).
pub fn dual_damage_text(die: Option<&Value>, bonus: f64) -> String {
    let d = to_js_string(die);
    if bonus != 0.0 && !bonus.is_nan() {
        format!("{d}d6 {}", signed_number(bonus))
    } else {
        format!("{d}d6")
    }
}
