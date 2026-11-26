//! O `id` de cada título, igual ao do site atual (os links `#âncora` entre
//! capítulos dependem dele): o texto do título em minúsculas, sem pontuação
//! nem símbolos, com os espaços trocados por `-`; repetido na mesma página
//! ganha `-1`, `-2`...; o `-` final cai.

use std::collections::HashMap;

use super::slug_table::REMOVED;

fn removed(c: char) -> bool {
    let cp = c as u32;
    REMOVED
        .binary_search_by(|&(lo, hi)| {
            if hi < cp {
                std::cmp::Ordering::Less
            } else if lo > cp {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// O `id` de um texto, sem conferir repetição.
pub(crate) fn slug(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|&c| !removed(c))
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

/// Os `id` de uma página: lembra os já dados para não repetir.
#[derive(Default)]
pub(crate) struct Slugger {
    seen: HashMap<String, usize>,
}

impl Slugger {
    /// O próximo `id` para o texto de um título.
    pub(crate) fn heading_id(&mut self, text: &str) -> String {
        // O site atual troca `{` por `${` no texto antes de calcular.
        let text = text.replace('{', "${");
        let original = slug(&text);
        let mut result = original.clone();
        while self.seen.contains_key(&result) {
            let count = self.seen.entry(original.clone()).or_insert(0);
            *count += 1;
            result = format!("{original}-{count}");
        }
        self.seen.insert(result.clone(), 0);
        if result.ends_with('-') {
            result.pop();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_the_current_site() {
        assert_eq!(
            slug("Preparo, Golpe e Recuperação"),
            "preparo-golpe-e-recuperação"
        );
        assert_eq!(
            slug("Firulas · Recompensa à Ousadia"),
            "firulas--recompensa-à-ousadia"
        );
        assert_eq!(slug("Salto (Velocidade 3)"), "salto-velocidade-3");
        assert_eq!(slug("Ações & Sistema"), "ações--sistema");
        assert_eq!(
            slug("\u{201c}Aspas\u{201d} \u{2014} e traço"),
            "aspas--e-traço"
        );
    }

    #[test]
    fn repeated_headings() {
        let mut s = Slugger::default();
        assert_eq!(s.heading_id("Exemplo"), "exemplo");
        assert_eq!(s.heading_id("Exemplo"), "exemplo-1");
        assert_eq!(s.heading_id("Exemplo"), "exemplo-2");
        assert_eq!(s.heading_id("Exemplo 1"), "exemplo-1-1");
    }

    #[test]
    fn trailing_dash_falls() {
        let mut s = Slugger::default();
        assert_eq!(s.heading_id("Fim ?"), "fim");
    }

    #[test]
    fn table_is_sorted() {
        assert!(REMOVED.windows(2).all(|w| w[0].1 < w[1].0));
        assert!(REMOVED.iter().all(|&(lo, hi)| lo <= hi));
    }
}
