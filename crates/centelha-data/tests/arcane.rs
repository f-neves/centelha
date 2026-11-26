//! O ramo `arcano` de `regras.json`: os 143 caminhos que a página das regras
//! das Artes lê, os números tipados e o texto igual ao do arquivo.

use std::path::Path;

use centelha_data::{ImprovisationCost, Node, load_rules};
use serde_json::Value;

fn content() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

/// A lista de caminhos da página (`arcano-caminhos.txt`): o apelido que a
/// página usa, o caminho dentro dele e o tipo esperado.
fn expected_paths() -> Vec<(String, String)> {
    let text = include_str!("arcano-caminhos.txt");
    text.lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let (path, kind) = l.rsplit_once(' ').unwrap();
            (path.to_owned(), kind.to_owned())
        })
        .collect()
}

/// O apelido da página para o caminho a partir de `arcano`.
fn root_of(alias: &str) -> &'static str {
    match alias {
        "G" => "improviso.graus",
        "COMB" => "improviso.combinacoes",
        "MANI" => "improviso.manifestacao",
        "ESTICAR" => "esticar",
        "MOLDES" => "moldes",
        "TEMPO" => "tempoDaArte",
        "RITUAL" => "ritual",
        "FONTE" => "fonteElemental",
        "CURA" => "cura",
        "FRAQ" => "fraquezas",
        "RECUP" => "recuperacaoMana",
        other => panic!("apelido sem ramo: {other}"),
    }
}

fn kind(n: &Node) -> &'static str {
    match n {
        Node::Text(_) => "text",
        Node::Number(_) => "number",
        Node::Bool(_) => "bool",
        Node::Null => "null",
        Node::List(_) => "list",
        Node::Map(_) => "map",
    }
}

#[test]
fn every_page_path_is_read_with_its_type() {
    let rules = load_rules(&content()).unwrap();
    let arcane = rules.arcane.as_ref().unwrap();
    let paths = expected_paths();
    assert_eq!(paths.len(), 143);
    for (path, want) in &paths {
        let (alias, rest) = path.split_once('.').unwrap();
        let full = format!("{}.{rest}", root_of(alias));
        let node = arcane
            .get(&full)
            .unwrap_or_else(|| panic!("sumiu: {path} ({full})"));
        assert_eq!(kind(node), want, "{path}");
        // a leitura tipada concorda com a por caminho
        match want.as_str() {
            "text" => assert!(arcane.text(&full).is_some(), "{path}"),
            "number" => assert!(arcane.number(&full).is_some(), "{path}"),
            "list" => assert!(arcane.list(&full).is_some(), "{path}"),
            other => panic!("tipo fora do esperado em {path}: {other}"),
        }
    }
    let texts = paths.iter().filter(|(_, k)| k == "text").count();
    let lists = paths.iter().filter(|(_, k)| k == "list").count();
    let numbers = paths.iter().filter(|(_, k)| k == "number").count();
    assert_eq!((texts, lists, numbers), (100, 41, 2));
}

#[test]
fn a_missing_path_is_none() {
    let rules = load_rules(&content()).unwrap();
    let a = rules.arcane.as_ref().unwrap();
    for p in [
        "nao-existe",
        "improviso.nao-existe",
        "cura.nota.mais",
        "esticar.tabela.9",
        "esticar.tabela.0.nada",
        "improviso.graus.alcance.99",
    ] {
        assert!(a.get(p).is_none(), "{p}");
    }
    assert_eq!(a.text("improviso.custoPorNivel"), None);
    assert_eq!(a.number("cura.nota"), None);
}

