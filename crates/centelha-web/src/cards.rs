//! O cartão das referências do glossário: passar o mouse (ou focar pelo
//! teclado) num termo marcado abre um cartão com o tipo, o nome, o resumo,
//! "abrir →" e "📌 fixar"; o clique abre o detalhe num diálogo na própria
//! página.
//!
//! Port do cartão e do diálogo de `src/components/Referencias.astro` do site
//! atual. A ilha não tem propriedades: lê os termos da página no
//! `<script type="application/json" id="ref-data">` que o gerador grava (só os
//! termos que aparecem nela) e acha os termos marcados (`.ref`) no documento.
//! O "fixar" grava na mesma chave do site atual (`centelha:consulta`, no
//! `sessionStorage`); o painel dos fixados vem com os marcadores, na fase 2.

use leptos::prelude::*;

/// Os ouvintes do cartão. Não desenha nada.
#[island]
pub fn RefCards() -> impl IntoView {
    Effect::new(move |_| browser::start());
}

#[cfg(feature = "hydrate")]
mod browser {
    use std::cell::RefCell;
    use std::rc::Rc;

    use leptos::prelude::window_event_listener;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;
    use web_sys::{Document, Element, HtmlDialogElement, HtmlElement, Window};

    /// O que o cartão e o diálogo mostram de um termo.
    struct Item {
        id: String,
        kind: String,
        name: String,
        url: String,
        summary: String,
        levels: Vec<Level>,
    }

    /// Um degrau da escala (as chaves do JSON vêm de `regras.json`).
    struct Level {
        number: String,
        label: String,
        text: String,
        conduct: String,
    }

    #[derive(Default)]
    struct State {
        pop: Option<Element>,
        current: Option<usize>,
        hide_timer: Option<i32>,
    }

    struct Cards {
        window: Window,
        document: Document,
        items: Vec<Item>,
        state: RefCell<State>,
    }

    /// Onde o cartão abre: um termo marcado ou um link da página.
    const SELECTOR: &str = "main a[href], main .ref";
    /// O tempo até o cartão sumir depois que o mouse sai (como no site atual).
    const HIDE_DELAY_MS: i32 = 240;

    pub fn start() {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        let Some(data) = document.get_element_by_id("ref-data") else {
            return;
        };
        let items = parse(&data.text_content().unwrap_or_default());
        if items.is_empty() {
            return;
        }
        let cards = Rc::new(Cards {
            window,
            document,
            items,
            state: RefCell::new(State::default()),
        });
        listen(&cards);
    }

