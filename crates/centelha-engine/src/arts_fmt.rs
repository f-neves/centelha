//! A formatação compartilhada dos Efeitos das Artes: a ordem fixa dos
//! parâmetros, o valor curto de cada um e quais rótulos abrem o cartão das
//! formas. Port de `src/lib/artes-fmt.ts`.
//!
//! Os textos saem como a fonte os escreve, byte a byte (acento, parênteses,
//! "Terra: 2d6 por nível"). A conferência é `tests/parity/arts.json`.

use centelha_data::effects::{EffectParameter, ParameterKind};
use serde_json::Value;

/// Até a Duração o parâmetro diz a forma do Efeito; daí para frente é a
/// execução (a jogada e a Dificuldade). É o `PAR_FORMA` da fonte.
pub const PARAMETER_FORM: u8 = 4;

/// A posição que um parâmetro sem nome conhecido ocupa: depois de todos.
const UNKNOWN_RANK: u8 = 9;

/// `idDe`: o id de uma referência. Uma referência do conteúdo pode vir como o
/// próprio texto ou como objeto, e do objeto sai o `id` como está (`x?.id`);
/// qualquer outra coisa não tem id (`undefined`, que aqui é `null`).
pub fn id_of(x: &Value) -> Value {
    match x {
        Value::String(_) => x.clone(),
        Value::Object(m) => m.get("id").cloned().unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

/// O cartão que o rótulo de um parâmetro abre ao passar o mouse
/// (`FormasPop.astro`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    Volume,
    Area,
}

impl Form {
    /// O texto que a fonte usa em `data-formas`.
    pub fn id(self) -> &'static str {
        match self {
            Form::Volume => "volume",
            Form::Area => "area",
        }
    }
}

/// `formaDe`: o rótulo "Volume..." abre o cartão do volume, "Área..." (com ou
/// sem acento) o da área, e os outros não abrem nenhum.
pub fn form_of(name: &str) -> Option<Form> {
    let n = name.to_lowercase();
    if n.starts_with("volume") {
        Some(Form::Volume)
    } else if n.starts_with("área") || n.starts_with("area") {
        Some(Form::Area)
    } else {
        None
    }
}

/// A coluna da tabela `RANK` da fonte para o nome em minúsculas.
fn rank_of_key(key: &str) -> Option<u8> {
    Some(match key {
        "alcance" => 0,
        "alvos" => 1,
        "área" | "area" | "volume" | "comprimento" | "altura" | "raio" | "profundidade" => 2,
        // as magnitudes do Efeito, todas na casa do Dano
        "dano" | "cura" | "bônus" | "absorção" | "absorve" | "resistência" | "quantidade"
        | "ganho" | "voo" | "visão" | "força" | "efeito" | "fah" | "faa" => 3,
        "duração" | "duracao" => 4,
        "ataque" | "conjurar" | "jogada" => 5,
        "dificuldade" => 6,
        "penalidade" => 7,
        _ => return None,
    })
}

/// `rank`: onde o parâmetro entra na ordem fixa (Alcance, Alvos, Área e as
/// medidas, Dano, Duração, a jogada, a Dificuldade, a Penalidade). Vale a
/// primeira palavra do nome; se ela não está na tabela, vale o parâmetro que
/// ele substitui; se nenhum casa, 9.
pub fn rank_by_name(name: &str, replaces: Option<&str>) -> u8 {
    let first = name.to_lowercase();
    let first = first.split(' ').next().unwrap_or("");
    rank_of_key(first)
        .or_else(|| rank_of_key(&replaces.unwrap_or("").to_lowercase()))
        .unwrap_or(UNKNOWN_RANK)
}

/// `rank` de um parâmetro do Efeito.
pub fn rank(p: &EffectParameter) -> u8 {
    rank_by_name(&p.name, p.replaces.as_deref())
}

/// `ordemPar`: os parâmetros na ordem fixa. Dentro do mesmo `rank` fica a ordem
/// do arquivo (a ordenação é estável e o desempate é o índice).
pub fn order_parameters(parameters: &[EffectParameter]) -> Vec<&EffectParameter> {
    let mut list: Vec<(usize, &EffectParameter)> = parameters.iter().enumerate().collect();
    list.sort_by_key(|(i, p)| (rank(p), *i));
    list.into_iter().map(|(_, p)| p).collect()
}

/// A escala de um parâmetro que substitui outro, com a unidade entre
/// parênteses: "1 · 2 · 3 (m)".
pub fn scale_text(p: &EffectParameter) -> String {
    let scale = p.scale.as_deref().unwrap_or(&[]).join(" · ");
    match p.unit.as_deref().filter(|u| !u.is_empty()) {
        Some(unit) => format!("{scale} ({unit})"),
        None => scale,
    }
}

