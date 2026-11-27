//! O filtro do catálogo das Artes: busca por texto e chips por Arte. Esconde
//! os cartões que não passam e a seção que fica vazia; o título de cada grupo
//! marca ou desmarca o grupo inteiro, e "Limpar" volta tudo.
//!
//! Port do script de `src/pages/artes/catalogo.astro` do site atual. A ilha
//! não tem propriedades: lê os campos (`[data-fa]`) e os cartões
//! (`.arte[data-cat]`, com `id` e `data-txt`) do próprio documento.

use leptos::prelude::*;

/// Liga o filtro. Não desenha nada.
#[island]
pub fn ArtFilter() -> impl IntoView {
    Effect::new(move |_| browser::start());
}

#[cfg(feature = "hydrate")]
mod browser {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;
    use web_sys::{Document, Element, HtmlElement, HtmlInputElement, NodeList};

    fn collect<T: JsCast>(list: Result<NodeList, wasm_bindgen::JsValue>) -> Vec<T> {
        let Ok(list) = list else {
            return Vec::new();
        };
        (0..list.length())
            .filter_map(|i| list.item(i))
            .filter_map(|n| n.dyn_into::<T>().ok())
            .collect()
    }

    fn listen(target: &web_sys::EventTarget, event: &str, f: impl FnMut() + 'static) {
        let closure = Closure::<dyn FnMut()>::new(f);
        let _ = target.add_event_listener_with_callback(event, closure.as_ref().unchecked_ref());
        closure.forget();
    }

    fn search(document: &Document) -> Option<HtmlInputElement> {
        document
            .query_selector("[data-fa=\"q\"]")
            .ok()
            .flatten()
            .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
    }

    fn boxes(document: &Document) -> Vec<HtmlInputElement> {
        collect(document.query_selector_all("[data-fa=\"arte\"]"))
    }

    pub fn start() {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        for field in collect::<Element>(document.query_selector_all("[data-fa]")) {
            let doc = document.clone();
            listen(&field, "input", move || apply(&doc));
        }
        // O título do grupo marca ou desmarca o grupo inteiro.
        for title in collect::<Element>(document.query_selector_all(".f-tit")) {
            let doc = document.clone();
            let t = title.clone();
            listen(&title, "click", move || {
                let Some(group) = t.parent_element() else {
                    return;
                };
                let list: Vec<HtmlInputElement> =
                    collect(group.query_selector_all("[data-fa=\"arte\"]"));
                let mark = list.iter().any(|c| !c.checked());
                for c in &list {
                    c.set_checked(mark);
                }
                apply(&doc);
            });
        }
        if let Some(clear) = document.get_element_by_id("limpar-filtros") {
            let doc = document.clone();
            listen(&clear, "click", move || {
                for c in boxes(&doc) {
                    c.set_checked(false);
                }
                if let Some(q) = search(&doc) {
                    q.set_value("");
                }
                apply(&doc);
            });
        }
        apply(&document);
    }

    fn shown(e: &HtmlElement) -> bool {
        e.style().get_property_value("display").unwrap_or_default() != "none"
    }

    fn apply(document: &Document) {
        let text = search(document)
            .map(|i| i.value().trim().to_lowercase())
            .unwrap_or_default();
        let all_boxes = boxes(document);
        let ids: Vec<String> = all_boxes
            .iter()
            .filter(|c| c.checked())
            .map(|c| c.value())
            .collect();
        for card in collect::<HtmlElement>(document.query_selector_all(".arte[data-cat]")) {
            let txt = card.dataset().get("txt").unwrap_or_default();
            let ok = (ids.is_empty() || ids.contains(&card.id()))
                && (text.is_empty() || txt.contains(&text));
            let _ = card
                .style()
                .set_property("display", if ok { "" } else { "none" });
        }
        for section in collect::<HtmlElement>(document.query_selector_all(".arte-sec")) {
            let alive = collect::<HtmlElement>(section.query_selector_all(".arte[data-cat]"))
                .iter()
                .any(shown);
            let _ = section
                .style()
                .set_property("display", if alive { "" } else { "none" });
        }
        for c in &all_boxes {
            if let Some(chip) = c.closest(".f-chip").ok().flatten() {
                let _ = chip.class_list().toggle_with_force("on", c.checked());
            }
        }
    }
}

#[cfg(not(feature = "hydrate"))]
mod browser {
    pub fn start() {}
}
