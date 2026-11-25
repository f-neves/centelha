//! As criaturas do bestiário: `content/data/bestiario/<id>.json`.
//!
//! Primeira versão: só os campos que o motor usa para os derivados da criatura
//! (PV, Defesas, Absorção, peso erguido, Tamanho, faixa, Energia, Iniciativa,
//! Mana e os ataques), mais o id, o nome e a ameaça. O resto do JSON (prosa,
//! deslocamento, habilidades especiais, sentidos, elementos, imagens...) é
//! ignorado na leitura por enquanto.
//!
//! A validação repete, para esses campos, o esquema da criatura da fonte do
//! conteúdo (inteiros, faixas, listas fechadas e as regras entre campos).

use std::collections::{BTreeMap, HashSet};

use serde::Deserialize;

use crate::DataError;

/// Os ids de Tamanho, do menor para o maior.
pub const SIZES: [&str; 8] = [
    "minusculo",
    "miudo",
    "pequeno",
    "medio",
    "grande",
    "enorme",
    "imenso",
    "colossal",
];

/// Os perfis de Couraça.
pub const CARAPACE_PROFILES: [&str; 3] = ["rigida", "couro", "mole"];

/// As classes do ataque natural.
pub const ATTACK_CLASSES: [&str; 8] = [
    "leve",
    "media",
    "pesada",
    "haste",
    "haste-media",
    "haste-guerra",
    "distancia",
    "arremesso",
];

/// As Habilidades que o ataque natural pode gravar (as de tiro e arremesso).
pub const RANGED_SKILLS: [&str; 3] = ["disparo", "atirador", "arremesso"];

/// Uma criatura do bestiário.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Creature {
    pub id: String,
    #[serde(rename = "nome")]
    pub name: String,
    #[serde(rename = "categoria")]
    pub category: String,
    #[serde(rename = "ameaca")]
    pub threat: i32,
    #[serde(rename = "centelha")]
    pub spark: i32,
    #[serde(rename = "atributos")]
    pub attributes: Attributes,
    #[serde(rename = "habilidades")]
    pub skills: BTreeMap<String, i32>,
    #[serde(rename = "habilidadesSec")]
    pub secondary_skills: BTreeMap<String, i32>,
    #[serde(rename = "vontade")]
    pub willpower: i32,
    #[serde(rename = "corpo")]
    pub body: Body,
    #[serde(rename = "enxame", default)]
    pub swarm: Option<Swarm>,
    #[serde(rename = "bonusPV", default)]
    pub hp_bonus: Option<i32>,
    #[serde(rename = "couraca")]
    pub carapace: i32,
    #[serde(rename = "perfilCouraca", default)]
    pub carapace_profile: Option<String>,
    #[serde(rename = "artes")]
    pub arts: BTreeMap<String, i32>,
    #[serde(rename = "equipamento")]
    pub equipment: Equipment,
    #[serde(rename = "ataques")]
    pub attacks: Vec<Attack>,
    #[serde(rename = "formaPrincipal", default)]
    pub main_form: Option<MainForm>,
    #[serde(rename = "formas", default)]
    pub forms: Option<Vec<Form>>,
}

/// Os nove Atributos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Attributes {
    #[serde(rename = "forca")]
    pub strength: i32,
    #[serde(rename = "destreza")]
    pub dexterity: i32,
    #[serde(rename = "vigor")]
    pub stamina: i32,
    #[serde(rename = "percepcao")]
    pub perception: i32,
    #[serde(rename = "inteligencia")]
    pub intelligence: i32,
    #[serde(rename = "raciocinio")]
    pub wits: i32,
    #[serde(rename = "influencia")]
    pub influence: i32,
    #[serde(rename = "perspicacia")]
    pub insight: i32,
    #[serde(rename = "compostura")]
    pub composure: i32,
}

