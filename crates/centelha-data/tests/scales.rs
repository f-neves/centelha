//! As escalas de Centelha e de Força de Vontade de `regras.json` se leem do
//! content/ de hoje, na ordem do arquivo.

use std::path::Path;

use centelha_data::load_rules;

#[test]
fn reads_the_spark_and_willpower_scales() {
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let rules = load_rules(&content).unwrap();

    let spark = &rules.spark_scale;
    assert_eq!(spark.len(), 13);
    let levels: Vec<f64> = spark.iter().map(|s| s.level).collect();
    assert_eq!(levels, (0..=12).map(f64::from).collect::<Vec<_>>());
    assert_eq!(
        (spark[0].label.as_str(), spark[6].label.as_str()),
        ("Mortal", "Semideus")
    );
    assert!(!spark[6].provisional && spark[7].provisional);
    assert!(spark.iter().all(|s| !s.text.is_empty()));

    let will = &rules.willpower_scale;
    let levels: Vec<f64> = will.iter().map(|s| s.level).collect();
    assert_eq!(levels, [0.0, 1.0, 5.0, 7.0, 9.0, 12.0]);
    assert_eq!(
        (will[0].label.as_str(), will[5].label.as_str()),
        ("Sem reserva", "Titânica")
    );
    assert!(will.iter().all(|s| !s.provisional && !s.text.is_empty()));
}
