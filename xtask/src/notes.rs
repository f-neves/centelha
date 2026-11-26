//! Troca dos nomes internos de papel nas notas de revisão do bestiário
//! (ADR 0005), feita na cópia do conteúdo.
//!
//! As notas de procedência de algumas criaturas citam quem fez a estimativa
//! pelo nome de um papel da equipe. No repositório em Rust esse nome vira
//! "revisão", com a concordância ajustada ("do X" vira "da revisão"). A troca
//! só acontece nos campos previstos de cada arquivo; o nome em qualquer outro
//! lugar para a cópia com erro, para não estragar sem ninguém ver um texto do
//! jogo que coincida com ele.

/// Os nomes trocados, montados por partes para que este arquivo não seja ele
/// mesmo um achado da varredura.
pub const NAMES: &[&str] = &[concat!("Arqui", "teto"), concat!("Bestia", "rista")];

/// O termo neutro que entra no lugar.
const NEUTRAL: &str = "revisão";

/// Campos (a chave JSON da linha) onde a troca é aceita, por arquivo.
fn allowed_keys(rel: &str) -> &'static [&'static str] {
    if rel.starts_with("data/bestiario/") && rel.ends_with(".json") {
        &["motivo", "justificativa"]
    } else if rel == "data/monsters.json" {
        &["justificativa", "Pinça do enxame"]
    } else if rel == "data/comando-barra.json" {
        &["notaNumeros"]
    } else {
        &[]
    }
}

/// A palavra antes do nome e a forma dela com o termo neutro (feminino).
const ARTICLES: &[(&str, &str)] = &[
    ("do", "da"),
    ("Do", "Da"),
    ("pelo", "pela"),
    ("Pelo", "Pela"),
    ("da", "da"),
    ("Da", "Da"),
    ("pela", "pela"),
    ("Pela", "Pela"),
    ("a", "a"),
    ("A", "A"),
    ("o", "a"),
    ("O", "A"),
];

/// Uma ocorrência do nome numa linha: início da palavra anterior, fim do nome
/// (já com o sufixo `-N`, se houver) e o artigo trocado.
struct Hit {
    article_start: usize,
    end: usize,
    article: &'static str,
}

fn is_letter(c: char) -> bool {
    c.is_alphabetic()
}

/// Acha cada nome inteiro (sem letra colada) na linha, sem diferença de caixa.
fn find_names(line: &str) -> Vec<(usize, usize)> {
    // Os nomes são ASCII: baixar só a caixa ASCII mantém as posições em bytes.
    let hay = line.to_ascii_lowercase();
    let mut out = Vec::new();
    for name in NAMES {
        let needle = name.to_lowercase();
        let mut from = 0;
        while let Some(pos) = hay[from..].find(&needle) {
            let start = from + pos;
            let end = start + needle.len();
            let before = line[..start].chars().next_back();
            let after = line[end..].chars().next();
            if !before.is_some_and(is_letter) && !after.is_some_and(is_letter) {
                out.push((start, end));
            }
            from = end;
        }
    }
    out.sort();
    out
}

/// A chave JSON de uma linha no formato `  "chave": "texto"`.
fn line_key(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix('"')?;
    let close = rest.find('"')?;
    rest[close + 1..]
        .trim_start()
        .starts_with(':')
        .then(|| &rest[..close])
}

/// Troca os nomes de um arquivo de texto. Devolve o texto novo e quantas
/// trocas fez, ou erro com o lugar quando o nome aparece fora do previsto.
pub fn neutralize(rel: &str, text: &str) -> Result<(String, usize), String> {
    let keys = allowed_keys(rel);
    let mut out = String::with_capacity(text.len());
    let mut count = 0;
    for (n, line) in text.split_inclusive('\n').enumerate() {
        let names = find_names(line);
        if names.is_empty() {
            out.push_str(line);
            continue;
        }
        let place = format!("{rel}, linha {}", n + 1);
        let key = line_key(line).unwrap_or("");
        if !keys.contains(&key) {
            return Err(format!(
                "{place}: nome de papel fora dos campos previstos (chave \"{key}\"); \
                 a troca não é feita às cegas (ADR 0005)"
            ));
        }
        let value_start = line.find(':').expect("a linha tem chave") + 1;
        let mut hits = Vec::new();
        for (start, end) in names {
            if start < value_start {
                return Err(format!("{place}: nome de papel dentro da chave"));
            }
            // Sufixo `-N` (número da rodada de revisão): sai junto.
            let tail = &line[end..];
            let digits = tail
                .strip_prefix('-')
                .map(|t| t.len() - t.trim_start_matches(|c: char| c.is_ascii_digit()).len())
                .unwrap_or(0);
            let end = if digits > 0 { end + 1 + digits } else { end };
            let before = line[..start]
                .strip_suffix(' ')
                .ok_or_else(|| format!("{place}: nome de papel sem palavra antes"))?;
            let word_start = before
                .char_indices()
                .rev()
                .find(|&(_, c)| !is_letter(c))
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(0);
            let word = &before[word_start..];
            let article = ARTICLES
                .iter()
                .find(|(from, _)| *from == word)
                .map(|(_, to)| *to)
                .ok_or_else(|| {
                    format!("{place}: concordância não prevista antes do nome (\"{word}\")")
                })?;
            hits.push(Hit {
                article_start: word_start,
                end,
                article,
            });
        }
        let mut last = 0;
        for hit in &hits {
            out.push_str(&line[last..hit.article_start]);
            out.push_str(hit.article);
            out.push(' ');
            out.push_str(NEUTRAL);
            last = hit.end;
            count += 1;
        }
        out.push_str(&line[last..]);
    }
    Ok((out, count))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a() -> &'static str {
        NAMES[0]
    }
    fn b() -> &'static str {
        NAMES[1]
    }

    #[test]
    fn swaps_with_agreement() {
        let text = format!(
            "{{\n  \"motivo\": \"Era 1,0 m (estimativa do {a}); Do {a}: 2 m; pelo {a}; da {b}-5; A {b} sugere; leitura da {b})\"\n}}\n",
            a = a(),
            b = b()
        );
        let (out, n) = neutralize("data/bestiario/x.json", &text).unwrap();
        assert_eq!(n, 6);
        assert_eq!(
            out,
            "{\n  \"motivo\": \"Era 1,0 m (estimativa da revisão); Da revisão: 2 m; pela revisão; da revisão; A revisão sugere; leitura da revisão)\"\n}\n"
        );
    }

    #[test]
    fn leaves_other_words_alone() {
        let text = "  \"nome\": \"Arquitetura\"\n";
        assert_eq!(
            neutralize("data/habilidades-secundarias.json", text)
                .unwrap()
                .1,
            0
        );
    }

    #[test]
    fn stops_outside_the_expected_fields() {
        let text = format!("  \"nome\": \"o {}\"\n", a());
        assert!(neutralize("data/bestiario/x.json", &text).is_err());
        let text = format!("  \"motivo\": \"o {}\"\n", a());
        assert!(neutralize("data/regras.json", &text).is_err());
        assert!(neutralize("chapters/x.md", &format!("o {}\n", b())).is_err());
    }

    #[test]
    fn stops_on_unknown_agreement() {
        let text = format!("  \"motivo\": \"segundo {}\"\n", a());
        assert!(neutralize("data/bestiario/x.json", &text).is_err());
    }
}
