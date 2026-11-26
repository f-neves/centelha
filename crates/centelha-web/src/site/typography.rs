//! A pontuação tipográfica que o site atual aplica por padrão no Markdown:
//! aspas retas viram curvas, `--` vira o traço longo (U+2014), três
//! pontos viram reticências e os pares de crase e de apóstrofo viram aspas.
//!
//! Reproduz o plugin padrão do gerador do site atual, que trabalha em duas
//! voltas:
//!
//! 1. as aspas, sobre o texto do documento inteiro emendado (cada parágrafo
//!    começa com um espaço; título e célula de tabela não), porque a aspa que
//!    abre e a que fecha podem estar em pedaços diferentes (antes e depois de
//!    um negrito, por exemplo);
//! 2. reticências, crases e traços, em cada pedaço de texto separado.
//!
//! As duas voltas cortam o texto em fichas como a biblioteca de origem:
//! espaço, palavra (letras e algarismos) e o resto (pontuação e símbolos),
//! com a sequência do mesmo sinal numa ficha só (`---` é uma ficha, e por
//! isso não vira travessão).

const OPEN_DOUBLE: char = '\u{201c}';
const CLOSE_DOUBLE: char = '\u{201d}';
const OPEN_SINGLE: char = '\u{2018}';
const CLOSE_SINGLE: char = '\u{2019}';

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    WhiteSpace,
    Word,
    /// Pontuação ou símbolo (as regras tratam os dois igual).
    Mark,
}

#[derive(Debug)]
struct Token {
    kind: Kind,
    start: usize,
    value: Vec<char>,
}

impl Token {
    fn is(&self, c: char) -> bool {
        self.value.len() == 1 && self.value[0] == c
    }
}

fn kind_of(c: char) -> Kind {
    if c.is_whitespace() {
        Kind::WhiteSpace
    } else if c.is_alphanumeric() {
        Kind::Word
    } else {
        Kind::Mark
    }
}

fn tokenize(text: &[char]) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    for (i, &c) in text.iter().enumerate() {
        let kind = kind_of(c);
        if let Some(last) = tokens.last_mut() {
            let continues =
                last.kind == kind && (kind != Kind::Mark || last.value.last() == Some(&c));
            if continues {
                last.value.push(c);
                continue;
            }
        }
        tokens.push(Token {
            kind,
            start: i,
            value: vec![c],
        });
    }
    tokens
}

/// Sinal que termina frase (`!`, `.`, `?`, reticências).
fn is_terminal(token: &Token) -> bool {
    token.kind == Kind::Mark
        && token
            .value
            .iter()
            .all(|c| matches!(c, '!' | '.' | '?' | '\u{2026}' | '\u{203d}'))
}

/// Sinal que, colado logo depois do fim de uma frase, ainda pertence a ela
/// (aspas e parênteses que fecham, mais pontuação final).
fn is_affix(token: &Token) -> bool {
    token.kind == Kind::Mark
        && token.value.iter().all(|c| {
            matches!(
                c,
                '!' | '"'
                    | '\''
                    | ')'
                    | '.'
                    | '?'
                    | '\u{2019}'
                    | '\u{201d}'
                    | '\u{2026}'
                    | '\u{203a}'
                    | '\u{203d}'
            )
        })
}

/// Onde as frases terminam: `ends[k]` diz que a frase acaba depois da ficha
/// `k`. A frase acaba no sinal final mais os sinais colados que ainda são
/// dela; não acaba se a frase seguinte começa com letra minúscula ou
/// algarismo (a biblioteca de origem junta as duas de volta). As regras das
/// aspas não enxergam fichas de outra frase.
fn sentence_ends(tokens: &[Token]) -> Vec<bool> {
    let mut ends = vec![false; tokens.len()];
    let mut k = 0;
    while k < tokens.len() {
        if !is_terminal(&tokens[k]) {
            k += 1;
            continue;
        }
        let mut end = k;
        while end + 1 < tokens.len() && is_affix(&tokens[end + 1]) {
            end += 1;
        }
        let first_after = tokens[end + 1..]
            .iter()
            .find(|t| t.kind != Kind::WhiteSpace);
        let joins_back = first_after.is_some_and(|t| {
            t.kind == Kind::Word
                && t.value
                    .first()
                    .is_some_and(|c| c.is_lowercase() || c.is_ascii_digit())
        });
        if !joins_back {
            ends[end] = true;
        }
        k = end + 1;
    }
    ends
}

