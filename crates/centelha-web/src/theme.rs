//! O seletor de tema, a primeira ilha do site.
//!
//! O tema vale antes de a ilha existir: um script curto no `<head>` lê a chave
//! `tema` do `localStorage` e põe o valor em `<html data-theme>` antes da
//! primeira pintura (como no site atual). A ilha só troca e grava.

use leptos::prelude::*;

/// Um tema do site. O valor gravado (no `localStorage` e no `data-theme`)
/// continua o português do site atual.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Classic,
    Dark,
    Readable,
}

impl Theme {
    /// Os três temas, na ordem em que aparecem no seletor.
    pub const ALL: [Theme; 3] = [Theme::Classic, Theme::Dark, Theme::Readable];

    /// O tema de quem nunca escolheu.
    pub const DEFAULT: Theme = Theme::Dark;

    /// O valor gravado.
    pub fn key(self) -> &'static str {
        match self {
            Theme::Classic => "classico",
            Theme::Dark => "escuro",
            Theme::Readable => "legivel",
        }
    }

    /// O nome no botão.
    pub fn label(self) -> &'static str {
        match self {
            Theme::Classic => "Clássico",
            Theme::Dark => "Escuro",
            Theme::Readable => "Legível",
        }
    }

    /// O tema de um valor gravado; `None` para um valor que não é tema.
    pub fn from_key(key: &str) -> Option<Theme> {
        Theme::ALL.into_iter().find(|t| t.key() == key)
    }
}

/// Três botões, um por tema. O gerador desenha o padrão marcado; no
/// navegador a ilha lê o tema que o script do `<head>` já aplicou.
#[island]
pub fn ThemeSwitcher() -> impl IntoView {
    let current = RwSignal::new(Theme::DEFAULT);

    // Efeitos só rodam no navegador, depois da hidratação.
    Effect::new(move |_| {
        if let Some(theme) = browser::applied_theme() {
            current.set(theme);
        }
        browser::mark_hydrated();
    });

    let buttons = Theme::ALL
        .into_iter()
        .map(|theme| {
            view! {
                <button
                    type="button"
                    class="btn"
                    data-theme-key=theme.key()
                    aria-pressed=move || (current.get() == theme).to_string()
                    on:click=move |_| {
                        current.set(theme);
                        browser::apply_theme(theme);
                    }
                >
                    {theme.label()}
                </button>
            }
        })
        .collect_view();

    view! {
        <div class="theme-switcher" role="group" aria-label="Tema">
            {buttons}
        </div>
    }
}

/// O que a ilha faz no documento. Fora do navegador não faz nada.
#[cfg(feature = "hydrate")]
mod browser {
    use super::Theme;
    use wasm_bindgen::JsCast;
    use web_sys::HtmlElement;

    fn root() -> Option<HtmlElement> {
        web_sys::window()?
            .document()?
            .document_element()?
            .dyn_into::<HtmlElement>()
            .ok()
    }

    /// O tema que está em `<html data-theme>`.
    pub fn applied_theme() -> Option<Theme> {
        Theme::from_key(&root()?.dataset().get("theme")?)
    }

    /// Aplica o tema e grava a escolha (sem `localStorage`, só aplica).
    pub fn apply_theme(theme: Theme) {
        if let Some(root) = root() {
            let _ = root.dataset().set("theme", theme.key());
        }
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.set_item("tema", theme.key());
        }
    }

    /// Marca no documento que as ilhas hidrataram: `<html data-islands>` e a
    /// marca de desempenho `island-hydrated` (as provas e a medida de abertura
    /// leem as duas).
    pub fn mark_hydrated() {
        if let Some(root) = root() {
            let _ = root.dataset().set("islands", "hydrated");
        }
        if let Some(performance) = web_sys::window().and_then(|w| w.performance()) {
            let _ = performance.mark("island-hydrated");
        }
    }
}

#[cfg(not(feature = "hydrate"))]
mod browser {
    use super::Theme;

    pub fn applied_theme() -> Option<Theme> {
        None
    }

    pub fn apply_theme(_theme: Theme) {}

    pub fn mark_hydrated() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip() {
        for theme in Theme::ALL {
            assert_eq!(Theme::from_key(theme.key()), Some(theme));
        }
        assert_eq!(Theme::from_key("sepia"), None);
        assert_eq!(Theme::DEFAULT.key(), "escuro");
    }
}
