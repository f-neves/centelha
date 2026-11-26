//! Tarefas do repositório, chamadas por `cargo xtask <comando>`.
//!
//! - `sync-content`: copia o conteúdo do site atual (pasta em `RPG_SYSTEM_DIR`)
//!   para `content/`, limpa o metadado das imagens e varre o resultado.
//! - `scan`: varre os arquivos versionados, os nomes e as mensagens de commit
//!   atrás de termos proibidos.
//! - `site`: monta o site estático em `dist/` (páginas, WebAssembly e cola).

mod images;
mod notes;
mod scan;
mod site;
mod sync;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("sync-content") => sync::run(&args[1..]),
        Some("scan") => scan::run(&args[1..]),
        Some("site") => site::run(&args[1..]),
        _ => {
            eprintln!(
                "uso: cargo xtask <sync-content [--allow-dirty] | scan [--no-commits] | site [--dev] [--busca] [--out <pasta>]>"
            );
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("erro: {msg}");
            ExitCode::FAILURE
        }
    }
}
