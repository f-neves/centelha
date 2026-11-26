//! O cartão das formas: passar o mouse (ou chegar pelo teclado) num rótulo de
//! Área ou de Volume (`data-formas="area"` ou `"volume"`) abre um cartão com os
//! desenhos das formas em que aquele tanto de chão ou de espaço pode ser
//! moldado. Esc fecha.
//!
//! Port de `src/components/FormasPop.astro` do site atual. Os desenhos ficam
//! numa fonte escondida (`ShapesSource`, HTML do gerador), e a ilha, sem
//! propriedades, copia a fonte para o cartão. Melhoria de acessibilidade: a
//! ilha põe `tabindex="0"` nos rótulos que não recebem foco, para o Tab chegar
//! neles (no site atual, só o mouse abria o cartão nos rótulos sem foco).

use leptos::prelude::*;

/// A fonte dos desenhos, escondida (copiada do site atual).
const SOURCE: &str = r##"<div id="formas-area" data-titulo="Área" data-nota="O mesmo tanto de chão, moldado de outro jeito. Cada forma tem a sua régua: a forma se escolhe, e ela decide as dimensões.">
<figure><svg viewBox="0 0 64 52"><rect x="18" y="9" width="28" height="28" rx="1" /></svg><figcaption>Quadrado</figcaption></figure>
<figure><svg viewBox="0 0 64 52"><rect x="6" y="14" width="52" height="18" rx="1" /></svg><figcaption>Retângulo</figcaption></figure>
<figure><svg viewBox="0 0 64 52"><rect x="4" y="20" width="56" height="7" rx="1" /></svg><figcaption>Linha</figcaption></figure>
<figure><svg viewBox="0 0 64 52"><circle cx="32" cy="23" r="14" /></svg><figcaption>Círculo</figcaption></figure>
<figure><svg viewBox="0 0 64 52"><path d="M32 42 L15.9 19.1 A28 28 0 0 1 48.1 19.1 Z" /></svg><figcaption>Leque</figcaption></figure>
</div>
<div id="formas-volume" data-titulo="Volume" data-nota="O mesmo tanto de espaço, moldado de outro jeito. Cada forma tem a sua régua: a forma se escolhe, e ela decide as dimensões.">
<figure><svg viewBox="0 0 64 52"><polygon points="16,20 26,12 50,12 40,20" class="face-topo" /><polygon points="40,20 50,12 50,36 40,44" class="face-lado" /><rect x="16" y="20" width="24" height="24" /></svg><figcaption>Cubo</figcaption></figure>
<figure><svg viewBox="0 0 64 52"><polygon points="8,26 18,18 52,18 42,26" class="face-topo" /><polygon points="42,26 52,18 52,34 42,42" class="face-lado" /><rect x="8" y="26" width="34" height="16" /></svg><figcaption>Bloco</figcaption></figure>
<figure><svg viewBox="0 0 64 52"><circle cx="32" cy="26" r="16" /><ellipse cx="32" cy="26" rx="16" ry="5.5" class="linha-fina" /></svg><figcaption>Esfera</figcaption></figure>
<figure><svg viewBox="0 0 64 52"><path d="M14 34 A18 18 0 0 1 50 34 Z" /><ellipse cx="32" cy="34" rx="18" ry="5" class="linha-fina" /></svg><figcaption>Cúpula</figcaption></figure>
<figure><svg viewBox="0 0 64 52"><path d="M18 16 V38 A14 5 0 0 0 46 38 V16" /><ellipse cx="32" cy="16" rx="14" ry="5" class="face-topo" /></svg><figcaption>Coluna</figcaption></figure>
</div>"##;

/// A fonte escondida dos desenhos e a ilha. Vai nas páginas que têm rótulos
/// `data-formas`.
#[component]
pub fn Shapes() -> impl IntoView {
    view! {
        <div class="formas-fonte" hidden aria-hidden="true" inner_html=SOURCE></div>
        <ShapesPopover/>
    }
}

/// Os ouvintes do cartão. Não desenha nada.
#[island]
pub fn ShapesPopover() -> impl IntoView {
    Effect::new(move |_| browser::start());
}

#[cfg(feature = "hydrate")]
mod browser {
    use std::cell::RefCell;
    use std::rc::Rc;

    use leptos::prelude::window_event_listener;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;
    use web_sys::{Element, HtmlElement};

    #[derive(Default)]
    struct State {
        pop: Option<Element>,
        target: Option<Element>,
        timer: Option<i32>,
    }

    const CLOSE_DELAY_MS: i32 = 200;

