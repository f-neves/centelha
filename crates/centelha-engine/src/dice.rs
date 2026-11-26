//! Os dados do combate: a leitura de uma expressão (`2d6 +3`, `1d6 −2 (I)`)
//! e a rolagem pelo Mulberry32 (port de `rolarExpr`, `d6` e das leituras de
//! `rolagem.ts`).
//!
//! A leitura segue as expressões regulares do TypeScript: os três traços
//! (U+2212, U+2013, U+2014) viram hífen, o que está entre parênteses sai, cada
//! `Nd6` (com `d` maiúsculo ou minúsculo; `d6` sem número vale 1) conta dados,
//! e o fixo é a soma dos `+N` e `-N` que sobram.

use crate::rng::Mulberry32;
use crate::sheet::value::is_js_space;

/// Uma expressão de dados lida: quantos d6 ela pede e o fixo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiceExpr {
    pub dice: f64,
    pub flat: f64,
}

/// Uma rolagem (`Rolada`): os dados depois do ajuste, o fixo, as faces e o
/// total.
#[derive(Debug, Clone, PartialEq)]
pub struct Roll {
    pub dice: f64,
    pub flat: f64,
    pub rolls: Vec<f64>,
    pub total: f64,
}

/// `String(expr).replace(/[−–—]/g, '-').replace(/\([^)]*\)/g, ' ')`.
fn clean(expr: &str) -> Vec<char> {
    let dashed: Vec<char> = expr
        .chars()
        .map(|c| match c {
            '\u{2212}' | '\u{2013}' | '\u{2014}' => '-',
            c => c,
        })
        .collect();
    let mut out = Vec::with_capacity(dashed.len());
    let mut i = 0;
    while i < dashed.len() {
        if dashed[i] == '('
            && let Some(close) = dashed[i + 1..].iter().position(|c| *c == ')')
        {
            out.push(' ');
            i += close + 2;
            continue;
        }
        out.push(dashed[i]);
        i += 1;
    }
    out
}

/// Os números de uma sequência de dígitos ASCII (`parseInt(n, 10)`).
fn digits_value(d: &[char]) -> f64 {
    d.iter().collect::<String>().parse::<f64>().unwrap_or(0.0)
}

/// `/(\d*)d6/gi` sobre o texto limpo: devolve o total de dados e o texto com
/// cada `Nd6` trocado por um espaço.
fn take_dice(t: &[char]) -> (f64, Vec<char>) {
    let mut dice = 0.0;
    let mut rest = Vec::with_capacity(t.len());
    let mut i = 0;
    while i < t.len() {
        let mut j = i;
        while j < t.len() && t[j].is_ascii_digit() {
            j += 1;
        }
        if j + 1 < t.len() && matches!(t[j], 'd' | 'D') && t[j + 1] == '6' {
            dice += if j == i { 1.0 } else { digits_value(&t[i..j]) };
            rest.push(' ');
            i = j + 2;
        } else {
            rest.push(t[i]);
            i += 1;
        }
    }
    (dice, rest)
}

/// `/[+-]\s*\d+/g`, cada um pelo `parseInt` sem os espaços.
fn sum_flat(t: &[char]) -> f64 {
    let mut flat = 0.0;
    let mut i = 0;
    while i < t.len() {
        if matches!(t[i], '+' | '-') {
            let mut j = i + 1;
            while j < t.len() && is_js_space(t[j]) {
                j += 1;
            }
            let start = j;
            while j < t.len() && t[j].is_ascii_digit() {
                j += 1;
            }
            if j > start {
                let v = digits_value(&t[start..j]);
                flat += if t[i] == '-' { -v } else { v };
                i = j;
                continue;
            }
        }
        i += 1;
    }
    flat
}

impl DiceExpr {
    /// Lê a expressão (`baseDadosDeExpr` e `flatDeExpr`).
    pub fn parse(expr: &str) -> DiceExpr {
        let (dice, rest) = take_dice(&clean(expr));
        DiceExpr {
            dice,
            flat: sum_flat(&rest),
        }
    }
}

/// `dadosAjustados`: com dado na base, o ajuste nunca leva abaixo de 1; sem
/// dado, o ajuste não cria dado.
pub fn adjusted_dice(base: f64, extra: f64) -> f64 {
    if base > 0.0 {
        crate::js::max(1.0, base + extra)
    } else {
        crate::js::max(0.0, base + extra)
    }
}

/// `d6`: `1 + floor(acaso() × 6)`.
pub fn d6(rng: &mut Mulberry32) -> f64 {
    1.0 + (rng.next_f64() * 6.0).floor()
}

/// O tamanho que o `Array.from({ length })` dá a um número (`NaN` e negativo
/// dão 0, a fração cai).
fn array_length(x: f64) -> usize {
    if x.is_nan() || x <= 0.0 {
        0
    } else {
        x.floor() as usize
    }
}

/// A soma das faces mais o fixo, nessa ordem (`reduce` a partir de 0).
pub(crate) fn sum_total(rolls: &[f64], flat: f64) -> f64 {
    rolls.iter().fold(0.0, |a, b| a + b) + flat
}

/// `rolarExpr`: rola a expressão com o ajuste de dados e de fixo, um d6 por
/// vez, na ordem.
pub fn roll_expr(expr: &str, extra_dice: f64, extra_flat: f64, rng: &mut Mulberry32) -> Roll {
    let e = DiceExpr::parse(expr);
    let dice = adjusted_dice(e.dice, extra_dice);
    let flat = e.flat + extra_flat;
    let rolls: Vec<f64> = (0..array_length(dice)).map(|_| d6(rng)).collect();
    let total = sum_total(&rolls, flat);
    Roll {
        dice,
        flat,
        rolls,
        total,
    }
}
