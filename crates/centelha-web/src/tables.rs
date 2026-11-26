//! A ordenação das tabelas dos capítulos: clicar no cabeçalho de uma coluna
//! ordena as linhas por ela, e outro clique inverte.
//!
//! Port do script de `src/pages/regras/[slug].astro` do site atual: vale
//! para toda tabela do capítulo com duas colunas ou mais, menos a última
//! coluna (normalmente a descrição). A coluna em que todas as células são
//! número (ou vazias) ordena pelo valor (`crate::sort::cell_value`: moeda em
//! cobre, frações, milhar e vírgula); as outras, pelo texto
//! (`crate::sort::compare_numeric_base`). A ordem é estável.
//!
//! Melhoria de acessibilidade, sem mudar o texto: o site atual põe
//! `role="button"` no próprio `<th>`, o que tira dele o papel de cabeçalho;
//! aqui o conteúdo do cabeçalho vai dentro de um `<button>` (Enter e espaço
//! já funcionam nele) e o `<th>` diz a ordem atual em `aria-sort`.

use leptos::prelude::*;

/// Liga a ordenação nas tabelas da página. Não desenha nada.
#[island]
pub fn TableSorter() -> impl IntoView {
    Effect::new(move |_| browser::start());
}

#[cfg(feature = "hydrate")]
mod browser {
    use std::cmp::Ordering;

    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;
    use web_sys::{Element, HtmlTableElement, HtmlTableRowElement, HtmlTableSectionElement};

    use crate::sort::{cell_value, compare_numeric_base, is_empty_cell};

    pub fn start() {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let Ok(tables) = document.query_selector_all("main table") else {
            return;
        };
        for i in 0..tables.length() {
            if let Some(table) = tables
                .item(i)
                .and_then(|n| n.dyn_into::<HtmlTableElement>().ok())
            {
                prepare(&document, &table);
            }
        }
    }

    fn header_cells(table: &HtmlTableElement) -> Vec<Element> {
        let Some(head) = table.t_head() else {
            return Vec::new();
        };
        let Some(row) = head.rows().item(0) else {
            return Vec::new();
        };
        let cells = row.unchecked_into::<HtmlTableRowElement>().cells();
        (0..cells.length()).filter_map(|i| cells.item(i)).collect()
    }

    fn prepare(document: &web_sys::Document, table: &HtmlTableElement) {
        let ths = header_cells(table);
        let has_body = table.t_bodies().item(0).is_some();
        if ths.len() < 2 || !has_body {
            return;
        }
        let last = ths.len() - 1;
        for (col, th) in ths.iter().enumerate() {
            if col == last {
                continue;
            }
            let _ = th.class_list().add_1("ord");
            let _ = th.set_attribute("aria-sort", "none");
            // O conteúdo do cabeçalho vai para dentro de um botão.
            let Ok(button) = document.create_element("button") else {
                continue;
            };
            let _ = button.set_attribute("type", "button");
            button.set_class_name("ord-btn");
            while let Some(child) = th.first_child() {
                let _ = button.append_child(&child);
            }
            let _ = th.append_child(&button);
            let table = table.clone();
            let on_click = Closure::<dyn FnMut()>::new(move || sort_by(&table, col));
            // O ouvinte fica no `<th>` inteiro (onde o CSS põe a mão e as
            // setas), como no site atual; o clique e o teclado do botão
            // sobem até ele.
            let _ = th.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref());
            on_click.forget();
        }
    }

    fn cell_text(row: &HtmlTableRowElement, col: usize) -> String {
        row.cells()
            .item(col as u32)
            .and_then(|c| c.text_content())
            .unwrap_or_default()
            .trim()
            .to_string()
    }

    fn sort_by(table: &HtmlTableElement, col: usize) {
        let ths = header_cells(table);
        let Some(th) = ths.get(col) else {
            return;
        };
        let ascending = !th.class_list().contains("asc");
        for other in &ths {
            let _ = other.class_list().remove_2("asc", "desc");
            if other.has_attribute("aria-sort") {
                let _ = other.set_attribute("aria-sort", "none");
            }
        }
        let _ = th
            .class_list()
            .add_1(if ascending { "asc" } else { "desc" });
        let _ = th.set_attribute(
            "aria-sort",
            if ascending { "ascending" } else { "descending" },
        );

        let Some(body) = table
            .t_bodies()
            .item(0)
            .and_then(|b| b.dyn_into::<HtmlTableSectionElement>().ok())
        else {
            return;
        };
        let rows_list = body.rows();
        let mut rows: Vec<(HtmlTableRowElement, String)> = (0..rows_list.length())
            .filter_map(|i| rows_list.item(i))
            .map(|r| r.unchecked_into::<HtmlTableRowElement>())
            .map(|r| {
                let text = cell_text(&r, col);
                (r, text)
            })
            .collect();
        let numeric = rows
            .iter()
            .all(|(_, v)| is_empty_cell(v) || cell_value(v).is_some());
        let key = |v: &str| -> f64 {
            if is_empty_cell(v) {
                f64::NEG_INFINITY
            } else {
                cell_value(v).unwrap_or(f64::NEG_INFINITY)
            }
        };
        rows.sort_by(|(_, a), (_, b)| {
            let order = if numeric {
                key(a).partial_cmp(&key(b)).unwrap_or(Ordering::Equal)
            } else {
                compare_numeric_base(a, b)
            };
            if ascending { order } else { order.reverse() }
        });
        for (row, _) in rows {
            let _ = body.append_child(&row);
        }
    }
}

#[cfg(not(feature = "hydrate"))]
mod browser {
    pub fn start() {}
}
