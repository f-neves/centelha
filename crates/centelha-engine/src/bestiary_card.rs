//! O card do bestiário: da fonte de uma criatura (`content/data/bestiario/<id>.json`)
//! até cada número e texto do card, como o site atual monta o `monsters.json`.
//!
//! A cadeia da fonte tem três passos, todos portados aqui:
//! - `vistaPublicaCriatura` (`src/lib/motor.ts`): os derivados da criatura (pelo
//!   [`creature::derive`]), a Defesa com a arma das mãos e com o equipamento, os
//!   ataques em texto (pool e dano), as habilidades especiais com a prosa das
//!   manobras, e a vista das outras formas;
//! - o `inimigos.json` (`scripts/gen-bestiario.mjs`): o bloco `stat` sem as notas,
//!   com as Habilidades juntas;
//! - o `build` de `scripts/gen-monsters.mjs`: a categoria, as tags, a classe de
//!   tempo do ataque, as Habilidades públicas, o texto da Técnica e da Arte
//!   citadas, e o card de cada forma.
//!
//! O card sai como JSON, com as chaves e a forma do `monsters.json`. A fonte entra
//! crua (`serde_json::Value`), porque o card copia boa parte dela (textos, listas
//! e o próprio JSON, em `edicao`).
//!
//! Mais as três contas que o card faz na hora, no navegador: a linha de PV na
//! zona de transição ([`hp_line_in_zone`]), o pool com a penalidade do
//! equipamento ([`pool_with_penalty`]) e o modificador da Aparência pela curva
//! do próprio card ([`card_appearance_mod`]).

use std::collections::{HashMap, HashSet};

use centelha_data::bestiary::{Creature, HpLine, Size};
use centelha_data::{Art, Rules, Technique};
use serde_json::{Map, Value, json};

use crate::creature::{self, Defense, Tables, weapon_attack, weapon_dice_text};
use crate::js;
use crate::sheet::value::{self as jv, is_js_space, num, to_js_string, truthy};

/// O que o card lê além das regras e dos catálogos do [`creature::Tables`].
pub struct CardTables {
    creature: Tables,
    /// `armas.json` cru: o bloco `arma` de cada peça (mãos, Defesa da arma,
    /// modos de dano, Perfuração).
    weapons: Vec<Value>,
    /// `regras.json` cru: os números do texto da Rajada.
    rules: Value,
    primary_skills: HashSet<String>,
    technique_names: HashMap<String, String>,
    art_names: HashMap<String, String>,
}

/// O que pode faltar para montar o card.
#[derive(Debug, Clone, PartialEq)]
pub enum CardError {
    /// A fonte (ou a forma montada) não tem a forma de uma criatura.
    Creature(String),
    /// A habilidade cita uma Técnica ou Arte que o catálogo não tem.
    MissingReference(String),
    /// A criatura não declara deslocamento.
    NoMovement(String),
    /// `armas.json` ou `regras.json` cru não é o que se espera.
    Tables(String),
}

impl CardTables {
    /// `weapons` e `rules` são o `armas.json` e o `regras.json` crus; `skills`
    /// é o `habilidades.json` cru (as Habilidades primárias, pelo id).
    pub fn new(
        creature: Tables,
        weapons: Value,
        rules: Value,
        skills: &Value,
        techniques: &[Technique],
        arts: &[Art],
    ) -> Result<Self, CardError> {
        let Value::Array(weapons) = weapons else {
            return Err(CardError::Tables("armas.json não é uma lista".into()));
        };
        let primary_skills = skills
            .as_array()
            .ok_or_else(|| CardError::Tables("habilidades.json não é uma lista".into()))?
            .iter()
            .filter_map(|h| h.get("id").and_then(Value::as_str).map(str::to_owned))
            .collect();
        Ok(CardTables {
            creature,
            weapons,
            rules,
            primary_skills,
            technique_names: techniques
                .iter()
                .map(|t| (t.id.clone(), t.name.clone()))
                .collect(),
            art_names: arts
                .iter()
                .map(|a| (a.id.clone(), a.name.clone()))
                .collect(),
        })
    }

    pub fn creature_tables(&self) -> &Tables {
        &self.creature
    }

    fn rules(&self) -> &Rules {
        self.creature.rules()
    }

    /// `ARMAS.find((w) => w.id === id)?.arma`: o bloco da primeira peça com o id.
    fn weapon_block(&self, id: Option<&Value>) -> Option<&Value> {
        let id = id?.as_str()?;
        self.weapons
            .iter()
            .find(|w| jv::is_str(w.get("id"), id))
            .and_then(|w| w.get("arma"))
            .filter(|a| !a.is_null())
    }
}

// ------------------------------------------------------------ as contas da hora

/// A linha de PV de uma criatura na zona de transição da tabela de massa
/// (`linhaPVNaZona`), para o card dizer por quê: a zona, a linha escolhida e
/// os dois PV da tabela (sem o `bonusPV`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HpLineInZone {
    pub zone_kg: [f64; 2],
    pub line: HpLine,
    pub hp_above: f64,
    pub hp_below: f64,
}

/// `linhaPVNaZona`: nada fora da zona, sem escolha ou sem linha acima.
pub fn hp_line_in_zone(
    rules: &Rules,
    kg: Option<f64>,
    stamina: f64,
    line: Option<HpLine>,
) -> Option<HpLineInZone> {
    let (kg, line) = (kg?, line?);
    let rows = &rules.derived.pv.mass.table;
    let z = rows
        .iter()
        .position(|r| r.zone_kg.is_some_and(|z| kg >= z[0] && kg <= z[1]))?;
    rows.get(z + 1)?;
    let hp = |i: usize| {
        let r = &rows[i];
        r.base + ((stamina * r.stamina_mult) / r.stamina_divisor).floor()
    };
    Some(HpLineInZone {
        zone_kg: rows[z].zone_kg?,
        line,
        hp_above: hp(z + 1),
        hp_below: hp(z),
    })
}

/// `massaDaConta` do `corpo` do card: a massa, ou o volume do incorpóreo
/// lido como carne e osso.
pub fn card_mass(rules: &Rules, body: &Value) -> Option<f64> {
    if truthy(body.get("semMassa")) {
        let v = body.get("volume").filter(|x| !x.is_null())?;
        return Some(jv::to_number(Some(v)) * rules.creature.mass_band.incorporeal_density_kg_m3);
    }
    body.get("massa")
        .filter(|x| !x.is_null())
        .map(|m| jv::to_number(Some(m)))
}

/// `poolComPenalidade`: o pool do card com a Penalidade da armadura tirada do
/// acerto (`"3d6+2 +5"` com 1 dá `"3d6+2 +4"`; `"4d6 +1"` com 2 dá
/// `"4d6 \u{2212}1"`). O pool que não se lê volta como veio.
pub fn pool_with_penalty(pool: &str, penalty: f64) -> String {
    if penalty == 0.0 || penalty.is_nan() {
        return pool.to_owned();
    }
    let t = pool.trim_matches(is_js_space);
    let Some((dice, rest)) = split_pool(t) else {
        return pool.to_owned();
    };
    let given = match rest {
        Some((sign, n)) => n * if sign == '+' { 1.0 } else { -1.0 },
        None => 0.0,
    };
    let a = given - penalty;
    let tail = if a > 0.0 {
        format!(" +{}", js::number_to_string(a))
    } else if a < 0.0 {
        format!(" \u{2212}{}", js::number_to_string(-a))
    } else {
        String::new()
    };
    format!("{dice}{tail}")
}

/// `^(\d+d6(?:\+2)?)(?:\s*([+−-])\s*(\d+))?$`.
fn split_pool(t: &str) -> Option<(&str, Option<(char, f64)>)> {
    let digits = t.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 || !t[digits..].starts_with("d6") {
        return None;
    }
    let mut end = digits + 2;
    if t[end..].starts_with("+2") {
        // o `+2` é da soma ímpar só quando o resto ainda fecha a expressão
        let rest = &t[end + 2..];
        if rest.is_empty() || rest_is_accuracy(rest).is_some() {
            end += 2;
        }
    }
    let rest = &t[end..];
    if rest.is_empty() {
        return Some((&t[..end], None));
    }
    rest_is_accuracy(rest).map(|acc| (&t[..end], Some(acc)))
}

