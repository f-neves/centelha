//! A ordem das tabelas dos capítulos, igual à do site atual: a comparação de
//! texto do `localeCompare(b, 'pt', { numeric: true, sensitivity: 'base' })` e
//! a leitura de valores da ordenação de `src/pages/regras/[slug].astro`
//! (moeda convertida para cobre, frações, separador de milhar e vírgula
//! decimal). Vale no gerador e no navegador (a ilha que ordena).

use std::cmp::Ordering;

/// A letra de base: sem acento e em minúscula (as letras acentuadas do
/// português e do espanhol, e as das línguas vizinhas no Latim estendido).
pub fn fold(c: char) -> char {
    let lower = lowercase(c);
    match lower {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => 'a',
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => 'c',
        'ď' => 'd',
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => 'e',
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => 'g',
        'ĥ' => 'h',
        'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' => 'i',
        'ĵ' => 'j',
        'ķ' => 'k',
        'ĺ' | 'ļ' | 'ľ' => 'l',
        'ñ' | 'ń' | 'ņ' | 'ň' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ō' | 'ŏ' | 'ő' => 'o',
        'ŕ' | 'ŗ' | 'ř' => 'r',
        'ś' | 'ŝ' | 'ş' | 'š' => 's',
        'ţ' | 'ť' => 't',
        'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => 'u',
        'ŵ' => 'w',
        'ý' | 'ÿ' | 'ŷ' => 'y',
        'ź' | 'ż' | 'ž' => 'z',
        other => other,
    }
}

/// A minúscula das letras latinas (ASCII, Latim-1 e Latim estendido A), sem
/// as tabelas de caixa do Unicode inteiro, que pesam no WebAssembly. As
/// outras letras passam como estão (não aparecem nas tabelas do livro).
fn lowercase(c: char) -> char {
    let u = c as u32;
    let lower = match u {
        0x41..=0x5A | 0xC0..=0xD6 | 0xD8..=0xDE => u + 32,
        // Latim estendido A: maiúscula no par, minúscula no ímpar (com os
        // trechos que invertem a paridade).
        0x100..=0x137 | 0x14A..=0x177 if u.is_multiple_of(2) => u + 1,
        0x139..=0x148 | 0x179..=0x17E if !u.is_multiple_of(2) => u + 1,
        _ => u,
    };
    char::from_u32(lower).unwrap_or(c)
}

/// A classe e a letra de base de um caractere na colação: espaço antes de
/// pontuação, pontuação antes de símbolo, símbolo antes de algarismo,
/// algarismo antes de letra.
pub fn primary_key(c: char) -> (u8, char) {
    let base = fold(c);
    let class = if base.is_whitespace() {
        0
    } else if base.is_ascii_punctuation() && !"$+<=>^`|~".contains(base) {
        1
    } else if base.is_alphanumeric() {
        if base.is_numeric() { 3 } else { 4 }
    } else {
        2
    };
    (class, base)
}

/// Compara como `localeCompare` com `numeric: true` e `sensitivity: 'base'`:
/// só a letra de base conta (acento e caixa empatam), e uma sequência de
/// algarismos vale pelo número (`2` antes de `10`).
pub fn compare_numeric_base(a: &str, b: &str) -> Ordering {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        let (da, db) = (a[i].is_ascii_digit(), b[j].is_ascii_digit());
        if da && db {
            let (si, sj) = (i, j);
            while i < a.len() && a[i].is_ascii_digit() {
                i += 1;
            }
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            let na: String = a[si..i].iter().collect();
            let nb: String = b[sj..j].iter().collect();
            let (na, nb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
            let order = na.len().cmp(&nb.len()).then_with(|| na.cmp(nb));
            if order != Ordering::Equal {
                return order;
            }
            continue;
        }
        let order = primary_key(a[i]).cmp(&primary_key(b[j]));
        if order != Ordering::Equal {
            return order;
        }
        i += 1;
        j += 1;
    }
    (a.len() - i).cmp(&(b.len() - j))
}

