//! `cargo xtask site`: monta o site estático em `dist/` (ADR 0003).
//!
//! 1. Confere as versões do `wasm-bindgen` e do `wasm-opt`.
//! 2. Roda o gerador do `centelha-web` (feature `ssr`), que grava as páginas.
//! 3. Compila a biblioteca do navegador (feature `hydrate`) para
//!    `wasm32-unknown-unknown`.
//! 4. `wasm-bindgen --target web` grava o WebAssembly e o script de cola em
//!    `dist/pkg/`.
//! 5. `wasm-opt -Oz` encolhe o WebAssembly (com a lista de recursos explícita:
//!    `--all-features` gera um arquivo que o navegador recusa).
//!
//! Opções: `--dev` compila o WebAssembly sem otimizar e pula o `wasm-opt` (o
//! WebAssembly de depuração escreve os avisos de hidratação no console);
//! `--out <pasta>` grava noutro lugar. O caminho base vem de `SITE_BASE_PATH`
//! (padrão `/centelha/`). As ferramentas são procuradas no `PATH`, ou onde
//! `WASM_BINDGEN` e `WASM_OPT` disserem.

use std::path::{Path, PathBuf};
use std::process::Command;

const WASM_BINDGEN_VERSION: &str = "0.2.130";
const BINARYEN_VERSION: &str = "133";
const PKG_NAME: &str = "centelha_web";
const WASM_FEATURES: &[&str] = &[
    "--enable-bulk-memory",
    "--enable-nontrapping-float-to-int",
    "--enable-sign-ext",
    "--enable-mutable-globals",
    "--enable-reference-types",
    "--enable-multivalue",
];

struct Options {
    dev: bool,
    out: PathBuf,
}

fn parse(args: &[String], root: &Path) -> Result<Options, String> {
    let mut options = Options {
        dev: false,
        out: root.join("dist"),
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--dev" => options.dev = true,
            "--out" => {
                let dir = it.next().ok_or("--out pede uma pasta")?;
                options.out = root.join(dir);
            }
            other => return Err(format!("opção desconhecida: {other}")),
        }
    }
    Ok(options)
}

/// Recusa uma pasta de saída que o build apagaria por engano: a raiz do
/// repositório, um ancestral dela ou uma pasta com `Cargo.toml`. Roda antes
/// de qualquer remoção.
fn check_out(out: &Path, root: &Path) -> Result<(), String> {
    if !out.exists() {
        return Ok(());
    }
    let refuse = |why: &str| {
        Err(format!(
            "--out {}: não uso esta pasta ({why})",
            out.display()
        ))
    };
    if out.join("Cargo.toml").exists() {
        return refuse("tem Cargo.toml");
    }
    let canonical = |p: &Path| {
        p.canonicalize()
            .map_err(|e| format!("{}: não resolvi o caminho ({e})", p.display()))
    };
    if canonical(root)?.starts_with(canonical(out)?) {
        return refuse("é a raiz do repositório ou uma pasta acima dela");
    }
    Ok(())
}

fn tool(env: &str, name: &str) -> String {
    std::env::var(env).unwrap_or_else(|_| name.to_string())
}

