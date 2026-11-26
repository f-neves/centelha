//! `cargo xtask sync-content`: a cópia do conteúdo do site atual.
//!
//! Lê a pasta do site atual em `RPG_SYSTEM_DIR` e copia só o que o site usa
//! para `content/`: capítulos, dados, imagens prontas e os ícones. Copia só
//! arquivos versionados lá (nada solto), monta tudo numa pasta de preparo,
//! limpa o metadado das imagens, varre o resultado e só então troca o
//! `content/`. Se a varredura achar qualquer coisa, `content/` fica como estava.
//! Única mudança no texto copiado: os nomes de papel das notas de revisão do
//! bestiário (ADR 0005, módulo `notes`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{images, notes, scan};

/// Uma origem: pasta (ou arquivo) no site atual, destino em `content/` e as
/// extensões aceitas.
struct Source {
    from: &'static str,
    to: &'static str,
    extensions: &'static [&'static str],
}

const SOURCES: &[Source] = &[
    Source {
        from: "src/content/chapters",
        to: "chapters",
        extensions: &["md"],
    },
    // Inclui `diagramas.json`, com os SVG dos diagramas já desenhados.
    Source {
        from: "src/data",
        to: "data",
        extensions: &["json"],
    },
    Source {
        from: "public/bestiario",
        to: "public/bestiario",
        extensions: &["webp"],
    },
    Source {
        from: "public/icones-bestiario",
        to: "public/icones-bestiario",
        extensions: &["png", "svg"],
    },
    Source {
        from: "public/favicon.ico",
        to: "public/favicon.ico",
        extensions: &["ico"],
    },
    Source {
        from: "public/favicon.svg",
        to: "public/favicon.svg",
        extensions: &["svg"],
    },
    Source {
        from: "public/favicon-16.png",
        to: "public/favicon-16.png",
        extensions: &["png"],
    },
    Source {
        from: "public/favicon-32.png",
        to: "public/favicon-32.png",
        extensions: &["png"],
    },
    Source {
        from: "public/icon-192-v2.png",
        to: "public/icon-192-v2.png",
        extensions: &["png"],
    },
    Source {
        from: "public/icon-512-v2.png",
        to: "public/icon-512-v2.png",
        extensions: &["png"],
    },
    Source {
        from: "public/icon-maskable-512-v2.png",
        to: "public/icon-maskable-512-v2.png",
        extensions: &["png"],
    },
    Source {
        from: "public/manifest.webmanifest",
        to: "public/manifest.webmanifest",
        extensions: &["webmanifest"],
    },
];

fn git_in(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
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
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Raiz deste repositório (onde está o `Cargo.toml` do workspace).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask mora na raiz")
        .to_path_buf()
}