/// O valor de "inteiro,fração" escritos só com algarismos, por conta, sem o
/// leitor de números com fração da biblioteca padrão (que pesa no
/// WebAssembly). Basta para ordenar.
fn decimal(int: &str, frac: &str) -> Option<f64> {
    if int.is_empty() || !int.bytes().chain(frac.bytes()).all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut value = 0.0;
    for b in int.bytes() {
        value = value * 10.0 + f64::from(b - b'0');
    }
    let mut scale = 1.0;
    for b in frac.bytes() {
        scale /= 10.0;
        value += f64::from(b - b'0') * scale;
    }
    Some(value)
}

fn fraction(c: char) -> Option<f64> {
    match c {
        '½' => Some(0.5),
        '¼' => Some(0.25),
        '¾' => Some(0.75),
        '⅓' => Some(1.0 / 3.0),
        '⅔' => Some(2.0 / 3.0),
        _ => None,
    }
}

fn currency(unit: &str) -> Option<f64> {
    match unit.to_ascii_lowercase().as_str() {
        "pc" => Some(1.0),
        "pp" => Some(10.0),
        "po" => Some(100.0),
        "pl" => Some(1000.0),
        _ => None,
    }
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Um número em `s` a partir de `i`, na ordem das alternativas do site
/// atual: fração, número com separador de milhar (`1.500`, `1.500,5`), ou
/// número simples com fração decimal opcional (`12`, `12,5`, `12.5`). Devolve o
/// valor e onde o número termina, para cada forma que casa (a mais longa
/// primeiro, como o recuo de uma expressão regular).
fn numbers_at(s: &[char], i: usize) -> Vec<(f64, usize)> {
    let mut out = Vec::new();
    if let Some(f) = s.get(i).copied().and_then(fraction) {
        out.push((f, i + 1));
        return out;
    }
    let digits = |from: usize| {
        let mut k = from;
        while k < s.len() && s[k].is_ascii_digit() {
            k += 1;
        }
        k
    };
    let lead_end = digits(i);
    if lead_end == i {
        return out;
    }
    // Milhar: 1 a 3 algarismos, depois grupos de `.ddd`, e uma `,dd` opcional.
    if lead_end - i <= 3 {
        let mut groups = Vec::new();
        let mut k = lead_end;
        while s.get(k) == Some(&'.') && digits(k + 1) == k + 4 {
            k += 4;
            groups.push(k);
        }
        for &end in groups.iter().rev() {
            let int: String = s[i..end].iter().filter(|c| **c != '.').collect();
            if s.get(end) == Some(&',') && digits(end + 1) > end + 1 {
                let frac_end = digits(end + 1);
                let frac: String = s[end + 1..frac_end].iter().collect();
                if let Some(v) = decimal(&int, &frac) {
                    out.push((v, frac_end));
                }
            }
            if let Some(v) = decimal(&int, "") {
                out.push((v, end));
            }
        }
    }
    // Simples, com fração decimal opcional (vírgula ou ponto).
    if matches!(s.get(lead_end), Some('.') | Some(',')) && digits(lead_end + 1) > lead_end + 1 {
        let frac_end = digits(lead_end + 1);
        let int: String = s[i..lead_end].iter().collect();
        let frac: String = s[lead_end + 1..frac_end].iter().collect();
        if let Some(v) = decimal(&int, &frac) {
            out.push((v, frac_end));
        }
    }
    let int: String = s[i..lead_end].iter().collect();
    if let Some(v) = decimal(&int, "") {
        out.push((v, lead_end));
    }
    out
}

/// O valor de uma célula para ordenar, como o `numOf` do site atual: moeda
/// (pc, pp, po, pl) convertida para cobre, fração sozinha, ou o primeiro
/// número do texto. `None` quando não há número.
pub fn cell_value(raw: &str) -> Option<f64> {
    let s: String = raw
        .chars()
        .map(|c| if c == '−' || c == '–' { '-' } else { c })
        .collect();
    let s = s.trim();
    let chars: Vec<char> = s.chars().collect();
    // Moeda: o primeiro trecho "-? número unidade" do texto.
    for start in 0..chars.len() {
        let (negative, mut k) = if chars[start] == '-' {
            (true, start + 1)
        } else {
            (false, start)
        };
        while k < chars.len() && chars[k].is_whitespace() {
            k += 1;
        }
        for (value, end) in numbers_at(&chars, k) {
            let mut u = end;
            while u < chars.len() && chars[u].is_whitespace() {
                u += 1;
            }
            if u + 2 > chars.len() {
                continue;
            }
            let unit: String = chars[u..u + 2].iter().collect();
            let bounded = chars.get(u + 2).is_none_or(|c| !is_word(*c));
            if let (Some(rate), true) = (currency(&unit), bounded) {
                let sign = if negative { -1.0 } else { 1.0 };
                return Some(sign * value * rate);
            }
        }
    }
    if chars.len() == 1
        && let Some(f) = fraction(chars[0])
    {
        return Some(f);
    }
    // O primeiro número do texto.
    for start in 0..chars.len() {
        let (negative, k) =
            if chars[start] == '-' && chars.get(start + 1).is_some_and(|c| c.is_ascii_digit()) {
                (true, start + 1)
            } else {
                (false, start)
            };
        if !chars.get(k).is_some_and(|c| c.is_ascii_digit()) {
            continue;
        }
        let mut end = k;
        while end < chars.len() && chars[end].is_ascii_digit() {
            end += 1;
        }
        let int: String = chars[k..end].iter().collect();
        let mut frac = String::new();
        if matches!(chars.get(end), Some('.') | Some(','))
            && chars.get(end + 1).is_some_and(|c| c.is_ascii_digit())
        {
            let mut f = end + 1;
            while f < chars.len() && chars[f].is_ascii_digit() {
                f += 1;
            }
            frac.extend(&chars[end + 1..f]);
        }
        let value = decimal(&int, &frac)?;
        return Some(if negative { -value } else { value });
    }
    None
}

/// Célula vazia para a ordem (vai para o começo na ordem crescente).
pub fn is_empty_cell(s: &str) -> bool {
    matches!(s, "" | "—" | "–")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_currency_fractions_and_numbers() {
        assert_eq!(cell_value("3 po"), Some(300.0));
        assert_eq!(cell_value("3.600 pc"), Some(3600.0));
        assert_eq!(cell_value("1.500,5 pp"), Some(15005.0));
        assert_eq!(cell_value("½ pc"), Some(0.5));
        assert_eq!(cell_value("−2 pl"), Some(-2000.0));
        assert_eq!(cell_value("12,5 PO"), Some(1250.0));
        assert_eq!(cell_value("¾"), Some(0.75));
        assert_eq!(cell_value("+3 dados"), Some(3.0));
        assert_eq!(cell_value("−1"), Some(-1.0));
        assert_eq!(cell_value("2,5 kg"), Some(2.5));
        assert_eq!(cell_value("leve"), None);
        // "pcs" não é a unidade (fronteira de palavra), vale o número.
        assert_eq!(cell_value("4 pcs"), Some(4.0));
    }

    #[test]
    fn text_order_is_numeric_and_ignores_accent_and_case() {
        assert_eq!(compare_numeric_base("Nível 2", "Nível 10"), Ordering::Less);
        assert_eq!(compare_numeric_base("ação", "Acao"), Ordering::Equal);
        assert_eq!(compare_numeric_base("Arco", "Espada"), Ordering::Less);
        assert_eq!(compare_numeric_base("Arco", "Arco longo"), Ordering::Less);
    }

    #[test]
    fn empty_cells() {
        assert!(is_empty_cell("—"));
        assert!(!is_empty_cell("0"));
    }
}
