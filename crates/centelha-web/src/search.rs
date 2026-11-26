//! A busca: o botão "Buscar" da barra lateral (e o 🔍 da barra do topo no
//! celular), a tecla `/`, a caixa de diálogo com os modos "Palavras" e
//! "Frase exata", o "Aa" (diferenciar maiúsculas), os resultados com o trecho
//! e as seções (âncoras), "Mostrar mais" e as setas entre o campo e os
//! resultados.
//!
//! Port de `src/components/Busca.astro` do site atual, que fala com o
//! `pagefind.js` direto (sem o PagefindUI). A marcação sai pronta do gerador
//! (`crate::site::pages`); a ilha não tem propriedades e só liga os
//! ouvintes. O índice do Pagefind só é baixado quando o leitor abre a caixa.
//! O modo e o Aa ficam no `localStorage`, chave `centelha-busca`, JSON
//! `{"modo":"palavras"|"frase","aa":true|false}`, a mesma chave e o mesmo
//! formato do site atual (R26). A regra do Aa é a de
//! `crate::search_match`.

use leptos::prelude::*;

/// Liga a busca. Não desenha nada.
#[island]
pub fn Search() -> impl IntoView {
    Effect::new(move |_| browser::start());
}

#[cfg(feature = "hydrate")]
mod browser {
    use std::cell::RefCell;
    use std::rc::Rc;

    use js_sys::{Array, Function, Promise, Reflect};
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::{JsFuture, spawn_local};
    use web_sys::{Element, HtmlDialogElement, HtmlElement, HtmlInputElement};

    use crate::search_match::{
        Mode, Verdict, excerpt, filter_case, pagefind_term, tokens, words_of,
    };

    const KEY: &str = "centelha-busca";
    /// Resultados por vez.
    const BATCH: usize = 8;
    /// Seções (âncoras) mostradas por página.
    const SUBS: usize = 3;
    /// Espera depois da última tecla, em ms.
    const DELAY: f64 = 150.0;

    struct Matched {
        data: JsValue,
        words: Vec<String>,
        verdict: Verdict,
    }

    #[derive(Default)]
    struct State {
        /// Cada busca nova invalida as respostas atrasadas das anteriores.
        seq: u32,
        /// Os resultados do Pagefind ainda sem os dados.
        candidates: Vec<JsValue>,
        /// Com Aa: os que passaram no filtro de caixa.
        queue: Vec<Matched>,
        next: usize,
        /// A busca mostrada: com Aa?
        active: Option<bool>,
        loader: Option<Promise>,
        warned: bool,
    }

    struct Ui {
        dialog: HtmlDialogElement,
        input: HtmlInputElement,
        list: Element,
        message: Element,
        more: HtmlElement,
        aa: HtmlInputElement,
        pf_url: String,
        state: RefCell<State>,
    }

    fn get(v: &JsValue, key: &str) -> JsValue {
        Reflect::get(v, &key.into()).unwrap_or(JsValue::UNDEFINED)
    }

    fn text(v: &JsValue, key: &str) -> String {
        get(v, key).as_string().unwrap_or_default()
    }

    fn numbers(v: &JsValue) -> Option<Vec<f64>> {
        v.dyn_ref::<Array>()
            .map(|a| a.iter().map(|x| x.as_f64().unwrap_or(f64::NAN)).collect())
    }

    async fn call(target: &JsValue, method: &str, args: &[JsValue]) -> Option<JsValue> {
        let f = get(target, method).dyn_into::<Function>().ok()?;
        let array: Array = args.iter().collect();
        let value = f.apply(target, &array).ok()?;
        match value.dyn_into::<Promise>() {
            Ok(p) => JsFuture::from(p).await.ok(),
            Err(v) => Some(v),
        }
    }

    impl Ui {
        fn mode(&self) -> Mode {
            let phrase = self
                .dialog
                .query_selector("input[name=\"busca-modo\"]:checked")
                .ok()
                .flatten()
                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                .is_some_and(|r| r.value() == "frase");
            if phrase { Mode::Phrase } else { Mode::Words }
        }