pub fn run(args: &[String]) -> Result<(), String> {
    let allow_dirty = args.iter().any(|a| a == "--allow-dirty");
    let origin = std::env::var("RPG_SYSTEM_DIR")
        .map(PathBuf::from)
        .map_err(|_| "defina RPG_SYSTEM_DIR com a pasta do site atual".to_string())?;
    if !origin.join("src/content/chapters").is_dir() {
        return Err(format!(
            "{} não parece a pasta do site atual",
            origin.display()
        ));
    }

    let commit = git_in(&origin, &["rev-parse", "HEAD"])?.trim().to_string();
    let commit_date = git_in(&origin, &["log", "-1", "--format=%cI"])?
        .trim()
        .to_string();
    let paths: Vec<&str> = SOURCES.iter().map(|s| s.from).collect();
    let mut status_args = vec!["status", "--porcelain", "--"];
    status_args.extend(&paths);
    let dirty = git_in(&origin, &status_args)?;
    if !dirty.trim().is_empty() && !allow_dirty {
        return Err(format!(
            "há mudança não commitada nas pastas copiadas:\n{dirty}(use --allow-dirty para copiar mesmo assim)"
        ));
    }

    let root = repo_root();
    let staging = root.join("target/sync-content");
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|e| format!("limpar {}: {e}", staging.display()))?;
    }

    let mut ls_args = vec!["ls-files", "-z", "--"];
    ls_args.extend(&paths);
    let tracked = git_in(&origin, &ls_args)?;
    let mut files = 0usize;
    let mut bytes_total = 0u64;
    let mut stripped = 0usize;
    let mut renamed = 0usize;
    let mut skipped = Vec::new();
    let mut per_source = Vec::new();
    for source in SOURCES {
        let mut n = 0usize;
        let mut b = 0u64;
        for rel in tracked.split('\0').filter(|r| !r.is_empty()) {
            let inside = rel == source.from || rel.starts_with(&format!("{}/", source.from));
            if !inside {
                continue;
            }
            let ext = Path::new(rel)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if !source.extensions.contains(&ext.as_str()) {
                skipped.push(rel.to_string());
                continue;
            }
            // Origem que é arquivo sozinho: o destino é o próprio `to`.
            let inner = rel
                .strip_prefix(source.from)
                .unwrap_or("")
                .trim_start_matches('/');
            let dest = if inner.is_empty() {
                staging.join(source.to)
            } else {
                staging.join(source.to).join(inner)
            };
            let raw = fs::read(origin.join(rel)).map_err(|e| format!("ler {rel}: {e}"))?;
            let data = match images::kind_of(Path::new(rel)) {
                Some(kind) => {
                    let clean = images::strip(kind, &raw).map_err(|e| format!("{rel}: {e}"))?;
                    if clean != raw {
                        stripped += 1;
                    }
                    clean
                }
                // Texto: os nomes de papel das notas de revisão viram
                // "revisão" (ADR 0005); fora dos campos previstos, para.
                None => match String::from_utf8(raw) {
                    Ok(text) => {
                        let (text, n) =
                            notes::neutralize(&format!("{}/{inner}", source.to), &text)?;
                        renamed += n;
                        text.into_bytes()
                    }
                    Err(e) => e.into_bytes(),
                },
            };
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("criar {}: {e}", parent.display()))?;
            }
            fs::write(&dest, &data).map_err(|e| format!("gravar {}: {e}", dest.display()))?;
            n += 1;
            b += data.len() as u64;
        }
        per_source.push(format!(
            "{} -> content/{}: {n} arquivos, {}",
            source.from,
            source.to,
            human(b)
        ));
        files += n;
        bytes_total += b;
    }

    // A varredura roda sobre o que vai ser gravado, não sobre a origem.
    let mut findings = Vec::new();
    for path in walk(&staging)? {
        let rel = path
            .strip_prefix(&staging)
            .expect("dentro")
            .to_string_lossy()
            .replace('\\', "/");
        let data = fs::read(&path).map_err(|e| format!("ler {}: {e}", path.display()))?;
        findings.extend(scan::scan_file(&format!("content/{rel}"), &path, &data));
    }
    if !findings.is_empty() {
        for f in &findings {
            eprintln!("{f}");
        }
        return Err(format!(
            "{} achado(s): nada foi copiado para content/",
            findings.len()
        ));
    }

    let mut origem = String::new();
    origem.push_str("Origem do conteúdo desta pasta (gerado por `cargo xtask sync-content`; não editar à mão).\n\n");
    origem.push_str(&format!("commit: {commit}\n"));
    origem.push_str(&format!("data do commit: {commit_date}\n"));
    if !dirty.trim().is_empty() {
        origem.push_str("atenção: copiado com mudanças não commitadas na origem\n");
    }
    origem.push_str(&format!("arquivos: {files} ({})\n", human(bytes_total)));
    origem.push_str(&format!("imagens com metadado removido: {stripped}\n"));
    origem.push_str(&format!(
        "nomes de papel trocados por \"revisão\" nas notas (ADR 0005): {renamed}\n\n"
    ));
    for line in &per_source {
        origem.push_str(&format!("- {line}\n"));
    }
    fs::write(staging.join("ORIGEM.txt"), origem).map_err(|e| format!("gravar ORIGEM.txt: {e}"))?;

    let content = root.join("content");
    if content.exists() {
        fs::remove_dir_all(&content).map_err(|e| format!("limpar content/: {e}"))?;
    }
    fs::rename(&staging, &content).map_err(|e| format!("mover para content/: {e}"))?;

    for line in &per_source {
        println!("{line}");
    }
    if !skipped.is_empty() {
        println!("ignorados (extensão fora da lista): {}", skipped.join(", "));
    }
    println!(
        "total: {files} arquivos, {}; {stripped} imagens limpas; {renamed} nomes trocados; commit {commit}",
        human(bytes_total)
    );
    Ok(())
}

fn walk(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in fs::read_dir(&d).map_err(|e| format!("ler {}: {e}", d.display()))? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn human(bytes: u64) -> String {
    if bytes >= 1 << 20 {
        format!("{:.1} MB", bytes as f64 / (1u64 << 20) as f64)
    } else {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    }
}