/// Os Atributos que uma forma troca (os que ela não diz vêm da principal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
pub struct AttributesPatch {
    #[serde(rename = "forca", default)]
    pub strength: Option<i32>,
    #[serde(rename = "destreza", default)]
    pub dexterity: Option<i32>,
    #[serde(rename = "vigor", default)]
    pub stamina: Option<i32>,
    #[serde(rename = "percepcao", default)]
    pub perception: Option<i32>,
    #[serde(rename = "inteligencia", default)]
    pub intelligence: Option<i32>,
    #[serde(rename = "raciocinio", default)]
    pub wits: Option<i32>,
    #[serde(rename = "influencia", default)]
    pub influence: Option<i32>,
    #[serde(rename = "perspicacia", default)]
    pub insight: Option<i32>,
    #[serde(rename = "compostura", default)]
    pub composure: Option<i32>,
}

/// O corpo: o que entra no Tamanho, na faixa, no PV e no peso erguido.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Body {
    #[serde(rename = "massa", default)]
    pub mass: Option<f64>,
    /// O incorpóreo: sem massa, com o volume no lugar (`semMassa`).
    #[serde(rename = "semMassa", default)]
    pub massless: bool,
    #[serde(default)]
    pub volume: Option<f64>,
    /// A área da silhueta, em m².
    pub area: f64,
    /// A criatura que não ergue peso (`naoErgue`).
    #[serde(rename = "naoErgue", default)]
    pub cannot_lift: bool,
    #[serde(rename = "tamanhoEscolhido", default)]
    pub chosen_size: Option<String>,
    /// A linha de PV escolhida na zona de transição: `acima` ou `abaixo`.
    #[serde(rename = "linhaPV", default)]
    pub hp_line: Option<String>,
    #[serde(rename = "linhaPVMotivo", default)]
    pub hp_line_reason: Option<String>,
    #[serde(rename = "segundoTamanho", default)]
    pub second_size: Option<SecondSize>,
}

/// O corpo que uma forma troca, campo por campo.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct BodyPatch {
    #[serde(rename = "massa", default)]
    pub mass: Option<f64>,
    #[serde(rename = "semMassa", default)]
    pub massless: Option<bool>,
    #[serde(default)]
    pub volume: Option<f64>,
    #[serde(default)]
    pub area: Option<f64>,
    #[serde(rename = "naoErgue", default)]
    pub cannot_lift: Option<bool>,
    #[serde(rename = "tamanhoEscolhido", default)]
    pub chosen_size: Option<String>,
    #[serde(rename = "linhaPV", default)]
    pub hp_line: Option<String>,
    #[serde(rename = "linhaPVMotivo", default)]
    pub hp_line_reason: Option<String>,
    #[serde(rename = "segundoTamanho", default)]
    pub second_size: Option<SecondSize>,
}

/// O segundo Tamanho (outra postura do mesmo corpo).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SecondSize {
    pub area: f64,
    #[serde(rename = "tamanhoEscolhido", default)]
    pub chosen_size: Option<String>,
}

/// O enxame: quantos indivíduos cabem num quadrado e o Tamanho que o enxame
/// vale.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Swarm {
    #[serde(rename = "individuosPorQuadrado")]
    pub individuals_per_square: i64,
    #[serde(rename = "tamanhoEquivalente")]
    pub equivalent_size: String,
}

/// O equipamento que a criatura sempre usa.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Equipment {
    pub arsenal: Vec<ArsenalItem>,
    #[serde(rename = "armaduras")]
    pub armors: Vec<WornArmor>,
    /// A dose de veneno carregada para a lâmina, em texto.
    #[serde(rename = "veneno", default)]
    pub poison: Option<String>,
}

/// O equipamento que uma forma troca.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct EquipmentPatch {
    #[serde(default)]
    pub arsenal: Option<Vec<ArsenalItem>>,
    #[serde(rename = "armaduras", default)]
    pub armors: Option<Vec<WornArmor>>,
    #[serde(rename = "veneno", default)]
    pub poison: Option<String>,
}