    pub fn start() {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        // O Tab chega nos rótulos.
        if let Ok(labels) = document.query_selector_all("[data-formas]:not([tabindex])") {
            for i in 0..labels.length() {
                if let Some(el) = labels.item(i).and_then(|n| n.dyn_into::<Element>().ok()) {
                    let _ = el.set_attribute("tabindex", "0");
                }
            }
        }
        let state = Rc::new(RefCell::new(State::default()));
        let s = state.clone();
        forget(window_event_listener(leptos::ev::mouseover, move |e| {
            if let Some(el) = label(e.target()) {
                cancel(&s);
                open(&s, &el);
            }
        }));
        let s = state.clone();
        forget(window_event_listener(leptos::ev::mouseout, move |e| {
            if label(e.target()).is_some() {
                schedule(&s);
            }
        }));
        let s = state.clone();
        forget(window_event_listener(leptos::ev::focusin, move |e| {
            if let Some(el) = label(e.target()) {
                open(&s, &el);
            }
        }));
        let s = state.clone();
        forget(window_event_listener(leptos::ev::focusout, move |_| {
            schedule(&s)
        }));
        let s = state.clone();
        forget(window_event_listener(leptos::ev::keydown, move |e| {
            if e.key() == "Escape" {
                close(&s);
            }
        }));
        let s = state;
        forget(window_event_listener(leptos::ev::scroll, move |_| {
            close(&s)
        }));
    }

    fn forget<T>(handle: T) {
        std::mem::forget(handle);
    }

    fn label(target: Option<web_sys::EventTarget>) -> Option<Element> {
        target?
            .dyn_into::<Element>()
            .ok()?
            .closest("[data-formas]")
            .ok()
            .flatten()
    }

    fn close(state: &Rc<RefCell<State>>) {
        let mut st = state.borrow_mut();
        if let Some(pop) = st.pop.take() {
            pop.remove();
        }
        st.target = None;
    }

    fn cancel(state: &Rc<RefCell<State>>) {
        if let Some(t) = state.borrow_mut().timer.take()
            && let Some(w) = web_sys::window()
        {
            w.clear_timeout_with_handle(t);
        }
    }

    fn schedule(state: &Rc<RefCell<State>>) {
        cancel(state);
        let s = state.clone();
        let callback = Closure::once_into_js(move || close(&s));
        let timer = web_sys::window().and_then(|w| {
            w.set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.unchecked_ref(),
                CLOSE_DELAY_MS,
            )
            .ok()
        });
        state.borrow_mut().timer = timer;
    }

    fn open(state: &Rc<RefCell<State>>, el: &Element) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        let kind = el.get_attribute("data-formas").unwrap_or_default();
        let Some(source) = document.get_element_by_id(&format!("formas-{kind}")) else {
            return;
        };
        if state
            .borrow()
            .target
            .as_ref()
            .is_some_and(|t| t.is_same_node(Some(el)))
        {
            return;
        }
        close(state);
        let Ok(pop) = document.create_element("div") else {
            return;
        };
        pop.set_class_name("formas-pop");
        let piece = |class: &str, text: &str| {
            let span = document.create_element("span").ok()?;
            span.set_class_name(class);
            span.set_text_content(Some(text));
            Some(span)
        };
        if let Some(t) = piece(
            "formas-tit",
            &source.get_attribute("data-titulo").unwrap_or_default(),
        ) {
            let _ = pop.append_child(&t);
        }
        if let Some(n) = piece(
            "formas-nota",
            &source.get_attribute("data-nota").unwrap_or_default(),
        ) {
            let _ = pop.append_child(&n);
        }
        if let Ok(grid) = document.create_element("div") {
            grid.set_class_name("formas-grade");
            grid.set_inner_html(&source.inner_html());
            let _ = pop.append_child(&grid);
        }
        let s = state.clone();
        let enter = Closure::<dyn FnMut()>::new(move || cancel(&s));
        let _ = pop.add_event_listener_with_callback("mouseenter", enter.as_ref().unchecked_ref());
        enter.forget();
        let s = state.clone();
        let leave = Closure::<dyn FnMut()>::new(move || schedule(&s));
        let _ = pop.add_event_listener_with_callback("mouseleave", leave.as_ref().unchecked_ref());
        leave.forget();
        let Some(body) = document.body() else { return };
        let _ = body.append_child(&pop);
        // Abaixo do rótulo, sem passar da borda direita.
        let r = el.get_bounding_client_rect();
        let sx = window.scroll_x().unwrap_or(0.0);
        let sy = window.scroll_y().unwrap_or(0.0);
        let width = document
            .document_element()
            .map(|d| f64::from(d.client_width()))
            .unwrap_or(0.0);
        let pop_width = pop
            .dyn_ref::<HtmlElement>()
            .map(|p| f64::from(p.offset_width()))
            .unwrap_or(0.0);
        let max = sx + width - pop_width - 10.0;
        let left = (sx + r.left()).min(max).max(8.0).round() as i64;
        let top = (sy + r.bottom() + 6.0).round() as i64;
        if let Some(p) = pop.dyn_ref::<HtmlElement>() {
            let _ = p.style().set_property("top", &format!("{top}px"));
            let _ = p.style().set_property("left", &format!("{left}px"));
        }
        let mut st = state.borrow_mut();
        st.pop = Some(pop);
        st.target = Some(el.clone());
    }
}

#[cfg(not(feature = "hydrate"))]
mod browser {
    pub fn start() {}
}
