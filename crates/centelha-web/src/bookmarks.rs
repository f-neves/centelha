//! O painel dos fixados ("📌 Consulta"): os termos que o leitor fixou no cartão
//! das referências (botão "fixar" ou tecla f), num painel no canto da tela,
//! de página em página.
//!
//! Port do painel de consulta de `src/components/Referencias.astro` do site
//! atual: a lista vem do `sessionStorage` (`centelha:consulta`, lista de
//! `tipo:id`, a mesma chave e o mesmo formato do site atual, R26), com filtro a
//! partir de quatro, "comparar" lado a lado a partir de dois (onde a página
//! tem o diálogo do termo), "limpar", o ✕ de cada um e o realce dos termos
//! fixados no texto. A ilha não tem propriedades: os dados dos termos vêm do
//! `<script type="application/json" id="ref-index">` da página (o glossário
//! inteiro, nome e resumo). Um item fixado no site atual que ainda não existe
//! aqui (uma Técnica, uma Arte) fica fora do painel até a fase 2, sem sair da
//! lista gravada.

use leptos::prelude::*;

/// O nome do evento que o cartão dispara ao fixar um termo.
pub const CHANGED: &str = "rs:consulta";

/// Os ouvintes do painel. Não desenha nada no gerador.
#[island]
pub fn Bookmarks() -> impl IntoView {
    Effect::new(move |_| browser::start());
}

#[cfg(feature = "hydrate")]
mod browser {
    use std::rc::Rc;

    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;
    use web_sys::{Document, Element, HtmlDialogElement, HtmlElement, HtmlInputElement};

    const KEY: &str = "centelha:consulta";

    struct Item {
        key: String,
        kind: String,
        name: String,
        summary: String,
        url: String,
    }

    fn field(v: &wasm_bindgen::JsValue, name: &str) -> String {
        js_sys::Reflect::get(v, &name.into())
            .ok()
            .and_then(|x| x.as_string())
            .unwrap_or_default()
    }

    fn index(document: &Document) -> Vec<Item> {
        let Some(data) = document.get_element_by_id("ref-index") else {
            return Vec::new();
        };
        let Ok(value) = js_sys::JSON::parse(&data.text_content().unwrap_or_default()) else {
            return Vec::new();
        };
        let Ok(list) = value.dyn_into::<js_sys::Array>() else {
            return Vec::new();
        };
        list.iter()
            .map(|v| {
                let kind = field(&v, "kind");
                let id = field(&v, "id");
                Item {
                    key: format!("{kind}:{id}"),
                    kind,
                    name: field(&v, "name"),
                    summary: field(&v, "summary"),
                    url: field(&v, "url"),
                }
            })
            .collect()
    }

    fn pins() -> Vec<String> {
        let storage = web_sys::window().and_then(|w| w.session_storage().ok().flatten());
        storage
            .and_then(|s| s.get_item(KEY).ok().flatten())
            .and_then(|t| js_sys::JSON::parse(&t).ok())
            .and_then(|v| v.dyn_into::<js_sys::Array>().ok())
            .map(|a| a.iter().filter_map(|x| x.as_string()).collect())
            .unwrap_or_default()
    }

    fn save(list: &[String]) {
        let Some(storage) = web_sys::window().and_then(|w| w.session_storage().ok().flatten())
        else {
            return;
        };
        let array = js_sys::Array::new();
        let mut seen = std::collections::HashSet::new();
        for k in list {
            if seen.insert(k.clone()) {
                array.push(&k.into());
            }
        }
        if let Ok(text) = js_sys::JSON::stringify(&array) {
            let _ = storage.set_item(KEY, &String::from(text));
        }
    }