        fn radios(&self) -> Vec<HtmlInputElement> {
            let Ok(list) = self.dialog.query_selector_all("input[name=\"busca-modo\"]") else {
                return Vec::new();
            };
            (0..list.length())
                .filter_map(|i| list.item(i))
                .filter_map(|n| n.dyn_into::<HtmlInputElement>().ok())
                .collect()
        }

        fn restore(&self) {
            let Some(saved) = web_sys::window()
                .and_then(|w| w.local_storage().ok().flatten())
                .and_then(|s| s.get_item(KEY).ok().flatten())
                .and_then(|t| js_sys::JSON::parse(&t).ok())
            else {
                return;
            };
            if text(&saved, "modo") == "frase" {
                for r in self.radios() {
                    r.set_checked(r.value() == "frase");
                }
            }
            self.aa
                .set_checked(get(&saved, "aa").as_bool() == Some(true));
        }

        fn save(&self) {
            let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten())
            else {
                return;
            };
            let mode = if self.mode() == Mode::Phrase {
                "frase"
            } else {
                "palavras"
            };
            let value = format!("{{\"modo\":\"{mode}\",\"aa\":{}}}", self.aa.checked());
            let _ = storage.set_item(KEY, &value);
        }

        /// O `pagefind.js`, carregado uma vez só.
        fn ensure(&self) -> Promise {
            let mut state = self.state.borrow_mut();
            if let Some(p) = &state.loader {
                return p.clone();
            }
            let import = Function::new_with_args(
                "u",
                "return import(u).then((m) => { if (m.init) m.init(); return m; }).catch(() => null)",
            );
            let promise = import
                .call1(&JsValue::NULL, &self.pf_url.clone().into())
                .ok()
                .and_then(|p| p.dyn_into::<Promise>().ok())
                .unwrap_or_else(|| Promise::resolve(&JsValue::NULL));
            state.loader = Some(promise.clone());
            promise
        }

