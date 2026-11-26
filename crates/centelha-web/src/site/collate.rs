//! A ordem alfabética do site atual, que ordena com
//! `a.localeCompare(b, 'pt')` (a colação do ICU para o português).
//!
//! Reproduz as três camadas dessa colação, o bastante para os nomes do livro:
//! 1. a letra de base, sem acento e sem caixa, com espaço antes de
//!    pontuação, pontuação antes de símbolo, símbolo antes de algarismo e
//!    algarismo antes de letra;
//! 2. o acento (sem acento vem antes);
//! 3. a caixa (minúscula vem antes).
//!
//! A prova é a comparação com a ordem do build do site atual (relatório da
//! tarefa do glossário).

use std::cmp::Ordering;

/// Compara dois textos como o `localeCompare` com `'pt'`.
pub fn compare(a: &str, b: &str) -> Ordering {
    let primary = |s: &str| -> Vec<(u8, char)> { s.chars().map(primary_key).collect() };
    let secondary = |s: &str| -> Vec<char> { s.chars().flat_map(char::to_lowercase).collect() };
    primary(a)
        .cmp(&primary(b))
        .then_with(|| secondary(a).cmp(&secondary(b)))
        .then_with(|| {
            // Na caixa, minúscula primeiro: inverter a comparação dos
            // caracteres põe `a` antes de `A`.
            let case = |s: &str| -> Vec<bool> { s.chars().map(char::is_uppercase).collect() };
            case(a).cmp(&case(b))
        })
}

/// A classe e a letra de base de um caractere.
fn primary_key(c: char) -> (u8, char) {
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

/// A letra sem acento e em minúscula (as letras acentuadas do português e do
/// espanhol; o resto passa como está, em minúscula).
fn fold(c: char) -> char {
    let lower = c.to_lowercase().next().unwrap_or(c);
    match lower {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        other => other,
    }
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
}
