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

#[cfg(test)]
mod tests {
    use super::{round, to_uint32};

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
