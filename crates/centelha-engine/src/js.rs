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

/// `a ** b` do JavaScript (`Math.pow`), pela `libm`.
pub fn pow(a: f64, b: f64) -> f64 {
    libm::pow(a, b)
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
    use super::{max, min, number_to_string, round, to_uint32};

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