    fn listen(cards: &Rc<Cards>) {
        let c = cards.clone();
        forget(window_event_listener(leptos::ev::mouseover, move |e| {
            if let Some(el) = closest(e.target(), SELECTOR) {
                c.cancel_hide();
                if let Some(i) = c.item_for(&el) {
                    c.show(&el, i);
                }
            }
        }));
        let c = cards.clone();
        forget(window_event_listener(leptos::ev::mouseout, move |e| {
            if closest(e.target(), SELECTOR).is_some() {
                c.schedule_hide();
            }
        }));
        let c = cards.clone();
        forget(window_event_listener(leptos::ev::focusin, move |e| {
            if let Some(el) = closest(e.target(), "main .ref")
                && let Some(i) = c.item_for(&el)
            {
                c.cancel_hide();
                c.show(&el, i);
            }
        }));
        let c = cards.clone();
        forget(window_event_listener(leptos::ev::focusout, move |_| {
            c.schedule_hide();
        }));
        let c = cards.clone();
        forget(window_event_listener(leptos::ev::scroll, move |_| c.hide()));
        let c = cards.clone();
        forget(window_event_listener(leptos::ev::keydown, move |e| {
            if e.key() == "Escape" {
                c.hide();
                return;
            }
            let typing = c
                .document
                .active_element()
                .is_some_and(|a| matches!(a.tag_name().as_str(), "INPUT" | "TEXTAREA"));
            let current = c.state.borrow().current;
            let open = c.state.borrow().pop.is_some();
            if e.key().eq_ignore_ascii_case("f")
                && open
                && !typing
                && let Some(i) = current
            {
                e.prevent_default();
                c.pin(i);
                c.hide();
            }
        }));
        let c = cards.clone();
        forget(window_event_listener(leptos::ev::click, move |e| {
            let Some(target) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
                return;
            };
            // O botão de fixar e os links do próprio cartão seguem o caminho deles.
            if target.closest(".ref-pop").ok().flatten().is_some() {
                return;
            }
            if let Some(dialog) = c.dialog() {
                let on_close = target.closest(".ref-modal-x").ok().flatten().is_some();
                let on_backdrop = target.is_same_node(Some(dialog.as_ref()));
                if on_close || on_backdrop {
                    dialog.close();
                    return;
                }
            }
            if let Some(el) = closest(Some(target.into()), SELECTOR)
                && let Some(i) = c.item_for(&el)
            {
                e.prevent_default();
                c.hide();
                c.open_modal(i);
            }
        }));
    }

    impl Cards {
        /// O termo de um elemento: o `data-ref` do termo marcado, ou o `#id`
        /// de um link da página que aponta para um termo.
        fn item_for(&self, el: &Element) -> Option<usize> {
            let id = match el.get_attribute("data-ref") {
                Some(id) => id,
                None => {
                    let href = el.get_attribute("href")?;
                    href.split_once('#')?.1.to_string()
                }
            };
            self.items.iter().position(|it| it.id == id)
        }

        fn dialog(&self) -> Option<HtmlDialogElement> {
            self.document
                .query_selector(".ref-modal")
                .ok()
                .flatten()?
                .dyn_into()
                .ok()
        }

        fn cancel_hide(&self) {
            if let Some(timer) = self.state.borrow_mut().hide_timer.take() {
                self.window.clear_timeout_with_handle(timer);
            }
        }

        fn schedule_hide(self: &Rc<Self>) {
            self.cancel_hide();
            let me = self.clone();
            let callback = Closure::once_into_js(move || me.hide());
            let timer = self
                .window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    callback.unchecked_ref(),
                    HIDE_DELAY_MS,
                )
                .ok();
            self.state.borrow_mut().hide_timer = timer;
        }

        fn hide(&self) {
            let mut state = self.state.borrow_mut();
            if let Some(pop) = state.pop.take() {
                pop.remove();
            }
            state.current = None;
        }

        fn show(self: &Rc<Self>, el: &Element, i: usize) {
            self.hide();
            let item = &self.items[i];
            let Ok(pop) = self.document.create_element("div") else {
                return;
            };
            pop.set_class_name("ref-pop");
            let html = format!(
                "<span class=\"ref-tipo\">{}</span><b>{}</b><span class=\"ref-resumo\">{}</span>\
                 <span class=\"ref-bar\"><a class=\"ref-go\" href=\"{}\">abrir →</a>\
                 <button class=\"ref-pin\" type=\"button\" title=\"Fixar na consulta (tecla f)\">📌 fixar <kbd>f</kbd></button></span>",
                escape(&capitalize(&item.kind)),
                escape(&item.name),
                escape(&item.summary),
                escape(&item.url),
            );
            pop.set_inner_html(&html);
            if let Ok(Some(pin)) = pop.query_selector(".ref-pin") {
                let me = self.clone();
                let on_pin = Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| {
                    e.prevent_default();
                    me.pin(i);
                    me.hide();
                });
                let _ =
                    pin.add_event_listener_with_callback("click", on_pin.as_ref().unchecked_ref());
                on_pin.forget();
            }
            let me = self.clone();
            let enter = Closure::<dyn FnMut()>::new(move || me.cancel_hide());
            let _ =
                pop.add_event_listener_with_callback("mouseenter", enter.as_ref().unchecked_ref());
            enter.forget();
            let me = self.clone();
            let leave = Closure::<dyn FnMut()>::new(move || me.schedule_hide());
            let _ =
                pop.add_event_listener_with_callback("mouseleave", leave.as_ref().unchecked_ref());
            leave.forget();
            let Some(body) = self.document.body() else {
                return;
            };
            let _ = body.append_child(&pop);
            // Abaixo do termo, sem passar da borda direita da tela.
            let rect = el.get_bounding_client_rect();
            let sx = self.window.scroll_x().unwrap_or(0.0);
            let sy = self.window.scroll_y().unwrap_or(0.0);
            let width = self
                .document
                .document_element()
                .map(|d| f64::from(d.client_width()))
                .unwrap_or(0.0);
            let pop_width = pop
                .dyn_ref::<HtmlElement>()
                .map(|p| f64::from(p.offset_width()))
                .unwrap_or(0.0);
            let left = (sx + rect.left())
                .min(sx + width - pop_width - 10.0)
                .max(8.0);
            if let Some(p) = pop.dyn_ref::<HtmlElement>() {
                let style = p.style();
                // Em pixels inteiros: formatar `f64` traria para o WebAssembly
                // o código de impressão de números com fração.
                let top = (sy + rect.bottom() + 6.0).round() as i64;
                let left = left.round() as i64;
                let _ = style.set_property("top", &format!("{top}px"));
                let _ = style.set_property("left", &format!("{left}px"));
            }
            let mut state = self.state.borrow_mut();
            state.pop = Some(pop);
            state.current = Some(i);
        }

        fn open_modal(&self, i: usize) {
            let Some(dialog) = self.dialog() else {
                return;
            };
            let item = &self.items[i];
            let mut html = format!(
                "<div class=\"rm-head\"><span class=\"ref-tipo\">{}</span><b>{}</b></div>",
                escape(&capitalize(&item.kind)),
                escape(&item.name)
            );
            if !item.summary.is_empty() {
                html.push_str(&format!(
                    "<p class=\"rm-desc\">{}</p>",
                    escape(&item.summary)
                ));
            }
            if !item.levels.is_empty() {
                html.push_str("<div class=\"rm-niveis\">");
                for n in &item.levels {
                    let label = if n.label.is_empty() {
                        String::new()
                    } else {
                        format!("<b>{}</b> · ", escape(&n.label))
                    };
                    let conduct = if n.conduct.is_empty() {
                        String::new()
                    } else {
                        format!(
                            "<span class=\"rm-conduta\"><b>Conduta:</b> {}</span>",
                            escape(&n.conduct)
                        )
                    };
                    html.push_str(&format!(
                        "<div class=\"rm-lv\"><span class=\"rm-n\">{}</span><span>{label}{}{conduct}</span></div>",
                        escape(&n.number),
                        escape(&n.text)
                    ));
                }
                html.push_str("</div>");
            }
            html.push_str(&format!(
                "<a class=\"rm-go\" href=\"{}\">ver no livro →</a>",
                escape(&item.url)
            ));
            if let Ok(Some(body)) = dialog.query_selector(".ref-modal-body") {
                body.set_inner_html(&html);
            }
            if dialog.open() {
                return;
            }
            // Abre sem rolar a página: o diálogo puxa o foco, e o navegador
            // rolaria até ele.
            let sx = self.window.scroll_x().unwrap_or(0.0);
            let sy = self.window.scroll_y().unwrap_or(0.0);
            let _ = dialog.show_modal();
            let _ = dialog.focus();
            self.window.scroll_to_with_x_and_y(sx, sy);
            for delay in [0, 16, 50] {
                let window = self.window.clone();
                let again = Closure::once_into_js(move || window.scroll_to_with_x_and_y(sx, sy));
                let _ = self
                    .window
                    .set_timeout_with_callback_and_timeout_and_arguments_0(
                        again.unchecked_ref(),
                        delay,
                    );
            }
        }

        /// Grava o termo nos fixados (a mesma chave e o mesmo formato do site
        /// atual: uma lista de `termo:id`, sem repetir).
        fn pin(&self, i: usize) {
            let Ok(Some(storage)) = self.window.session_storage() else {
                return;
            };
            let key = "centelha:consulta";
            let entry = format!("{}:{}", self.items[i].kind, self.items[i].id);
            let list = storage
                .get_item(key)
                .ok()
                .flatten()
                .and_then(|s| js_sys::JSON::parse(&s).ok())
                .and_then(|v| v.dyn_into::<js_sys::Array>().ok())
                .unwrap_or_default();
            let present = list
                .iter()
                .any(|v| v.as_string().as_deref() == Some(entry.as_str()));
            if !present {
                list.push(&entry.into());
            }
            if let Ok(text) = js_sys::JSON::stringify(&list) {
                let _ = storage.set_item(key, &String::from(text));
            }
            // O painel dos fixados (`crate::bookmarks`) se redesenha.
            if let Ok(event) = web_sys::Event::new(crate::bookmarks::CHANGED) {
                let _ = self.window.dispatch_event(&event);
            }
        }
    }

    fn closest(target: Option<web_sys::EventTarget>, selector: &str) -> Option<Element> {
        target?
            .dyn_into::<Element>()
            .ok()?
            .closest(selector)
            .ok()
            .flatten()
    }

    fn forget<T>(handle: T) {
        std::mem::forget(handle);
    }

    /// Os termos do JSON da página.
    fn parse(json: &str) -> Vec<Item> {
        let Ok(value) = js_sys::JSON::parse(json) else {
            return Vec::new();
        };
        let Ok(list) = value.dyn_into::<js_sys::Array>() else {
            return Vec::new();
        };
        list.iter()
            .map(|v| Item {
                id: field(&v, "id"),
                kind: field(&v, "kind"),
                name: field(&v, "name"),
                url: field(&v, "url"),
                summary: field(&v, "summary"),
                levels: js_sys::Reflect::get(&v, &"levels".into())
                    .ok()
                    .and_then(|n| n.dyn_into::<js_sys::Array>().ok())
                    .map(|levels| {
                        levels
                            .iter()
                            .map(|l| Level {
                                number: field(&l, "nivel"),
                                label: field(&l, "rotulo"),
                                text: field(&l, "texto"),
                                conduct: field(&l, "conduta"),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            })
            .collect()
    }

    /// Um campo de texto (ou número, como texto) de um objeto do JSON.
    fn field(v: &wasm_bindgen::JsValue, name: &str) -> String {
        let Ok(x) = js_sys::Reflect::get(v, &name.into()) else {
            return String::new();
        };
        if let Some(s) = x.as_string() {
            s
        } else if let Some(n) = x.as_f64() {
            // Os números do JSON (o nível de uma escala) são inteiros.
            (n as i64).to_string()
        } else {
            String::new()
        }
    }

    fn capitalize(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().chain(chars).collect(),
            None => String::new(),
        }
    }

    fn escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }
}

#[cfg(not(feature = "hydrate"))]
mod browser {
    pub fn start() {}
}
