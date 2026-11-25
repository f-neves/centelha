//! Writes the static pages (target/site) and exits; no server keeps running.

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use leptos::prelude::*;
    use leptos_axum::generate_route_list_with_ssg;
    use oficial::shell;

    let conf = get_configuration(None).unwrap();
    let options = conf.leptos_options;
    let (_routes, static_routes) = generate_route_list_with_ssg({
        let options = options.clone();
        move || shell(options.clone())
    });
    static_routes.generate(&options).await;
    println!("paginas estaticas gravadas em {}", options.site_root);
}

#[cfg(not(feature = "ssr"))]
pub fn main() {}
