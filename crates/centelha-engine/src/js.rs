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

#[cfg(test)]
mod tests {
    use super::to_uint32;

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
