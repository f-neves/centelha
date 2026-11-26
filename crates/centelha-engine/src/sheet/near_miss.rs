//! O Quase-Acerto (port de `quase-acerto.ts`): a classe da arma pelo dano
//! médio, o que a arma e as armaduras põem na conta, o raspão do encontro e a
//! saída do ataque (acerto, raspão ou erro).

use serde_json::Value;

use super::SheetTables;
use super::numbers::{NearMiss, average_damage_of_expr};
use super::value::{get, or, property_key, to_number, truthy};
use crate::js;

/// A metade da arma no Quase-Acerto (`qaDaArma`).
#[derive(Debug, Clone, PartialEq)]
pub struct WeaponNearMiss {
    /// `leve`, `media` ou `pesada`; `None` sem arma nem expressão que se leia.
    pub class: Option<String>,
    pub bonus: f64,
    pub damage: f64,
}

/// A metade das armaduras (`qaDeArmaduras`).
#[derive(Debug, Clone, PartialEq)]
pub struct ArmorNearMiss {
    pub bonus: f64,
    pub reduction: f64,
    /// As classes das peças que contam, na ordem (`nenhuma` na peça sem
    /// classe).
    pub classes: Vec<String>,
}

/// O que o atacante leva ao encontro (`quaseAcertoDoEncontro`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearMissAttacker {
    pub weapon_bonus: f64,
    pub weapon_damage: f64,
    pub spark: f64,
}

/// O que o alvo leva ao encontro.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearMissTarget {
    pub armor_bonus: f64,
    pub armor_reduction: f64,
    pub spark: f64,
}

/// A margem do raspão e o dano dele.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Graze {
    pub margin: f64,
    pub damage: f64,
}

/// O raspão entre um atacante e um alvo (`quaseAcerto`, o `Raspao`).
#[derive(Debug, Clone, PartialEq)]
pub struct NearMissOutcome {
    pub margin: f64,
    pub damage: f64,
    /// O dano do raspão ignora a Absorção (`quaseAcerto.ignoraSoak`, que só
    /// desliga com `false`).
    pub ignores_soak: bool,
    pub weapon: WeaponNearMiss,
    pub armor: ArmorNearMiss,
}

/// A saída de um ataque (`Saida`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackOutcome {
    Hit,
    Graze,
    Miss,
}

impl AttackOutcome {
    /// O nome gravado (`acerto`, `raspao`, `erro`).
    pub fn id(self) -> &'static str {
        match self {
            AttackOutcome::Hit => "acerto",
            AttackOutcome::Graze => "raspao",
            AttackOutcome::Miss => "erro",
        }
    }
}

/// `Number(x) || 0` de um número opcional da tabela.
fn or_zero(x: Option<f64>) -> f64 {
    x.filter(|v| !v.is_nan()).unwrap_or(0.0)
}

impl SheetTables {
    /// `qaDaArma`: a classe da arma pelo dano médio (o do catálogo, quando a
    /// arma está nele pelo id ou pelo nome, ou o da expressão do dano) e o que
    /// a classe dá.
    pub fn weapon_near_miss(&self, weapon: Option<&Value>, damage: Option<&str>) -> WeaponNearMiss {
        let q = self.rules.near_miss.as_ref();
        let limits = q.and_then(|q| q.class_by_average_damage);
        let light = limits.and_then(|l| l.light_up_to).unwrap_or(2.0);
        let medium = limits.and_then(|l| l.medium_up_to).unwrap_or(5.5);
        let from_catalog = self.catalog.weapon_lookup(weapon).and_then(|w| {
            let dice = to_number(get(Some(w), "dado"));
            let b = to_number(get(Some(w), "danoBonus"));
            let b = if b.is_nan() { 0.0 } else { b };
            (dice.is_finite() && dice > 0.0).then_some(dice * 3.5 + b)
        });
        let avg = from_catalog.or_else(|| average_damage_of_expr(damage.unwrap_or("")));
        let class = avg.map(|m| {
            if m <= light {
                "leve"
            } else if m <= medium {
                "media"
            } else {
                "pesada"
            }
        });
        let w = class.and_then(|c| q.and_then(|q| q.by_weapon_class.get(c)));
        WeaponNearMiss {
            class: class.map(str::to_owned),
            bonus: or_zero(w.and_then(|w| w.bonus)),
            damage: or_zero(w.and_then(|w| w.damage)),
        }
    }

