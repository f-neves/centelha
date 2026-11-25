use dioxus::prelude::*;

#[component]
pub fn CanvasDemo() -> Element {
    rsx! {
        canvas {
            width: "120",
            height: "120",
            onmounted: move |evt| {
                #[cfg(target_arch = "wasm32")]
                {
                    use dioxus::web::WebEventExt;
                    use wasm_bindgen::JsCast;
                    let canvas: web_sys::HtmlCanvasElement = evt.as_web_event().dyn_into().unwrap();
                    let ctx: web_sys::CanvasRenderingContext2d =
                        canvas.get_context("2d").unwrap().unwrap().dyn_into().unwrap();
                    ctx.set_fill_style_str("#b5482a");
                    ctx.fill_rect(20.0, 20.0, 80.0, 80.0);
                }
                #[cfg(not(target_arch = "wasm32"))]
                let _ = evt;
            },
        }
    }
}
