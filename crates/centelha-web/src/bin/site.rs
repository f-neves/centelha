//! O gerador do site estático: `centelha-site --content <pasta> --out <pasta>`.
//!
//! O caminho base vem de `SITE_BASE_PATH` (padrão `/centelha/`). Quem chama
//! normalmente é o `cargo xtask site`, que depois grava o WebAssembly em
//! `<out>/pkg/`.

use std::path::PathBuf;
use std::process::ExitCode;

use centelha_web::site::{DEFAULT_BASE, Site};

fn main() -> ExitCode {
    let mut content = PathBuf::from("content");
    let mut out = PathBuf::from("dist");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = args.next();
        match (arg.as_str(), value) {
            ("--content", Some(v)) => content = PathBuf::from(v),
            ("--out", Some(v)) => out = PathBuf::from(v),
            _ => {
                eprintln!("uso: centelha-site [--content <pasta>] [--out <pasta>]");
                return ExitCode::from(2);
            }
        }
    }
    let base = std::env::var("SITE_BASE_PATH").unwrap_or_else(|_| DEFAULT_BASE.to_string());

    let site = match Site::load(&content, &base) {
        Ok(site) => site,
        Err(e) => {
            eprintln!("erro: {e}");
            return ExitCode::FAILURE;
        }
    };
    match site.write(&out) {
        Ok(pages) => {
            println!(
                "{} páginas gravadas em {} (caminho base {})",
                pages.len(),
                out.display(),
                site.base()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("erro: {e}");
            ExitCode::FAILURE
        }
    }
}
