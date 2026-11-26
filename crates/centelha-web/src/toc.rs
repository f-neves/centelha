//! "Nesta página": o sumário flutuante dos títulos da página (h2 e h3), com a
//! seção visível em realce.
//!
//! Port de `src/components/NestaPagina.astro` do site atual, com uma troca: lá
//! o botão e a lista são montados por JavaScript a partir dos títulos; aqui o
//! gerador já desenha a lista (os títulos e as âncoras saem do Markdown) num
//! `<details>`, que abre e fecha sem JavaScript. A ilha, sem propriedades,
//! só acrescenta o que o site atual faz além disso: fechar ao escolher um
//! título, com Esc ou com um clique fora; esconder o botão ao rolar para baixo
//! e mostrar ao rolar para cima; e marcar o título da seção que está no alto
//! da tela. Ela lê a lista do próprio documento: com os títulos nas
//! propriedades, o WebAssembly levaria o leitor de JSON e quase dobraria de
//! tamanho.

use leptos::prelude::*;

/// Um título da página.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocEntry {
    /// A âncora (`id` do título).
    pub id: String,
    /// 2 ou 3.
    pub level: u8,
    pub text: String,
}

/// O sumário (HTML do gerador) com a ilha que o completa no navegador. A
/// casca só o põe na página quando há dois títulos ou mais, como no site
/// atual.
#[component]
pub fn OnThisPage(entries: Vec<TocEntry>) -> impl IntoView {
    let links = entries
        .into_iter()
        .map(|e| {
            let class = if e.level == 3 { "otp-h3" } else { "otp-h2" };
            view! {
                <a href=format!("#{}", e.id) data-id=e.id.clone() class=class>
                    {e.text}
                </a>
            }
        })
        .collect_view();
    view! {
        <details class="otp" id="otp">
            <summary class="btn" id="otp-fab">"❡ Nesta página"</summary>
            <nav id="otp-panel" aria-label="Nesta página">
                {links}
            </nav>
        </details>
        <OnThisPageControls/>
    }
}

/// Os ouvintes do sumário. Não desenha nada.
#[island]
pub fn OnThisPageControls() -> impl IntoView {
    Effect::new(move |_| browser::enhance());
}

#[cfg(feature = "hydrate")]
mod browser {
    use std::cell::Cell;
    use std::rc::Rc;

    use leptos::prelude::{window_event_listener, window_event_listener_untyped};
    use wasm_bindgen::JsCast;
    use web_sys::{Element, HtmlDetailsElement, HtmlElement};

    fn by_id<T: JsCast>(id: &str) -> Option<T> {
        web_sys::window()?
            .document()?
            .get_element_by_id(id)?
            .dyn_into::<T>()
            .ok()
    }

    /// Liga os ouvintes. Rodam enquanto a página viver.
    pub fn enhance() {
        let (Some(details), Some(fab), Some(panel)) = (
            by_id::<HtmlDetailsElement>("otp"),
            by_id::<HtmlElement>("otp-fab"),
            by_id::<HtmlElement>("otp-panel"),
        ) else {
            return;
        };
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };

        // Os títulos da página, na ordem do sumário.
        let mut heads: Vec<(Element, Element)> = Vec::new();
        if let Ok(links) = panel.query_selector_all("a[data-id]") {
            for i in 0..links.length() {
                let Some(link) = links.item(i).and_then(|n| n.dyn_into::<Element>().ok()) else {
                    continue;
                };
                let id = link.get_attribute("data-id").unwrap_or_default();
                if let Some(head) = document.get_element_by_id(&id) {
                    heads.push((head, link));
                }
            }
        }

        // Escolher um título fecha o sumário.
        {
            let details = details.clone();
            let panel_clicks = window_event_listener(leptos::ev::click, move |e| {
                let target = e.target().and_then(|t| t.dyn_into::<Element>().ok());
                let Some(target) = target else { return };
                let inside = details.contains(Some(&target));
                let on_link = target.closest("#otp-panel a").ok().flatten().is_some();
                // Um clique num título, ou fora do sumário, fecha.
                if on_link || !inside {
                    details.set_open(false);
                }
            });
            std::mem::forget(panel_clicks);
        }

        // Esc fecha.
        {
            let details = details.clone();
            let keys = window_event_listener(leptos::ev::keydown, move |e| {
                if e.key() == "Escape" {
                    details.set_open(false);
                }
            });
            std::mem::forget(keys);
        }

