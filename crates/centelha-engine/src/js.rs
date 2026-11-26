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

/// `a ** b` do JavaScript (`Math.pow`), arredondado para o `f64` mais perto.
///
/// O `pow` da `libm` (o do fdlibm) erra o último bit em alguns casos, e o
/// `Math.pow` do V8 que gera as referências não erra (por exemplo, `20 ** 0.4`).
/// Com base positiva e finita e expoente finito, a conta vai em precisão dupla
/// dobrada (uns 106 bits: o logaritmo por um passo de Newton sobre o `log` da
/// `libm`, a exponencial pela série de Taylor), e só o resultado final é
/// arredondado. Os casos de borda (base zero, negativa ou infinita, expoente
/// zero ou infinito, resultado fora da faixa normal) seguem pela `libm`.
pub fn pow(a: f64, b: f64) -> f64 {
    if !(a > 0.0 && a.is_finite() && b.is_finite()) || b == 0.0 || a == 1.0 {
        return libm::pow(a, b);
    }
    let t = dd::mul_f(dd::ln(a), b);
    if !(t.0 > -700.0 && t.0 < 709.0) {
        return libm::pow(a, b);
    }
    let r = dd::exp(t);
    r.0 + r.1
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

    /// O `Math.pow` do Node, bit a bit (o `pow` da `libm` erra o último bit
    /// no primeiro).
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
            (3.0, -2.0, 0.1111111111111111),
            (0.0, 0.5, 0.0),
            (-8.0, 2.0, 64.0),
        ] {
            assert_eq!(pow(a, b).to_bits(), f64::to_bits(want), "{a} ** {b}");
        }
        assert!(pow(-8.0, 0.5).is_nan());
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