/// `\s*([+−-])\s*(\d+)$`.
fn rest_is_accuracy(rest: &str) -> Option<(char, f64)> {
    let r = rest.trim_start_matches(is_js_space);
    let mut cs = r.chars();
    let sign = cs.next().filter(|c| matches!(c, '+' | '\u{2212}' | '-'))?;
    let n = cs.as_str().trim_start_matches(is_js_space);
    (!n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        .then(|| (sign, jv::string_to_number(n)))
}

/// O modificador da Aparência pela curva do próprio card (`apMod`,
/// `BestaCard.astro:47-48`): 1 dá -4, de 4 a 6 dá 0, 10 dá +4, e fora de 1 a 10
/// extrapola um por ponto. **Não é a regra** (`character::appearance_mod`, a
/// curva de `regras.json`); a diferença foi levada à equipe das regras. Um
/// valor fracionário entre 1 e 10 não tem modificador.
pub fn card_appearance_mod(n: f64) -> Option<f64> {
    const CURVE: [f64; 10] = [-4.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 4.0];
    if n > 10.0 {
        Some(4.0 + (n - 10.0))
    } else if n < 1.0 {
        Some(-5.0 - (0.0 - n))
    } else if n.fract() == 0.0 {
        Some(CURVE[n as usize - 1])
    } else {
        None
    }
}

// ------------------------------------------------------------ textos e listas

const SIZE_LABEL: [(Size, &str); 8] = [
    (Size::Minuscule, "Minúsculo"),
    (Size::Tiny, "Miúdo"),
    (Size::Small, "Pequeno"),
    (Size::Medium, "Médio"),
    (Size::Large, "Grande"),
    (Size::Huge, "Enorme"),
    (Size::Immense, "Imenso"),
    (Size::Colossal, "Colossal"),
];

fn size_label(s: Size) -> &'static str {
    SIZE_LABEL
        .iter()
        .find(|(x, _)| *x == s)
        .map_or("", |(_, l)| l)
}

const CATEGORY_NAME: [(&str, &str); 16] = [
    ("animal", "Animal"),
    ("aberracao", "Aberração"),
    ("besta-magica", "Besta mágica"),
    ("celestial", "Celestial"),
    ("humanoide", "Humanoide"),
    ("humanoide-monstruoso", "Humanoide monstruoso"),
    ("morto-vivo", "Morto-vivo"),
    ("corruptor", "Corruptor"),
    ("planta", "Planta"),
    ("construto", "Construto"),
    ("gigante", "Gigante"),
    ("limo", "Limo"),
    ("elemental", "Elemental"),
    ("dragao", "Dragão"),
    ("fada", "Fada"),
    ("outsider", "Extraplanar"),
];

/// Os campos da habilidade especial, na ordem da ficha (`CAMPOS_HABILIDADE`).
const ABILITY_FIELDS: [(&str, &str); 7] = [
    ("ativacao", "Ativação"),
    ("duracao", "Duração"),
    ("alcance", "Alcance"),
    ("area", "Área"),
    ("dano", "Dano"),
    ("resistencia", "Resistência"),
    ("limiteDeUso", "Limite de uso"),
];

/// As exceções da classe do ataque por criatura (`CLASSE_OVERRIDE`).
const CLASS_OVERRIDE: [(&str, &str); 2] =
    [("golem-de-ferro", "media"), ("golem-de-pedra", "media")];

/// A letra sem acento, para a categoria (`normalize('NFD')` sem as marcas) e
/// para comparar nomes sem caixa nem acento.
fn base_letter(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => 'A',
        'ç' => 'c',
        'Ç' => 'C',
        'è' | 'é' | 'ê' | 'ë' => 'e',
        'È' | 'É' | 'Ê' | 'Ë' => 'E',
        'ì' | 'í' | 'î' | 'ï' => 'i',
        'Ì' | 'Í' | 'Î' | 'Ï' => 'I',
        'ñ' => 'n',
        'Ñ' => 'N',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
        'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' => 'O',
        'ù' | 'ú' | 'û' | 'ü' => 'u',
        'Ù' | 'Ú' | 'Û' | 'Ü' => 'U',
        'ý' | 'ÿ' => 'y',
        'Ý' => 'Y',
        c => c,
    }
}

fn unaccent(s: &str) -> String {
    s.chars()
        .filter(|c| !('\u{300}'..='\u{36f}').contains(c))
        .map(base_letter)
        .collect()
}

/// `idCategoria`: sem acento, em minúsculas, espaços viram hífen.
fn category_id(v: Option<&Value>) -> String {
    let s = if truthy(v) {
        to_js_string(v)
    } else {
        String::new()
    };
    let mut out = String::new();
    let mut in_space = false;
    for c in unaccent(&s).to_lowercase().chars() {
        if is_js_space(c) {
            if !in_space {
                out.push('-');
            }
            in_space = true;
        } else {
            out.push(c);
            in_space = false;
        }
    }
    out
}

/// `localeCompare(..., { sensitivity: 'base' }) === 0`: sem caixa nem acento.
fn same_base(a: &str, b: &str) -> bool {
    unaccent(a).to_lowercase() == unaccent(b).to_lowercase()
}

/// `String(s).trim().replace(/\.$/, '')`.
fn trim_period(s: &str) -> &str {
    let t = s.trim_matches(is_js_space);
    t.strip_suffix('.').unwrap_or(t)
}

/// `dado.replace(/[+-]\d+$/, '')`.
fn strip_flat(d: &str) -> &str {
    let digits = d.bytes().rev().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return d;
    }
    let cut = d.len() - digits;
    match d[..cut].chars().last() {
        Some('+' | '-') => &d[..cut - 1],
        _ => d,
    }
}

/// `Number(d?.match(/^\d+/)?.[0] ?? 0)`.
fn leading_number(d: Option<&str>) -> f64 {
    let Some(d) = d else { return 0.0 };
    let n = d.bytes().take_while(u8::is_ascii_digit).count();
    if n == 0 {
        0.0
    } else {
        jv::string_to_number(&d[..n])
    }
}

/// `x % 2` verdadeiro (o resto do JavaScript, com o sinal de `x`).
fn is_odd(x: f64) -> bool {
    let r = x % 2.0;
    r != 0.0 && !r.is_nan()
}

fn n2s(x: f64) -> String {
    js::number_to_string(x)
}

/// `x ? ' +' + x : ''` e o menos tipográfico: ` +3`, ` −2`.
fn signed_tail(x: f64) -> String {
    if x > 0.0 {
        format!(" +{}", n2s(x))
    } else if x < 0.0 {
        format!(" \u{2212}{}", n2s(-x))
    } else {
        String::new()
    }
}

fn obj(v: Option<&Value>) -> Option<&Map<String, Value>> {
    v.and_then(Value::as_object)
}

