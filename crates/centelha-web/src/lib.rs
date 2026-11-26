//! O site do Centelha.
//!
//! As páginas do livro saem prontas em HTML no build (feature `ssr`, binário
//! `centelha-site`), com o texto inteiro: ler e buscar não dependem de
//! WebAssembly. As partes interativas são ilhas do Leptos (`#[island]`), que
//! hidratam no navegador (feature `hydrate`, biblioteca `cdylib`). As duas
//! features nunca se combinam (ADR 0003).

pub mod cards;
pub mod shell;
pub mod theme;
pub mod toc;

#[cfg(feature = "ssr")]
pub mod site;

/// Ponto de entrada do navegador: o script de carga das ilhas chama esta
/// função depois de iniciar o WebAssembly e, em seguida, hidrata cada ilha.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    leptos::mount::hydrate_islands();
}
