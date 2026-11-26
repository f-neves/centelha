//! A aritmética do JavaScript que o motor reproduz.
//!
//! Os números do TypeScript são `f64`. Aqui ficam as conversões e
//! arredondamentos do JavaScript que não têm equivalente direto em Rust.

/// `x >>> 0` do JavaScript (a operação ToUint32): trunca para inteiro e reduz
/// módulo 2^32. `NaN` e infinitos dão 0.
pub fn to_uint32(x: f64) -> u32 {
    if !x.is_finite() {
        return 0;
    }
    // o resto de inteiro por 2^32 é exato em f64, e o resultado fica em [0, 2^32)
    let r = x.trunc().rem_euclid(4_294_967_296.0);
    r as u32
}

/// O tamanho que o `Array.from({ length })` dá a um número (`NaN` e negativo
/// dão 0, a fração cai).
pub fn array_length(x: f64) -> usize {
    if x.is_nan() || x <= 0.0 {
        0
    } else {
        x.floor() as usize
    }
}

/// `x || 0` com um número: `NaN` e `-0` viram `0`.
pub fn or_zero(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 { 0.0 } else { x }
}

/// `Math.round` do JavaScript: o meio sobe (para +infinito), inclusive nos
/// negativos (`round(-2.5) == -2`, `round(2.5) == 3`). Diferente do
/// `f64::round` do Rust, que afasta o meio do zero.
pub fn round(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let f = x.floor();
    // x - floor(x) é exato em f64; comparar a diferença evita o erro de
    // floor(x + 0.5) em 0,49999999999999994 e nos números grandes
    let r = if x - f >= 0.5 { f + 1.0 } else { f };
    // o JavaScript devolve -0 para -0,5 <= x < 0 e para -0
    if r == 0.0 && x.is_sign_negative() {
        -0.0
    } else {
        r
    }
}

/// `Math.log10` (pela `libm`, a mesma conta no nativo e no WebAssembly).
pub fn log10(x: f64) -> f64 {
    libm::log10(x)
}

/// `a ** b` do JavaScript (`Math.pow`), com o arredondamento correto (o `f64`
/// mais perto do valor exato, e o par no empate).
///
/// O `Math.pow` do V8 coincide com o arredondamento correto em cerca de 99,96%
/// dos casos; no resto, difere por 1 ulp (por exemplo, `10 ** 23`). O `pow` da
/// `libm` (o do fdlibm) difere do V8 em cerca de 10% dos casos (por exemplo,
/// `20 ** 0.4`). O arredondamento correto é determinístico, igual no nativo e
/// no WebAssembly, e erra menos contra o V8. Portar o `pow` do V8 (o plano B)
/// fica para quando uma referência de paridade acusar uma das diferenças.
///
/// A conta:
/// - expoente inteiro de 1 a 128 com base de mantissa pequena: a potência
///   exata em inteiro de 128 bits, arredondada uma vez (com o par no empate,
///   como em `5 ** 23`);
/// - base positiva normal e expoente finito, com `|b ln a|` abaixo de 700 (o
///   resultado entre cerca de `1e-304` e `e^709`): precisão dupla dobrada (uns
///   106 bits: o logaritmo por um passo de Newton sobre o `log` da `libm`, a
///   exponencial pela série de Taylor), e só o resultado final é arredondado.
///   Quando esse resultado cai a menos de `2^-80` (relativo) de um meio-ponto
///   entre dois `f64`, a precisão dobrada não decide o lado. Com expoente
///   `p/q` de denominador potência de 2 até 16 (e `p` até 64), a comparação
///   exata em inteiros (`a^p` contra `m^q`, com `m` o meio-ponto) decide, com
///   o par no empate: é o empate com base potência perfeita
///   (`60219687609 ** 1.5`, exatamente um meio-ponto) e o quase empate
///   (`(1 - 2^-53) ** 1.5`, a `2^-106` do meio-ponto). Senão, com a base a
///   menos de `2^-20` de 1 e `|b|` até 1024, decide a comparação de `b ln a`
///   com `ln m` em ponto fixo de 124 bits, quando a diferença passa do erro
///   da conta (`64 (|b| + 2)` vezes `2^-124`; é o quase empate de `(1 - k 2^-53) ** b` com `b` decimal, como
///   `0.9999999999999994 ** 1.1`). Fora disso, fica a precisão dobrada;
/// - o resto pela `libm`: base zero, subnormal, negativa ou infinita, expoente
///   zero ou infinito, e o resultado fora da faixa acima (abaixo de cerca de
///   `1e-304` ou acima de `e^709`), onde o arredondamento é o da `libm` (que
///   erra o último bit em algumas potências pequenas demais, abaixo de
///   `1e-304`).
pub fn pow(a: f64, b: f64) -> f64 {
    if !(a >= f64::MIN_POSITIVE && a.is_finite() && b.is_finite()) || b == 0.0 || a == 1.0 {
        return libm::pow(a, b);
    }
    if let Some(x) = exact_integer_power(a, b) {
        return x;
    }
    let t = dd::mul_f(dd::ln(a), b);
    if !(t.0 > -700.0 && t.0 < 709.0) {
        return libm::pow(a, b);
    }
    let r = dd::exp(t);
    if near_midpoint(r.0, r.1) {
        // perto do meio-ponto, a precisão dobrada não decide o lado: com o
        // expoente racional de denominador potência de 2, a comparação exata
        // decide; com a base perto de 1, os logaritmos em ponto fixo de 124
        // bits decidem; sem nenhum dos dois, fica a precisão dobrada
        if let Some(x) = exact_side(a, b, r.0, r.1).or_else(|| near_one_side(a, b, r.0, r.1)) {
            return x;
        }
    }
    r.0 + r.1
}