fn list(v: Option<&Value>) -> &[Value] {
    v.and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

fn put(m: &mut Map<String, Value>, k: &str, v: Option<Value>) {
    if let Some(v) = v {
        m.insert(k.to_owned(), v);
    }
}

/// `item.formas === 'todas' || item.formas?.includes(formaId)`.
fn in_form(item: &Value, form_id: Option<&str>) -> bool {
    match item.get("formas") {
        Some(Value::String(s)) => s == "todas" || s.contains(form_id.unwrap_or("undefined")),
        Some(Value::Array(a)) => {
            form_id.is_some_and(|id| a.iter().any(|x| jv::is_str(Some(x), id)))
        }
        _ => false,
    }
}

fn num_field(v: Option<&Value>, k: &str) -> f64 {
    jv::num_or(jv::get(v, k), 0.0)
}

// ------------------------------------------------------------ a forma

/// `comForma`: a entrada de uma forma por cima da principal. O objeto troca
/// chave por chave, a lista troca inteira; o deslocamento herda o que a forma
/// não diz; a área e a massa da forma desfazem a escolha de Tamanho e de linha
/// de PV da principal. O `bonusPV` é da criatura, não da forma.
pub fn form_entry(main: &Value, form: &Value) -> Value {
    let mut out = main.as_object().cloned().unwrap_or_default();
    if let Some(f) = form.as_object() {
        for (k, v) in f {
            if k == "bonusPV" {
                continue;
            }
            match (v, out.get_mut(k)) {
                (Value::Object(vm), Some(Value::Object(om))) => {
                    for (kk, vv) in vm {
                        om.insert(kk.clone(), vv.clone());
                    }
                }
                _ => {
                    out.insert(k.clone(), v.clone());
                }
            }
        }
        if truthy(f.get("deslocamento")) {
            let mut d = obj(main.get("deslocamento")).cloned().unwrap_or_default();
            jv::spread_into(&mut d, f.get("deslocamento"));
            out.insert("deslocamento".into(), Value::Object(d));
        }
        if let Some(body) = obj(f.get("corpo"))
            && let Some(Value::Object(ob)) = out.get_mut("corpo")
        {
            if body.contains_key("area") && !body.contains_key("tamanhoEscolhido") {
                ob.shift_remove("tamanhoEscolhido");
            }
            if body.contains_key("massa") && !body.contains_key("linhaPV") {
                ob.shift_remove("linhaPV");
            }
            if (body.contains_key("massa") || body.contains_key("linhaPV"))
                && !body.contains_key("linhaPVMotivo")
            {
                ob.shift_remove("linhaPVMotivo");
            }
        }
    }
    Value::Object(out)
}

// ------------------------------------------------------------ a vista pública

/// O que a vista (`vistaPublicaCriatura`) entrega ao gerador do card: a ficha
/// pública e o bloco `motor`.
struct View {
    f: Map<String, Value>,
    stat: Map<String, Value>,
    motor: Map<String, Value>,
    forms: Vec<FormView>,
}

struct FormView {
    id: Value,
    name: Value,
    other_form: Option<Value>,
    view: View,
}

/// A peça de um ataque que o texto das habilidades usa (`pecas`).
struct Piece {
    name: String,
    dice: f64,
    extra: f64,
    charge_strength: f64,
    damage_type: String,
    pierce_text: String,
    no_damage: bool,
}

fn parse_creature(v: &Value) -> Result<Creature, CardError> {
    serde_json::from_value(v.clone())
        .map_err(|e| CardError::Creature(format!("{}: {e}", to_js_string(v.get("id")))))
}

/// `textoDaRajada`.
fn flurry_text(t: &CardTables, cap: Option<f64>, attack: Option<&str>) -> String {
    let r = jv::get(
        jv::get(
            jv::get(jv::get(Some(&t.rules), "combate"), "pgr"),
            "reforma",
        ),
        "rajada",
    );
    let field = |k: &str| to_js_string(jv::get(r, k));
    let tail = match cap.filter(|c| *c != 0.0 && !c.is_nan()) {
        Some(c) => format!(
            "Até {} golpes por ação{}.",
            n2s(c),
            attack.map_or(String::new(), |a| format!(" ({a})"))
        ),
        None => "O limite depende da classe da arma.".into(),
    };
    format!(
        "Só corpo a corpo. Cada golpe além do primeiro soma {} Tick de Golpe e {} de Recuperação, com {}d6 acumulativo no acerto. {tail}",
        field("golpeExtraTicksDeGolpe"),
        field("golpeExtraTicksDeRecuperacao"),
        field("penDadosPorGolpeExtra"),
    )
}

/// `tetoDaRajadaDaCriatura`: o teto da Rajada pela classe do ataque.
fn flurry_cap_of(t: &CardTables, a: &Value) -> Option<f64> {
    let class = if truthy(a.get("arma")) {
        jv::get(t.weapon_block(a.get("arma")), "classe")
    } else {
        a.get("classe")
    };
    let teto = jv::get(
        jv::get(
            jv::get(
                jv::get(jv::get(Some(&t.rules), "combate"), "pgr"),
                "reforma",
            ),
            "rajada",
        ),
        "teto",
    );
    jv::get(teto, &jv::property_key(class))
        .filter(|x| !x.is_null())
        .map(|x| jv::to_number(Some(x)))
}

/// `textoDaRegraDaCriatura`: o resumo da manobra do livro.
fn rule_text(
    t: &CardTables,
    id: Option<&str>,
    cap: Option<f64>,
    attack: Option<&str>,
) -> Option<String> {
    let id = id.filter(|s| !s.is_empty())?;
    if id == "rajada" {
        return Some(flurry_text(t, cap, attack));
    }
    let s = match id {
        "agarrar" => {
            "Ataque de Força ou Destreza + Briga contra a Defesa de agarrão. O acerto agarra e não fere. Quem controla rola Manter a cada 6 Ticks; o agarrado não age nem rola. Não existe Rajada de agarrão."
        }
        "investida" => {
            "Gaste o Preparo correndo em vez de andando. A Investida soma a Força mais uma vez ao dano, sem dado a mais (uma mão ou o corpo, de × 1 para × 2; arma de duas mãos, de × 2 para × 3; o golpe comum, sem investir, soma a Força como sempre) e aplica −2 adicionais na Defesa durante o Preparo."
        }
        "furtividade" => {
            "A aproximação escondida usa Furtividade contra a Percepção Passiva de quem pode perceber. O Mestre aplica as circunstâncias e o Tamanho na mesa."
        }
        "derrubar" => {
            "Ataque de Força ou Destreza + Briga contra a melhor Defesa do alvo. O sucesso deixa o alvo Caído; levantar-se é uma ação de Velocidade 3."
        }
        "empurrar" => {
            "Ataque comum contra a Defesa do alvo. O sucesso afasta o alvo 1 m mais 1 m por Margem, para longe de quem empurra."
        }
        "empunhadura-dupla" => {
            "Um golpe com cada mão na mesma ação, ambos a −1d6, cada um com seu acerto e dano. Só as mãos fazem par; o ciclo depende das classes das armas. Não acumula com Rajada."
        }
        "horda" => {
            "Trate o bando de capangas iguais como um esquadrão. A quantidade determina a Magnitude, que sobe um degrau a cada dobra do grupo; resolva os golpes pela Regra de Horda do livro."
        }
        _ => return None,
    };
    Some(s.to_owned())
}

/// `jogadaDaManobra`: a jogada do agarrão e da derrubada desta criatura.
fn maneuver_roll_text(rule: Option<&str>, c: &Value) -> Option<String> {
    if !matches!(rule, Some("agarrar" | "derrubar")) {
        return None;
    }
    let at = c.get("atributos");
    let strength = num_field(at, "forca");
    let dex = num_field(at, "destreza");
    let brawl = num_field(c.get("habilidades"), "briga");
    let (name, value) = if strength >= dex {
        ("Força", strength)
    } else {
        ("Destreza", dex)
    };
    let sum = value + brawl;
    let spark = num_field(Some(c), "centelha");
    let bonus = (if is_odd(sum) { 2.0 } else { 0.0 }) + 2.0 * js::min(spark, brawl);
    Some(format!(
        "Nesta criatura: {}d6{} ({name} {} + Briga {}).",
        n2s((sum / 2.0).floor()),
        if bonus != 0.0 && !bonus.is_nan() {
            format!(" +{}", n2s(bonus))
        } else {
            String::new()
        },
        n2s(value),
        n2s(brawl),
    ))
}

/// `danoDaInvestidaNoCard`: o dano do golpe ligado à Investida, com a Força a mais.
fn charge_text(rule: Option<&str>, piece: Option<&Piece>, strength: f64) -> Option<String> {
    if rule != Some("investida") {
        return None;
    }
    let Some(a) = piece else {
        return Some(format!(
            "Nesta criatura: a Força {} mais uma vez no dano do golpe.",
            n2s(strength)
        ));
    };
    let fixed = a.extra + a.charge_strength;
    Some(format!(
        "Nesta criatura, {} investindo: {}d6{} {}{} (a Força {} mais uma vez).",
        a.name,
        n2s(a.dice),
        signed_tail(fixed),
        a.damage_type,
        a.pierce_text,
        n2s(a.charge_strength),
    ))
}

/// `forcaDaInvestida`.
fn charge_strength(rules: &Rules, strength: f64, multiplier: f64, ranged: bool) -> f64 {
    let extra = rules
        .combat
        .movement
        .and_then(|m| m.charge)
        .and_then(|c| c.extra_strength)
        .unwrap_or(1.0);
    if ranged || multiplier.is_nan() || multiplier <= 0.0 {
        0.0
    } else {
        js::max(0.0, strength) * extra
    }
}

fn damage_sigla(kind: Option<&Value>) -> String {
    let k = kind.and_then(Value::as_str);
    let key = if k == Some("perfuracao") {
        Some("perfurante")
    } else {
        k
    };
    match key {
        Some("corte") => "(C)".into(),
        Some("perfurante") => "(P)".into(),
        Some("impacto") => "(I)".into(),
        _ => format!("({})", to_js_string(kind)),
    }
}

/// `vistaPublicaCriatura`.
fn public_view(t: &CardTables, original: &Value, with_forms: bool) -> Result<View, CardError> {
    let rules = t.rules();
    let form_id = jv::get(original.get("formaPrincipal"), "id").and_then(Value::as_str);
    // as habilidades que valem nesta forma
    let mut c = original.clone();
    if let Some(m) = c.as_object_mut() {
        let kept: Vec<Value> = list(original.get("habilidadesEspeciais"))
            .iter()
            .filter(|h| in_form(h, form_id))
            .cloned()
            .collect();
        m.insert("habilidadesEspeciais".into(), Value::Array(kept));
    }
    let typed = parse_creature(&c)?;
    let r = creature::derive(&t.creature, &typed);
    let attacks = list(c.get("ataques"));
    let at = c.get("atributos");
    let strength = num_field(at, "forca");

    // a arma de cada ataque (o bloco `arma` do catálogo), pelo índice
    let blocks: Vec<Option<&Value>> = attacks
        .iter()
        .map(|a| t.weapon_block(a.get("arma")))
        .collect();
    let block_id = |i: usize| blocks[i].map(|_| to_js_string(attacks[i].get("arma")));
    let two_handed: Vec<usize> = (0..attacks.len())
        .filter(|&i| {
            blocks[i].is_some_and(|w| {
                jv::num_or(w.get("maos"), f64::NAN) == 2.0
                    && !jv::is_str(w.get("classe"), "distancia")
            })
        })
        .collect();
    let two_ids: Vec<String> = two_handed.iter().filter_map(|&i| block_id(i)).collect();
    let is_two = |i: usize| block_id(i).is_some_and(|id| two_ids.contains(&id));
    let body_block = attacks
        .iter()
        .filter(|a| !truthy(a.get("arma")))
        .fold(0.0, |m, a| {
            js::max(m, jv::num_or(a.get("bloqueiaComoArma"), 0.0))
        });
    let without_two = |idx: &[usize]| {
        idx.iter().fold(0.0, |m, &i| {
            let a = &attacks[i];
            let v = if truthy(a.get("arma")) {
                jv::num_or(jv::get(blocks[i], "defesaArma"), 0.0)
            } else {
                jv::num_or(a.get("bloqueiaComoArma"), 0.0)
            };
            js::max(m, v)
        })
    };
    let all: Vec<usize> = (0..attacks.len()).collect();
    let weapon_block_bonus = if two_handed.is_empty() {
        without_two(&all)
    } else {
        two_handed.iter().fold(f64::NEG_INFINITY, |m, &i| {
            js::max(m, jv::num_or(jv::get(blocks[i], "defesaArma"), 0.0))
        }) + body_block
    };
    let defense = js::max(r.dodge, r.block + weapon_block_bonus);
    let other = if two_handed.is_empty() {
        None
    } else {
        (0..attacks.len()).find(|&i| truthy(attacks[i].get("arma")) && !is_two(i))
    };
    let main_weapon = if two_handed.is_empty() {
        None
    } else {
        (0..attacks.len()).find(|&i| is_two(i))
    };
    let other_defense = other.map(|o| {
        let rest: Vec<usize> = (0..attacks.len()).filter(|&i| !is_two(i)).collect();
        let bl = without_two(&rest);
        let mut m = Map::new();
        m.insert(
            "arma".into(),
            attacks[o].get("nome").cloned().unwrap_or(Value::Null),
        );
        put(
            &mut m,
            "armaPrincipal",
            main_weapon.and_then(|p| attacks[p].get("nome").cloned()),
        );
        m.insert("bloqueio".into(), num(r.block + bl));
        m.insert("defesa".into(), num(js::max(r.dodge, r.block + bl)));
        if let Some(v) = &r.with_equipment {
            m.insert("vestida".into(), num(js::max(v.dodge, v.block + bl)));
        }
        Value::Object(m)
    });
    let worn = r.with_equipment.map(|v| {
        json!({
            "esquiva": num(v.dodge),
            "bloqueio": num(v.block + weapon_block_bonus),
            "defesa": num(js::max(v.dodge, v.block + weapon_block_bonus)),
            "contraTiro": num(v.against_shots.unwrap_or(v.dodge)),
            "absorcao": {"impacto": num(v.soak.impact), "corte": num(v.soak.cut), "perfuracao": num(v.soak.pierce)},
            "nivel": num(v.pierce_level.unwrap_or(0.0)),
            "penalidade": num(v.penalty),
        })
    });

    // os ataques em texto
    let spark = num_field(Some(&c), "centelha");
    let mut pieces = Vec::with_capacity(attacks.len());
    let mut attack_out = Vec::with_capacity(attacks.len());
    for (i, a) in attacks.iter().enumerate() {
        let w = blocks[i];
        let d = &r.attacks[i];
        let skill = d.skill.as_deref();
        let skill_value = skill.map_or(0.0, |h| {
            jv::num_or(
                jv::coalesce(
                    jv::get(c.get("habilidades"), h),
                    jv::get(c.get("habilidadesSec"), h),
                ),
                0.0,
            )
        });
        let natural = !truthy(a.get("arma"));
        let attr = match skill {
            Some("disparo") => "vigor",
            Some("atirador") if natural => {
                let (v, dx, p) = (
                    num_field(at, "vigor"),
                    num_field(at, "destreza"),
                    num_field(at, "percepcao"),
                );
                if v >= dx && v >= p {
                    "vigor"
                } else if dx >= p {
                    "destreza"
                } else {
                    "percepcao"
                }
            }
            Some("atirador") => "percepcao",
            _ => {
                if strength >= num_field(at, "destreza") {
                    "forca"
                } else {
                    "destreza"
                }
            }
        };
        let sum = skill.map_or(0.0, |h| {
            creature::roll_attribute(&typed, h, natural) + skill_value
        });
        let accuracy = d.accuracy + 2.0 * js::min(spark, skill_value);
        let wd = typed.attacks[i]
            .weapon
            .as_deref()
            .and_then(|id| weapon_attack(&t.creature, id, strength, false, typed.attacks[i].size));
        let dice: Option<String> = match &wd {
            Some(w) => Some(weapon_dice_text(w.dice, w.bonus)),
            None => a.get("dado").and_then(Value::as_str).map(str::to_owned),
        };
        let dice_is_null = wd.is_none() && matches!(a.get("dado"), Some(Value::Null));
        let extra = if dice_is_null {
            0.0
        } else {
            let base = dice
                .as_deref()
                .and_then(|x| creature::average_damage(rules, Some(strip_flat(x)), 0.0, 0.0, 0.0))
                .unwrap_or(0.0);
            d.average_damage.unwrap_or(0.0) - base
        };
        let modes = jv::get(w, "modos");
        let mode = list(modes)
            .iter()
            .find(|m| jv::strict_eq(m.get("tipo"), jv::get(w, "fichaModo")))
            .or_else(|| list(modes).iter().find(|m| truthy(m.get("principal"))));
        let kind = jv::coalesce(
            jv::coalesce(mode.and_then(|m| m.get("tipo")), jv::get(w, "tipoDano")),
            a.get("tipoDano"),
        );
        let sigla = damage_sigla(kind);
        let pierce =
            (jv::is_str(kind, "perfuracao") || jv::is_str(kind, "perfurante")).then(|| {
                jv::num_or(
                    jv::coalesce(
                        jv::coalesce(mode.and_then(|m| m.get("perf")), jv::get(w, "pen")),
                        a.get("nivelPerfuracao"),
                    ),
                    0.0,
                )
            });
        let ranged = matches!(skill, Some("disparo" | "atirador" | "arremesso"));
        let cs = match &wd {
            Some(w) => charge_strength(rules, w.strength, w.strength_multiplier, ranged),
            None => charge_strength(
                rules,
                strength,
                jv::num_or(a.get("multiplicadorForca"), 0.0),
                ranged,
            ),
        };
        let pierce_text = pierce.map_or(String::new(), |p| format!(" \u{b7} perf. {}", n2s(p)));
        let lead = leading_number(dice.as_deref());
        pieces.push(Piece {
            name: to_js_string(a.get("nome")),
            dice: lead,
            extra,
            charge_strength: cs,
            damage_type: sigla.clone(),
            pierce_text: pierce_text.clone(),
            no_damage: dice_is_null,
        });
        let class = jv::coalesce(jv::get(w, "classe"), a.get("classe")).cloned();
        let mut m = Map::new();
        m.insert("nome".into(), a.get("nome").cloned().unwrap_or(Value::Null));
        m.insert("atrib".into(), json!(attr));
        m.insert("pericia".into(), skill.map_or(Value::Null, |s| json!(s)));
        m.insert("dado".into(), num(lead));
        put(&mut m, "tipo", kind.cloned());
        put(&mut m, "classe", class);
        m.insert("ticks".into(), d.speed.map_or(Value::Null, num));
        m.insert("acerto".into(), num(d.accuracy));
        let odd = is_odd(sum);
        m.insert(
            "pool".into(),
            json!(format!(
                "{}d6{}{}",
                n2s((sum / 2.0).floor()),
                if odd { "+2" } else { "" },
                if accuracy != 0.0 && !accuracy.is_nan() {
                    format!(
                        "{}{}",
                        if accuracy > 0.0 { " +" } else { " \u{2212}" },
                        n2s(accuracy.abs())
                    )
                } else {
                    String::new()
                }
            )),
        );
        if skill.is_none()
            && natural
            && (jv::is_str(a.get("classe"), "distancia")
                || jv::is_str(a.get("classe"), "arremesso"))
        {
            m.insert("semJogada".into(), json!(true));
        }
        let damage = if dice_is_null {
            "nenhum".to_owned()
        } else {
            let base = dice
                .as_deref()
                .map_or("undefined".to_owned(), |x| strip_flat(x).to_owned());
            format!("{base}{} {sigla}{pierce_text}", signed_tail(extra))
        };
        m.insert("dano".into(), json!(damage));
        m.insert("perfArma".into(), pierce.map_or(Value::Null, num));
        m.insert("forcaInvestida".into(), num(cs));
        attack_out.push(Value::Object(m));
    }

    // o deslocamento declarado e a locomoção por modo
    let moves = obj(c.get("deslocamento"));
    let main_move = moves.and_then(|d| {
        d.get("terra")
            .filter(|x| !x.is_null())
            .or_else(|| d.values().find(|v| truthy(Some(v))))
    });
    let batalha = jv::num_or(jv::get(main_move, "batalha"), 0.0);
    let declared = json!({
        "batalha": num(batalha),
        "arranque": num(jv::num_or(jv::coalesce(jv::get(main_move, "arranque"), jv::get(main_move, "batalha")), 0.0)),
        "corrida": num(jv::num_or(jv::coalesce(jv::get(main_move, "corrida"), jv::get(main_move, "batalha")), 0.0)),
    });
    let mut locomotion = Map::new();
    locomotion.insert(
        "terra".into(),
        jv::get(moves.and_then(|d| d.get("terra")), "batalha")
            .cloned()
            .unwrap_or(Value::Null),
    );
    if let Some(d) = moves {
        for (k, v) in d {
            if k == "terra" {
                continue;
            }
            let key = if k == "nado" { "natacao" } else { k.as_str() };
            let val = if v.is_null() {
                Value::Null
            } else {
                v.get("batalha").cloned().unwrap_or(Value::Null)
            };
            locomotion.insert(key.to_owned(), val);
        }
    }

    // as habilidades especiais, com a prosa das manobras
    let mut abilities = Vec::new();
    for h in list(c.get("habilidadesEspeciais")) {
        let linked = if truthy(h.get("ataque")) {
            attacks
                .iter()
                .position(|a| jv::strict_eq(a.get("nome"), h.get("ataque")))
        } else {
            None
        };
        let rule = h.get("regra").and_then(Value::as_str);
        let piece = linked.map(|i| &pieces[i]).filter(|p| !p.no_damage);
        let parts = [
            rule_text(
                t,
                rule,
                linked.and_then(|i| flurry_cap_of(t, &attacks[i])),
                linked.map(|i| pieces[i].name.as_str()),
            ),
            maneuver_roll_text(rule, &c),
            charge_text(rule, piece, strength),
            truthy(h.get("texto")).then(|| to_js_string(h.get("texto"))),
            truthy(h.get("efeito")).then(|| to_js_string(h.get("efeito"))),
        ];
        let prose = parts
            .iter()
            .flatten()
            .filter(|s| !s.is_empty())
            .map(|s| trim_period(s).to_owned())
            .collect::<Vec<_>>()
            .join(". ");
        let fields: Vec<(&str, String)> = ABILITY_FIELDS
            .iter()
            .filter(|(k, _)| truthy(h.get(*k)))
            .map(|(k, label)| (*label, to_js_string(h.get(*k))))
            .collect();
        let mut desc_parts = vec![prose.clone()];
        desc_parts.extend(
            fields
                .iter()
                .map(|(l, v)| format!("{l}: {}", trim_period(v))),
        );
        let mut description = desc_parts
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(". ");
        if description.is_empty() {
            description = if truthy(h.get("regra")) {
                to_js_string(h.get("regra"))
            } else {
                String::new()
            };
        }
        let mut m = Map::new();
        m.insert("nome".into(), h.get("nome").cloned().unwrap_or(Value::Null));
        put(&mut m, "classe", h.get("classe").cloned());
        if truthy(h.get("base")) {
            m.insert("base".into(), h["base"].clone());
        }
        if truthy(h.get("regra")) {
            m.insert("regra".into(), h["regra"].clone());
        }
        if truthy(h.get("tecnica")) {
            m.insert("tecnica".into(), h["tecnica"].clone());
        }
        if truthy(h.get("arte")) {
            m.insert("arte".into(), h["arte"].clone());
        }
        m.insert("descricao".into(), json!(description));
        if !fields.is_empty() {
            m.insert("texto".into(), json!(prose));
            m.insert(
                "campos".into(),
                Value::Array(
                    fields
                        .iter()
                        .map(|(l, v)| json!({"rotulo": l, "valor": v}))
                        .collect(),
                ),
            );
        }
        abilities.push(Value::Object(m));
    }

    // a ficha pública
    let size = r.size.map(size_label);
    let body = c.get("corpo");
    let measure = {
        let mut s = format!("{} m de altura", to_js_string(jv::get(body, "altura")));
        if truthy(jv::get(body, "comprimento")) {
            s.push_str(&format!(
                ", {} m de comprimento",
                to_js_string(jv::get(body, "comprimento"))
            ));
        }
        s
    };
    let weight = if truthy(jv::get(body, "semMassa")) {
        "sem massa".to_owned()
    } else {
        format!("{} kg", to_js_string(jv::get(body, "massa")))
    };
    let mut virtues = jv::spread(c.get("virtudes"));
    virtues.insert(
        "valor".into(),
        jv::get(c.get("virtudes"), "bravura")
            .cloned()
            .unwrap_or(Value::Null),
    );
    let images = list(c.get("imagens"));
    let mut tech_ids: Vec<Value> = Vec::new();
    for h in list(c.get("habilidadesEspeciais")) {
        if truthy(h.get("tecnica")) {
            let id = jv::get(h.get("tecnica"), "id")
                .cloned()
                .unwrap_or(Value::Null);
            if !tech_ids.contains(&id) {
                tech_ids.push(id);
            }
        }
    }
    let elements = |k: &str| -> Value {
        Value::Array(
            list(jv::get(c.get("elementos"), k))
                .iter()
                .filter(|x| in_form(x, form_id))
                .map(|x| x.get("tipo").cloned().unwrap_or(Value::Null))
                .collect(),
        )
    };
    let mut f = Map::new();
    f.insert("id".into(), c.get("id").cloned().unwrap_or(Value::Null));
    f.insert(
        "categoria".into(),
        c.get("categoria").cloned().unwrap_or(Value::Null),
    );
    put(&mut f, "nomeIngles", c.get("nomeIngles").cloned());
    put(&mut f, "descricao", c.get("descricao").cloned());
    f.insert(
        "imagem".into(),
        images.first().cloned().unwrap_or(Value::Null),
    );
    if images.get(1).is_some_and(|x| truthy(Some(x))) {
        f.insert("imagem2".into(), images[1].clone());
    }
    f.insert("porte".into(), size.map_or(Value::Null, |s| json!(s)));
    f.insert(
        "dimensoes".into(),
        json!({"medida": measure, "peso": weight}),
    );
    f.insert("virtues".into(), Value::Object(virtues));
    put(&mut f, "aparencia", c.get("aparencia").cloned());
    f.insert("habilidades".into(), Value::Array(abilities));
    put(&mut f, "lore", c.get("lore").cloned());
    f.insert(
        "ecologia".into(),
        json!({
            "tipo": c.get("categoria").cloned().unwrap_or(Value::Null),
            "terreno": jv::get(c.get("ecologia"), "terreno").cloned(),
            "clima": jv::get(c.get("ecologia"), "clima").cloned(),
        }),
    );
    f.insert("deslocamentoDeclarado".into(), declared);
    f.insert("locomocao".into(), Value::Object(locomotion));
    for k in ["fraquezas", "resistencias", "imunidades"] {
        f.insert(k.into(), elements(k));
    }

    let defense_text = |d: Defense| match d {
        Defense::Immune => json!("-"),
        Defense::Value(x) => num(x),
    };
    let mut stat = Map::new();
    stat.insert("id".into(), c.get("id").cloned().unwrap_or(Value::Null));
    stat.insert("nome".into(), c.get("nome").cloned().unwrap_or(Value::Null));
    put(&mut stat, "tipo", c.get("papel").cloned());
    put(&mut stat, "ameaca", c.get("ameaca").cloned());
    put(&mut stat, "centelha", c.get("centelha").cloned());
    stat.insert("pv".into(), r.hp.map_or(Value::Null, num));
    stat.insert("defesa".into(), num(defense));
    stat.insert("defesaMental".into(), defense_text(r.mental_defense));
    stat.insert("defesaSocial".into(), defense_text(r.social_defense));
    put(&mut stat, "vontade", c.get("vontade").cloned());
    stat.insert(
        "soak".into(),
        json!({"impacto": num(r.soak.impact), "corte": num(r.soak.cut), "perfuracao": num(r.soak.pierce)}),
    );
    stat.insert("resistPerf".into(), json!(0));
    stat.insert(
        "iniciativa".into(),
        json!(format!("1d6 + {}", n2s(r.initiative))),
    );
    put(&mut stat, "atributos", c.get("atributos").cloned());
    stat.insert("ataques".into(), Value::Array(attack_out));
    stat.insert("tecnicas".into(), Value::Array(tech_ids));
    stat.insert(
        "artes".into(),
        Value::Array(
            obj(c.get("artes"))
                .map(|m| {
                    jv::keys(m)
                        .into_iter()
                        .map(|k| json!({"id": k, "nivel": m[k.as_str()]}))
                        .collect()
                })
                .unwrap_or_default(),
        ),
    );
    put(&mut stat, "vestida", worn);
    put(&mut stat, "defesaOutraArma", other_defense);

    let mut skills = Map::new();
    skills.insert("prontidao".into(), json!(0));
    skills.insert("furtividade".into(), json!(0));
    jv::spread_into(&mut skills, c.get("habilidades"));
    jv::spread_into(&mut skills, c.get("habilidadesSec"));

    let mut motor = Map::new();
    if with_forms {
        let mut edition = original.clone();
        if let Some(m) = edition.as_object_mut() {
            m.shift_remove("revisao");
        }
        motor.insert("edicao".into(), edition);
    }
    motor.insert("energia".into(), num(r.energy));
    motor.insert("pericias".into(), Value::Object(skills));
    motor.insert("mana".into(), r.mana.map_or(Value::Null, num));
    motor.insert(
        "couraca".into(),
        jv::coalesce(c.get("couraca"), Some(&json!(0)))
            .cloned()
            .unwrap_or(json!(0)),
    );
    if truthy(c.get("perfilCouraca")) {
        motor.insert("perfilCouraca".into(), c["perfilCouraca"].clone());
    }
    if truthy(jv::get(c.get("equipamento"), "veneno")) {
        motor.insert("veneno".into(), c["equipamento"]["veneno"].clone());
    }
    put(&mut motor, "enxame", c.get("enxame").cloned());
    put(&mut motor, "corpo", c.get("corpo").cloned());
    put(
        &mut motor,
        "formaPrincipal",
        c.get("formaPrincipal").cloned(),
    );
    motor.insert(
        "segundoTamanho".into(),
        r.second_size.map_or(Value::Null, |s| json!(size_label(s))),
    );
    motor.insert(
        "comoLuta".into(),
        Value::Array(
            list(c.get("comoLuta"))
                .iter()
                .filter(|x| in_form(x, form_id))
                .map(|x| {
                    let mut m = Map::new();
                    put(&mut m, "nome", x.get("nome").cloned());
                    put(&mut m, "texto", x.get("texto").cloned());
                    Value::Object(m)
                })
                .collect(),
        ),
    );
    motor.insert(
        "sentidos".into(),
        Value::Array(
            list(c.get("sentidos"))
                .iter()
                .filter(|x| in_form(x, form_id) && truthy(x.get("descricao")))
                .map(|x| {
                    let mut m = Map::new();
                    put(&mut m, "nome", x.get("nome").cloned());
                    put(&mut m, "descricao", x.get("descricao").cloned());
                    if truthy(x.get("classe")) {
                        m.insert("classe".into(), x["classe"].clone());
                    }
                    Value::Object(m)
                })
                .collect(),
        ),
    );

    let mut forms = Vec::new();
    if with_forms {
        for form in list(original.get("formas")) {
            let mut entry = form_entry(original, form);
            if let Some(m) = entry.as_object_mut() {
                m.insert(
                    "id".into(),
                    original.get("id").cloned().unwrap_or(Value::Null),
                );
                m.insert(
                    "nome".into(),
                    original.get("nome").cloned().unwrap_or(Value::Null),
                );
                m.insert(
                    "formaPrincipal".into(),
                    json!({"id": form.get("id"), "nome": form.get("nome")}),
                );
            }
            forms.push(FormView {
                id: form.get("id").cloned().unwrap_or(Value::Null),
                name: form.get("nome").cloned().unwrap_or(Value::Null),
                other_form: form.get("outraForma").filter(|x| truthy(Some(x))).cloned(),
                view: public_view(t, &entry, false)?,
            });
        }
    }
    Ok(View {
        f,
        stat,
        motor,
        forms,
    })
}

// ------------------------------------------------------------ o card

/// `classeDoAtaque`: a classe de tempo do ataque pelo nome (a arma do
/// catálogo, as palavras que decidem) ou pela Velocidade.
fn attack_class(t: &CardTables, id: &str, name: &str, ticks: Option<f64>) -> String {
    if let Some((_, c)) = CLASS_OVERRIDE.iter().find(|(x, _)| *x == id) {
        return (*c).to_owned();
    }
    let n = name.to_lowercase();
    if let Some(w) = t.weapons.iter().find(|w| {
        w.get("nome")
            .and_then(Value::as_str)
            .is_some_and(|x| n.starts_with(&x.to_lowercase()))
    }) {
        // a peça achatada: a classe do bloco `arma`
        return to_js_string(jv::coalesce(
            jv::get(w.get("arma"), "classe"),
            w.get("classe"),
        ));
    }
    if art_in_name(&n) {
        return "arte".into();
    }
    if ["arco", "besta", "funda"].iter().any(|w| has_word(&n, w)) {
        return "distancia".into();
    }
    if ["dardo", "azagaia", "arremess", "bumerangue", "pilum"]
        .iter()
        .any(|w| n.contains(w))
    {
        return "arremesso".into();
    }
    if ["forcado", "tridente", "lança", "pique", "alabarda", "chuço"]
        .iter()
        .any(|w| n.contains(w))
    {
        return "haste".into();
    }
    let v = ticks.unwrap_or(5.0);
    if v <= 5.0 {
        "leve".into()
    } else if v == 6.0 {
        "media".into()
    } else {
        "pesada".into()
    }
}

/// `/\([a-zà-ú]+ \d\)/`.
fn art_in_name(n: &str) -> bool {
    let cs: Vec<char> = n.chars().collect();
    (0..cs.len()).any(|i| {
        if cs[i] != '(' {
            return false;
        }
        let mut j = i + 1;
        while j < cs.len() && (cs[j].is_ascii_lowercase() || ('\u{e0}'..='\u{fa}').contains(&cs[j]))
        {
            j += 1;
        }
        j > i + 1
            && cs.get(j) == Some(&' ')
            && cs.get(j + 1).is_some_and(char::is_ascii_digit)
            && cs.get(j + 2) == Some(&')')
    })
}

/// `\bpalavra\b`, com a fronteira de palavra do JavaScript (letras ASCII,
/// dígitos e `_`).
fn has_word(n: &str, w: &str) -> bool {
    let word = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
    n.match_indices(w)
        .any(|(i, _)| !word(n[..i].chars().last()) && !word(n[i + w.len()..].chars().next()))
}

/// `descricaoDaRef`: a Técnica ou a Arte do livro que a habilidade cita.
fn reference_text(t: &CardTables, x: &Value) -> Result<Value, CardError> {
    let (kind, names, r) = if truthy(x.get("tecnica")) {
        ("Técnica", &t.technique_names, &x["tecnica"])
    } else if truthy(x.get("arte")) {
        ("Arte", &t.art_names, &x["arte"])
    } else {
        return Ok(x.get("descricao").cloned().unwrap_or(Value::Null));
    };
    let id = to_js_string(r.get("id"));
    let Some(name) = names.get(&id) else {
        return Err(CardError::MissingReference(format!(
            "{}: {kind} {id} não está no catálogo",
            to_js_string(x.get("nome"))
        )));
    };
    let shown = if same_base(name, &to_js_string(x.get("nome"))) {
        String::new()
    } else {
        format!(" {name}")
    };
    Ok(json!(format!(
        "{kind}{shown} do livro, nível {}",
        to_js_string(r.get("nivel"))
    )))
}

/// As justificativas dos ataques, da fonte crua (da forma, quando ela tem a
/// lista dela).
fn justifications(raw: Option<&Value>, form_id: Option<&Value>) -> Map<String, Value> {
    let Some(raw) = raw else { return Map::new() };
    let form = form_id.and_then(|id| {
        list(raw.get("formas"))
            .iter()
            .find(|f| jv::strict_eq(f.get("id"), Some(id)))
    });
    let attacks = jv::coalesce(form.and_then(|f| f.get("ataques")), raw.get("ataques"));
    let mut m = Map::new();
    for a in list(attacks) {
        if truthy(a.get("justificativa")) {
            m.insert(to_js_string(a.get("nome")), a["justificativa"].clone());
        }
    }
    m
}

/// O `build` de `gen-monsters.mjs`: o card de uma criatura (ou de uma forma).
fn build(
    t: &CardTables,
    c: &Map<String, Value>,
    view: &View,
    raw_of_creature: Option<&Value>,
) -> Result<Value, CardError> {
    let f = &view.f;
    let motor = &view.motor;
    let raw = motor.get("edicao").or(raw_of_creature);
    let form_id = if motor.contains_key("formaPrincipal") && !motor.contains_key("edicao") {
        jv::get(motor.get("formaPrincipal"), "id")
    } else {
        None
    };
    let just = justifications(raw, form_id);
    let category = category_id(jv::or(f.get("categoria"), c.get("categoria")));
    let category_name = CATEGORY_NAME
        .iter()
        .find(|(k, _)| *k == category)
        .map_or(category.clone(), |(_, n)| (*n).to_owned());
    let mut tags = vec![json!(category)];
    if jv::num_or(c.get("centelha"), 0.0) > 0.0 {
        tags.push(json!("centelha"));
    }
    if list(c.get("artes"))
        .iter()
        .any(|a| jv::num_or(a.get("nivel"), f64::NAN) > 0.0)
    {
        tags.push(json!("mágico"));
        tags.push(json!("arcano"));
    }
    let porte = f.get("porte").filter(|x| truthy(Some(x))).cloned();
    let id = to_js_string(c.get("id"));

    let mut out = Map::new();
    if let Some(e) = motor.get("edicao") {
        out.insert("edicao".into(), e.clone());
    }
    out.insert("id".into(), c.get("id").cloned().unwrap_or(Value::Null));
    out.insert("nome".into(), c.get("nome").cloned().unwrap_or(Value::Null));
    out.insert(
        "nomeIngles".into(),
        jv::or(f.get("nomeIngles"), None)
            .cloned()
            .unwrap_or(Value::Null),
    );
    out.insert("categoria".into(), json!(category));
    out.insert("categoriaNome".into(), json!(category_name));
    put(&mut out, "tipo", c.get("tipo").cloned());
    let concept: Vec<String> = [
        Some(category_name.clone()),
        porte.as_ref().map(|p| to_js_string(Some(p))),
    ]
    .into_iter()
    .flatten()
    .filter(|s| !s.is_empty())
    .collect();
    out.insert("conceito".into(), json!(concept.join(" ")));
    out.insert(
        "descricao".into(),
        jv::or(f.get("descricao"), Some(&json!("")))
            .cloned()
            .unwrap_or(json!("")),
    );
    out.insert("tags".into(), Value::Array(tags));
    put(&mut out, "ameaca", c.get("ameaca").cloned());
    put(&mut out, "centelha", c.get("centelha").cloned());
    out.insert("tamanho".into(), porte.clone().unwrap_or(Value::Null));
    let mut body = jv::spread(motor.get("corpo"));
    let dims = f.get("dimensoes");
    body.insert(
        "medida".into(),
        jv::or(jv::get(dims, "medida"), None)
            .cloned()
            .unwrap_or(Value::Null),
    );
    body.insert(
        "peso".into(),
        jv::or(jv::get(dims, "peso"), None)
            .cloned()
            .unwrap_or(Value::Null),
    );
    out.insert("corpo".into(), Value::Object(body));
    let eco = f.get("ecologia");
    out.insert(
        "ecologia".into(),
        json!({
            "tipo": jv::or(jv::get(eco, "tipo"), None).cloned().unwrap_or(Value::Null),
            "terreno": jv::or(jv::get(eco, "terreno"), Some(&json!([]))).cloned(),
            "clima": jv::or(jv::get(eco, "clima"), Some(&json!([]))).cloned(),
        }),
    );
    out.insert(
        "imagens".into(),
        Value::Array(
            [f.get("imagem"), f.get("imagem2")]
                .into_iter()
                .filter(|x| truthy(*x))
                .map(|x| x.cloned().unwrap_or(Value::Null))
                .collect(),
        ),
    );
    out.insert("semImagem".into(), json!(!truthy(f.get("imagem"))));
    put(&mut out, "atributos", c.get("atributos").cloned());
    // as Habilidades públicas
    let all_skills = jv::or(c.get("pericias"), motor.get("pericias"));
    let mut primary = Map::new();
    primary.insert("prontidao".into(), json!(0));
    primary.insert("furtividade".into(), json!(0));
    let mut secondary = Map::new();
    if let Some(m) = obj(all_skills) {
        for k in jv::keys(m) {
            if t.primary_skills.contains(k.as_str()) {
                primary.insert(k.clone(), m[k.as_str()].clone());
            } else {
                secondary.insert(k.clone(), m[k.as_str()].clone());
            }
        }
    }
    out.insert("habilidades".into(), Value::Object(primary));
    out.insert("habilidadesSec".into(), Value::Object(secondary));
    out.insert(
        "vontade".into(),
        jv::coalesce(c.get("vontade"), Some(&json!(5)))
            .cloned()
            .unwrap_or(json!(5)),
    );
    put(&mut out, "aparencia", f.get("aparencia").cloned());
    put(&mut out, "virtudes", f.get("virtues").cloned());

    // o combate
    let soak = c.get("soak");
    let mut combat = Map::new();
    combat.insert("pv".into(), c.get("pv").cloned().unwrap_or(Value::Null));
    combat.insert(
        "energia".into(),
        jv::coalesce(motor.get("energia"), None)
            .cloned()
            .unwrap_or(Value::Null),
    );
    put(&mut combat, "defesa", c.get("defesa").cloned());
    put(&mut combat, "defesaSocial", c.get("defesaSocial").cloned());
    put(&mut combat, "defesaMental", c.get("defesaMental").cloned());
    combat.insert(
        "absorcao".into(),
        json!({
            "impacto": jv::get(soak, "impacto"),
            "corte": jv::get(soak, "corte"),
            "perfuracao": jv::get(soak, "perfuracao"),
        }),
    );
    combat.insert(
        "resistenciaPerfuracao".into(),
        jv::or(c.get("resistPerf"), Some(&json!(0)))
            .cloned()
            .unwrap_or(json!(0)),
    );
    if truthy(motor.get("veneno")) {
        combat.insert("veneno".into(), motor["veneno"].clone());
    }
    if truthy(motor.get("couraca")) {
        combat.insert("couraca".into(), motor["couraca"].clone());
        if truthy(motor.get("perfilCouraca")) {
            combat.insert("perfilCouraca".into(), motor["perfilCouraca"].clone());
        }
    }
    if truthy(c.get("vestida")) {
        combat.insert("vestida".into(), c["vestida"].clone());
    }
    if truthy(c.get("defesaOutraArma")) {
        combat.insert("defesaOutraArma".into(), c["defesaOutraArma"].clone());
    }
    for k in ["fraquezas", "resistencias", "imunidades"] {
        if !list(f.get(k)).is_empty() {
            combat.insert(k.into(), f[k].clone());
        }
    }
    put(&mut combat, "iniciativa", c.get("iniciativa").cloned());
    let d = f
        .get("deslocamentoDeclarado")
        .filter(|x| truthy(Some(x)))
        .ok_or_else(|| CardError::NoMovement(id.clone()))?;
    combat.insert(
        "deslocamento".into(),
        json!({"batalha": d.get("batalha"), "arranque": d.get("arranque"), "corrida": d.get("corrida")}),
    );
    let attacks = list(c.get("ataques"))
        .iter()
        .map(|a| {
            let ticks = a
                .get("ticks")
                .filter(|x| !x.is_null())
                .map(|x| jv::to_number(Some(x)));
            let class = match a.get("classe").filter(|x| !x.is_null()) {
                Some(cl) => cl.clone(),
                None => json!(attack_class(t, &id, &to_js_string(a.get("nome")), ticks)),
            };
            let mut m = Map::new();
            put(&mut m, "nome", a.get("nome").cloned());
            put(&mut m, "pool", a.get("pool").cloned());
            put(&mut m, "dano", a.get("dano").cloned());
            m.insert(
                "perfArma".into(),
                jv::coalesce(a.get("perfArma"), None)
                    .cloned()
                    .unwrap_or(Value::Null),
            );
            put(&mut m, "speed", a.get("ticks").cloned());
            m.insert("classe".into(), class);
            m.insert(
                "forcaInvestida".into(),
                jv::coalesce(a.get("forcaInvestida"), Some(&json!(0)))
                    .cloned()
                    .unwrap_or(json!(0)),
            );
            if truthy(a.get("semJogada")) {
                m.insert("semJogada".into(), json!(true));
            }
            if truthy(a.get("notas")) {
                m.insert("notas".into(), a["notas"].clone());
            }
            Value::Object(m)
        })
        .collect();
    combat.insert("ataques".into(), Value::Array(attacks));
    out.insert("combate".into(), Value::Object(combat));

    if !just.is_empty() {
        out.insert("justificativaDosAtaques".into(), Value::Object(just));
    }
    let mut abilities = Vec::new();
    for x in list(f.get("habilidades")) {
        let mut m = Map::new();
        put(&mut m, "nome", x.get("nome").cloned());
        let desc = if truthy(x.get("descricao")) {
            x["descricao"].clone()
        } else {
            reference_text(t, x)?
        };
        m.insert("descricao".into(), desc);
        for k in ["classe", "base", "regra"] {
            if truthy(x.get(k)) {
                m.insert(k.into(), x[k].clone());
            }
        }
        if truthy(x.get("campos")) {
            put(&mut m, "texto", x.get("texto").cloned());
            m.insert("campos".into(), x["campos"].clone());
        }
        abilities.push(Value::Object(m));
    }
    out.insert("habilidadesEspeciais".into(), Value::Array(abilities));
    if truthy(motor.get("enxame")) {
        out.insert("enxame".into(), motor["enxame"].clone());
    }
    out.insert(
        "comoLuta".into(),
        jv::or(motor.get("comoLuta"), Some(&json!([])))
            .cloned()
            .unwrap_or(json!([])),
    );
    out.insert(
        "sentidos".into(),
        jv::or(motor.get("sentidos"), Some(&json!([])))
            .cloned()
            .unwrap_or(json!([])),
    );
    out.insert(
        "segundoTamanho".into(),
        jv::or(motor.get("segundoTamanho"), None)
            .cloned()
            .unwrap_or(Value::Null),
    );
    if truthy(motor.get("formaPrincipal")) {
        out.insert("formaPrincipal".into(), motor["formaPrincipal"].clone());
    }
    let mut forms = Vec::new();
    for fo in &view.forms {
        let mut m = Map::new();
        m.insert("id".into(), fo.id.clone());
        m.insert("nome".into(), fo.name.clone());
        put(&mut m, "outraForma", fo.other_form.clone());
        m.insert("criatura".into(), build(t, &fo.view.stat, &fo.view, raw)?);
        forms.push(Value::Object(m));
    }
    out.insert("formas".into(), Value::Array(forms));
    out.insert("poderes".into(), json!([]));
    if let Some(Value::Object(l)) = f.get("locomocao")
        && !l.is_empty()
    {
        out.insert("locomocao".into(), Value::Object(l.clone()));
    }
    out.insert(
        "tecnicas".into(),
        jv::or(c.get("tecnicas"), Some(&json!([])))
            .cloned()
            .unwrap_or(json!([])),
    );
    out.insert(
        "artes".into(),
        Value::Array(
            list(c.get("artes"))
                .iter()
                .map(|a| {
                    let id = match a.get("id") {
                        Some(x) if truthy(jv::get(Some(x), "id")) => x["id"].clone(),
                        x => x.cloned().unwrap_or(Value::Null),
                    };
                    json!({"id": id, "nivel": a.get("nivel")})
                })
                .collect(),
        ),
    );
    if !jv::nullish(motor.get("mana")) {
        out.insert("mana".into(), motor["mana"].clone());
    }
    out.insert(
        "lore".into(),
        Value::Array(
            list(f.get("lore"))
                .iter()
                .map(|s| json!({"titulo": s.get("titulo"), "texto": s.get("texto")}))
                .collect(),
        ),
    );
    Ok(Value::Object(out))
}

/// O card de uma criatura, a partir da fonte crua: a entrada do
/// `monsters.json` do site atual, com o card de cada forma em `formas`.
pub fn bestiary_card(t: &CardTables, source: &Value) -> Result<Value, CardError> {
    let view = public_view(t, source, true)?;
    // o passo do `inimigos.json`: o `stat` com as Habilidades juntas
    let mut stat = view.stat.clone();
    stat.insert(
        "pericias".into(),
        view.motor.get("pericias").cloned().unwrap_or(Value::Null),
    );
    build(t, &stat, &view, None)
}
