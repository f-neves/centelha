//! Bestiary card: the same components render to static HTML (feature `ssr`)
//! and hydrate in the browser (feature `hydrate`).

use leptos::html::Canvas;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Attributes {
    #[serde(rename = "forca")]
    pub strength: i32,
    #[serde(rename = "destreza")]
    pub dexterity: i32,
    #[serde(rename = "vigor")]
    pub stamina: i32,
    #[serde(rename = "influencia")]
    pub presence: i32,
    #[serde(rename = "perspicacia")]
    pub manipulation: i32,
    #[serde(rename = "compostura")]
    pub composure: i32,
    #[serde(rename = "percepcao")]
    pub perception: i32,
    #[serde(rename = "inteligencia")]
    pub intelligence: i32,
    #[serde(rename = "raciocinio")]
    pub wits: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Attack {
    #[serde(rename = "nome")]
    pub name: String,
    #[serde(rename = "dado", default)]
    pub die: Option<String>,
    #[serde(rename = "tipoDano", default)]
    pub damage_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Creature {
    pub id: String,
    #[serde(rename = "nome")]
    pub name: String,
    #[serde(rename = "ameaca")]
    pub threat: i32,
    #[serde(rename = "centelha")]
    pub spark: i32,
    #[serde(rename = "vontade")]
    pub willpower: i32,
    #[serde(rename = "atributos")]
    pub attributes: Attributes,
    #[serde(rename = "ataques", default)]
    pub attacks: Vec<Attack>,
}

impl Attributes {
    fn rows(&self) -> [(&'static str, i32); 9] {
        [
            ("Força", self.strength),
            ("Destreza", self.dexterity),
            ("Vigor", self.stamina),
            ("Influência", self.presence),
            ("Perspicácia", self.manipulation),
            ("Compostura", self.composure),
            ("Percepção", self.perception),
            ("Inteligência", self.intelligence),
            ("Raciocínio", self.wits),
        ]
    }
}

/// Which page the static file holds. Serialized into the page head so the
/// client hydrates exactly the tree the generator rendered.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Page {
    Index(Vec<(String, String)>),
    Creature(Creature),
}

#[component]
pub fn App(page: Page) -> impl IntoView {
    match page {
        Page::Index(entries) => view! { <Index entries/> }.into_any(),
        Page::Creature(creature) => view! { <CreatureCard creature/> }.into_any(),
    }
}

#[component]
pub fn Index(entries: Vec<(String, String)>) -> impl IntoView {
    view! {
        <main class="index">
            <h1>"Bestiário"</h1>
            <ul>
                {entries
                    .into_iter()
                    .map(|(id, name)| view! { <li><a href=format!("/{id}.html")>{name}</a></li> })
                    .collect_view()}
            </ul>
        </main>
    }
}

#[component]
pub fn CreatureCard(creature: Creature) -> impl IntoView {
    let image = format!("/img/{}_criatura.webp", creature.id);
    view! {
        <main class="card">
            <p><a href="/index.html">"Voltar ao bestiário"</a></p>
            <h1>{creature.name.clone()}</h1>
            <img src=image alt=creature.name.clone()/>
            <ul class="stats">
                <li>"Ameaça: " {creature.threat}</li>
                <li>"Centelha: " {creature.spark}</li>
                <li>"Força de Vontade: " {creature.willpower}</li>
                <li>"Defesa: (motor, fase 1)"</li>
            </ul>
            <h2>"Atributos"</h2>
            <table class="attributes"><tbody>
                {creature
                    .attributes
                    .rows()
                    .into_iter()
                    .map(|(label, value)| view! { <tr><th>{label}</th><td>{value}</td></tr> })
                    .collect_view()}
            </tbody></table>
            <h2>"Ataques"</h2>
            <ul class="attacks">
                {creature
                    .attacks
                    .into_iter()
                    .map(|attack| {
                        let details = [attack.die, attack.damage_type]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>()
                            .join(", ");
                        let details = (!details.is_empty()).then(|| format!(" ({details})"));
                        view! { <li>{attack.name}{details}</li> }
                    })
                    .collect_view()}
            </ul>
            <DiceRoller/>
            <CanvasProof/>
        </main>
    }
}

#[component]
pub fn DiceRoller() -> impl IntoView {
    let result = RwSignal::new(None::<u32>);
    view! {
        <p class="dice">
            <button on:click=move |_| result.set(Some(roll_d6()))>"Rolar 1d6"</button>
            " Resultado: "
            <output>{move || result.get().map(|n| n.to_string()).unwrap_or_else(|| "nenhum".into())}</output>
        </p>
    }
}

/// Canvas drawing from a component: the element comes through a `NodeRef`,
/// and the effect only runs in the browser, after the node is mounted.
#[component]
pub fn CanvasProof() -> impl IntoView {
    let canvas = NodeRef::<Canvas>::new();
    Effect::new(move |_| {
        if let Some(el) = canvas.get() {
            use wasm_bindgen::JsCast;
            let ctx = el.get_context("2d").ok().flatten().unwrap();
            let ctx = ctx.unchecked_into::<web_sys::CanvasRenderingContext2d>();
            ctx.set_fill_style_str("#b5482c");
            ctx.fill_rect(10.0, 10.0, 40.0, 40.0);
        }
    });
    view! { <canvas node_ref=canvas width="60" height="60" aria-hidden="true"></canvas> }
}

fn roll_d6() -> u32 {
    #[cfg(feature = "hydrate")]
    {
        (js_sys::Math::random() * 6.0) as u32 + 1
    }
    #[cfg(not(feature = "hydrate"))]
    {
        1
    }
}

/// Client entry point: reads the page payload from the head and hydrates `<body>`.
#[cfg(feature = "entry")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    let payload = leptos::prelude::document()
        .get_element_by_id("page-data")
        .and_then(|el| el.text_content())
        .expect("page-data missing");
    let page: Page = serde_json::from_str(&payload).expect("invalid page-data");
    leptos::mount::hydrate_body(move || view! { <App page/> });
}