/// O lado do arredondamento perto do meio-ponto, pela conta exata: com
/// `b = p/q` (`q` potência de 2 até 16, `p` até 64, `b` positivo: assim o
/// `f64` `b` é exatamente `p/q`), o valor exato `a^(p/q)`
/// está acima, abaixo ou em cima do meio-ponto `m` entre os dois `f64`
/// vizinhos conforme `a^p` está acima, abaixo ou igual a `m^q`, em inteiros
/// sem limite de tamanho. No empate, fica o de mantissa par. `None` quando
/// `b` não tem essa forma.
fn exact_side(a: f64, b: f64, hi: f64, lo: f64) -> Option<f64> {
    if b.is_nan() || b <= 0.0 {
        return None;
    }
    let (p, q) = [1u32, 2, 4, 8, 16].into_iter().find_map(|q| {
        let p = b * f64::from(q);
        (p.fract() == 0.0 && (1.0..=64.0).contains(&p) && p / f64::from(q) == b)
            .then_some((p as u32, q))
    })?;
    let next = if (lo > 0.0) == (hi > 0.0) {
        f64::from_bits(hi.to_bits() + 1)
    } else {
        f64::from_bits(hi.to_bits() - 1)
    };
    let (low, high) = if hi < next { (hi, next) } else { (next, hi) };
    let (lm, le) = mantissa(low);
    let (hm, he) = mantissa(high);
    let g = le.min(he);
    // m = (low + high) / 2, exato: (lm 2^(le-g) + hm 2^(he-g)) 2^(g-1)
    let sum = (u128::from(lm) << (le - g)) + (u128::from(hm) << (he - g));
    let mid = big::Big::from_u128(sum, g - 1);
    let (am, ae) = mantissa(a);
    let lhs = big::Big::from_u128(u128::from(am), ae).pow(p);
    let rhs = mid.pow(q);
    Some(match big::cmp(&lhs, &rhs) {
        std::cmp::Ordering::Greater => high,
        std::cmp::Ordering::Less => low,
        std::cmp::Ordering::Equal => {
            if low.to_bits() % 2 == 0 {
                low
            } else {
                high
            }
        }
    })
}