    /// `qaDeArmaduras`: soma o bônus das peças e fica com a maior redução. A
    /// peça de classe que a tabela não tem não conta.
    pub fn armor_near_miss(&self, armors: &[Value]) -> ArmorNearMiss {
        let q = self.rules.near_miss.as_ref();
        let mut a = ArmorNearMiss {
            bonus: 0.0,
            reduction: 0.0,
            classes: Vec::new(),
        };
        for p in armors {
            let c = or(get(Some(p), "classe"), None);
            let c = if truthy(c) {
                property_key(c)
            } else {
                "nenhuma".to_owned()
            };
            let Some(t) = q.and_then(|q| q.by_armor_class.get(&c)) else {
                continue;
            };
            a.bonus += or_zero(t.bonus);
            a.reduction = js::max(a.reduction, or_zero(t.reduction));
            a.classes.push(c);
        }
        a
    }

    /// `qaDaPeca`: o Quase-Acerto da arma e o das armaduras, juntos.
    pub fn near_miss(&self, weapon: Option<&Value>, damage: &str, armors: &[Value]) -> NearMiss {
        let w = self.weapon_near_miss(weapon, Some(damage));
        let a = self.armor_near_miss(armors);
        NearMiss {
            weapon_bonus: w.bonus,
            weapon_damage: w.damage,
            weapon_class: w.class,
            armor_bonus: a.bonus,
            armor_reduction: a.reduction,
            armor_classes: a.classes,
        }
    }

    /// `quaseAcerto`: o raspão de um atacante (a arma pelo id ou nome, ou a
    /// expressão do dano, e a Centelha) contra as armaduras do alvo. A
    /// Centelha do alvo não entra (0).
    pub fn near_miss_between(
        &self,
        weapon: Option<&Value>,
        damage: Option<&str>,
        spark: Option<f64>,
        armors: &[Value],
    ) -> NearMissOutcome {
        let weapon = self.weapon_near_miss(weapon, damage);
        let armor = self.armor_near_miss(armors);
        let g = encounter_near_miss(
            NearMissAttacker {
                weapon_bonus: weapon.bonus,
                weapon_damage: weapon.damage,
                spark: spark.unwrap_or(0.0),
            },
            NearMissTarget {
                armor_bonus: armor.bonus,
                armor_reduction: armor.reduction,
                spark: 0.0,
            },
        );
        NearMissOutcome {
            margin: g.margin,
            damage: g.damage,
            ignores_soak: self.rules.near_miss.as_ref().and_then(|q| q.ignores_soak) != Some(false),
            weapon,
            armor,
        }
    }
}

/// `quaseAcertoDoEncontro`: a margem do raspão (o bônus da arma mais o da
/// armadura) e o dano (o da arma menos a redução da armadura, mais a
/// diferença de Centelha, nunca abaixo de 0).
pub fn encounter_near_miss(attacker: NearMissAttacker, target: NearMissTarget) -> Graze {
    Graze {
        margin: attacker.weapon_bonus + target.armor_bonus,
        damage: js::max(
            0.0,
            attacker.weapon_damage - target.armor_reduction + attacker.spark - target.spark,
        ),
    }
}

/// `errouPor`: quanto faltou para acertar (0 ou menos é acerto).
pub fn missed_by(total: f64, defense: f64) -> f64 {
    defense - total + 1.0
}

/// `raspa`: o que faltou cabe na margem do raspão.
pub fn grazes(missed: f64, margin: f64) -> bool {
    missed > 0.0 && missed <= margin
}

/// `saidaDoAtaque`: acerto, raspão ou erro.
pub fn attack_outcome(total: f64, defense: f64, margin: f64) -> AttackOutcome {
    let missed = missed_by(total, defense);
    if missed <= 0.0 {
        AttackOutcome::Hit
    } else if grazes(missed, margin) {
        AttackOutcome::Graze
    } else {
        AttackOutcome::Miss
    }
}
