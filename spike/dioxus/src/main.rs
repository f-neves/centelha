use dioxus::prelude::*;
use serde::Deserialize;

mod canvas;

#[derive(Deserialize, Clone, PartialEq)]
pub struct Attributes {
    #[serde(rename = "forca")]
    pub strength: u8,
    #[serde(rename = "destreza")]
    pub dexterity: u8,
    #[serde(rename = "vigor")]
    pub stamina: u8,
    #[serde(rename = "influencia")]
    pub influence: u8,
    #[serde(rename = "perspicacia")]
    pub insight: u8,
    #[serde(rename = "compostura")]
    pub composure: u8,
    #[serde(rename = "percepcao")]
    pub perception: u8,
    #[serde(rename = "inteligencia")]
    pub intelligence: u8,
    #[serde(rename = "raciocinio")]
    pub wits: u8,
}

#[derive(Deserialize, Clone, PartialEq)]
pub struct Attack {
    #[serde(rename = "nome")]
    pub name: String,
    #[serde(rename = "dado")]
    pub die: Option<String>,
    #[serde(rename = "tipoDano")]
    pub damage_type: Option<String>,
}

#[derive(Deserialize, Clone, PartialEq)]
pub struct Creature {
    pub id: String,
    #[serde(rename = "nome")]
    pub name: String,
    #[serde(rename = "ameaca")]
    pub threat: u8,
    #[serde(rename = "centelha")]
    pub spark: u8,
    #[serde(rename = "vontade")]
    pub willpower: u8,
    #[serde(rename = "atributos")]
    pub attributes: Attributes,
    #[serde(rename = "ataques", default)]
    pub attacks: Vec<Attack>,
}

const SOURCES: [&str; 3] = [
    include_str!("../assets/aboleth.json"),
    include_str!("../assets/aasimar.json"),
    include_str!("../assets/ankheg.json"),
];

// Pasta copiada sem reprocessar: o otimizador de imagem do dx regrava WebP com perda como WebP sem perda (8x maior).
const IMAGES: Asset = asset!("/assets/img", AssetOptions::folder());
const STYLE: Asset = asset!("/assets/style.css");

fn creatures() -> Vec<Creature> {
    SOURCES
        .iter()
        .map(|s| serde_json::from_str(s).expect("JSON de criatura inválido"))
        .collect()
}

fn image_for(id: &str) -> String {
    format!("{IMAGES}/{id}_criatura.webp")
}

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    #[route("/")]
    Index {},
    #[route("/criatura/:id")]
    CreaturePage { id: String },
}

// O CLI chama este endpoint no `dx bundle --ssg` para saber quais rotas pré-renderizar.
#[server(endpoint = "static_routes", output = server_fn::codec::Json)]
async fn static_routes() -> Result<Vec<String>, ServerFnError> {
    let mut routes: Vec<String> = Route::static_routes()
        .iter()
        .map(ToString::to_string)
        .collect();
    for c in creatures() {
        routes.push(Route::CreaturePage { id: c.id }.to_string());
    }
    Ok(routes)
}

fn main() {
    dioxus::LaunchBuilder::new()
        .with_cfg(server_only! {
            ServeConfig::builder()
                .incremental(
                    dioxus::server::IncrementalRendererConfig::new()
                        .static_dir(
                            std::env::current_exe()
                                .unwrap()
                                .parent()
                                .unwrap()
                                .join("public"),
                        )
                        .clear_cache(false),
                )
                .enable_out_of_order_streaming()
        })
        .launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Stylesheet { href: STYLE }
        Router::<Route> {}
    }
}

#[component]
fn Index() -> Element {
    rsx! {
        main { class: "pagina",
            h1 { "Bestiário (prova de conceito)" }
            ul {
                for c in creatures() {
                    li {
                        Link { to: Route::CreaturePage { id: c.id.clone() }, "{c.name}" }
                    }
                }
            }
            canvas::CanvasDemo {}
        }
    }
}

#[component]
fn CreaturePage(id: String) -> Element {
    let found = creatures().into_iter().find(|c| c.id == id);
    rsx! {
        main { class: "pagina",
            Link { to: Route::Index {}, "Voltar ao índice" }
            match found {
                Some(c) => rsx! { CreatureCard { creature: c } },
                None => rsx! { p { "Criatura não encontrada." } },
            }
        }
    }
}

fn roll_d6() -> u8 {
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Math::random() * 6.0).floor() as u8 + 1
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        (nanos % 6) as u8 + 1
    }
}

#[component]
fn CreatureCard(creature: Creature) -> Element {
    let mut roll = use_signal(|| None::<u8>);
    let a = &creature.attributes;
    let attrs = [
        ("Força", a.strength),
        ("Destreza", a.dexterity),
        ("Vigor", a.stamina),
        ("Influência", a.influence),
        ("Perspicácia", a.insight),
        ("Compostura", a.composure),
        ("Percepção", a.perception),
        ("Inteligência", a.intelligence),
        ("Raciocínio", a.wits),
    ];
    rsx! {
        article { class: "cartao",
            h1 { "{creature.name}" }
            img {
                class: "retrato",
                src: image_for(&creature.id),
                alt: "{creature.name}",
                width: "600",
                height: "400",
            }
            dl { class: "numeros",
                dt { "Ameaça" }
                dd { "{creature.threat}" }
                dt { "Centelha" }
                dd { "{creature.spark}" }
                dt { "Força de Vontade" }
                dd { "{creature.willpower}" }
                dt { "Defesa" }
                dd { "(motor, fase 1)" }
            }
            h2 { "Atributos" }
            ul { class: "atributos",
                for (label, value) in attrs {
                    li {
                        span { "{label}" }
                        strong { "{value}" }
                    }
                }
            }
            h2 { "Ataques" }
            ul {
                for atk in creature.attacks.iter() {
                    li {
                        "{atk.name}"
                        if let Some(d) = &atk.die {
                            ", {d}"
                        }
                        if let Some(t) = &atk.damage_type {
                            " ({t})"
                        }
                    }
                }
            }
            div { class: "rolagem",
                button { onclick: move |_| roll.set(Some(roll_d6())), "Rolar 1d6" }
                match roll() {
                    Some(v) => rsx! {
                        output { "Resultado: {v}" }
                    },
                    None => rsx! {
                        output { "Ainda sem rolagem" }
                    },
                }
            }
        }
    }
}
