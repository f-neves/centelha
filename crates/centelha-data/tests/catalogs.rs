//! Os quatro catálogos da ficha e das páginas de catálogo se leem de
//! content/, com as contagens de hoje e as ligações entre eles.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use centelha_data::{load_arts, load_effects, load_paths, load_techniques};

fn content() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

#[test]
fn reads_the_four_catalogs() {
    let c = content();
    let techniques = load_techniques(&c).unwrap();
    let paths = load_paths(&c).unwrap();
    let arts = load_arts(&c).unwrap();
    let effects = load_effects(&c).unwrap();
    assert_eq!(
        (techniques.len(), paths.len(), arts.len(), effects.len()),
        (461, 50, 24, 140)
    );

    // cada Técnica aponta para um Caminho que existe, e cada pré-requisito
    // para uma Técnica que existe
    let path_ids: HashSet<&str> = paths.iter().map(|p| p.id.as_str()).collect();
    let technique_ids: HashSet<&str> = techniques.iter().map(|t| t.id.as_str()).collect();
    for t in &techniques {
        assert!(path_ids.contains(t.path.as_str()), "{}: {}", t.id, t.path);
        for p in &t.prereq {
            assert!(technique_ids.contains(p.as_str()), "{}: {p}", t.id);
        }
        assert!((1.0..=6.0).contains(&t.level), "{}: {}", t.id, t.level);
    }
    // cada Efeito aponta para Artes que existem; cada Arte tem os 6 níveis
    let art_ids: HashSet<&str> = arts.iter().map(|a| a.id.as_str()).collect();
    for e in &effects {
        for a in &e.arts {
            assert!(art_ids.contains(a.id.as_str()), "{}: {}", e.id, a.id);
        }
    }
    for a in &arts {
        assert_eq!(a.levels.len(), 6, "{}", a.id);
    }
    // os módulos fora do jogo: as nove Técnicas do Fôlego
    let out: Vec<_> = techniques.iter().filter(|t| t.module.is_some()).collect();
    assert_eq!(out.len(), 9);
    assert!(out.iter().all(|t| t.module.as_deref() == Some("folego")));
}