/// A primeira volta: troca as aspas retas no texto emendado. Devolve um texto
/// do mesmo tamanho (cada aspa vira um caractere só).
pub(crate) fn quotes(text: &[char]) -> Vec<char> {
    let mut tokens = tokenize(text);
    let ends = sentence_ends(&tokens);
    // A ficha `j` é vizinha de `i` só se nenhuma frase termina entre as duas.
    let same_sentence = |a: usize, b: usize| (a.min(b)..a.max(b)).all(|k| !ends[k]);
    for i in 0..tokens.len() {
        let quote = if tokens[i].is('"') {
            0
        } else if tokens[i].is('\'') {
            1
        } else {
            continue;
        };
        let open = [OPEN_DOUBLE, OPEN_SINGLE];
        let close = [CLOSE_DOUBLE, CLOSE_SINGLE];
        let near = |j: usize| (j < tokens.len() && same_sentence(i, j)).then_some(j);
        let previous = i.checked_sub(1).and_then(near).map(|p| tokens[p].kind);
        let next = near(i + 1).map(|j| tokens[j].kind);
        let next_next = near(i + 2).map(|j| tokens[j].kind);
        let next_value: String = near(i + 1)
            .map(|j| tokens[j].value.iter().collect())
            .unwrap_or_default();

        let replacement = if next == Some(Kind::Mark) && next_next != Some(Kind::Word) {
            // Aspa seguida de pontuação fora de palavra: fecha.
            close[quote]
        } else if next == Some(Kind::Mark)
            && (next_value == "\"" || next_value == "'")
            && next_next == Some(Kind::Word)
        {
            // Duas aspas seguidas antes de uma palavra: as duas abrem.
            let inner = if next_value == "\"" { 0 } else { 1 };
            tokens[i + 1].value = vec![open[inner]];
            open[quote]
        } else if next.is_some() && is_decade(&next_value) {
            // A década abreviada (`'80s`).
            close[quote]
        } else if matches!(previous, Some(Kind::WhiteSpace | Kind::Mark))
            && next == Some(Kind::Word)
        {
            open[quote]
        } else if previous == Some(Kind::Word)
            || next.is_none()
            || next == Some(Kind::WhiteSpace)
            || (quote == 1 && next_value == "s")
        {
            close[quote]
        } else {
            open[quote]
        };
        tokens[i].value = vec![replacement];
    }
    let mut out = text.to_vec();
    for token in &tokens {
        if token.value.len() == 1 {
            out[token.start] = token.value[0];
        }
    }
    out
}

fn is_decade(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 3 && b[0].is_ascii_digit() && b[1].is_ascii_digit() && b[2] == b's'
}

/// A segunda volta, num pedaço de texto: reticências, crases e traços.
pub(crate) fn dashes_and_ellipses(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    for token in tokenize(&chars) {
        let value: String = token.value.iter().collect();
        let is_dots = token.kind == Kind::Mark && token.value.len() >= 3 && token.value[0] == '.';
        match value.as_str() {
            _ if is_dots => out.push('\u{2026}'),
            "``" => out.push(OPEN_DOUBLE),
            "''" => out.push(CLOSE_DOUBLE),
            "--" => out.push('\u{2014}'),
            _ => out.push_str(&value),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(text: &str) -> String {
        let chars: Vec<char> = text.chars().collect();
        quotes(&chars).into_iter().collect()
    }

    #[test]
    fn double_quotes() {
        assert_eq!(q(" disse \"olá\" e saiu"), " disse “olá” e saiu");
        assert_eq!(q(" \"fim.\""), " “fim.”");
        assert_eq!(q("\"...\""), "”...”");
        assert_eq!(q(" \"(isto)\""), " “(isto)”");
        // Duas aspas antes de uma palavra abrem as duas.
        assert_eq!(q(" \"'dentro' fora\""), " “‘dentro’ fora”");
    }

    #[test]
    fn sentence_end_hides_the_next_sentence() {
        // Célula de tabela emendada na seguinte: a aspa depois do `!` fecha
        // porque a frase acaba ali (a próxima começa com maiúscula).
        assert_eq!(q(" (\"vem, covarde!\")Social"), " (“vem, covarde!”)Social");
        // Com minúscula, a frase continua e a aspa enxerga a palavra (e abre,
        // como na biblioteca de origem).
        assert_eq!(q(" (\"vem!\")social"), " (“vem!“)social");
        assert_eq!(q(" fim.\"Olá\""), " fim.”Olá”");
    }

    #[test]
    fn single_quotes() {
        assert_eq!(q(" d'água"), " d’água");
        assert_eq!(q(" the '80s"), " the ’80s");
    }

    #[test]
    fn grouped_marks_are_left_alone() {
        assert_eq!(q(" \"\"vazio"), " \"\"vazio");
    }

    #[test]
    fn second_pass() {
        assert_eq!(dashes_and_ellipses("a--b"), "a\u{2014}b");
        assert_eq!(dashes_and_ellipses("a---b"), "a---b");
        assert_eq!(dashes_and_ellipses("e..."), "e…");
        assert_eq!(dashes_and_ellipses("e...."), "e…");
        assert_eq!(dashes_and_ellipses("``x''"), "“x”");
        assert_eq!(dashes_and_ellipses("1-2"), "1-2");
    }
}
