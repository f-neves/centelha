//! O botão de voltar ao topo: aparece depois de duas telas de rolagem, no
//! canto oposto ao "Nesta página", e rola até o alto apagando a âncora do
//! endereço.
//!
//! Port de `src/components/VoltarAoTopo.astro` do site atual, que o usa só nas
//! três páginas das Artes. O botão sai escondido do gerador (sem JavaScript
//! ele não faria nada); a ilha o mostra e o esconde conforme a rolagem.

use leptos::prelude::*;

/// O botão, escondido até a rolagem passar de duas telas.
#[island]
pub fn BackToTop() -> impl IntoView {
    Effect::new(move |_| browser::start());
    view! {
        <button
            class="topo-btn"
            id="topo-btn"
            type="button"
            hidden
            aria-label="Voltar ao topo da página"
            title="Voltar ao topo"
            on:click=move |_| browser::to_top()
        >
            "↑ Topo"
        </button>
    }
}

#[cfg(feature = "hydrate")]
mod browser {
    use leptos::prelude::window_event_listener;
    use wasm_bindgen::JsCast;
    use web_sys::HtmlElement;

    fn update() {
        let Some(window) = web_sys::window() else {
            return;
        };
        let height = window
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let y = window.scroll_y().unwrap_or(0.0);
        let button = window
            .document()
            .and_then(|d| d.get_element_by_id("topo-btn"))
            .and_then(|b| b.dyn_into::<HtmlElement>().ok());
        if let Some(button) = button {
            button.set_hidden(y < height * 2.0);
        }
    }

    pub fn start() {
        update();
        std::mem::forget(window_event_listener(leptos::ev::scroll, |_| update()));
    }

    /// Rola até o alto e tira a âncora do endereço.
    pub fn to_top() {
        let Some(window) = web_sys::window() else {
            return;
        };
        let options = web_sys::ScrollToOptions::new();
        options.set_top(0.0);
        options.set_behavior(web_sys::ScrollBehavior::Smooth);
        window.scroll_to_with_scroll_to_options(&options);
        let location = window.location();
        let url = format!(
            "{}{}",
            location.pathname().unwrap_or_default(),
            location.search().unwrap_or_default()
        );
        if let Ok(history) = window.history() {
            let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&url));
        }
    }
}

#[cfg(not(feature = "hydrate"))]
mod browser {
    pub fn start() {}
    pub fn to_top() {}
}
