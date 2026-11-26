//! A regra de casamento da busca, sem DOM nem Pagefind: port de
//! `src/lib/busca.ts` do site atual.
//!
//! O Pagefind acha as palavras (por radical: "lutando" acha "luta" e
//! "lutador") e ignora a caixa. Com o "Aa" ligado, a busca devolve o mesmo
//! que sem ele, menos os resultados cuja palavra casada tem outra caixa que a
//! digitada. O Pagefind diz quais palavras casou em `locations`, posições em
//! `content.split(' ')`; este módulo só olha a caixa dessas palavras. Os
//! testes são os de `scripts/test-busca.mjs` do site atual.

/// Palavras (em qualquer ordem) ou frase exata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Words,
    Phrase,
}

/// A letra sem acento, com a mesma caixa (o `normalize('NFD')` sem as
/// marcas, para as letras do português e do espanhol).
fn strip_accent(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'Á' | 'À' | 'Â' | 'Ã' | 'Ä' | 'Å' => 'A',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'É' | 'È' | 'Ê' | 'Ë' => 'E',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'O',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
        'ç' => 'c',
        'Ç' => 'C',
        'ñ' => 'n',
        'Ñ' => 'N',
        'ý' | 'ÿ' => 'y',
        'Ý' => 'Y',
        other => other,
    }
}

fn plain(s: &str) -> Vec<char> {
    s.chars().map(strip_accent).collect()
}

// As letras e a caixa só no latim (ASCII, Latim-1 e Latim estendido A), sem
// as tabelas Unicode da biblioteca padrão, que pesam no WebAssembly. Basta
// para o texto do livro (português e nomes próprios).

/// Letra ou algarismo (`[\p{L}\p{N}]`, no latim).
fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || (('\u{c0}'..='\u{ff}').contains(&c) && c != '\u{d7}' && c != '\u{f7}')
        || ('\u{100}'..='\u{17f}').contains(&c)
}

fn lower(c: char) -> char {
    let n = c as u32;
    let shifted = |d: u32| char::from_u32(n + d).unwrap_or(c);
    match n {
        0x41..=0x5a => shifted(32),
        0xc0..=0xde if n != 0xd7 => shifted(32),
        // Latim estendido A: maiúscula no par e minúscula no ímpar, ou o
        // contrário entre ĺ e ň e entre ź e ž.
        0x100..=0x137 | 0x14a..=0x177 if n.is_multiple_of(2) => shifted(1),
        0x139..=0x148 | 0x179..=0x17e if !n.is_multiple_of(2) => shifted(1),
        0x178 => '\u{ff}',
        _ => c,
    }
}

fn is_upper(c: char) -> bool {
    lower(c) != c
}

fn is_lower(c: char) -> bool {
    is_word_char(c) && !c.is_ascii_digit() && !is_upper(c)
}