#[test]
fn the_typed_numbers_have_the_json_values() {
    let rules = load_rules(&content()).unwrap();
    let a = rules.arcane.as_ref().unwrap();
    assert_eq!(
        a.improvisation_cost,
        Some(ImprovisationCost {
            standard: 1.0,
            healing: 2.0
        })
    );
    assert_eq!(a.healing_cost_per_level, Some(2.0));
    assert_eq!(a.elemental_source_discount, Some(1.0));

    let rows = &a.stretch_table;
    assert_eq!(rows.len(), 4);
    for (i, r) in rows.iter().enumerate() {
        assert_eq!(r.above, i as f64);
        assert_eq!(r.cost, format!("nível × {}", i + 1));
        assert_eq!(r.speed, format!("×{}", i + 1));
    }

    let m = a.manifestation.as_ref().unwrap();
    assert_eq!(m.openings, [60.0, 90.0, 120.0]);
    assert_eq!(
        (m.default_opening, m.min_slices, m.slices_per_level),
        (60.0, 1.0, 1.0)
    );
    assert_eq!(
        (
            m.side_state.earth,
            m.side_state.metal,
            m.side_state.air,
            m.side_state.mist
        ),
        (0.5, 0.5, 2.0, 2.0)
    );
    assert_eq!((m.side_state.default, m.min_base_height), (1.0, 1.0));

    let d = a.detour.as_ref().unwrap();
    assert_eq!((d.ticks_per_meter, d.min_meters), (1.0, 1.0));
    assert_eq!((d.base_difficulty, d.difficulty_per_meter), (5.0, 5.0));
    assert_eq!((d.no_damage_factor, d.half_fraction), (2.0, 0.5));
    assert_eq!((d.identified_bonus, d.identified_margin_bonus), (2.0, 4.0));

    // a leitura por caminho dá os mesmos números
    assert_eq!(a.number("improviso.custoPorNivel.cura"), Some(2.0));
    assert_eq!(
        a.number("improviso.manifestacao.numeros.aberturaPadrao"),
        Some(60.0)
    );
    // o teto de nível, que já existia, segue igual
    assert!(a.art_level_cap.is_some());
}

/// Confere o nó contra o JSON do arquivo: mesmo tipo, mesmo texto (sem trim),
/// mesmos números, mesmas chaves.
fn same(node: &Node, value: &Value, at: &str) {
    match (node, value) {
        (Node::Text(a), Value::String(b)) => assert_eq!(a, b, "{at}"),
        (Node::Number(a), Value::Number(b)) => assert_eq!(*a, b.as_f64().unwrap(), "{at}"),
        (Node::Bool(a), Value::Bool(b)) => assert_eq!(a, b, "{at}"),
        (Node::Null, Value::Null) => {}
        (Node::List(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{at}");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                same(x, y, &format!("{at}[{i}]"));
            }
        }
        (Node::Map(a), Value::Object(b)) => {
            assert_eq!(a.len(), b.len(), "{at}");
            for (k, x) in a {
                same(
                    x,
                    b.get(k).unwrap_or_else(|| panic!("{at}.{k} sobra")),
                    &format!("{at}.{k}"),
                );
            }
        }
        _ => panic!("tipo diferente em {at}: {node:?} contra {value:?}"),
    }
}

#[test]
fn the_tree_is_the_json_untouched() {
    let rules = load_rules(&content()).unwrap();
    let a = rules.arcane.as_ref().unwrap();
    let raw: Value =
        serde_json::from_str(&std::fs::read_to_string(content().join("data/regras.json")).unwrap())
            .unwrap();
    // o teto de nível tem campo próprio, fora da árvore
    let arcano: Value = raw["arcano"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(k, _)| k.as_str() != "tetoNivelArte")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    same(&a.tree, &arcano, "arcano");
    // e as chaves de cada ramo estão na ordem do arquivo
    let order: Vec<&str> = a
        .tree
        .as_map()
        .unwrap()
        .iter()
        .map(|(k, _)| k.as_str())
        .collect();
    assert_eq!(order.first(), Some(&"nota"));
    let improv: Vec<&str> = a
        .get("improviso")
        .unwrap()
        .as_map()
        .unwrap()
        .iter()
        .map(|(k, _)| k.as_str())
        .collect();
    assert_eq!(
        &improv[..4],
        ["regra", "pontos", "custoPorNivel", "desconto"]
    );
}

#[test]
fn the_null_and_the_flag_of_the_data_are_kept() {
    let rules = load_rules(&content()).unwrap();
    let a = rules.arcane.as_ref().unwrap();
    assert_eq!(
        a.get("moldes.lista.3.curvavel").and_then(Node::as_bool),
        Some(true)
    );
    assert!(a.get("moldes.lista.3.hexagonos").unwrap().is_null());
    assert_eq!(
        a.get("ritual.backlash").and_then(Node::as_bool),
        Some(false)
    );
}