    fn escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }

    fn capitalize(s: &str) -> String {
        let mut c = s.chars();
        match c.next() {
            Some(f) => f.to_uppercase().chain(c).collect(),
            None => String::new(),
        }
    }

    pub fn start() {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let items = Rc::new(index(&document));
        render(&document, &items);
        let doc = document.clone();
        let it = items.clone();
        let changed = Closure::<dyn FnMut()>::new(move || render(&doc, &it));
        if let Some(w) = web_sys::window() {
            let _ = w
                .add_event_listener_with_callback(super::CHANGED, changed.as_ref().unchecked_ref());
        }
        changed.forget();
        // O "comparar" alarga o diálogo; ao fechar, ele volta ao normal.
        if let Some(dialog) = document.query_selector(".ref-modal").ok().flatten() {
            let d = dialog.clone();
            let on_close = Closure::<dyn FnMut()>::new(move || {
                let _ = d.class_list().remove_1("wide");
            });
            let _ =
                dialog.add_event_listener_with_callback("close", on_close.as_ref().unchecked_ref());
            on_close.forget();
        }
    }

    fn render(document: &Document, items: &Rc<Vec<Item>>) {
        let keys = pins();
        let list: Vec<&Item> = keys
            .iter()
            .filter_map(|k| items.iter().find(|i| &i.key == k))
            .collect();
        mark_pinned(document, &keys);
        let existing = document.get_element_by_id("consulta");
        if list.is_empty() {
            if let Some(panel) = existing {
                panel.remove();
            }
            return;
        }
        let panel = match existing {
            Some(p) => p,
            None => {
                let Ok(p) = document.create_element("aside") else {
                    return;
                };
                p.set_id("consulta");
                let _ = p.set_attribute("aria-label", "Consulta fixada");
                if let Some(body) = document.body() {
                    let _ = body.append_child(&p);
                }
                p
            }
        };
        let can_compare = list.len() >= 2
            && document
                .query_selector(".ref-modal")
                .ok()
                .flatten()
                .is_some();
        let mut html = format!(
            "<div class=\"cs-head\"><span>📌 Consulta <span class=\"cs-n\">{}</span></span>{}<button class=\"cs-clear\" type=\"button\" title=\"Limpar\">limpar</button></div>",
            list.len(),
            if can_compare {
                "<button class=\"cs-comparar\" type=\"button\" title=\"Comparar lado a lado\">comparar</button>"
            } else {
                ""
            }
        );
        if list.len() >= 4 {
            html.push_str("<input class=\"cs-filter\" type=\"search\" placeholder=\"filtrar fixados…\" aria-label=\"Filtrar fixados\">");
        }
        for it in &list {
            html.push_str(&format!(
                "<div class=\"cs-card\" data-txt=\"{}\"><span class=\"ref-tipo\">{}</span><a href=\"{}\"><b>{}</b></a><button class=\"cs-x\" type=\"button\" aria-label=\"Remover\" data-k=\"{}\">✕</button><span class=\"cs-resumo\">{}</span></div>",
                escape(&format!("{} {} {}", it.name, it.kind, it.summary).to_lowercase()),
                escape(&capitalize(&it.kind)),
                escape(&it.url),
                escape(&it.name),
                escape(&it.key),
                escape(&it.summary),
            ));
        }
        panel.set_inner_html(&html);
        wire(document, &panel, items);
    }

    fn wire(document: &Document, panel: &Element, items: &Rc<Vec<Item>>) {
        let on = |selector: &str, event: &str, f: Box<dyn FnMut(web_sys::Event)>| {
            if let Ok(list) = panel.query_selector_all(selector) {
                let f = Closure::<dyn FnMut(web_sys::Event)>::new(f);
                for i in 0..list.length() {
                    if let Some(el) = list.item(i) {
                        let _ =
                            el.add_event_listener_with_callback(event, f.as_ref().unchecked_ref());
                    }
                }
                f.forget();
            }
        };
        let (doc, it) = (document.clone(), items.clone());
        on(
            ".cs-clear",
            "click",
            Box::new(move |_| {
                save(&[]);
                render(&doc, &it);
            }),
        );
        let (doc, it) = (document.clone(), items.clone());
        on(
            ".cs-x",
            "click",
            Box::new(move |e| {
                let key = e
                    .current_target()
                    .and_then(|t| t.dyn_into::<Element>().ok())
                    .and_then(|el| el.get_attribute("data-k"))
                    .unwrap_or_default();
                let left: Vec<String> = pins().into_iter().filter(|k| *k != key).collect();
                save(&left);
                render(&doc, &it);
            }),
        );
        let p = panel.clone();
        on(
            ".cs-filter",
            "input",
            Box::new(move |e| {
                let q = e
                    .target()
                    .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
                    .map(|i| i.value().to_lowercase())
                    .unwrap_or_default();
                if let Ok(cards) = p.query_selector_all(".cs-card") {
                    for i in 0..cards.length() {
                        if let Some(card) =
                            cards.item(i).and_then(|n| n.dyn_into::<HtmlElement>().ok())
                        {
                            let txt = card.get_attribute("data-txt").unwrap_or_default();
                            let show = q.is_empty() || txt.contains(&q);
                            let _ = card
                                .style()
                                .set_property("display", if show { "" } else { "none" });
                        }
                    }
                }
            }),
        );
        let (doc, it) = (document.clone(), items.clone());
        on(
            ".cs-comparar",
            "click",
            Box::new(move |_| compare(&doc, &it)),
        );
    }

    /// Os fixados lado a lado, no diálogo do termo.
    fn compare(document: &Document, items: &Rc<Vec<Item>>) {
        let Some(dialog) = document
            .query_selector(".ref-modal")
            .ok()
            .flatten()
            .and_then(|d| d.dyn_into::<HtmlDialogElement>().ok())
        else {
            return;
        };
        let keys = pins();
        let mut html = String::from("<div class=\"rm-compare\">");
        for it in keys
            .iter()
            .filter_map(|k| items.iter().find(|i| &i.key == k))
        {
            html.push_str(&format!(
                "<div class=\"rm-col\"><span class=\"ref-tipo\">{}</span><b class=\"rm-cnome\">{}</b><p class=\"rm-cdesc\">{}</p><a class=\"rm-go\" href=\"{}\">abrir →</a></div>",
                escape(&capitalize(&it.kind)),
                escape(&it.name),
                escape(&it.summary),
                escape(&it.url),
            ));
        }
        html.push_str("</div>");
        if let Ok(Some(body)) = dialog.query_selector(".ref-modal-body") {
            body.set_inner_html(&html);
        }
        let _ = dialog.class_list().add_1("wide");
        if !dialog.open() {
            let _ = dialog.show_modal();
        }
    }

    /// Realça no texto os termos fixados.
    fn mark_pinned(document: &Document, keys: &[String]) {
        let Ok(refs) = document.query_selector_all("main .ref[data-ref]") else {
            return;
        };
        for i in 0..refs.length() {
            if let Some(el) = refs.item(i).and_then(|n| n.dyn_into::<Element>().ok()) {
                let key = format!("termo:{}", el.get_attribute("data-ref").unwrap_or_default());
                let _ = el
                    .class_list()
                    .toggle_with_force("ref-pinned", keys.contains(&key));
            }
        }
    }
}

#[cfg(not(feature = "hydrate"))]
mod browser {
    pub fn start() {}
}