/// O lado do arredondamento perto do meio-ponto com a base perto de 1
/// (`|a - 1| < 2^-20`, `|b|` até 1024): compara `b ln a` com `ln m`, com `m`
/// o meio-ponto entre os dois `f64` vizinhos, em ponto fixo de 124 bits
/// (o `ln(1 + u)` pela série). Quando a diferença fica dentro do erro da
/// conta (`64 (|b| + 2)` vezes `2^-124`), não dá para decidir, e volta `None`.
fn near_one_side(a: f64, b: f64, hi: f64, lo: f64) -> Option<f64> {
    if !((a - 1.0).abs() < 1.0 / 1048576.0 && b.abs() <= 1024.0 && b != 0.0) {
        return None;
    }
    let next = if (lo > 0.0) == (hi > 0.0) {
        f64::from_bits(hi.to_bits() + 1)
    } else {
        f64::from_bits(hi.to_bits() - 1)
    };
    let (low, high) = if hi < next { (hi, next) } else { (next, hi) };
    if !(low > 0.5 && high < 2.0) {
        return None;
    }
    let one = fixed::ONE;
    let mid = (fixed::from_f64(low) + fixed::from_f64(high)) / 2;
    let la = fixed::ln1p(fixed::from_f64(a) - one);
    let lm = fixed::ln1p(mid - one);
    let (mb, eb) = mantissa(b.abs());
    let bl = fixed::mul_shift(la, i128::from(mb), (-eb) as u32);
    let bl = if b < 0.0 { -bl } else { bl };
    let diff = bl - lm;
    // o erro da conta: uns poucos 2^-124 em cada logaritmo, vezes |b|
    let tol = (b.abs() as i128 + 2) * 64;
    if diff.abs() <= tol {
        return None;
    }
    Some(if diff > 0 { high } else { low })
}

/// Ponto fixo com 124 bits depois da vírgula em `i128`, só o que o
/// [`near_one_side`] usa.
mod fixed {
    pub const FRAC: u32 = 124;
    pub const ONE: i128 = 1 << FRAC;

    /// Um `f64` entre 0,5 e 2 com a mantissa cabendo em 124 bits de fração
    /// (múltiplo de `2^-124`), exato.
    pub fn from_f64(x: f64) -> i128 {
        let (m, e) = super::mantissa(x);
        let shift = e + i64::from(FRAC);
        i128::from(m) << shift
    }

    /// `(x × y) >> s`, com o produto em 256 bits (truncado na direção do zero).
    pub fn mul_shift(x: i128, y: i128, s: u32) -> i128 {
        let neg = (x < 0) != (y < 0);
        let (a, b) = (x.unsigned_abs(), y.unsigned_abs());
        let (a1, a0) = (a >> 64, a & u128::from(u64::MAX));
        let (b1, b0) = (b >> 64, b & u128::from(u64::MAX));
        let low = a0 * b0;
        let mid1 = a1 * b0;
        let mid2 = a0 * b1;
        let (mid, carry_mid) = mid1.overflowing_add(mid2);
        let (lo, carry_lo) = low.overflowing_add(mid << 64);
        let hi = a1 * b1 + (mid >> 64) + (u128::from(carry_mid) << 64) + u128::from(carry_lo);
        let r = if s == 0 {
            lo
        } else if s < 128 {
            (lo >> s) | (hi << (128 - s))
        } else {
            hi >> (s - 128)
        };
        let r = r as i128;
        if neg { -r } else { r }
    }

    /// `ln(1 + u)` pela série, para `|u|` abaixo de `2^-20`.
    pub fn ln1p(u: i128) -> i128 {
        let mut sum = 0i128;
        let mut term = u;
        let mut n = 1i128;
        while term != 0 {
            sum += if n % 2 == 1 { term / n } else { -(term / n) };
            term = mul_shift(term, u, FRAC);
            n += 1;
        }
        sum
    }
}

/// A mantissa inteira e o expoente de um `f64` normal positivo
/// (`x = m × 2^e`).
fn mantissa(x: f64) -> (u64, i64) {
    let bits = x.to_bits();
    let m = (bits & ((1u64 << 52) - 1)) | (1u64 << 52);
    let e = ((bits >> 52) & 0x7ff) as i64 - 1075;
    (m, e)
}

