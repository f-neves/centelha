//! O filtro da página de Equipamentos: busca por texto, caixas de classe e de
//! tag. Esconde as linhas de arma que não passam, some com a tabela que fica
//! vazia (e com o título dela) e atualiza o contador.
//!
//! Port do script de `src/pages/equipamentos.astro` do site atual. A ilha não
//! tem propriedades: lê os campos (`[data-ef]`) e as linhas
//! (`tr[data-arma]`, com `data-classe`, `data-tags` e `data-txt`) do próprio
//! documento.

use leptos::prelude::*;

/// Liga o filtro. Não desenha nada.
#[island]
pub fn EquipmentFilter() -> impl IntoView {
    Effect::new(move |_| browser::start());
}

#[cfg(feature = "hydrate")]
mod browser {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;
    use web_sys::{Document, HtmlElement, HtmlInputElement};

    pub fn start() {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let Ok(fields) = document.query_selector_all("[data-ef]") else {
            return;
        };
        for i in 0..fields.length() {
            let Some(field) = fields.item(i) else {
                continue;
            };
            let doc = document.clone();
            let on_input = Closure::<dyn FnMut()>::new(move || apply(&doc));
            let _ =
                field.add_event_listener_with_callback("input", on_input.as_ref().unchecked_ref());
            on_input.forget();
        }
        apply(&document);
    }

    fn all<T: JsCast>(document: &Document, selector: &str) -> Vec<T> {
        let Ok(list) = document.query_selector_all(selector) else {
            return Vec::new();
        };
        (0..list.length())
            .filter_map(|i| list.item(i))
            .filter_map(|n| n.dyn_into::<T>().ok())
            .collect()
    }

    fn checked(document: &Document, kind: &str) -> Vec<String> {
        all::<HtmlInputElement>(document, &format!("[data-ef=\"{kind}\"]:checked"))
            .iter()
            .map(HtmlInputElement::value)
            .collect()
    }

    fn apply(document: &Document) {
        let text = document
            .query_selector("[data-ef=\"q\"]")
            .ok()
            .flatten()
            .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
            .map(|i| i.value().trim().to_lowercase())
            .unwrap_or_default();
        let classes = checked(document, "classe");
        let tags = checked(document, "tag");
        let mut count = 0;
        for row in all::<HtmlElement>(document, "tr[data-arma]") {
            let data = row.dataset();
            let class = data.get("classe").unwrap_or_default();
            let row_tags = data.get("tags").unwrap_or_default();
            let row_text = data.get("txt").unwrap_or_default();
            let ok = (classes.is_empty() || classes.contains(&class))
                && (tags.is_empty() || tags.iter().any(|t| row_tags.split(' ').any(|r| r == t)))
                && (text.is_empty() || row_text.contains(&text));
            let _ = row
                .style()
                .set_property("display", if ok { "" } else { "none" });
            if ok {
                count += 1;
            }
        }
        // Filtrar por classe esvazia uma das tabelas: some com ela e com o
        // título, para não sobrar um cabeçalho de uma lista que não existe.
        for block in all::<HtmlElement>(document, "[data-bloco]") {
            let alive = block
                .query_selector_all("tr[data-arma]")
                .ok()
                .map(|rows| {
                    (0..rows.length())
                        .filter_map(|i| rows.item(i))
                        .filter_map(|n| n.dyn_into::<HtmlElement>().ok())
                        .any(|r| {
                            r.style().get_property_value("display").unwrap_or_default() != "none"
                        })
                })
                .unwrap_or(false);
            block.set_hidden(!alive);
        }
        if let Ok(Some(counter)) = document.query_selector(".ef-count") {
            let plural = if count == 1 { "" } else { "s" };
            counter.set_text_content(Some(&format!("{count} arma{plural}")));
        }
    }
}

#[cfg(not(feature = "hydrate"))]
mod browser {
    pub fn start() {}
}
