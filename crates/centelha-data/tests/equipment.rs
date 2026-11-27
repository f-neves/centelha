//! Os campos de armas, armaduras e escudos que as páginas e as contas leem:
//! uma peça de cada para ver o valor, e as contagens do `content/` para ver
//! que nenhum campo se perde.

use std::path::PathBuf;

use centelha_data::{
    AttackMode, DamageType, RangeBand, Weapon, load_armors, load_shields, load_weapons,
    parse_armors, parse_shields, parse_weapons,
};

fn content() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

fn weapon<'a>(list: &'a [Weapon], id: &str) -> &'a Weapon {
    list.iter()
        .find(|w| w.id == id)
        .unwrap_or_else(|| panic!("sem a arma {id}"))
}

#[test]
fn reads_the_weapon_fields_of_a_ranged_weapon() {
    let list = load_weapons(&content()).unwrap();
    let bow = weapon(&list, "arco-curto");
    let s = bow.stats.as_ref().unwrap();
    assert_eq!((bow.kind.as_deref(), bow.weight), (Some("arma"), Some(0.6)));
    assert_eq!(bow.price.unwrap().copper, Some(100.0));
    assert!(
        bow.description
            .as_deref()
            .unwrap()
            .starts_with("Pequeno e fácil de guardar")
    );
    assert!(!bow.creature_only);
    assert_eq!(s.attribute.as_deref(), Some("percepcao"));
    assert_eq!((s.effective, s.max_distance), (Some(30.0), Some(120.0)));
    assert_eq!(s.range_band, Some(RangeBand::Short));
    assert_eq!(s.free_range_fraction, Some(0.333));
    assert_eq!(
        (s.defense, s.strength_multiplier, s.strength_cap),
        (Some(0.0), Some(1.0), Some(3.0))
    );
    assert_eq!(
        (s.damage_type, s.pierce_level),
        (Some(DamageType::Pierce), Some(1.0))
    );
    assert_eq!(
        s.modes,
        [AttackMode {
            kind: DamageType::Pierce,
            pierce_level: Some(1.0),
            primary: true
        }]
    );
    assert!(!s.no_damage && !s.range_by_mass);
    assert_eq!(
        weapon(&list, "arco-composto")
            .stats
            .as_ref()
            .unwrap()
            .strength_min,
        Some(4.0)
    );
}

#[test]
fn reads_the_special_weapon_flags() {
    let list = load_weapons(&content()).unwrap();
    // a só de criatura (sem preço nem peso), que o alcance sai da massa
    let rock = weapon(&list, "rocha");
    assert!(rock.creature_only);
    assert!(rock.stats.as_ref().unwrap().range_by_mass);
    assert_eq!((rock.price, rock.weight), (None, None));
    // a que não causa dano
    assert!(weapon(&list, "rede").stats.as_ref().unwrap().no_damage);
    // a de três modos, e o que a ficha mostra
    let halberd = weapon(&list, "alabarda").stats.as_ref().unwrap();
    assert_eq!(halberd.sheet_mode, Some(DamageType::Cut));
    let kinds: Vec<DamageType> = halberd.modes.iter().map(|m| m.kind).collect();
    assert_eq!(
        kinds,
        [DamageType::Cut, DamageType::Pierce, DamageType::Impact]
    );
    assert_eq!(halberd.modes[1].pierce_level, Some(1.0));
    assert!(halberd.modes.iter().all(|m| m.primary));
    let secondary = weapon(&list, "adaga").stats.as_ref().unwrap();
    assert_eq!(
        secondary
            .modes
            .iter()
            .map(|m| m.primary)
            .collect::<Vec<_>>(),
        [true, false]
    );
    let timing = weapon(&list, "corrente-farpada")
        .stats
        .as_ref()
        .unwrap()
        .timing
        .unwrap();
    assert_eq!((timing.windup, timing.blow, timing.recovery), (3, 1, 3));
}