/// Inteiros sem limite de tamanho vezes uma potência de 2, só o que o
/// [`exact_side`] usa.
mod big {
    use std::cmp::Ordering;

    /// `limbs × 2^exp`, com os pedaços de 32 bits do menos significativo.
    pub struct Big {
        limbs: Vec<u32>,
        exp: i64,
    }

    impl Big {
        pub fn from_u128(mut v: u128, exp: i64) -> Big {
            let mut limbs = Vec::new();
            while v > 0 {
                limbs.push(v as u32);
                v >>= 32;
            }
            Big { limbs, exp }
        }

        fn mul(&self, o: &Big) -> Big {
            let mut r = vec![0u32; self.limbs.len() + o.limbs.len()];
            for (i, &x) in self.limbs.iter().enumerate() {
                let mut carry = 0u64;
                for (j, &y) in o.limbs.iter().enumerate() {
                    let t = u64::from(x) * u64::from(y) + u64::from(r[i + j]) + carry;
                    r[i + j] = t as u32;
                    carry = t >> 32;
                }
                let mut k = i + o.limbs.len();
                while carry > 0 {
                    let t = u64::from(r[k]) + carry;
                    r[k] = t as u32;
                    carry = t >> 32;
                    k += 1;
                }
            }
            while r.last() == Some(&0) {
                r.pop();
            }
            Big {
                limbs: r,
                exp: self.exp + o.exp,
            }
        }

        pub fn pow(&self, n: u32) -> Big {
            let mut r = Big {
                limbs: vec![1],
                exp: 0,
            };
            for _ in 0..n {
                r = r.mul(self);
            }
            r
        }

        /// O mesmo valor com o expoente `e` (menor ou igual ao de agora).
        fn at(&self, e: i64) -> Vec<u32> {
            let shift = (self.exp - e) as usize;
            let (words, bits) = (shift / 32, shift % 32);
            let mut out = vec![0u32; words];
            let mut carry = 0u32;
            for &x in &self.limbs {
                if bits == 0 {
                    out.push(x);
                } else {
                    out.push((x << bits) | carry);
                    carry = x >> (32 - bits);
                }
            }
            if carry > 0 {
                out.push(carry);
            }
            while out.last() == Some(&0) {
                out.pop();
            }
            out
        }
    }

    pub fn cmp(a: &Big, b: &Big) -> Ordering {
        let e = a.exp.min(b.exp);
        let (x, y) = (a.at(e), b.at(e));
        x.len()
            .cmp(&y.len())
            .then_with(|| x.iter().rev().cmp(y.iter().rev()))
    }
}

/// O resultado em precisão dupla dobrada (`hi + lo`, com `hi` já o `f64` mais
/// perto) está a menos de `2^-80` (relativo) do meio-ponto entre `hi` e o
/// vizinho do lado de `lo`?
fn near_midpoint(hi: f64, lo: f64) -> bool {
    if lo == 0.0 || !hi.is_normal() {
        return false;
    }
    let bits = hi.to_bits();
    let next = if (lo > 0.0) == (hi > 0.0) {
        f64::from_bits(bits + 1)
    } else {
        f64::from_bits(bits - 1)
    };
    let half = (next - hi).abs() / 2.0;
    (lo.abs() - half).abs() <= hi.abs() * libm::scalbn(1.0, -80)
}

