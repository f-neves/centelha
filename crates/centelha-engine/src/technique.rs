//! As contas das Técnicas que o catálogo e a árvore usam: o modificador pela
//! trilha de melhora (`modProeza`, `src/lib/data.ts`), a Técnica que o site
//! oferece (`tecnicaDisponivel`, `src/lib/modulos.ts`) e o fecho dos
//! pré-requisitos (`closure`, `src/components/ArvoreTecnicas.astro`).

use std::collections::{HashMap, HashSet};

use centelha_data::{Rules, Technique};

use crate::character::feat_cost;

/// O valor que marca "sem número" nas trilhas (o travessão).
const NO_VALUE: &str = "\u{2014}";

/// O modificador de uma Técnica: o valor do nível e o texto da dica
/// (`rótulo: valor · unidade`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TechniqueModifier {
    pub value: String,
    pub title: String,
}

/// `modProeza`: o modificador pela trilha de melhora e o nível. `None` na
/// trilha `estado` (capacidade sem número), na trilha que não existe, no
/// nível fora de 1 a 6 e no valor vazio ou de travessão.
pub fn technique_modifier(rules: &Rules, effect: &str, level: f64) -> Option<TechniqueModifier> {
    let track = rules.feat_scales.as_ref()?.tracks.get(effect)?;
    if effect == "estado" {
        return None;
    }
    let i = level - 1.0;
    if !(i >= 0.0 && i.fract() == 0.0) {
        return None;
    }
    let v = track.values.get(i as usize)?;
    if v.is_empty() || v == NO_VALUE {
        return None;
    }
    let unit = match track.unit.as_deref() {
        Some(u) if !u.is_empty() => format!(" \u{b7} {u}"),
        _ => String::new(),
    };
    Some(TechniqueModifier {
        value: v.clone(),
        title: format!("{}: {v}{unit}", track.label),
    })
}

/// `tecnicaDisponivel`: o site oferece a Técnica? As do módulo Fôlego saíram
/// do jogo.
pub fn technique_available(t: &Technique) -> bool {
    t.module.as_deref() != Some("folego")
}

/// O fecho dos pré-requisitos de uma Técnica (`closure`): ela mesma e tudo o
/// que ela pede, direta ou indiretamente, entre as Técnicas dadas (a árvore
/// passa só as disponíveis). Pré-requisito fora da lista não entra.
///
/// A ordem é a da busca da fonte (pilha: o último pré-requisito empilhado
/// sai primeiro), a ordem em que a árvore lista a cadeia.
pub fn prerequisite_closure<'a>(techniques: &'a [Technique], id: &'a str) -> Vec<&'a str> {
    let by_id: HashMap<&str, &Technique> = techniques.iter().map(|t| (t.id.as_str(), t)).collect();
    let mut seen = HashSet::new();
    let mut order = Vec::new();
    let mut stack = vec![id];
    while let Some(x) = stack.pop() {
        if !seen.insert(x) {
            continue;
        }
        order.push(x);
        if let Some(t) = by_id.get(x) {
            for p in &t.prereq {
                if by_id.contains_key(p.as_str()) {
                    stack.push(p);
                }
            }
        }
    }
    order
}

/// O XP da cadeia: o custo de cada Técnica do fecho pelo nível (uma Técnica
/// fora da lista custa o nível 0).
pub fn chain_xp(rules: &Rules, techniques: &[Technique], chain: &[&str]) -> f64 {
    let by_id: HashMap<&str, &Technique> = techniques.iter().map(|t| (t.id.as_str(), t)).collect();
    chain.iter().fold(0.0, |s, id| {
        s + feat_cost(rules, by_id.get(id).map_or(0.0, |t| t.level))
    })
}
