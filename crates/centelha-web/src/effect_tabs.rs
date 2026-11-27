//! As abas da página dos Efeitos Especiais: um grupo de Artes por vez
//! (elementais ou universais), os blocos de cada Arte abertos só na tela
//! larga, e a âncora de um Efeito (`#ef-<arte>-<efeito>`) abre a aba e o bloco
//! certos antes de rolar até ele.
//!
//! Port do script de `src/pages/artes/efeitos.astro` do site atual. Sem a
//! ilha, a página fica inteira: os dois grupos à mostra, com o título de
//! cada um. A ilha não tem propriedades: lê as abas (`.ef-aba[data-aba]`),
//! os grupos (`.ef-grupo[data-cat]`), as pílulas do índice (`.ef-indice a`)
//! e os blocos (`details.ef-bloco`) do próprio documento. O "aberto na tela
//! larga" depende da tela, então entra depois de montar, como o resto: o
//! gerador desenha os blocos fechados, como o site atual sem script.

use leptos::prelude::*;

/// Liga as abas. Não desenha nada.
#[island]
pub fn EffectTabs() -> impl IntoView {
    Effect::new(move |_| browser::start());
}

#[cfg(feature = "hydrate")]
mod browser {
    use std::rc::Rc;

    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;
    use web_sys::{Document, Element, HtmlDetailsElement, HtmlElement, NodeList};

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

    struct Page {
        tabs: Vec<Element>,
        groups: Vec<HtmlElement>,
        pills: Vec<Element>,
    }

    impl Page {
        fn switch(&self, cat: &str) {
            for t in &self.tabs {
                let on = t.get_attribute("data-aba").as_deref() == Some(cat);
                let _ = t.set_attribute("aria-selected", if on { "true" } else { "false" });
            }
            for g in &self.groups {
                g.set_hidden(g.dataset().get("cat").as_deref() != Some(cat));
            }
            for p in &self.pills {
                let _ = p
                    .class_list()
                    .toggle_with_force("fora", !p.class_list().contains(cat));
            }
        }
    }

    fn open_block(el: Option<Element>) {
        if let Some(block) = el
            .and_then(|e| e.closest("details.ef-bloco").ok().flatten())
            .and_then(|b| b.dyn_into::<HtmlDetailsElement>().ok())
        {
            block.set_open(true);
        }
    }

    /// O alvo da âncora do endereço, se houver.
    fn hash_target(document: &Document) -> Option<Element> {
        let hash = web_sys::window()?.location().hash().ok()?;
        let id = hash.strip_prefix('#')?;
        if id.is_empty() {
            return None;
        }
        let id = js_sys::decode_uri_component(id)
            .ok()
            .and_then(|s| s.as_string())
            .unwrap_or_else(|| id.to_string());
        document.get_element_by_id(&id)
    }

    fn open_from_hash(document: &Document, page: &Page) {
        let Some(target) = hash_target(document) else {
            return;
        };
        let Some(group) = target.closest(".ef-grupo").ok().flatten() else {
            return;
        };
        let Some(cat) = group.get_attribute("data-cat") else {
            return;
        };
        page.switch(&cat);
        open_block(Some(target.clone()));
        target.scroll_into_view_with_bool(true);
    }

    pub fn start() {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        let page = Rc::new(Page {
            tabs: collect(document.query_selector_all(".ef-aba")),
            groups: collect(document.query_selector_all(".ef-grupo")),
            pills: collect(document.query_selector_all(".ef-indice a")),
        });
        if page.tabs.is_empty() || page.groups.is_empty() {
            return;
        }
        // Com aba, o título e a chamada do grupo viram repetição.
        for e in collect::<HtmlElement>(document.query_selector_all(".ef-grupo-tit, .ef-grupo-sub"))
        {
            e.set_hidden(true);
        }
        for t in &page.tabs {
            let p = page.clone();
            let cat = t.get_attribute("data-aba").unwrap_or_default();
            listen(t, "click", move || p.switch(&cat));
        }
        // Os blocos de cada Arte: fechados no celular, abertos na tela larga.
        let wide = window
            .match_media("(min-width: 901px)")
            .ok()
            .flatten()
            .is_some_and(|m| m.matches());
        for b in collect::<HtmlDetailsElement>(document.query_selector_all("details.ef-bloco")) {
            b.set_open(wide);
        }
        // A pílula de uma Arte do outro grupo troca de aba e abre o bloco.
        for pill in &page.pills {
            let p = page.clone();
            let doc = document.clone();
            let el = pill.clone();
            listen(pill, "click", move || {
                let universal = el.class_list().contains("universal");
                p.switch(if universal { "universal" } else { "elemental" });
                let id = el
                    .get_attribute("href")
                    .and_then(|h| h.strip_prefix('#').map(str::to_string));
                open_block(id.and_then(|id| doc.get_element_by_id(&id)));
            });
        }
        page.switch("elemental");
        open_from_hash(&document, &page);
        let doc = document.clone();
        let p = page.clone();
        listen(&window, "hashchange", move || open_from_hash(&doc, &p));
    }
}

#[cfg(not(feature = "hydrate"))]
mod browser {
    pub fn start() {}
}