/// `valorPar`: o valor curto de um parâmetro: o fixo, a régua da Duração, a
/// nota de quem substitui outro. `arts` são as Artes em que o Efeito aparece
/// ali: sob a Terra o Dano diz que o dado dobra ("Terra dobra o dado").
/// `None` é o `undefined` de um parâmetro fixo sem valor.
pub fn parameter_value(p: &EffectParameter, arts: &[&str]) -> Option<String> {
    if p.kind == ParameterKind::Fixed {
        return p.value.clone();
    }
    if p.kind == ParameterKind::Replaces {
        return Some(match p.note.as_deref().filter(|n| !n.is_empty()) {
            Some(note) => note.to_owned(),
            None => scale_text(p),
        });
    }
    if let Some(ruler) = p.ruler.as_deref().filter(|r| !r.is_empty()) {
        return Some(if ruler == "longa" { "Longa" } else { "Breve" }.to_owned());
    }
    let name = p.name.to_lowercase();
    // a Cura tem régua e preço próprios (2 de Mana por nível), então não segue o dano
    if name.starts_with("cura") {
        return Some("2 · 1d6 · 1d6+2 · 2d6 · 2d6+2 · 3d6 (2 de Mana por nível)".into());
    }
    if name.starts_with("dano") {
        return Some(
            if arts.contains(&"terra") {
                "1d6 por nível (Terra: 2d6 por nível)"
            } else {
                "1d6 por nível"
            }
            .into(),
        );
    }
    Some("normal".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parameter(json: Value) -> EffectParameter {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn id_of_takes_text_or_object() {
        assert_eq!(id_of(&json!("fogo")), json!("fogo"));
        assert_eq!(id_of(&json!({"id": "agua"})), json!("agua"));
        assert_eq!(id_of(&json!({"id": 3})), json!(3));
        assert_eq!(id_of(&json!({"nome": "x"})), Value::Null);
        assert_eq!(id_of(&json!(null)), Value::Null);
        assert_eq!(id_of(&json!(7)), Value::Null);
    }

    #[test]
    fn form_of_ignores_case_and_accent_of_area() {
        assert_eq!(form_of("Volume (m³)"), Some(Form::Volume));
        assert_eq!(form_of("Área"), Some(Form::Area));
        assert_eq!(form_of("AREA"), Some(Form::Area));
        assert_eq!(form_of("Alcance"), None);
        assert_eq!(form_of(" volume"), None);
    }

    #[test]
    fn rank_falls_back_to_the_replaced_parameter_then_to_nine() {
        assert_eq!(rank_by_name("Dano de fogo", None), 3);
        assert_eq!(rank_by_name("Volume", Some("area")), 2);
        assert_eq!(rank_by_name("Nuvem", Some("area")), 2);
        assert_eq!(rank_by_name("Nuvem", Some("Alvos")), 1);
        assert_eq!(rank_by_name("Nuvem", None), 9);
        assert_eq!(rank_by_name("", None), 9);
    }

    #[test]
    fn the_order_is_stable_inside_a_rank() {
        let list = vec![
            parameter(json!({"nome": "Dificuldade", "tipo": "fixo", "valor": "10"})),
            parameter(json!({"nome": "Raio", "tipo": "padrao"})),
            parameter(json!({"nome": "Alcance", "tipo": "padrao"})),
            parameter(json!({"nome": "Altura", "tipo": "padrao"})),
        ];
        let names: Vec<&str> = order_parameters(&list)
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert_eq!(names, ["Alcance", "Raio", "Altura", "Dificuldade"]);
    }

    #[test]
    fn the_short_value_follows_the_rule_of_each_kind() {
        let dano = parameter(json!({"nome": "Dano", "tipo": "padrao"}));
        assert_eq!(
            parameter_value(&dano, &["fogo"]).as_deref(),
            Some("1d6 por nível")
        );
        assert_eq!(
            parameter_value(&dano, &["fogo", "terra"]).as_deref(),
            Some("1d6 por nível (Terra: 2d6 por nível)")
        );
        let duration = parameter(json!({"nome": "Duração", "tipo": "padrao", "regua": "longa"}));
        assert_eq!(parameter_value(&duration, &[]).as_deref(), Some("Longa"));
        let swap = parameter(
            json!({"nome": "Volume", "tipo": "substitui", "substitui": "area",
            "escala": ["1", "2"], "unidade": "m"}),
        );
        assert_eq!(parameter_value(&swap, &[]).as_deref(), Some("1 · 2 (m)"));
        let noted = parameter(
            json!({"nome": "Volume", "tipo": "substitui", "substitui": "area",
            "escala": ["1"], "nota": "uma nota"}),
        );
        assert_eq!(parameter_value(&noted, &[]).as_deref(), Some("uma nota"));
        let fixed = parameter(json!({"nome": "Alcance", "tipo": "fixo", "valor": "Toque"}));
        assert_eq!(parameter_value(&fixed, &[]).as_deref(), Some("Toque"));
        let other = parameter(json!({"nome": "Alvos", "tipo": "padrao"}));
        assert_eq!(parameter_value(&other, &[]).as_deref(), Some("normal"));
    }
}