fn exec(cmd: &mut Command, what: &str) -> Result<(), String> {
    let status = cmd
        .status()
        .map_err(|e| format!("{what}: não rodou ({e})"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{what}: terminou com {status}"))
    }
}

fn version_of(program: &str) -> Result<String, String> {
    let out = Command::new(program)
        .arg("--version")
        .output()
        .map_err(|e| {
            format!(
                "{program} não encontrado ({e}); ponha no PATH ou aponte WASM_BINDGEN / WASM_OPT"
            )
        })?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Confere as versões das ferramentas de fora do Cargo.
fn check_tools(dev: bool) -> Result<(String, String), String> {
    let bindgen = tool("WASM_BINDGEN", "wasm-bindgen");
    let found = version_of(&bindgen)?;
    if found.split_whitespace().nth(1) != Some(WASM_BINDGEN_VERSION) {
        return Err(format!(
            "wasm-bindgen: achei {found:?}, o ADR 0003 pede {WASM_BINDGEN_VERSION} \
             (cargo install wasm-bindgen-cli --version {WASM_BINDGEN_VERSION} --locked)"
        ));
    }
    let opt = tool("WASM_OPT", "wasm-opt");
    if !dev {
        let found = version_of(&opt)?;
        let wanted = format!("version {BINARYEN_VERSION}");
        let ok = found
            .split_once(&wanted)
            .is_some_and(|(_, rest)| !rest.starts_with(|c: char| c.is_ascii_digit()));
        if !ok {
            return Err(format!(
                "wasm-opt: achei {found:?}, o ADR 0003 pede o binaryen version_{BINARYEN_VERSION} \
                 (aponte WASM_OPT para o wasm-opt certo)"
            ));
        }
    }
    Ok((bindgen, opt))
}

/// Raiz deste repositório (onde está o `Cargo.toml` do workspace).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask mora na raiz")
        .to_path_buf()
}

pub fn run(args: &[String]) -> Result<(), String> {
    let root = repo_root();
    let options = parse(args, &root)?;
    check_out(&options.out, &root)?;
    let (bindgen, opt) = check_tools(options.dev)?;
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    if options.out.exists() {
        std::fs::remove_dir_all(&options.out)
            .map_err(|e| format!("{}: não apaguei ({e})", options.out.display()))?;
    }

    // 2. As páginas.
    exec(
        Command::new(&cargo)
            .current_dir(&root)
            .args([
                "run",
                "--quiet",
                "-p",
                "centelha-web",
                "--features",
                "ssr",
                "--bin",
                "centelha-site",
                "--",
                "--content",
            ])
            .arg(root.join("content"))
            .arg("--out")
            .arg(&options.out),
        "gerador do site",
    )?;

    // 3. O WebAssembly das ilhas.
    let mut build = Command::new(&cargo);
    build.current_dir(&root).args([
        "build",
        "-p",
        "centelha-web",
        "--lib",
        "--target",
        "wasm32-unknown-unknown",
        "--features",
        "hydrate",
    ]);
    if !options.dev {
        build.arg("--release");
    }
    exec(&mut build, "compilação do WebAssembly")?;
    let profile = if options.dev { "debug" } else { "release" };
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(|dir| root.join(dir))
        .unwrap_or_else(|| root.join("target"));
    let wasm = target_dir
        .join("wasm32-unknown-unknown")
        .join(profile)
        .join(format!("{PKG_NAME}.wasm"));

    // 4. A cola.
    let pkg = options.out.join("pkg");
    let mut bind = Command::new(&bindgen);
    bind.args(["--target", "web", "--no-typescript", "--out-dir"])
        .arg(&pkg)
        .arg(&wasm);
    if options.dev {
        bind.arg("--debug").arg("--keep-debug");
    }
    exec(&mut bind, "wasm-bindgen")?;

    // 5. O encolhimento.
    let final_wasm = pkg.join(format!("{PKG_NAME}_bg.wasm"));
    if !options.dev {
        exec(
            Command::new(&opt)
                .arg("-Oz")
                .args(WASM_FEATURES)
                .arg("-o")
                .arg(&final_wasm)
                .arg(&final_wasm),
            "wasm-opt",
        )?;
    }

    let size = |p: &Path| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
    println!(
        "site em {}: WebAssembly {} B ({}), cola {} B",
        options.out.display(),
        size(&final_wasm),
        if options.dev {
            "depuração"
        } else {
            "release, wasm-opt -Oz"
        },
        size(&pkg.join(format!("{PKG_NAME}.js"))),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_the_repo_and_its_ancestors() {
        let root = repo_root();
        assert!(check_out(&root, &root).is_err());
        assert!(check_out(&root.join("."), &root).is_err());
        assert!(check_out(root.parent().unwrap(), &root).is_err());
        assert!(check_out(&root.join("xtask"), &root).is_err());
    }

    #[test]
    fn accepts_a_new_or_plain_folder() {
        let root = repo_root();
        assert!(check_out(&root.join("dist-que-nao-existe"), &root).is_ok());
        assert!(check_out(&root.join("docs"), &root).is_ok());
    }
}
