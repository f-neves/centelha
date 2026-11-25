//! Paridade do Mulberry32 com o TypeScript (tests/parity/mulberry32.json).

use std::path::Path;

use centelha_engine::js::to_uint32;
use centelha_engine::rng::Mulberry32;
use serde::Deserialize;

#[derive(Deserialize)]
struct Reference {
    steps: usize,
    #[serde(rename = "f64Steps")]
    f64_steps: usize,
    sequences: Vec<Sequence>,
}

#[derive(Deserialize)]
struct Sequence {
    /// a semente como o TypeScript a recebeu (número de JavaScript)
    seed: f64,
    #[serde(rename = "seedU32")]
    seed_u32: u32,
    u32: Vec<u32>,
    f64: Vec<f64>,
}

#[test]
fn mulberry32_matches_typescript() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/parity/mulberry32.json");
    let text = std::fs::read_to_string(&path).unwrap();
    let reference: Reference = serde_json::from_str(&text).unwrap();
    assert!(!reference.sequences.is_empty());
    let mut cases = 0;
    for s in &reference.sequences {
        assert_eq!(to_uint32(s.seed), s.seed_u32, "semente {}", s.seed);
        assert_eq!(s.u32.len(), reference.steps);
        assert_eq!(s.f64.len(), reference.f64_steps);

        let mut r = Mulberry32::new(s.seed_u32);
        for (i, want) in s.u32.iter().enumerate() {
            assert_eq!(r.next_u32(), *want, "semente {}, passo {i}", s.seed);
            cases += 1;
        }
        let mut r = Mulberry32::new(s.seed_u32);
        for (i, want) in s.f64.iter().enumerate() {
            let got = r.next_f64();
            assert_eq!(
                got.to_bits(),
                want.to_bits(),
                "semente {}, passo {i}",
                s.seed
            );
            cases += 1;
        }
    }
    println!("mulberry32: {cases} casos de paridade");
}