/// Uma peça do arsenal: `a:<id>` (arma) ou `e:<id>` (escudo).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ArsenalItem {
    #[serde(rename = "ref")]
    pub reference: String,
}

/// Uma armadura do catálogo, vestida ou não.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WornArmor {
    pub base: String,
    #[serde(rename = "vestida")]
    pub worn: bool,
}

/// Um ataque: o natural (com a conta toda no JSON) ou o com a arma do livro
/// (só o nome, o id da arma e, se for de outro Tamanho, o Tamanho dela).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Attack {
    #[serde(rename = "nome")]
    pub name: String,
    #[serde(rename = "arma", default)]
    pub weapon: Option<String>,
    /// O Tamanho da arma de outro Tamanho.
    #[serde(rename = "tamanho", default)]
    pub size: Option<String>,
    #[serde(rename = "classe", default)]
    pub class: Option<String>,
    /// As três parcelas da Velocidade, em Ticks.
    #[serde(default)]
    pub pgr: Option<[i32; 3]>,
    /// O dado de dano (`2d6+1`); `null` no golpe sem dano.
    #[serde(rename = "dado", default)]
    pub dice: Option<String>,
    #[serde(rename = "multiplicadorForca", default)]
    pub strength_multiplier: Option<i32>,
    #[serde(rename = "habilidade", default)]
    pub skill: Option<String>,
    #[serde(rename = "acerto", default)]
    pub accuracy: Option<i32>,
    #[serde(rename = "bloqueiaComoArma", default)]
    pub blocks_as_weapon: Option<i32>,
    #[serde(rename = "tipoDano", default)]
    pub damage_type: Option<String>,
    #[serde(rename = "nivelPerfuracao", default)]
    pub pierce_level: Option<i32>,
}

/// A forma principal de quem tem formas.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MainForm {
    pub id: String,
    #[serde(rename = "nome")]
    pub name: String,
}

/// Uma forma: o que ela troca da principal (o que não diz, herda).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Form {
    pub id: String,
    #[serde(rename = "nome")]
    pub name: String,
    #[serde(rename = "corpo", default)]
    pub body: Option<BodyPatch>,
    #[serde(rename = "couraca", default)]
    pub carapace: Option<i32>,
    #[serde(rename = "perfilCouraca", default)]
    pub carapace_profile: Option<String>,
    #[serde(rename = "atributos", default)]
    pub attributes: Option<AttributesPatch>,
    #[serde(rename = "equipamento", default)]
    pub equipment: Option<EquipmentPatch>,
    #[serde(rename = "artes", default)]
    pub arts: Option<BTreeMap<String, i32>>,
    #[serde(rename = "ataques", default)]
    pub attacks: Option<Vec<Attack>>,
}

/// Lê uma criatura a partir do texto do JSON e a valida.
pub fn parse_creature(json: &str) -> Result<Creature, DataError> {
    let mut de = serde_json::Deserializer::from_str(json);
    let creature: Creature =
        serde_path_to_error::deserialize(&mut de).map_err(|e| DataError::Json {
            file: None,
            path: e.path().to_string(),
            reason: e.inner().to_string(),
        })?;
    de.end().map_err(|e| DataError::Json {
        file: None,
        path: String::new(),
        reason: e.to_string(),
    })?;
    validate(&creature)?;
    Ok(creature)
}