/// `a ** b` exato, quando `b` é inteiro positivo e a potência da mantissa de
/// `a` cabe em 128 bits: `a = m × 2^e` com `m` ímpar, `a^b = m^b × 2^(e b)`.
/// O `u128 as f64` arredonda corretamente (par no empate), e a escala por
/// potência de 2 é exata na faixa normal. `None` fora disso.
fn exact_integer_power(a: f64, b: f64) -> Option<f64> {
    if !((1.0..=128.0).contains(&b) && b.fract() == 0.0) {
        return None;
    }
    let n = b as u32;
    let bits = a.to_bits();
    let mant = (bits & ((1u64 << 52) - 1)) | (1u64 << 52);
    let exp = ((bits >> 52) & 0x7ff) as i32 - 1075;
    let tz = mant.trailing_zeros();
    let m = u128::from(mant >> tz);
    let e = exp + tz as i32;
    let width = 128 - m.leading_zeros();
    if width * n > 128 {
        return None;
    }
    let p = m.checked_pow(n)?;
    let scale = e.checked_mul(n as i32)?;
    let f = p as f64;
    // a escala só é exata se o resultado ficar na faixa normal
    let top = libm::ilogb(f) + scale;
    if !(-1021..=1023).contains(&top) {
        return None;
    }
    Some(libm::scalbn(f, scale))
}

/// A aritmética de precisão dupla dobrada (um número como a soma de dois
/// `f64`, o alto e o baixo), só o que o [`pow`] usa.
mod dd {
    /// `(alto, baixo)`.
    #[derive(Clone, Copy)]
    pub struct Dd(pub f64, pub f64);

    fn two_sum(a: f64, b: f64) -> Dd {
        let s = a + b;
        let bb = s - a;
        Dd(s, (a - (s - bb)) + (b - bb))
    }

    fn quick(a: f64, b: f64) -> Dd {
        let s = a + b;
        Dd(s, b - (s - a))
    }

    fn two_prod(a: f64, b: f64) -> Dd {
        let p = a * b;
        Dd(p, libm::fma(a, b, -p))
    }

    pub fn add(a: Dd, b: Dd) -> Dd {
        let s = two_sum(a.0, b.0);
        quick(s.0, s.1 + a.1 + b.1)
    }

    fn neg(a: Dd) -> Dd {
        Dd(-a.0, -a.1)
    }

    pub fn mul(a: Dd, b: Dd) -> Dd {
        let p = two_prod(a.0, b.0);
        quick(p.0, p.1 + a.0 * b.1 + a.1 * b.0)
    }

    pub fn mul_f(a: Dd, b: f64) -> Dd {
        let p = two_prod(a.0, b);
        quick(p.0, p.1 + a.1 * b)
    }

    fn div_f(a: Dd, b: f64) -> Dd {
        let q = a.0 / b;
        let r = add(a, neg(two_prod(q, b)));
        quick(q, r.0 / b)
    }

    /// `ln 2` em precisão dupla dobrada.
    const LN2: Dd = Dd(std::f64::consts::LN_2, 2.319_046_813_846_299_6e-17);

    /// `e^a`, com `|a.0|` até uns 709: `a = k ln 2 + r`, e a série de Taylor em
    /// `r / 2^8`, elevada ao quadrado oito vezes.
    pub fn exp(a: Dd) -> Dd {
        let k = (a.0 / LN2.0).round();
        let r = add(a, neg(mul_f(LN2, k)));
        let r = Dd(r.0 / 256.0, r.1 / 256.0);
        let mut sum = Dd(1.0, 0.0);
        let mut term = Dd(1.0, 0.0);
        for n in 1..=20 {
            term = div_f(mul(term, r), f64::from(n));
            sum = add(sum, term);
            if term.0.abs() < 1e-36 {
                break;
            }
        }
        for _ in 0..8 {
            sum = mul(sum, sum);
        }
        let e = k as i32;
        Dd(libm::scalbn(sum.0, e), libm::scalbn(sum.1, e))
    }

    /// `ln x` para `x` positivo e finito: o `log` da `libm` e um passo de
    /// Newton com a exponencial acima.
    pub fn ln(x: f64) -> Dd {
        let l0 = libm::log(x);
        let e = exp(Dd(l0, 0.0));
        let diff = add(Dd(x, 0.0), neg(e));
        add(Dd(l0, 0.0), Dd(diff.0 / e.0, 0.0))
    }
}

/// `Math.max(a, b)`: `NaN` contamina, e `max(-0, 0)` é `0`.
pub fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a.is_sign_negative() { b } else { a }
    } else if a > b {
        a
    } else {
        b
    }
}