        // Ao rolar: some ao descer, volta ao subir (e nas pontas da página), e
        // marca a seção do alto da tela.
        let last_y = Rc::new(Cell::new(scroll_y()));
        let update = move || {
            let y = scroll_y();
            let previous = last_y.get();
            let (height, page_height) = sizes();
            let at_edge = y < 80.0 || y + height > page_height - 80.0;
            if !details.open() {
                let classes = fab.class_list();
                if y > previous + 4.0 && !at_edge {
                    let _ = classes.add_1("otp-oculto");
                } else if y < previous - 4.0 || at_edge {
                    let _ = classes.remove_1("otp-oculto");
                }
            }
            last_y.set(y);
            mark_active(&heads, height * 0.25);
        };
        update();
        let scrolls = window_event_listener(leptos::ev::scroll, move |_| update());
        std::mem::forget(scrolls);

        hold_anchor();
    }

    /// O corpo do capítulo faz o layout preguiçoso (`content-visibility`), e
    /// as fontes chegam depois da primeira rolagem: os blocos acima do alvo
    /// de um `#âncora` mudam de altura depois que o navegador rolou até ele,
    /// e o alvo sai do lugar. Por isso, ao abrir a página com âncora e a cada
    /// troca de âncora, rola de novo até o alvo algumas vezes no primeiro
    /// segundo e meio, a menos que o leitor mexa na página nesse meio-tempo.
    fn hold_anchor() {
        let touched = Rc::new(Cell::new(false));
        for event in ["wheel", "touchstart", "keydown", "mousedown"] {
            let touched = touched.clone();
            let handle = window_event_listener_untyped(event, move |_| touched.set(true));
            std::mem::forget(handle);
        }
        let schedule = {
            let touched = touched.clone();
            move || {
                for delay in [0, 150, 400, 800, 1500] {
                    let touched = touched.clone();
                    let again = wasm_bindgen::closure::Closure::once_into_js(move || {
                        if !touched.get() {
                            scroll_to_hash();
                        }
                    });
                    if let Some(window) = web_sys::window() {
                        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                            again.unchecked_ref(),
                            delay,
                        );
                    }
                }
            }
        };
        schedule();
        let changes = window_event_listener(leptos::ev::hashchange, move |_| {
            touched.set(false);
            schedule();
        });
        std::mem::forget(changes);
    }

    /// Põe no alto da tela o elemento da âncora do endereço, se houver.
    fn scroll_to_hash() {
        let Some(window) = web_sys::window() else {
            return;
        };
        let hash = window.location().hash().unwrap_or_default();
        let id = percent_decode(hash.trim_start_matches('#'));
        if id.is_empty() {
            return;
        }
        let target = window.document().and_then(|d| d.get_element_by_id(&id));
        if let Some(target) = target {
            let options = web_sys::ScrollIntoViewOptions::new();
            options.set_behavior(web_sys::ScrollBehavior::Instant);
            target.scroll_into_view_with_scroll_into_view_options(&options);
        }
    }

    /// Desfaz o `%XX` da âncora (os `id` têm letras acentuadas).
    fn percent_decode(text: &str) -> String {
        let bytes = text.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            let hex = |b: u8| (b as char).to_digit(16);
            if bytes[i] == b'%'
                && i + 2 < bytes.len()
                && let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2]))
            {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
            out.push(bytes[i]);
            i += 1;
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    fn scroll_y() -> f64 {
        web_sys::window()
            .and_then(|w| w.scroll_y().ok())
            .unwrap_or(0.0)
    }

    /// A altura da janela e a da página.
    fn sizes() -> (f64, f64) {
        let window = web_sys::window();
        let height = window
            .as_ref()
            .and_then(|w| w.inner_height().ok())
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let page = window
            .and_then(|w| w.document())
            .and_then(|d| d.body())
            .map(|b| f64::from(b.scroll_height()))
            .unwrap_or(0.0);
        (height, page)
    }

    /// Realça o último título que já passou do primeiro quarto da tela.
    fn mark_active(heads: &[(Element, Element)], line: f64) {
        let active = heads
            .iter()
            .rposition(|(head, _)| head.get_bounding_client_rect().top() <= line);
        for (i, (_, link)) in heads.iter().enumerate() {
            let _ = link
                .class_list()
                .toggle_with_force("active", Some(i) == active);
        }
    }
}

#[cfg(not(feature = "hydrate"))]
mod browser {
    pub fn enhance() {}
}