/// Lê todas as criaturas de `<content_dir>/data/bestiario/`, em ordem de nome
/// de arquivo, e confere que o id de cada uma é o nome do arquivo.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_bestiary(content_dir: &std::path::Path) -> Result<Vec<Creature>, DataError> {
    let dir = content_dir.join("data").join("bestiario");
    let io = |file: &std::path::Path, e: std::io::Error| DataError::Io {
        file: file.display().to_string(),
        reason: e.to_string(),
    };
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| io(&dir, e))? {
        let path = entry.map_err(|e| io(&dir, e))?.path();
        if path.extension().is_some_and(|x| x == "json") {
            files.push(path);
        }
    }
    files.sort();
    let mut creatures = Vec::with_capacity(files.len());
    for path in files {
        let name = format!(
            "data/bestiario/{}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        let text = std::fs::read_to_string(&path).map_err(|e| io(&path, e))?;
        let creature = parse_creature(&text).map_err(|e| e.in_file(name.clone()))?;
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        if creature.id != stem {
            return Err(DataError::invalid(
                "id",
                format!("o id {} não é o nome do arquivo", creature.id),
            )
            .in_file(name));
        }
        creatures.push(creature);
    }
    Ok(creatures)
}

// ----- validação -----

/// O id dos dados: minúsculas e algarismos, separados por hífen.
fn is_id(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// O dado de dano: `NdM` com M = 6 e um bônus opcional (`2d6`, `1d6+2`, `3d6-1`).
fn is_dice(s: &str) -> bool {
    let Some((count, rest)) = s.split_once("d6") else {
        return false;
    };
    let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    if !digits(count) {
        return false;
    }
    match rest.as_bytes().first() {
        None => true,
        Some(b'+') | Some(b'-') => digits(&rest[1..]),
        Some(_) => false,
    }
}

fn check(ok: bool, path: impl Into<String>, reason: impl Into<String>) -> Result<(), DataError> {
    if ok {
        Ok(())
    } else {
        Err(DataError::invalid(path, reason))
    }
}

fn check_range(value: i32, min: i32, max: i32, path: &str) -> Result<(), DataError> {
    check(
        (min..=max).contains(&value),
        path,
        format!("{value} fora de {min} a {max}"),
    )
}

fn check_in(value: &str, list: &[&str], path: &str) -> Result<(), DataError> {
    check(
        list.contains(&value),
        path,
        format!("\"{value}\" não é um de: {}", list.join(", ")),
    )
}

fn check_attributes(a: &Attributes, path: &str) -> Result<(), DataError> {
    let all = [
        ("forca", a.strength),
        ("destreza", a.dexterity),
        ("vigor", a.stamina),
        ("percepcao", a.perception),
        ("inteligencia", a.intelligence),
        ("raciocinio", a.wits),
        ("influencia", a.influence),
        ("perspicacia", a.insight),
        ("compostura", a.composure),
    ];
    for (key, value) in all {
        check(value >= 0, format!("{path}.{key}"), "negativo")?;
    }
    Ok(())
}

fn check_attributes_patch(a: &AttributesPatch, path: &str) -> Result<(), DataError> {
    let all = [
        ("forca", a.strength),
        ("destreza", a.dexterity),
        ("vigor", a.stamina),
        ("percepcao", a.perception),
        ("inteligencia", a.intelligence),
        ("raciocinio", a.wits),
        ("influencia", a.influence),
        ("perspicacia", a.insight),
        ("compostura", a.composure),
    ];
    for (key, value) in all {
        if let Some(v) = value {
            check(v >= 0, format!("{path}.{key}"), "negativo")?;
        }
    }
    Ok(())
}

fn check_arts(arts: &BTreeMap<String, i32>, path: &str) -> Result<(), DataError> {
    for (id, level) in arts {
        check(*level >= 0, format!("{path}.{id}"), "negativo")?;
    }
    Ok(())
}

fn check_equipment_parts(
    arsenal: Option<&[ArsenalItem]>,
    armors: Option<&[WornArmor]>,
    path: &str,
) -> Result<(), DataError> {
    for (i, item) in arsenal.unwrap_or_default().iter().enumerate() {
        let ok = item
            .reference
            .split_once(':')
            .is_some_and(|(kind, id)| (kind == "a" || kind == "e") && is_id(id));
        check(
            ok,
            format!("{path}.arsenal[{i}].ref"),
            "a referência é a:<id> ou e:<id>",
        )?;
    }
    for (i, armor) in armors.unwrap_or_default().iter().enumerate() {
        check(
            is_id(&armor.base),
            format!("{path}.armaduras[{i}].base"),
            "id inválido",
        )?;
    }
    Ok(())
}

fn check_second_size(s: &SecondSize, path: &str) -> Result<(), DataError> {
    if let Some(size) = &s.chosen_size {
        check_in(size, &SIZES, &format!("{path}.tamanhoEscolhido"))?;
    }
    Ok(())
}

fn check_attacks(attacks: &[Attack], path: &str) -> Result<(), DataError> {
    for (i, a) in attacks.iter().enumerate() {
        let p = format!("{path}[{i}]");
        if let Some(weapon) = &a.weapon {
            // o ataque com a arma do livro: o resto vem do catálogo
            check(is_id(weapon), format!("{p}.arma"), "id inválido")?;
            if let Some(size) = &a.size {
                check_in(size, &SIZES, &format!("{p}.tamanho"))?;
            }
            continue;
        }
        let class = a
            .class
            .as_deref()
            .ok_or_else(|| DataError::invalid(format!("{p}.classe"), "falta a classe"))?;
        check_in(class, &ATTACK_CLASSES, &format!("{p}.classe"))?;
        let pgr = a
            .pgr
            .ok_or_else(|| DataError::invalid(format!("{p}.pgr"), "falta a Velocidade"))?;
        check(
            pgr[0] >= 0 && pgr[1] == 1 && pgr[2] >= 0,
            format!("{p}.pgr"),
            "a Velocidade é [n ≥ 0, 1, n ≥ 0]",
        )?;
        if let Some(dice) = &a.dice {
            check(is_dice(dice), format!("{p}.dado"), "o dado é NdM com M = 6")?;
        }
        let mult = a.strength_multiplier.ok_or_else(|| {
            DataError::invalid(format!("{p}.multiplicadorForca"), "falta o multiplicador")
        })?;
        check_range(mult, 0, 2, &format!("{p}.multiplicadorForca"))?;
        check(
            a.accuracy.is_some(),
            format!("{p}.acerto"),
            "falta o Acerto",
        )?;
        if let Some(level) = a.pierce_level {
            check(level >= 0, format!("{p}.nivelPerfuracao"), "negativo")?;
        }
        if let Some(skill) = &a.skill {
            check_in(skill, &RANGED_SKILLS, &format!("{p}.habilidade"))?;
        }
    }
    Ok(())
}

fn validate(c: &Creature) -> Result<(), DataError> {
    check(is_id(&c.id), "id", "id inválido")?;
    check(is_id(&c.category), "categoria", "id inválido")?;
    check_range(c.threat, 1, 6, "ameaca")?;
    check_range(c.spark, 0, 12, "centelha")?;
    check_range(c.carapace, 0, 12, "couraca")?;
    check(c.willpower >= 0, "vontade", "negativo")?;
    check_attributes(&c.attributes, "atributos")?;
    check_arts(&c.arts, "artes")?;
    if let Some(profile) = &c.carapace_profile {
        check_in(profile, &CARAPACE_PROFILES, "perfilCouraca")?;
    }

    let b = &c.body;
    check(
        b.mass.is_some() || b.massless,
        "corpo.massa",
        "corpo exige massa ou semMassa",
    )?;
    check(
        b.massless == b.volume.is_some(),
        "corpo.volume",
        "semMassa e volume devem ser declarados juntos",
    )?;
    if let Some(size) = &b.chosen_size {
        check_in(size, &SIZES, "corpo.tamanhoEscolhido")?;
    }
    if let Some(line) = &b.hp_line {
        check_in(line, &["acima", "abaixo"], "corpo.linhaPV")?;
    }
    check(
        b.hp_line_reason.is_none() || b.hp_line.is_some(),
        "corpo.linhaPVMotivo",
        "linhaPVMotivo exige linhaPV",
    )?;
    if let Some(s) = &b.second_size {
        check_second_size(s, "corpo.segundoTamanho")?;
    }

    if let Some(s) = &c.swarm {
        check(
            s.individuals_per_square > 0,
            "enxame.individuosPorQuadrado",
            "precisa ser positivo",
        )?;
        check_in(
            &s.equivalent_size,
            &["pequeno", "medio"],
            "enxame.tamanhoEquivalente",
        )?;
    }

    check_equipment_parts(
        Some(&c.equipment.arsenal),
        Some(&c.equipment.armors),
        "equipamento",
    )?;
    check_attacks(&c.attacks, "ataques")?;

    check(
        c.main_form.is_some() == c.forms.is_some(),
        "formas",
        "formaPrincipal e formas devem existir juntas",
    )?;
    if let (Some(main), Some(forms)) = (&c.main_form, &c.forms) {
        check(!forms.is_empty(), "formas", "a lista de formas está vazia")?;
        let mut ids = HashSet::new();
        check(is_id(&main.id), "formaPrincipal.id", "id inválido")?;
        ids.insert(main.id.as_str());
        for (i, f) in forms.iter().enumerate() {
            let p = format!("formas[{i}]");
            check(is_id(&f.id), format!("{p}.id"), "id inválido")?;
            check(
                ids.insert(f.id.as_str()),
                "formas",
                "ids de formas devem ser únicos",
            )?;
            if let Some(n) = f.carapace {
                check_range(n, 0, 12, &format!("{p}.couraca"))?;
            }
            if let Some(profile) = &f.carapace_profile {
                check_in(profile, &CARAPACE_PROFILES, &format!("{p}.perfilCouraca"))?;
            }
            if let Some(a) = &f.attributes {
                check_attributes_patch(a, &format!("{p}.atributos"))?;
            }
            if let Some(arts) = &f.arts {
                check_arts(arts, &format!("{p}.artes"))?;
            }
            if let Some(e) = &f.equipment {
                check_equipment_parts(
                    e.arsenal.as_deref(),
                    e.armors.as_deref(),
                    &format!("{p}.equipamento"),
                )?;
            }
            if let Some(attacks) = &f.attacks {
                check_attacks(attacks, &format!("{p}.ataques"))?;
            }
            if let Some(fb) = &f.body {
                if let Some(size) = &fb.chosen_size {
                    check_in(size, &SIZES, &format!("{p}.corpo.tamanhoEscolhido"))?;
                    check(
                        fb.area.is_some(),
                        format!("{p}.corpo.tamanhoEscolhido"),
                        "Tamanho próprio da forma exige área própria",
                    )?;
                }
                if let Some(line) = &fb.hp_line {
                    check_in(line, &["acima", "abaixo"], &format!("{p}.corpo.linhaPV"))?;
                }
                check(
                    fb.hp_line_reason.is_none() || fb.hp_line.is_some(),
                    format!("{p}.corpo.linhaPVMotivo"),
                    "linhaPVMotivo exige linhaPV",
                )?;
                if let Some(s) = &fb.second_size {
                    check_second_size(s, &format!("{p}.corpo.segundoTamanho"))?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids() {
        assert!(is_id("punho-de-ferro"));
        assert!(is_id("a1"));
        assert!(!is_id("Punho"));
        assert!(!is_id("a--b"));
        assert!(!is_id("-a"));
        assert!(!is_id(""));
    }

    #[test]
    fn dice() {
        for ok in ["1d6", "12d6", "2d6+1", "3d6-2"] {
            assert!(is_dice(ok), "{ok}");
        }
        for bad in ["d6", "1d8", "1d6+", "1d6x", "a1d6", "1d6+1+1"] {
            assert!(!is_dice(bad), "{bad}");
        }
    }

    #[test]
    fn error_points_to_the_json_path() {
        let err = parse_creature(r#"{"id": "x", "nome": 3}"#).unwrap_err();
        assert_eq!(err.path(), "nome");
        assert!(matches!(err, DataError::Json { .. }));
    }
}