/// `Math.min(a, b)`: `NaN` contamina, e `min(-0, 0)` é `-0`.
pub fn min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a.is_sign_negative() { a } else { b }
    } else if a < b {
        a
    } else {
        b
    }
}

/// O número como texto, como o JavaScript escreve (`String(x)`, `${x}`): a
/// forma mais curta que volta ao mesmo número, sem expoente entre 1e-6 e
/// 1e21, e `-0` como `0`.
pub fn number_to_string(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if x == 0.0 {
        return "0".into();
    }
    let a = x.abs();
    if (1e-6..1e21).contains(&a) {
        return format!("{x}");
    }
    let e = format!("{x:e}");
    match e.split_once('e') {
        Some((m, exp)) if !exp.starts_with('-') => format!("{m}e+{exp}"),
        _ => e,
    }
}

#[cfg(test)]
mod tests {
    use super::{max, min, number_to_string, pow, round, to_uint32};

    #[test]
    fn numbers_as_text_like_javascript() {
        let cases = [
            (0.0, "0"),
            (-0.0, "0"),
            (3.0, "3"),
            (-2.5, "-2.5"),
            (0.1 + 0.2, "0.30000000000000004"),
            (1e21, "1e+21"),
            (1.5e-7, "1.5e-7"),
            (1e-7, "1e-7"),
            (1e-6, "0.000001"),
            (123456789012345680000.0, "123456789012345680000"),
            (f64::NAN, "NaN"),
            (f64::NEG_INFINITY, "-Infinity"),
        ];
        for (x, want) in cases {
            assert_eq!(number_to_string(x), want, "{x:e}");
        }
    }

    #[test]
    fn max_and_min_like_javascript() {
        assert!(max(-0.0, 0.0).is_sign_positive());
        assert!(max(0.0, -0.0).is_sign_positive());
        assert!(min(0.0, -0.0).is_sign_negative());
        assert!(max(1.0, f64::NAN).is_nan());
        assert_eq!(min(2.0, 3.0), 2.0);
    }

    /// O `Math.pow` do Node, bit a bit, onde ele arredonda corretamente (o
    /// `pow` da `libm` erra o último bit no primeiro).
    #[test]
    fn pow_like_javascript() {
        for (a, b, want) in [
            (20.0, 0.4, 3.3144540173399872),
            (2.0, 0.5, std::f64::consts::SQRT_2),
            (10.0, -0.7, 0.19952623149688797),
            (0.1, 0.15, 0.7079457843841379),
            (1.5, 2.5, 2.7556759606310752),
            (7.0, 0.7, 3.9045287771227217),
            (2.0, 10.0, 1024.0),
            // base subnormal: pela `libm`
            (5e-324, 0.5, 2.2227587494850775e-162),
            (1e-310, 0.5, 9.999999999999986e-156),
            (0.5, 1074.0, 5e-324),
            (2.0, -1074.0, 5e-324),
            (1e-310, 2.0, 0.0),
            // potência inteira com empate exato: o par
            (5.0, 23.0, 11920928955078124.0),
            (7.0, 19.0, 11398895185373144.0),
            (3.0, 35.0, 50031545098999704.0),
            (5.0, 22.0, 2384185791015625.0),
            (2.5, 10.0, 9536.7431640625),
            (1.5, 40.0, 11057332.320940012),
            (9.0, 0.5, 3.0),
            (3.0, -2.0, 0.1111111111111111),
            (0.0, 0.5, 0.0),
            (-8.0, 2.0, 64.0),
        ] {
            assert_eq!(pow(a, b).to_bits(), f64::to_bits(want), "{a} ** {b}");
        }
        assert!(pow(-8.0, 0.5).is_nan());
        // empate com expoente fracionário (base potência perfeita, o
        // resultado exatamente no meio-ponto): o par, como o arredondamento
        // correto e o Node
        for (a, b, want) in [
            (60219687609.0, 1.5, 14777730680185772.0),
            (43315183129.0, 1.5, 9014885858356868.0),
            (68184176641.0, 1.5, 17804320388674560.0),
            (2634129.0, 2.5, 11261405560364344.0),
        ] {
            assert_eq!(pow(a, b).to_bits(), f64::to_bits(want), "{a} ** {b}");
        }
        // empates em que o Node sai do par (dá ...8002); o Rust fica com o
        // arredondamento correto
        assert_eq!(pow(2563201.0, 2.5), 10518568985608000.0);
        assert_eq!(pow(6569999366401.0, 1.25), 10518568985608000.0);
        // onde o V8 erra o arredondamento, o Rust fica com o correto
        // (`Math.pow(10, 23)` dá 1.0000000000000001e+23 no Node)
        assert_eq!(pow(10.0, 23.0), 1e23);
    }

