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

/// Os Tamanhos, do menor para o maior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub enum Size {
    #[serde(rename = "minusculo")]
    Minuscule,
    #[serde(rename = "miudo")]
    Tiny,
    #[serde(rename = "pequeno")]
    Small,
    #[serde(rename = "medio")]
    Medium,
    #[serde(rename = "grande")]
    Large,
    #[serde(rename = "enorme")]
    Huge,
    #[serde(rename = "imenso")]
    Immense,
    #[serde(rename = "colossal")]
    Colossal,
}

impl Size {
    /// Os oito, do menor para o maior.
    pub const ALL: [Size; 8] = [
        Size::Minuscule,
        Size::Tiny,
        Size::Small,
        Size::Medium,
        Size::Large,
        Size::Huge,
        Size::Immense,
        Size::Colossal,
    ];

    /// O id nos dados (`medio`).
    pub fn id(self) -> &'static str {
        match self {
            Size::Minuscule => "minusculo",
            Size::Tiny => "miudo",
            Size::Small => "pequeno",
            Size::Medium => "medio",
            Size::Large => "grande",
            Size::Huge => "enorme",
            Size::Immense => "imenso",
            Size::Colossal => "colossal",
        }
    }
}

/// O perfil da cobertura da Couraça.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum CarapaceProfile {
    #[serde(rename = "rigida")]
    Rigid,
    #[serde(rename = "couro")]
    Leather,
    #[serde(rename = "mole")]
    Soft,
}

/// A linha de PV escolhida na zona de transição da tabela de massa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum HpLine {
    #[serde(rename = "acima")]
    Above,
    #[serde(rename = "abaixo")]
    Below,
}

/// A classe do ataque (e da arma do catálogo).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum AttackClass {
    #[serde(rename = "leve")]
    Light,
    #[serde(rename = "media")]
    Medium,
    #[serde(rename = "pesada")]
    Heavy,
    #[serde(rename = "haste")]
    Polearm,
    #[serde(rename = "haste-media")]
    MediumPolearm,
    #[serde(rename = "haste-guerra")]
    WarPolearm,
    #[serde(rename = "distancia")]
    Ranged,
    #[serde(rename = "arremesso")]
    Thrown,
}

/// A categoria da criatura.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Category {
    #[serde(rename = "aberracao")]
    Aberration,
    #[serde(rename = "animal")]
    Animal,
    #[serde(rename = "besta-magica")]
    MagicalBeast,
    #[serde(rename = "celestial")]
    Celestial,
    #[serde(rename = "construto")]
    Construct,
    #[serde(rename = "corruptor")]
    Corruptor,
    #[serde(rename = "dragao")]
    Dragon,
    #[serde(rename = "elemental")]
    Elemental,
    #[serde(rename = "fada")]
    Fey,
    #[serde(rename = "gigante")]
    Giant,
    #[serde(rename = "humanoide")]
    Humanoid,
    #[serde(rename = "humanoide-monstruoso")]
    MonstrousHumanoid,
    #[serde(rename = "limo")]
    Ooze,
    #[serde(rename = "morto-vivo")]
    Undead,
    #[serde(rename = "outsider")]
    Outsider,
    #[serde(rename = "planta")]
    Plant,
}

/// As Habilidades que o ataque natural pode gravar (as de tiro e arremesso).
pub const RANGED_SKILLS: [&str; 3] = ["disparo", "atirador", "arremesso"];

/// Uma criatura do bestiário.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Creature {
    pub id: String,
    #[serde(rename = "nome")]
    pub name: String,
    #[serde(rename = "categoria")]
    pub category: Category,
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
    pub carapace_profile: Option<CarapaceProfile>,
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

impl Attributes {
    /// O Atributo pela chave dos dados (`raciocinio`), para as regras que
    /// listam chaves em `regras.json`.
    pub fn get(&self, key: &str) -> Option<i32> {
        Some(match key {
            "forca" => self.strength,
            "destreza" => self.dexterity,
            "vigor" => self.stamina,
            "percepcao" => self.perception,
            "inteligencia" => self.intelligence,
            "raciocinio" => self.wits,
            "influencia" => self.influence,
            "perspicacia" => self.insight,
            "compostura" => self.composure,
            _ => return None,
        })
    }

