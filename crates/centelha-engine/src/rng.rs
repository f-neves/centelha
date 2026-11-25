//! O gerador aleatório do combate: Mulberry32.
//!
//! É o mesmo gerador do motor TypeScript, bit a bit: a mesma semente dá a
//! mesma sequência nos dois lados. Nenhum outro gerador aleatório entra no
//! motor.

/// Mulberry32: estado de 32 bits, uma saída por passo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mulberry32 {
    state: u32,
}

impl Mulberry32 {
    /// Começa da semente. Uma semente de JavaScript (número qualquer) passa
    /// antes por [`crate::js::to_uint32`], como o TypeScript faz com `>>> 0`.
    pub fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    /// A próxima saída, como inteiro de 32 bits sem sinal.
    pub fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_add(0x6d2b_79f5);
        let mut t = self.state;
        // Math.imul é a multiplicação de 32 bits que descarta o excesso
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        t ^ (t >> 14)
    }

    /// A próxima saída em [0, 1): a de 32 bits dividida por 2^32 (exata em f64).
    pub fn next_f64(&mut self) -> f64 {
        f64::from(self.next_u32()) / 4_294_967_296.0
    }
}

#[cfg(test)]
mod tests {
    use super::Mulberry32;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Mulberry32::new(10);
        let mut b = Mulberry32::new(10);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn f64_in_unit_interval() {
        let mut r = Mulberry32::new(u32::MAX);
        for _ in 0..10_000 {
            let x = r.next_f64();
            assert!((0.0..1.0).contains(&x));
        }
    }
}