    /// Os empates com expoente fracionário: `(s^k) ** (j/k)` com `s` ímpar e
    /// `s^j` de exatamente 54 bits é um meio-ponto entre dois `f64`, e o
    /// arredondamento correto é o par (o `u128 as f64`).
    #[test]
    fn pow_ties_with_fractional_exponent() {
        let mut n = 0;
        for (root, power, b) in [(2u32, 3u32, 1.5), (4, 5, 1.25), (2, 5, 2.5)] {
            let mut s: u64 = 1;
            loop {
                let r = u128::from(s).pow(power);
                if r >= 1 << 54 {
                    break;
                }
                let a = u128::from(s).pow(root);
                // amostra: um em cada 16 da faixa de 54 bits
                if r >= 1 << 53 && a < 1 << 53 && (s / 2).is_multiple_of(16) {
                    assert_eq!(pow(a as f64, b), r as f64, "{a} ** {b}");
                    n += 1;
                }
                s += 2;
            }
        }
        assert!(n > 1500, "{n}");
    }

    /// O quase empate: `(1 - 2^-53) ** 1.5` fica a `2^-106` acima do meio-ponto,
    /// e o arredondamento correto (e o Node) sobe.
    #[test]
    fn pow_near_tie() {
        let x = 1.0 - f64::EPSILON / 2.0;
        assert_eq!(pow(x, 1.5), x);
        // 1,1 não é 11/10 em f64: a conta exata não vale, e a base perto de 1
        // decide pelos logaritmos em ponto fixo
        assert_eq!(pow(0.9999999999999994, 1.1), 0.9999999999999993);
        assert_eq!(pow(0.9999999999999994, 0.9), 0.9999999999999994);
        assert_eq!(
            1.0 - pow(91.9425 / 122.59 / 0.75, 1.5),
            1.1102230246251565e-16
        );
    }

    #[test]
    fn round_like_javascript() {
        let cases = [
            (2.5, 3.0),
            (-2.5, -2.0),
            (1.5, 2.0),
            (-1.5, -1.0),
            (0.49999999999999994, 0.0),
            (-0.5000000000000001, -1.0),
            (4503599627370497.0, 4503599627370497.0),
            (-7.0, -7.0),
            (1234.565 * 100.0, 123457.0),
        ];
        for (x, want) in cases {
            assert_eq!(round(x).to_bits(), f64::to_bits(want), "round({x})");
        }
        assert!(round(-0.4).is_sign_negative());
        assert!(round(-0.0).is_sign_negative());
        assert!(round(0.4).is_sign_positive());
    }

    #[test]
    fn like_unsigned_shift() {
        assert_eq!(to_uint32(0.0), 0);
        assert_eq!(to_uint32(-0.0), 0);
        assert_eq!(to_uint32(1.0), 1);
        assert_eq!(to_uint32(-1.0), u32::MAX);
        assert_eq!(to_uint32(3.7), 3);
        assert_eq!(to_uint32(-3.7), u32::MAX - 2);
        assert_eq!(to_uint32(4_294_967_296.0), 0);
        assert_eq!(to_uint32(4_294_967_301.0), 5);
        assert_eq!(to_uint32(-2_147_483_648.0), 2_147_483_648);
        assert_eq!(to_uint32(f64::NAN), 0);
        assert_eq!(to_uint32(f64::INFINITY), 0);
        assert_eq!(to_uint32(1e300), 0);
    }
}