    fn entries(&self) -> [(&'static str, i32); 9] {
        [
            ("forca", self.strength),
            ("destreza", self.dexterity),
            ("vigor", self.stamina),
            ("percepcao", self.perception),
            ("inteligencia", self.intelligence),
            ("raciocinio", self.wits),
            ("influencia", self.influence),
            ("perspicacia", self.insight),
            ("compostura", self.composure),
        ]
    }
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

impl AttributesPatch {
    fn entries(&self) -> [(&'static str, Option<i32>); 9] {
        [
            ("forca", self.strength),
            ("destreza", self.dexterity),
            ("vigor", self.stamina),
            ("percepcao", self.perception),
            ("inteligencia", self.intelligence),
            ("raciocinio", self.wits),
            ("influencia", self.influence),
            ("perspicacia", self.insight),
            ("compostura", self.composure),
        ]
    }
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
    /// A área da silhueta, em m². Obrigatória na criatura; numa criatura
    /// montada a partir de uma forma ela sempre vem da principal ou da forma.
    pub area: Option<f64>,
    /// A criatura que não ergue peso (`naoErgue`).
    #[serde(rename = "naoErgue", default)]
    pub cannot_lift: bool,
    #[serde(rename = "tamanhoEscolhido", default)]
    pub chosen_size: Option<Size>,
    #[serde(rename = "linhaPV", default)]
    pub hp_line: Option<HpLine>,
    /// O porquê da linha de PV, nas palavras da ficha (`linhaPVMotivo`).
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
    pub chosen_size: Option<Size>,
    #[serde(rename = "linhaPV", default)]
    pub hp_line: Option<HpLine>,
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
    pub chosen_size: Option<Size>,
}

/// O enxame: quantos indivíduos cabem num quadrado e o Tamanho que o enxame
/// vale (só Pequeno ou Médio).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Swarm {
    #[serde(rename = "individuosPorQuadrado")]
    pub individuals_per_square: i64,
    #[serde(rename = "tamanhoEquivalente")]
    pub equivalent_size: Size,
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

/// As três parcelas da Velocidade, em Ticks (P/G/R).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(from = "[i32; 3]")]
pub struct Timing {
    /// Preparo.
    pub windup: i32,
    /// Golpe.
    pub blow: i32,
    /// Recuperação.
    pub recovery: i32,
}

impl From<[i32; 3]> for Timing {
    fn from([windup, blow, recovery]: [i32; 3]) -> Self {
        Timing {
            windup,
            blow,
            recovery,
        }
    }
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
    pub size: Option<Size>,
    #[serde(rename = "classe", default)]
    pub class: Option<AttackClass>,
    #[serde(rename = "pgr", default)]
    pub timing: Option<Timing>,
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
    pub carapace_profile: Option<CarapaceProfile>,
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
    let creature: Creature = crate::json::parse(json)?;
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
pub(crate) fn is_id(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// A referência do arsenal, como a fonte: `^[ae]:[a-z0-9-]+$` (o resto pode ter
/// hífen em qualquer posição, sem a forma estrita do id).
fn is_arsenal_ref(s: &str) -> bool {
    s.split_once(':').is_some_and(|(kind, rest)| {
        (kind == "a" || kind == "e")
            && !rest.is_empty()
            && rest
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
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
        check(
            is_arsenal_ref(&item.reference),
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

fn check_attacks(attacks: &[Attack], path: &str) -> Result<(), DataError> {
    for (i, a) in attacks.iter().enumerate() {
        let p = format!("{path}[{i}]");
        if let Some(weapon) = &a.weapon {
            // o ataque com a arma do livro: o resto vem do catálogo
            check(is_id(weapon), format!("{p}.arma"), "id inválido")?;
            continue;
        }
        check(a.class.is_some(), format!("{p}.classe"), "falta a classe")?;
        let t = a
            .timing
            .ok_or_else(|| DataError::invalid(format!("{p}.pgr"), "falta a Velocidade"))?;
        check(
            t.windup >= 0 && t.blow == 1 && t.recovery >= 0,
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
    check_range(c.threat, 1, 6, "ameaca")?;
    check_range(c.spark, 0, 12, "centelha")?;
    check_range(c.carapace, 0, 12, "couraca")?;
    check(c.willpower >= 0, "vontade", "negativo")?;
    for (key, value) in c.attributes.entries() {
        check(value >= 0, format!("atributos.{key}"), "negativo")?;
    }
    check_arts(&c.arts, "artes")?;

    let b = &c.body;
    check(b.area.is_some(), "corpo.area", "falta a área")?;
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
    check(
        b.hp_line_reason.is_none() || b.hp_line.is_some(),
        "corpo.linhaPVMotivo",
        "linhaPVMotivo exige linhaPV",
    )?;

    if let Some(s) = &c.swarm {
        check(
            s.individuals_per_square > 0,
            "enxame.individuosPorQuadrado",
            "precisa ser positivo",
        )?;
        check(
            matches!(s.equivalent_size, Size::Small | Size::Medium),
            "enxame.tamanhoEquivalente",
            "só pequeno ou medio",
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
            if let Some(a) = &f.attributes {
                for (key, value) in a.entries() {
                    if let Some(v) = value {
                        check(v >= 0, format!("{p}.atributos.{key}"), "negativo")?;
                    }
                }
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
                // O Tamanho próprio da forma sem a área própria é o formato
                // legado que a leitura da fonte ainda tolera (o esquema de
                // entrada recusa, a leitura aceita): aceito igual, sem conferir.
                check(
                    fb.hp_line_reason.is_none() || fb.hp_line.is_some(),
                    format!("{p}.corpo.linhaPVMotivo"),
                    "linhaPVMotivo exige linhaPV",
                )?;
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
    fn arsenal_refs_like_the_source() {
        for ok in ["a:espada-longa", "e:broquel", "a:-x", "a:x--y", "a:x-"] {
            assert!(is_arsenal_ref(ok), "{ok}");
        }
        for bad in ["x:espada", "a:", "a:Espada", "espada", "a:esp ada"] {
            assert!(!is_arsenal_ref(bad), "{bad}");
        }
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

    #[test]
    fn size_ids_follow_the_order() {
        let ids: Vec<&str> = Size::ALL.iter().map(|s| s.id()).collect();
        assert_eq!(
            ids,
            [
                "minusculo",
                "miudo",
                "pequeno",
                "medio",
                "grande",
                "enorme",
                "imenso",
                "colossal"
            ]
        );
        assert!(Size::ALL.windows(2).all(|w| w[0] < w[1]));
    }
}
