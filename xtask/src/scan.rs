//! Varredura de rastro: termos proibidos em conteúdo, nomes de arquivo,
//! mensagens de commit e metadado de imagem.
//!
//! Os termos são montados por partes (`concat!`) para que este arquivo não
//! seja ele mesmo um achado.

use std::path::Path;
use std::process::Command;

use crate::images;

/// Termos procurados como trecho, em qualquer posição (sem diferença de caixa).
const SUBSTRINGS: &[&str] = &[
    concat!("cla", "ude"),
    concat!("anthro", "pic"),
    concat!("co", "dex"),
    concat!("open", "ai"),
    concat!("cha", "tgpt"),
    concat!("copi", "lot"),
    concat!("gem", "ini"),
    concat!("co-authored", "-by"),
    concat!("generated ", "with"),
    concat!("c2", "pa"),
    concat!("trained", "algorithmic", "media"),
];

/// Termos curtos procurados como palavra (sem letra colada antes ou depois).
/// Também os nomes internos de papel trocados nas notas (ADR 0005).
const WORDS: &[&str] = &[
    concat!("g", "pt"),
    concat!("l", "lm"),
    concat!("arqui", "teto"),
    concat!("arqui", "teta"),
    concat!("arqui", "tetos"),
    concat!("bestia", "rista"),
    concat!("bestia", "ristas"),
];

/// Termos procurados no metadado de imagem e em arquivo binário.
const BINARY_MARKERS: &[&str] = &[
    concat!("cla", "ude"),
    concat!("anthro", "pic"),
    concat!("open", "ai"),
    concat!("cha", "tgpt"),
    concat!("g", "pt-image"),
    concat!("c2", "pa"),
    concat!("trained", "algorithmic", "media"),
];

/// Um achado: onde e qual termo.
#[derive(Debug)]
pub struct Finding {
    pub place: String,
    pub term: String,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: termo proibido \"{}\"", self.place, self.term)
    }
}

/// Procura os termos de texto numa sequência de bytes já em minúsculas.
fn text_terms(lower: &[u8]) -> Vec<&'static str> {
    let mut found = Vec::new();
    for term in SUBSTRINGS {
        if find(lower, term.as_bytes(), false) {
            found.push(*term);
        }
    }
    for term in WORDS {
        if find(lower, term.as_bytes(), true) {
            found.push(*term);
        }
    }
    found
}

fn find(hay: &[u8], needle: &[u8], whole_word: bool) -> bool {
    if needle.len() > hay.len() {
        return false;
    }
    (0..=hay.len() - needle.len()).any(|i| {
        if &hay[i..i + needle.len()] != needle {
            return false;
        }
        if !whole_word {
            return true;
        }
        let before = i.checked_sub(1).map(|j| hay[j]);
        let after = hay.get(i + needle.len()).copied();
        !before.is_some_and(|b| b.is_ascii_alphabetic())
            && !after.is_some_and(|b| b.is_ascii_alphabetic())
    })
}

/// Varre um texto (mensagem de commit, nome de arquivo).
pub fn scan_text(place: &str, text: &str) -> Vec<Finding> {
    text_terms(text.to_ascii_lowercase().as_bytes())
        .into_iter()
        .map(|term| Finding {
            place: place.to_string(),
            term: term.to_string(),
        })
        .collect()
}

/// Varre um arquivo pelo conteúdo: imagem pelo metadado, texto pelos termos,
/// binário pelos marcadores.
pub fn scan_file(place: &str, path: &Path, bytes: &[u8]) -> Vec<Finding> {
    let mut out = scan_text(&format!("{place} (nome)"), place);
    if let Some(kind) = images::kind_of(path) {
        match images::non_pixel_bytes(kind, bytes) {
            Ok(meta) => out.extend(binary_findings(place, &meta)),
            Err(e) => out.push(Finding {
                place: place.to_string(),
                term: e,
            }),
        }
    } else if std::str::from_utf8(bytes).is_ok() {
        out.extend(
            text_terms(&bytes.to_ascii_lowercase())
                .into_iter()
                .map(|term| Finding {
                    place: place.to_string(),
                    term: term.to_string(),
                }),
        );
    } else {
        out.extend(binary_findings(place, bytes));
    }
    out
}

fn binary_findings(place: &str, bytes: &[u8]) -> Vec<Finding> {
    let lower = bytes.to_ascii_lowercase();
    BINARY_MARKERS
        .iter()
        .filter(|m| find(&lower, m.as_bytes(), false))
        .map(|m| Finding {
            place: place.to_string(),
            term: (*m).to_string(),
        })
        .collect()
}

fn git(args: &[&str]) -> Result<Vec<u8>, String> {
    let out = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| format!("git não rodou: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {} falhou: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(out.stdout)
}

/// `cargo xtask scan`: arquivos versionados (conteúdo e nome) e mensagens de
/// todos os commits do ramo atual. `--no-commits` pula as mensagens.
pub fn run(args: &[String]) -> Result<(), String> {
    let with_commits = !args.iter().any(|a| a == "--no-commits");
    let mut findings = Vec::new();
    let files = git(&["ls-files", "-z"])?;
    let mut count = 0usize;
    for name in files.split(|&b| b == 0).filter(|n| !n.is_empty()) {
        let name = String::from_utf8_lossy(name).into_owned();
        let path = Path::new(&name);
        // Arquivo apagado na árvore e ainda no índice: nada a ler.
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        findings.extend(scan_file(&name, path, &bytes));
        count += 1;
    }
    let mut commits = 0usize;
    if with_commits && git(&["rev-parse", "--verify", "HEAD"]).is_ok() {
        let log = git(&["log", "--format=%H%n%an <%ae>%n%cn <%ce>%n%B%x00"])?;
        for entry in log.split(|&b| b == 0) {
            let text = String::from_utf8_lossy(entry);
            let text = text.trim_start();
            if text.is_empty() {
                continue;
            }
            let hash = &text[..text.len().min(10)];
            findings.extend(scan_text(&format!("commit {hash}"), text));
            commits += 1;
        }
    }
    for f in &findings {
        eprintln!("{f}");
    }
    if findings.is_empty() {
        println!("varredura limpa: {count} arquivos, {commits} commits");
        Ok(())
    } else {
        Err(format!("{} achado(s) na varredura", findings.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_terms_ignoring_case() {
        let msg = format!("Ajuste\n\n{}: Fulano", concat!("Co-Authored", "-By"));
        assert_eq!(scan_text("m", &msg).len(), 1);
        assert_eq!(scan_text("m", concat!("feito com Cha", "tGPT")).len(), 1);
    }

    #[test]
    fn short_words_need_boundaries() {
        assert_eq!(scan_text("m", concat!("modelo G", "PT-4")).len(), 1);
        assert!(scan_text("m", concat!("abg", "ptcd")).is_empty());
        assert!(scan_text("m", "Defesa, Absorção e Desgaste").is_empty());
        assert_eq!(
            scan_text("m", concat!("estimativa do Arqui", "teto-RS")).len(),
            1
        );
        assert!(scan_text("m", "Arquitetura e Engenharia").is_empty());
    }
}
