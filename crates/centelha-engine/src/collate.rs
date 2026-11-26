//! A ordem alfabética do site atual: a colação do `localeCompare` com `'pt'`
//! (a do ICU para o português), nos textos do livro.
//!
//! Três camadas, como o ICU compara:
//! 1. a letra de base, sem acento e sem caixa, com espaço antes de
//!    pontuação, pontuação antes de símbolo, símbolo antes de algarismo e
//!    algarismo antes de letra;
//! 2. o acento (a letra sem acento vem antes);
//! 3. a caixa (a minúscula vem antes).
//!
//! [`compare`] é o `localeCompare(b, 'pt')`; [`compare_base`] o mesmo com
//! `sensitivity: 'base'` (só a primeira camada); [`compare_numeric_base`]
//! com `numeric: true` e `sensitivity: 'base'` (a ordem das tabelas do site:
//! uma sequência de algarismos vale pelo número, `2` antes de `10`).
//!
//! **O que está coberto** ([`covers`]): o ASCII imprimível, o espaço, e as
//! letras acentuadas que se decompõem numa letra ASCII com acento (as de
//! [`fold`]: as do português e do espanhol, e as de Latim-1 e Latim estendido A
//! que são letra de base mais um sinal), nas duas caixas. É o que os nomes do
//! livro usam hoje. A prova é a comparação com o `localeCompare` do Node
//! (`tests/parity/collate.json`), e um teste falha quando um nome de um
//! catálogo de `content/` trouxer caractere fora disso.
//!
//! **Fora da cobertura, a ordem pode diferir do ICU:**
//! - as letras latinas que o ICU lê como outra base ou expande (`ð đ ħ ł ŀ ø ſ`,
//!   `ß æ œ þ ĳ ŉ ŋ ŧ ĸ ı`) e as maiúsculas sem par aqui (`Ÿ İ`) entram pelo
//!   ponto de código, depois do `z`;
//! - as letras de outros alfabetos (grego, cirílico...) entram pelo ponto de
//!   código e sem dobrar a caixa (`Α` e `α` não empatam);
//! - os símbolos e a pontuação fora do ASCII (`×`, `−`, `·`, aspas curvas)
//!   entram pelo ponto de código, depois da pontuação ASCII.
//!
//! As tabelas de caixa e de decomposição do Unicode inteiro ficaram de fora
//! porque pesam no WebAssembly.

use std::cmp::Ordering;

/// O caractere está na cobertura da colação (ver o cabeçalho): ASCII
/// imprimível, espaço, ou letra que [`fold`] leva a uma letra de base ASCII.
pub fn covers(c: char) -> bool {
    (' '..='~').contains(&c) || c.is_whitespace() || {
        let b = fold(c);
        b != lowercase(c) && b.is_ascii_lowercase()
    }
}

/// A minúscula das letras latinas (ASCII, Latim-1 e Latim estendido A). As
/// outras passam como estão.
fn lowercase(c: char) -> char {
    let u = c as u32;
    let lower = match u {
        0x41..=0x5A | 0xC0..=0xD6 | 0xD8..=0xDE => u + 32,
        // Latim estendido A: maiúscula no par, minúscula no ímpar (com os
        // trechos que invertem a paridade)
        0x100..=0x137 | 0x14A..=0x177 if u.is_multiple_of(2) => u + 1,
        0x139..=0x148 | 0x179..=0x17E if !u.is_multiple_of(2) => u + 1,
        _ => u,
    };
    char::from_u32(lower).unwrap_or(c)
}

fn is_upper(c: char) -> bool {
    lowercase(c) != c
}