#[test]
fn counts_every_weapon_field_of_the_content() {
    let list = load_weapons(&content()).unwrap();
    let stats = || list.iter().filter_map(|w| w.stats.as_ref());
    assert_eq!(list.len(), 48);
    assert_eq!(list.iter().filter(|w| w.creature_only).count(), 1);
    assert_eq!(list.iter().filter(|w| w.price.is_some()).count(), 36);
    assert_eq!(list.iter().filter(|w| w.weight.is_some()).count(), 47);
    assert_eq!(list.iter().filter(|w| w.description.is_some()).count(), 48);
    assert_eq!(stats().count(), 48);
    assert_eq!(stats().filter(|s| s.attribute.is_some()).count(), 48);
    assert_eq!(stats().filter(|s| s.defense.is_some()).count(), 48);
    assert_eq!(stats().filter(|s| s.damage_type.is_some()).count(), 48);
    assert_eq!(stats().filter(|s| s.pierce_level.is_some()).count(), 48);
    assert_eq!(stats().filter(|s| s.effective.is_some()).count(), 21);
    assert_eq!(stats().filter(|s| s.max_distance.is_some()).count(), 13);
    assert_eq!(stats().filter(|s| s.range_band.is_some()).count(), 6);
    assert_eq!(
        stats().filter(|s| s.free_range_fraction.is_some()).count(),
        6
    );
    assert_eq!(stats().filter(|s| s.sheet_mode.is_some()).count(), 2);
    assert_eq!(stats().filter(|s| s.no_damage).count(), 1);
    assert_eq!(stats().filter(|s| s.range_by_mass).count(), 1);
    let modes: Vec<&AttackMode> = stats().flat_map(|s| &s.modes).collect();
    assert_eq!(modes.len(), 65);
    assert_eq!(modes.iter().filter(|m| m.primary).count(), 52);
    assert_eq!(
        modes.iter().filter(|m| m.pierce_level.is_some()).count(),
        28
    );
    // o envelope guarda a peça inteira, com o bloco que o tipo nem lê
    let env = weapon(&list, "arco-curto").envelope();
    assert_eq!(env["arma"]["alcance"], "curto");
    assert!(env["armadura"].is_null() && env["municao"].is_null());
}

#[test]
fn reads_armors_and_shields() {
    let armors = load_armors(&content()).unwrap();
    assert_eq!(armors.len(), 14);
    let none = &armors[0];
    assert_eq!(
        (none.id.as_str(), none.kind.as_deref()),
        ("nenhuma", Some("armadura"))
    );
    let s = none.stats.as_ref().unwrap();
    assert_eq!(s.class.as_deref(), Some("nenhuma"));
    assert_eq!((s.pierce_level, s.penalty), (Some(0.0), Some(0.0)));
    assert_eq!(none.access, Some(10.0));
    let gambeson = armors.iter().find(|a| a.id == "gambeson").unwrap();
    assert_eq!(gambeson.price.unwrap().copper, Some(140.0));
    assert_eq!((gambeson.weight, gambeson.access), (Some(3.5), Some(9.0)));
    let soak = gambeson.stats.as_ref().unwrap().soak.unwrap();
    assert_eq!(
        (soak.impact, soak.cut, soak.pierce),
        (Some(3.0), Some(4.0), Some(1.0))
    );
    assert_eq!(armors.iter().filter(|a| a.price.is_some()).count(), 13);
    assert_eq!(armors.iter().filter(|a| a.access.is_some()).count(), 9);
    assert_eq!(armors.iter().filter(|a| a.weight.is_some()).count(), 14);
    assert!(
        armors
            .iter()
            .all(|a| a.stats.as_ref().is_some_and(|s| s.class.is_some()))
    );
    assert!(armors.iter().all(|a| a.description.is_some()));

    let shields = load_shields(&content()).unwrap();
    assert_eq!(shields.len(), 8);
    let targe = shields.iter().find(|s| s.id == "targe").unwrap();
    assert_eq!(
        (targe.kind.as_deref(), targe.weight, targe.access),
        (Some("escudo"), Some(2.5), Some(7.0))
    );
    assert_eq!(targe.price.unwrap().copper, Some(30.0));
    let st = targe.stats.as_ref().unwrap();
    assert_eq!((st.melee_block, st.penalty), (Some(1.0), Some(0.0)));
    assert_eq!(st.vs_fast_projectile.unwrap().blocks, Some(false));
    assert_eq!(shields.iter().filter(|s| s.price.is_some()).count(), 7);
    assert!(
        shields
            .iter()
            .all(|s| s.access.is_some() && s.weight.is_some())
    );
}

#[test]
fn a_field_of_the_wrong_type_says_where() {
    let bad = r#"[{"id":"x","nome":"X","tags":[],"arma":{"classe":"leve","pericia":"armas","dado":"um"}}]"#;
    let e = parse_weapons(bad).unwrap_err().to_string();
    assert!(e.contains("arma.dado"), "{e}");
    let bad = r#"[{"id":"x","nome":"X","armadura":{"soak":{"impacto":"alto"}}}]"#;
    let e = parse_armors(bad).unwrap_err().to_string();
    assert!(e.contains("armadura.soak.impacto"), "{e}");
    let e = parse_shields(r#"[{"id":"x","nome":"X","peso":"leve"}]"#)
        .unwrap_err()
        .to_string();
    assert!(e.contains("peso"), "{e}");
}