/// `escHtml`.
pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// `tokensDe`: as palavras do que foi digitado (letras e números).
pub fn tokens(raw: &str) -> Vec<String> {
    raw.split(|c: char| !is_word_char(c))
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

/// `termoPagefind`: na frase, entre aspas.
pub fn pagefind_term(tokens: &[String], mode: Mode) -> String {
    let joined = tokens.join(" ");
    match mode {
        Mode::Phrase => format!("\"{joined}\""),
        Mode::Words => joined,
    }
}

/// `palavrasDe`: as palavras do texto, na contagem do Pagefind.
pub fn words_of(content: &str) -> Vec<String> {
    content.split(' ').map(str::to_string).collect()
}

/// `segmentos`: os pedaços de uma palavra, pela pontuação e pela emenda de
/// camelCase.
fn segments(word: &str) -> Vec<String> {
    let chars: Vec<char> = word.chars().collect();
    let mut out = Vec::new();
    let mut cur = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if is_word_char(c) {
            cur.push(c);
            let next_upper = chars.get(i + 1).is_some_and(|n| is_upper(*n));
            if is_lower(c) && next_upper {
                out.push(std::mem::take(&mut cur));
            }
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// `parente`: quantas letras do começo batem (sem acento nem caixa), se
/// forem ao menos 3 (ou todas, na palavra mais curta); 0 se não.
fn kin(typed: &str, piece: &str) -> usize {
    let t: Vec<char> = plain(typed).into_iter().map(lower).collect();
    let s: Vec<char> = plain(piece).into_iter().map(lower).collect();
    let mut cp = 0;
    while cp < t.len() && cp < s.len() && t[cp] == s[cp] {
        cp += 1;
    }
    if cp >= 3.min(t.len()).min(s.len()) {
        cp
    } else {
        0
    }
}

/// O que a palavra casada é para a digitada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Case {
    Right,
    Wrong,
    Unrelated,
}

/// `caixa`.
pub fn case_of(token: &str, word: &str) -> Case {
    let t = plain(token);
    let mut related = false;
    for seg in segments(word) {
        let cp = kin(token, &seg);
        if cp == 0 {
            continue;
        }
        related = true;
        let s = plain(&seg);
        if (0..cp).all(|i| is_upper(t[i]) == is_upper(s[i])) {
            return Case::Right;
        }
    }
    if related {
        Case::Wrong
    } else {
        Case::Unrelated
    }
}

/// `Veredito`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Verdict {
    pub ok: bool,
    pub accepted: Vec<usize>,
    pub neutral: usize,
    pub broken_premise: bool,
}

/// `filtrarCaixa`. `locations` é `None` quando o campo falta (ou não é lista);
/// cada posição vem como `f64`, como o JSON a traz.
pub fn filter_case(
    words: &[String],
    locations: Option<&[f64]>,
    tokens: &[String],
    mode: Mode,
    word_count: Option<f64>,
) -> Verdict {
    if tokens.is_empty() {
        return Verdict {
            ok: true,
            ..Verdict::default()
        };
    }
    let list = locations.unwrap_or(&[]);
    let outside = list
        .iter()
        .any(|&i| i.fract() != 0.0 || !i.is_finite() || i < 0.0 || i >= words.len() as f64);
    if locations.is_none() || outside || word_count.is_some_and(|c| c != words.len() as f64) {
        return Verdict {
            ok: true,
            accepted: Vec::new(),
            neutral: tokens.len(),
            broken_premise: true,
        };
    }
    let mut locs: Vec<usize> = list.iter().map(|&i| i as usize).collect();
    locs.sort_unstable();
    locs.dedup();
    if mode == Mode::Phrase {
        let n = tokens.len();
        // Listas ordenadas em vez de conjuntos: menos código no WebAssembly.
        let mut accepted: Vec<usize> = Vec::new();
        let mut sequences = 0;
        for &i in &locs {
            if !(0..n).all(|j| locs.binary_search(&(i + j)).is_ok()) {
                continue;
            }
            sequences += 1;
            if (0..n).all(|j| case_of(&tokens[j], &words[i + j]) != Case::Wrong) {
                accepted.extend(i..i + n);
            }
        }
        if sequences == 0 {
            return Verdict {
                ok: true,
                accepted: Vec::new(),
                neutral: n,
                broken_premise: false,
            };
        }
        accepted.sort_unstable();
        accepted.dedup();
        return Verdict {
            ok: !accepted.is_empty(),
            accepted,
            neutral: 0,
            broken_premise: false,
        };
    }
    let mut accepted: Vec<usize> = Vec::new();
    let mut ok = true;
    let mut neutral = 0;
    for tk in tokens {
        let mut related = false;
        let mut right = false;
        for &i in &locs {
            match case_of(tk, &words[i]) {
                Case::Unrelated => {}
                Case::Right => {
                    related = true;
                    right = true;
                    accepted.push(i);
                }
                Case::Wrong => related = true,
            }
        }
        if !related {
            neutral += 1;
        } else if !right {
            ok = false;
        }
    }
    accepted.sort_unstable();
    accepted.dedup();
    Verdict {
        ok,
        accepted,
        neutral,
        broken_premise: false,
    }
}

/// `trechoDe`: o trecho em HTML ao redor de `center`, com `<mark>` nas
/// palavras de `marked`; o texto vem escapado.
pub fn excerpt(
    words: &[String],
    marked: &[usize],
    center: usize,
    before: usize,
    after: usize,
) -> String {
    let from = center.saturating_sub(before);
    let to = (center + after).min(words.len());
    let parts: Vec<String> = (from..to)
        .map(|i| {
            let w = escape_html(&words[i]);
            if marked.contains(&i) {
                format!("<mark>{w}</mark>")
            } else {
                w
            }
        })
        .collect();
    let mut out = String::new();
    if from > 0 {
        out.push_str("\u{2026} ");
    }
    out.push_str(&parts.join(" "));
    if to < words.len() {
        out.push_str(" \u{2026}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn v(words: &[&str], locs: &[f64], typed: &str) -> Verdict {
        filter_case(&w(words), Some(locs), &tokens(typed), Mode::Words, None)
    }

    fn vp(words: &[&str], locs: &[f64], typed: &str) -> Verdict {
        filter_case(&w(words), Some(locs), &tokens(typed), Mode::Phrase, None)
    }

    const K: &[&str] = &["o", "Kael", "viu", "a", "torre", "Uldun"];

    #[test]
    fn case_filter_like_the_current_site() {
        assert!(v(K, &[1.0], "Kael").ok);
        assert!(!v(K, &[1.0], "kael").ok);
        assert!(!v(K, &[1.0], "KAEL").ok);
        assert!(v(K, &[5.0], "Uldun").ok && !v(K, &[5.0], "uldun").ok);
        assert!(v(&["veja", "exemploKael", "abaixo"], &[1.0], "Kael").ok);
        assert!(!v(&["veja", "exemploKael", "abaixo"], &[1.0], "kael").ok);
        assert!(v(&["A", "absorção", "do", "alvo"], &[1.0], "absorcao").ok);
        assert!(!v(&["A", "Absorção", "do", "alvo"], &[1.0], "absorcao").ok);
        assert!(v(&["A", "Absorção", "do", "alvo"], &[1.0], "Absorcao").ok);
        assert!(!v(&["A", "Luta", "Luta", "fim"], &[1.0, 2.0], "lutando").ok);
        assert!(v(&["A", "Luta", "e", "luta", "fim"], &[1.0, 3.0], "lutando").ok);
        assert!(
            v(
                &["a", "mágica,", "os", "mágicos", "e", "a", "magia"],
                &[1.0, 3.0, 6.0],
                "magia"
            )
            .ok
        );
        assert!(!v(&["a", "Magia", "e", "Magnitude"], &[1.0, 3.0], "magia").ok);
        assert!(v(K, &[1.0, 5.0], "Kael Uldun").ok);
        assert!(!v(K, &[1.0, 5.0], "Kael uldun").ok);
        assert!(!v(&["veja", "\u{201c}Kael\u{201d}", "agora"], &[1.0], "kael").ok);
        assert!(!v(&["veja", "(Kael)", "agora"], &[1.0], "kael").ok);
        let neutral = v(&["x", "kaxyz"], &[1.0], "Kael");
        assert!(neutral.ok && neutral.neutral == 1);
        assert!(!v(&["x", "kaelzinho"], &[1.0], "Kael").ok);
        assert!(!v(&["Ka"], &[0.0], "ka").ok && v(&["Ka"], &[0.0], "Ka").ok);
        assert!(v(K, &[], "kael").ok);
    }

    #[test]
    fn broken_premise_lets_everything_through() {
        let out = v(K, &[1.0, 99.0], "kael");
        assert!(out.ok && out.broken_premise && out.neutral == 1 && out.accepted.is_empty());
        assert!(v(K, &[-1.0], "kael").ok && v(K, &[1.5], "kael").ok);
        let missing = filter_case(&w(K), None, &tokens("kael"), Mode::Words, None);
        assert!(missing.ok && missing.broken_premise);
        let empty = v(K, &[], "kael");
        assert!(empty.ok && !empty.broken_premise && empty.neutral == 1);
        let count = filter_case(&w(K), Some(&[1.0]), &tokens("kael"), Mode::Words, Some(7.0));
        assert!(count.ok && count.broken_premise);
        assert!(!filter_case(&w(K), Some(&[1.0]), &tokens("kael"), Mode::Words, Some(6.0)).ok);
        assert!(v(K, &[1.0], "").ok && v(K, &[1.0], "\"").ok);
    }

    #[test]
    fn phrase_needs_the_whole_sequence() {
        let fr = &["a", "na", "Defesa", "contra", "projéteis", "rápidos"];
        assert!(vp(fr, &[2.0, 3.0, 4.0], "Defesa contra projéteis").ok);
        assert!(vp(fr, &[2.0, 3.0, 4.0], "Defesa contra projeteis").ok);
        assert!(!vp(fr, &[2.0, 3.0, 4.0], "defesa contra projéteis").ok);
        assert!(vp(fr, &[2.0, 4.0], "Defesa contra projéteis").ok);
        let two = &["x", "Defesa", "a", "defesa", "contra", "projéteis"];
        assert!(vp(two, &[1.0, 3.0, 4.0, 5.0], "defesa contra projéteis").ok);
    }

    #[test]
    fn excerpt_windows_and_escapes() {
        let words: Vec<String> = (0..40)
            .map(|i| {
                if i == 20 {
                    "Kael".into()
                } else {
                    format!("p{i}")
                }
            })
            .collect();
        let out = excerpt(&words, &[20], 20, 5, 5);
        assert!(
            out.starts_with("\u{2026} ")
                && out.ends_with(" \u{2026}")
                && out.contains("<mark>Kael</mark>")
        );
        assert_eq!(
            excerpt(&w(&["só", "<isto>"]), &[], 0, 5, 5),
            "só &lt;isto&gt;"
        );
        assert_eq!(pagefind_term(&tokens("\"a b\""), Mode::Phrase), "\"a b\"");
        assert_eq!(lower('Ç'), 'ç');
        assert_eq!(lower('Ã'), 'ã');
        assert!(is_upper('É') && !is_upper('é') && !is_upper('7'));
        assert_eq!(tokens("Força, de-Vontade!"), ["Força", "de", "Vontade"]);
    }
}
