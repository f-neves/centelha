//! Tarefas do repositório, chamadas por `cargo xtask <comando>`.
//!
//! - `sync-content`: copia o conteúdo do site atual (pasta em `RPG_SYSTEM_DIR`)
//!   para `content/`, limpa o metadado das imagens e varre o resultado.
//! - `scan`: varre os arquivos versionados, os nomes e as mensagens de commit
//!   atrás de termos proibidos.

mod images;
mod scan;
mod sync;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("sync-content") => sync::run(&args[1..]),
        Some("scan") => scan::run(&args[1..]),
        _ => {
            eprintln!("uso: cargo xtask <sync-content [--allow-dirty] | scan [--no-commits]>");
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