/// A letra de base: sem acento e em minúscula.
pub fn fold(c: char) -> char {
    match lowercase(c) {
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

/// A ordem da pontuação e dos símbolos ASCII na colação do ICU (a da tabela
/// padrão do Unicode), do `_` ao `$`.
const PUNCTUATION: &str = "_-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$";

/// A classe e o peso de base de um caractere: espaço (0); pontuação e símbolo
/// ASCII (1, na ordem de [`PUNCTUATION`]); os outros símbolos (2, pelo ponto
/// de código); algarismo (3); letra (4, pela letra de base).
pub fn primary_key(c: char) -> (u8, u32) {
    let base = fold(c);
    if base.is_whitespace() {
        (0, 0)
    } else if let Some(i) = PUNCTUATION.find(base) {
        (1, i as u32)
    } else if base.is_numeric() {
        (3, base as u32)
    } else if base.is_alphanumeric() {
        (4, base as u32)
    } else {
        (2, base as u32)
    }
}

/// O sinal diacrítico de uma letra, na ordem do ICU: agudo, grave, breve,
/// circunflexo, caron, anel, trema, agudo duplo, til, ponto, cedilha, ogonek,
/// mácron. A letra sem acento vale 0.
fn accent(c: char) -> u8 {
    match lowercase(c) {
        'á' | 'é' | 'í' | 'ó' | 'ú' | 'ý' | 'ć' | 'ń' | 'ś' | 'ź' | 'ĺ' | 'ŕ' => 1,
        'à' | 'è' | 'ì' | 'ò' | 'ù' => 2,
        'ă' | 'ĕ' | 'ĭ' | 'ŏ' | 'ŭ' | 'ğ' => 3,
        'â' | 'ê' | 'î' | 'ô' | 'û' | 'ĉ' | 'ĝ' | 'ĥ' | 'ĵ' | 'ŝ' | 'ŵ' | 'ŷ' => 4,
        'č' | 'ď' | 'ě' | 'ľ' | 'ň' | 'ř' | 'š' | 'ť' | 'ž' => 5,
        'å' | 'ů' => 6,
        'ä' | 'ë' | 'ï' | 'ö' | 'ü' | 'ÿ' => 7,
        'ő' | 'ű' => 8,
        'ã' | 'õ' | 'ñ' | 'ĩ' | 'ũ' => 9,
        'ċ' | 'ė' | 'ġ' | 'ż' => 10,
        'ç' | 'ķ' | 'ļ' | 'ņ' | 'ŗ' | 'ş' | 'ţ' | 'ģ' => 11,
        'ą' | 'ę' | 'į' | 'ų' => 12,
        'ā' | 'ē' | 'ī' | 'ō' | 'ū' => 13,
        _ => 0,
    }
}

/// `localeCompare(b, 'pt', { sensitivity: 'base' })`: só a letra de base.
pub fn compare_base(a: &str, b: &str) -> Ordering {
    a.chars().map(primary_key).cmp(b.chars().map(primary_key))
}

/// `localeCompare(b, 'pt')`: a letra de base, depois o acento, depois a caixa
/// (a minúscula antes).
pub fn compare(a: &str, b: &str) -> Ordering {
    compare_base(a, b)
        .then_with(|| a.chars().map(accent).cmp(b.chars().map(accent)))
        .then_with(|| a.chars().map(is_upper).cmp(b.chars().map(is_upper)))
}

/// `localeCompare(b, 'pt', { numeric: true, sensitivity: 'base' })`: só a
/// letra de base conta, e uma sequência de algarismos vale pelo número.
pub fn compare_numeric_base(a: &str, b: &str) -> Ordering {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i].is_ascii_digit() && b[j].is_ascii_digit() {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(words: &[&str]) -> Vec<String> {
        let mut v: Vec<String> = words.iter().map(|s| s.to_string()).collect();
        v.sort_by(|a, b| compare(a, b));
        v
    }

    #[test]
    fn accents_and_case_come_after_the_base_letter() {
        assert_eq!(
            sorted(&["Ação", "Acao", "acao", "Abc"]),
            ["Abc", "acao", "Acao", "Ação"]
        );
        assert_eq!(
            sorted(&["Éter", "Esquiva", "Ataque"]),
            ["Ataque", "Esquiva", "Éter"]
        );
    }

    #[test]
    fn space_and_digits_come_before_letters() {
        assert_eq!(sorted(&["Pool", "Po ol", "P2"]), ["P2", "Po ol", "Pool"]);
        assert_eq!(
            sorted(&["Força de Vontade", "Força"]),
            ["Força", "Força de Vontade"]
        );
    }

    #[test]
    fn numbers_by_value() {
        assert_eq!(compare_numeric_base("Nível 2", "Nível 10"), Ordering::Less);
        assert_eq!(compare_numeric_base("nivel 2", "Nível 02"), Ordering::Equal);
    }
}
