//! O catálogo da ficha montado pelos tipos do `centelha-data` é o mesmo que o
//! montado pelo texto dos arquivos, para o `content/` inteiro: as peças
//! achatadas, os envelopes e a ordem das chaves de cada um.

mod common;

use centelha_data::{
    Armor, Shield, Weapon, load_armors, load_shields, load_weapons, parse_weapons,
};
use centelha_engine::sheet::{Catalog, ItemKind};
use common::repo;

struct Both {
    json: Catalog,
    typed: Catalog,
    weapons: Vec<Weapon>,
    armors: Vec<Armor>,
    shields: Vec<Shield>,
}

fn both() -> Both {
    let content = repo().join("content");
    let read = |name: &str| std::fs::read_to_string(content.join("data").join(name)).unwrap();
    let json = Catalog::from_json(
        &read("armas.json"),
        &read("armaduras.json"),
        &read("escudos.json"),
    )
    .unwrap();
    let weapons = load_weapons(&content).unwrap();
    let armors = load_armors(&content).unwrap();
    let shields = load_shields(&content).unwrap();
    let typed = Catalog::from_data(&weapons, &armors, &shields).unwrap();
    Both {
        json,
        typed,
        weapons,
        armors,
        shields,
    }
}

#[test]
fn the_two_constructors_give_the_same_catalog() {
    let b = both();
    assert_eq!(b.json, b.typed);
    // 48 armas (uma só de criatura), 14 armaduras e 8 escudos
    assert_eq!(b.weapons.len(), 48);
    assert_eq!(b.armors.len(), 14);
    assert_eq!(b.shields.len(), 8);
}

/// A ordem das chaves faz parte da paridade (a cópia da peça vai para a
/// ficha): confere também o texto de cada peça, achatada e em envelope.
#[test]
fn every_piece_has_the_same_keys_in_the_same_order() {
    let b = both();
    let text = |v: Option<&serde_json::Value>| serde_json::to_string(v.unwrap()).unwrap();
    for w in &b.weapons {
        assert_eq!(
            text(b.json.weapon(&w.id)),
            text(b.typed.weapon(&w.id)),
            "{}",
            w.id
        );
        let (j, t) = (
            b.json.catalog_copy(ItemKind::Weapon, &w.id),
            b.typed.catalog_copy(ItemKind::Weapon, &w.id),
        );
        assert_eq!(j, t, "{}", w.id);
        assert_eq!(text(j.as_ref()), text(t.as_ref()), "{}", w.id);
    }
    for a in &b.armors {
        assert_eq!(
            text(b.json.armor(&a.id)),
            text(b.typed.armor(&a.id)),
            "{}",
            a.id
        );
        let (j, t) = (
            b.json.catalog_copy(ItemKind::Armor, &a.id),
            b.typed.catalog_copy(ItemKind::Armor, &a.id),
        );
        assert_eq!(text(j.as_ref()), text(t.as_ref()), "{}", a.id);
    }
    for s in &b.shields {
        assert_eq!(
            text(b.json.shield(&s.id)),
            text(b.typed.shield(&s.id)),
            "{}",
            s.id
        );
        let (j, t) = (
            b.json.catalog_copy(ItemKind::Shield, &s.id),
            b.typed.catalog_copy(ItemKind::Shield, &s.id),
        );
        assert_eq!(text(j.as_ref()), text(t.as_ref()), "{}", s.id);
    }
}

/// O teste falha se os dois divergirem: um número trocado, ou só a ordem das
/// chaves, numa arma lida pelos tipos dá outro catálogo.
#[test]
fn a_changed_value_is_a_different_catalog() {
    let b = both();
    let content = repo().join("content");
    let mut list: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(content.join("data/armas.json")).unwrap())
            .unwrap();
    list[0]["arma"]["dado"] = serde_json::json!(99);
    let changed = parse_weapons(&serde_json::to_string(&list).unwrap()).unwrap();
    assert_eq!(changed[0].stats.as_ref().unwrap().dice, 99);
    let other = Catalog::from_data(&changed, &b.armors, &b.shields).unwrap();
    assert_ne!(other, b.json);
    // a ordem das chaves de uma peça também conta: trocar o `nome` de lugar
    // dá outro catálogo, e o texto da peça muda
    let mut reordered = list.clone();
    let first = reordered[0].as_object_mut().unwrap();
    let name = first.shift_remove("nome").unwrap();
    first.insert("nome".into(), name);
    let swapped = parse_weapons(&serde_json::to_string(&reordered).unwrap()).unwrap();
    let c = Catalog::from_data(&swapped, &b.armors, &b.shields).unwrap();
    let id = &b.weapons[0].id;
    let text = |v: Option<&serde_json::Value>| serde_json::to_string(v.unwrap()).unwrap();
    assert_ne!(c, b.json);
    assert_ne!(text(c.weapon(id)), text(b.json.weapon(id)));
}
