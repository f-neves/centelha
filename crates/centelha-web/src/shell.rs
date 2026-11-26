//! Os botões da casca que precisam de JavaScript, como ilhas pequenas: abrir e
//! fechar a gaveta da navegação no celular, e recolher e expandir a barra
//! lateral no desktop.
//!
//! Port do script de `src/layouts/Base.astro` do site atual. Cada botão é uma
//! ilha separada porque os botões ficam em lugares diferentes da página (a
//! barra de cima, a barra lateral, o véu sobre o texto); o estado mora no
//! documento, como no site atual: a classe `drawer-open` no `<body>`, a classe
//! `sidebar-collapsed` no `<html>` e a chave `sidebar` do `localStorage`, que o
//! script do `<head>` lê antes da primeira pintura.

use leptos::prelude::*;

/// "☰ Menu" na barra de cima (só no celular): abre a gaveta. Também liga a
/// tecla Esc, que fecha a gaveta.
#[island]
pub fn DrawerToggle() -> impl IntoView {
    Effect::new(move |_| browser::close_drawer_on_escape());
    view! {
        <button
            type="button"
            class="btn drawer-toggle"
            aria-label="Abrir navegação"
            aria-expanded="false"
            on:click=move |_| browser::set_drawer(true)
        >
            "☰ Menu"
        </button>
    }
}

/// "✕" dentro da gaveta: fecha.
#[island]
pub fn DrawerClose() -> impl IntoView {
    view! {
        <button
            type="button"
            class="btn drawer-close"
            aria-label="Fechar navegação"
            on:click=move |_| browser::set_drawer(false)
        >
            "✕"
        </button>
    }
}

/// O véu escuro atrás da gaveta aberta: um clique fora fecha.
#[island]
pub fn Scrim() -> impl IntoView {
    view! { <div class="scrim" on:click=move |_| browser::set_drawer(false)></div> }
}

/// "«" na barra lateral (só no desktop): recolhe a barra.
#[island]
pub fn SidebarCollapse() -> impl IntoView {
    view! {
        <button
            type="button"
            class="btn sidebar-collapse"
            aria-label="Recolher menu"
            title="Recolher menu"
            on:click=move |_| browser::set_sidebar_collapsed(true)
        >
            "«"
        </button>
    }
}

/// "» Menu" no canto (só no desktop, com a barra recolhida): expande.
#[island]
pub fn SidebarExpand() -> impl IntoView {
    view! {
        <button
            type="button"
            class="btn sidebar-expand"
            aria-label="Expandir menu"
            title="Expandir menu"
            on:click=move |_| browser::set_sidebar_collapsed(false)
        >
            "» Menu"
        </button>
    }
}

/// O que os botões fazem no documento. Fora do navegador não fazem nada.
#[cfg(feature = "hydrate")]
mod browser {
    use leptos::prelude::window_event_listener;

    fn document() -> Option<web_sys::Document> {
        web_sys::window()?.document()
    }

    /// Abre ou fecha a gaveta e diz ao botão do menu se ela está aberta.
    pub fn set_drawer(open: bool) {
        let Some(document) = document() else { return };
        if let Some(body) = document.body() {
            let _ = body.class_list().toggle_with_force("drawer-open", open);
        }
        if let Ok(Some(toggle)) = document.query_selector(".drawer-toggle") {
            let _ = toggle.set_attribute("aria-expanded", if open { "true" } else { "false" });
        }
    }

    /// Esc fecha a gaveta (o ouvinte vive enquanto a página viver).
    pub fn close_drawer_on_escape() {
        let handle = window_event_listener(leptos::ev::keydown, |e| {
            if e.key() == "Escape" {
                set_drawer(false);
            }
        });
        std::mem::forget(handle);
    }

    /// Recolhe ou expande a barra lateral e grava a escolha.
    pub fn set_sidebar_collapsed(collapsed: bool) {
        if let Some(root) = document().and_then(|d| d.document_element()) {
            let _ = root
                .class_list()
                .toggle_with_force("sidebar-collapsed", collapsed);
        }
        let storage = web_sys::window().and_then(|w| w.local_storage().ok().flatten());
        if let Some(storage) = storage {
            let _ = storage.set_item("sidebar", if collapsed { "collapsed" } else { "open" });
        }
    }
}

#[cfg(not(feature = "hydrate"))]
mod browser {
    pub fn set_drawer(_open: bool) {}
    pub fn close_drawer_on_escape() {}
    pub fn set_sidebar_collapsed(_collapsed: bool) {}
}
