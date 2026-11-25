//! Static site generator: renders every page with hydration markers and
//! writes `dist/`. The wasm bundle goes to `dist/pkg/` (see build script).

use card::{App, Creature, Page};
use hydration_context::SsrSharedContext;
use leptos::prelude::*;
use leptos::reactive::owner::Owner;
use std::{fs, path::Path, sync::Arc};

const STYLE: &str = "body{font-family:system-ui,sans-serif;max-width:40rem;margin:0 auto;padding:1rem}\
img{display:block;width:100%;height:auto}\
table.attributes th{text-align:left;padding-right:1rem}";

fn render(page: Page, title: &str) -> String {
    let payload = serde_json::to_string(&page).unwrap().replace("</", "<\\/");
    let owner = Owner::new_root(Some(Arc::new(SsrSharedContext::new())));
    let body = owner.with(move || view! { <App page/> }.to_html());
    format!(
        "<!DOCTYPE html><html lang=\"pt-BR\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
<title>{title}</title><style>{STYLE}</style>\
<script id=\"page-data\" type=\"application/json\">{payload}</script>\
<link rel=\"modulepreload\" href=\"/pkg/card.js\">\
<script type=\"module\">import init,{{hydrate}} from '/pkg/card.js';\
init({{module_or_path:'/pkg/card_bg.wasm'}}).then(()=>hydrate());</script>\
</head><body>{body}</body></html>"
    )
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let data = root.join("..").join("dados");
    let dist = root.join("dist");
    fs::create_dir_all(dist.join("img")).unwrap();

    let mut entries = Vec::new();
    for id in ["aboleth", "aasimar", "ankheg"] {
        let json = fs::read_to_string(data.join(format!("{id}.json"))).unwrap();
        let creature: Creature = serde_json::from_str(&json).unwrap();
        let image = format!("{id}_criatura.webp");
        fs::copy(data.join(&image), dist.join("img").join(&image)).unwrap();
        entries.push((creature.id.clone(), creature.name.clone()));
        let title = creature.name.clone();
        fs::write(dist.join(format!("{id}.html")), render(Page::Creature(creature), &title)).unwrap();
    }
    fs::write(dist.join("index.html"), render(Page::Index(entries), "Bestiário")).unwrap();
    println!("dist/ gerado em {}", dist.display());
}
