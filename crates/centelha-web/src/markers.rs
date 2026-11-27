//! Os marcadores (☆ e ★): os botões `[data-bm]` guardam no aparelho o que o
//! leitor marcou.
//!
//! Port de `src/components/Marcadores.astro` do site atual: a lista fica no
//! `localStorage`, chave `centelha:marcadores`, lista JSON de ids sem
//! repetição (a mesma chave e o mesmo formato, R26). O botão diz o estado em
//! `aria-pressed` e troca a estrela. Ao mudar, avisa o documento com o evento
//! `marcadores:change` (com a lista em `detail`), como lá.

use leptos::prelude::*;

/// Liga os marcadores da página. Não desenha nada.
#[island]
pub fn Markers() -> impl IntoView {
    Effect::new(move |_| browser::start());
}

#[cfg(feature = "hydrate")]
mod browser {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;
    use web_sys::{Document, Element};

    const KEY: &str = "centelha:marcadores";
    const EMPTY: &str = "\u{2606}";
    const FULL: &str = "\u{2605}";

    fn load() -> Vec<String> {
        let list: Vec<String> = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(KEY).ok().flatten())
            .and_then(|t| js_sys::JSON::parse(&t).ok())
            .and_then(|v| v.dyn_into::<js_sys::Array>().ok())
            .map(|a| a.iter().filter_map(|x| x.as_string()).collect())
            .unwrap_or_default();
        let mut seen = std::collections::HashSet::new();
        list.into_iter()
            .filter(|k| seen.insert(k.clone()))
            .collect()
    }

    fn to_array(list: &[String]) -> js_sys::Array {
        let array = js_sys::Array::new();
        for k in list {
            array.push(&k.into());
        }
        array
    }

    fn save(list: &[String]) {
        let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) else {
            return;
        };
        if let Ok(text) = js_sys::JSON::stringify(&to_array(list)) {
            let _ = storage.set_item(KEY, &String::from(text));
        }
    }

    fn reflect(document: &Document, list: &[String]) {
        let Ok(buttons) = document.query_selector_all("[data-bm]") else {
            return;
        };
        for i in 0..buttons.length() {
            let Some(b) = buttons.item(i).and_then(|n| n.dyn_into::<Element>().ok()) else {
                continue;
            };
            let id = b.get_attribute("data-bm").unwrap_or_default();
            let on = list.contains(&id);
            let _ = b.set_attribute("aria-pressed", if on { "true" } else { "false" });
            let text = b.text_content().unwrap_or_default();
            if text == EMPTY || text == FULL {
                b.set_text_content(Some(if on { FULL } else { EMPTY }));
            }
        }
    }

    pub fn start() {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let doc = document.clone();
        let on_click = Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| {
            let Some(button) = e
                .target()
                .and_then(|t| t.dyn_into::<Element>().ok())
                .and_then(|t| t.closest("[data-bm]").ok().flatten())
            else {
                return;
            };
            let id = button.get_attribute("data-bm").unwrap_or_default();
            // Relê a lista: outra aba pode ter mudado.
            let mut list = load();
            if let Some(i) = list.iter().position(|k| *k == id) {
                list.remove(i);
            } else {
                list.push(id);
            }
            save(&list);
            reflect(&doc, &list);
            let init = web_sys::CustomEventInit::new();
            init.set_detail(&to_array(&list));
            if let Ok(event) =
                web_sys::CustomEvent::new_with_event_init_dict("marcadores:change", &init)
            {
                let _ = doc.dispatch_event(&event);
            }
        });
        let _ =
            document.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref());
        on_click.forget();
        reflect(&document, &load());
    }
}

#[cfg(not(feature = "hydrate"))]
mod browser {
    pub fn start() {}
}