        fn item(&self, data: &JsValue, matched: Option<(&[String], &Verdict)>) -> Option<Element> {
            let document = self.dialog.owner_document()?;
            let li = document.create_element("li").ok()?;
            li.set_class_name("busca-res");
            let h = document.create_element("h3").ok()?;
            h.set_class_name("busca-res-titulo");
            let a = document.create_element("a").ok()?;
            let url = text(data, "url");
            let _ = a.set_attribute("href", &url);
            let title = text(&get(data, "meta"), "title");
            a.set_text_content(Some(if title.is_empty() { &url } else { &title }));
            let _ = h.append_child(&a);
            let _ = li.append_child(&h);
            let p = document.create_element("p").ok()?;
            p.set_class_name("busca-res-trecho");
            let marked: &[usize] = matched.map(|(_, v)| v.accepted.as_slice()).unwrap_or(&[]);
            match (matched, marked.first()) {
                (Some((words, _)), Some(&first)) => {
                    p.set_inner_html(&excerpt(words, marked, first, 12, 28));
                }
                _ => p.set_inner_html(&text(data, "excerpt")),
            }
            let _ = li.append_child(&p);
            let subs: Vec<JsValue> = get(data, "sub_results")
                .dyn_into::<Array>()
                .map(|a| a.iter().collect())
                .unwrap_or_default();
            let mut subs: Vec<JsValue> = subs
                .into_iter()
                .filter(|s| get(s, "anchor").is_truthy())
                .collect();
            if !marked.is_empty() {
                subs.retain(|s| match numbers(&get(s, "locations")) {
                    None => true,
                    Some(locs) => safe(&locs).iter().any(|i| marked.contains(i)),
                });
            }
            subs.truncate(SUBS);
            if !subs.is_empty() {
                let ul = document.create_element("ul").ok()?;
                ul.set_class_name("busca-subs");
                for s in &subs {
                    let si = document.create_element("li").ok()?;
                    let sa = document.create_element("a").ok()?;
                    let _ = sa.set_attribute("href", &text(s, "url"));
                    sa.set_text_content(Some(&text(s, "title")));
                    let sp = document.create_element("p").ok()?;
                    let in_sub: Vec<usize> = numbers(&get(s, "locations"))
                        .map(|l| safe(&l))
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|i| marked.contains(i))
                        .collect();
                    match (matched, in_sub.first()) {
                        (Some((words, _)), Some(&first)) => {
                            sp.set_inner_html(&excerpt(words, marked, first, 8, 22));
                        }
                        _ => sp.set_inner_html(&text(s, "excerpt")),
                    }
                    let _ = si.append_child(&sa);
                    let _ = si.append_child(&sp);
                    let _ = ul.append_child(&si);
                }
                let _ = li.append_child(&ul);
            }
            Some(li)
        }
    }

    /// `locaisSeguros`.
    fn safe(locs: &[f64]) -> Vec<usize> {
        locs.iter()
            .filter(|i| i.fract() == 0.0 && i.is_finite() && **i >= 0.0)
            .map(|&i| i as usize)
            .collect()
    }

    async fn show_more(ui: Rc<Ui>, mine: u32) {
        let Some(aa) = ui.state.borrow().active else {
            return;
        };
        let remaining = if !aa {
            let (batch, total) = {
                let mut s = ui.state.borrow_mut();
                let end = (s.next + BATCH).min(s.candidates.len());
                let batch: Vec<JsValue> = s.candidates[s.next..end].to_vec();
                s.next = end;
                (batch, s.candidates.len())
            };
            let mut data = Vec::new();
            for r in &batch {
                data.push(call(r, "data", &[]).await.unwrap_or(JsValue::NULL));
            }
            if mine != ui.state.borrow().seq {
                return;
            }
            for d in &data {
                if let Some(li) = ui.item(d, None) {
                    let _ = ui.list.append_child(&li);
                }
            }
            total - ui.state.borrow().next
        } else {
            let mut s = ui.state.borrow_mut();
            let end = (s.next + BATCH).min(s.queue.len());
            for m in &s.queue[s.next..end] {
                if let Some(li) = ui.item(&m.data, Some((&m.words, &m.verdict))) {
                    let _ = ui.list.append_child(&li);
                }
            }
            s.next = end;
            s.queue.len() - end
        };
        ui.more.set_hidden(remaining == 0);
        if remaining > 0 {
            ui.more
                .set_text_content(Some(&format!("Mostrar mais ({remaining})")));
        }
    }

    async fn search(ui: Rc<Ui>) {
        let mine = {
            let mut s = ui.state.borrow_mut();
            s.seq += 1;
            s.seq
        };
        // O botão da busca anterior não pode ser clicado enquanto a nova corre.
        ui.more.set_hidden(true);
        let raw = ui.input.value().trim().to_string();
        let mode = ui.mode();
        let aa = ui.aa.checked();
        let toks = tokens(&raw);
        if toks.is_empty() {
            ui.list.set_inner_html("");
            {
                let mut s = ui.state.borrow_mut();
                s.candidates.clear();
                s.queue.clear();
                s.next = 0;
                s.active = None;
            }
            ui.message.set_text_content(Some(if raw.is_empty() {
                ""
            } else {
                "Digite ao menos uma palavra."
            }));
            return;
        }
        let loading = ui.state.borrow().loader.is_none();
        ui.message.set_text_content(Some(if loading {
            "Carregando o índice…"
        } else {
            "Buscando…"
        }));
        let pf = JsFuture::from(ui.ensure()).await.unwrap_or(JsValue::NULL);
        if mine != ui.state.borrow().seq {
            return;
        }
        if pf.is_null() || pf.is_undefined() {
            ui.message.set_text_content(Some(
                "A busca fica disponível no site publicado (índice gerado no build).",
            ));
            return;
        }
        let term = pagefind_term(&toks, mode);
        let options = js_sys::Object::new();
        let Some(resp) = call(
            &pf,
            "debouncedSearch",
            &[term.into(), options.into(), DELAY.into()],
        )
        .await
        else {
            return;
        };
        if resp.is_null() || mine != ui.state.borrow().seq {
            return;
        }
        // A lista e o estado trocam juntos.
        ui.list.set_inner_html("");
        let candidates: Vec<JsValue> = get(&resp, "results")
            .dyn_into::<Array>()
            .map(|a| a.iter().collect())
            .unwrap_or_default();
        {
            let mut s = ui.state.borrow_mut();
            s.active = Some(aa);
            s.candidates = candidates.clone();
            s.queue.clear();
            s.next = 0;
        }
        if aa {
            for chunk in candidates.chunks(12) {
                let mut data = Vec::new();
                for r in chunk {
                    data.push(call(r, "data", &[]).await.unwrap_or(JsValue::NULL));
                }
                if mine != ui.state.borrow().seq {
                    return;
                }
                for d in data {
                    let words = words_of(&text(&d, "content"));
                    let locs = numbers(&get(&d, "locations"));
                    let count = get(&d, "word_count").as_f64();
                    let verdict = filter_case(&words, locs.as_deref(), &toks, mode, count);
                    if verdict.broken_premise {
                        let mut s = ui.state.borrow_mut();
                        if !s.warned {
                            s.warned = true;
                            web_sys::console::warn_1(
                                &format!(
                                    "Busca: os locations do pagefind não batem com o texto de {}; o Aa não está filtrando a caixa nos resultados assim.",
                                    text(&d, "url")
                                )
                                .into(),
                            );
                        }
                    }
                    if verdict.ok {
                        ui.state.borrow_mut().queue.push(Matched {
                            data: d,
                            words,
                            verdict,
                        });
                    }
                }
            }
        }
        let total = if aa {
            ui.state.borrow().queue.len()
        } else {
            candidates.len()
        };
        let label = format!(
            "\u{201c}{raw}\u{201d}{}{}",
            if mode == Mode::Phrase {
                " (frase exata)"
            } else {
                ""
            },
            if aa { " com Aa" } else { "" }
        );
        let message = if total > 0 {
            format!(
                "{total}{}{label}",
                if total == 1 {
                    " resultado para "
                } else {
                    " resultados para "
                }
            )
        } else {
            format!(
                "Nenhum resultado para {label}{}",
                if aa { ". Tente desligar o Aa." } else { "." }
            )
        };
        ui.message.set_text_content(Some(&message));
        if total > 0 {
            show_more(ui.clone(), mine).await;
        }
    }

    fn run_search(ui: &Rc<Ui>) {
        spawn_local(search(ui.clone()));
    }

    fn open(ui: &Rc<Ui>) {
        if !ui.dialog.open() {
            let _ = ui.dialog.show_modal();
        }
        let _ = ui.input.focus();
        let _ = ui.ensure();
        let empty_list = ui.list.child_element_count() == 0;
        let no_message = ui.message.text_content().unwrap_or_default().is_empty();
        if !ui.input.value().trim().is_empty() && empty_list && no_message {
            run_search(ui);
        }
    }

    fn listen(target: &web_sys::EventTarget, event: &str, f: impl FnMut(web_sys::Event) + 'static) {
        let closure = Closure::<dyn FnMut(web_sys::Event)>::new(f);
        let _ = target.add_event_listener_with_callback(event, closure.as_ref().unchecked_ref());
        closure.forget();
    }

    fn find<T: JsCast>(root: &Element, selector: &str) -> Option<T> {
        root.query_selector(selector)
            .ok()
            .flatten()?
            .dyn_into::<T>()
            .ok()
    }

    pub fn start() {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let Some(dialog) = document
            .query_selector(".busca-modal")
            .ok()
            .flatten()
            .and_then(|d| d.dyn_into::<HtmlDialogElement>().ok())
        else {
            return;
        };
        let (Some(input), Some(list), Some(message), Some(more), Some(aa)) = (
            find::<HtmlInputElement>(&dialog, ".busca-entrada"),
            find::<Element>(&dialog, ".busca-lista"),
            find::<Element>(&dialog, ".busca-msg"),
            find::<HtmlElement>(&dialog, ".busca-mais"),
            find::<HtmlInputElement>(&dialog, ".busca-aa-caixa"),
        ) else {
            return;
        };
        let pf_url = dialog.get_attribute("data-pf").unwrap_or_default();
        let ui = Rc::new(Ui {
            dialog,
            input,
            list,
            message,
            more,
            aa,
            pf_url,
            state: RefCell::new(State::default()),
        });
        ui.restore();
        let u = ui.clone();
        listen(&ui.input, "input", move |_| run_search(&u));
        for r in ui.radios() {
            let u = ui.clone();
            listen(&r, "change", move |_| {
                u.save();
                run_search(&u);
            });
        }
        let u = ui.clone();
        listen(&ui.aa, "change", move |_| {
            u.save();
            run_search(&u);
        });
        let u = ui.clone();
        listen(&ui.more, "click", move |_| {
            let seq = u.state.borrow().seq;
            spawn_local(show_more(u.clone(), seq));
        });
        if let Ok(buttons) = document.query_selector_all(".busca-abrir") {
            for i in 0..buttons.length() {
                if let Some(b) = buttons.item(i) {
                    let u = ui.clone();
                    listen(&b, "click", move |_| open(&u));
                }
            }
        }
        if let Some(close) = find::<Element>(&ui.dialog, ".busca-fechar") {
            let u = ui.clone();
            listen(&close, "click", move |_| u.dialog.close());
        }
        let u = ui.clone();
        listen(&ui.dialog, "click", move |e| {
            let on_backdrop = e
                .target()
                .and_then(|t| t.dyn_into::<Element>().ok())
                .is_some_and(|t| t.is_same_node(Some(&u.dialog)));
            if on_backdrop {
                u.dialog.close();
            }
        });
        // Melhoria de acessibilidade: as setas andam entre o campo e os links
        // dos resultados (o Enter segue o link, o Esc fecha a caixa e o foco
        // volta para onde estava, pelo próprio `<dialog>`).
        let u = ui.clone();
        listen(&ui.dialog, "keydown", move |e| {
            let Some(e) = e.dyn_ref::<web_sys::KeyboardEvent>() else {
                return;
            };
            let step: i32 = match e.key().as_str() {
                "ArrowDown" => 1,
                "ArrowUp" => -1,
                _ => return,
            };
            let mut stops: Vec<HtmlElement> = vec![u.input.clone().unchecked_into()];
            if let Ok(links) = u.list.query_selector_all("a") {
                stops.extend(
                    (0..links.length())
                        .filter_map(|i| links.item(i))
                        .filter_map(|n| n.dyn_into::<HtmlElement>().ok()),
                );
            }
            if stops.len() < 2 {
                return;
            }
            let active = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.active_element());
            let Some(at) = stops
                .iter()
                .position(|s| active.as_ref().is_some_and(|a| a.is_same_node(Some(s))))
            else {
                return;
            };
            e.prevent_default();
            let len = stops.len() as i32;
            let next = (at as i32 + step).rem_euclid(len) as usize;
            let _ = stops[next].focus();
        });
        let u = ui.clone();
        listen(&document, "keydown", move |e| {
            let Some(e) = e.dyn_ref::<web_sys::KeyboardEvent>() else {
                return;
            };
            if e.key() != "/" || e.ctrl_key() || e.meta_key() || e.alt_key() {
                return;
            }
            let active: Option<Element> = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.active_element());
            if let Some(a) = &active {
                let tag = a.tag_name();
                let editable = a
                    .dyn_ref::<HtmlElement>()
                    .is_some_and(|h| h.is_content_editable());
                let in_field = matches!(tag.as_str(), "INPUT" | "TEXTAREA" | "SELECT") || editable;
                // A / é texto quando se digita num campo, inclusive o da busca aberta.
                let inside = u.dialog.contains(Some(a));
                let own_open = u.dialog.open() && a.is_same_node(Some(&u.input));
                if in_field && (!inside || own_open) {
                    return;
                }
            }
            e.prevent_default();
            open(&u);
        });
    }
}

#[cfg(not(feature = "hydrate"))]
mod browser {
    pub fn start() {}
}
