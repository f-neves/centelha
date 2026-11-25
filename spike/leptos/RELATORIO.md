# Spike A · Leptos 0.8 + Tauri 2 (25/11/2026)

Pasta: `centelha-rs\spike\leptos\`. Página de prova: cartão de criatura (aboleth, aasimar, ankheg) mais índice, com botão "Rolar 1d6" e um quadrado desenhado em `<canvas>`.

## Tabela de medidas

| # | Item | Resultado |
|---|------|-----------|
| 1 | Web estática com hidratação | Funciona. Caminho principal **feito à mão** (binário `ssg` com feature `ssr` grava o HTML com marcadores; biblioteca com `hydrate` em wasm32 + `wasm-bindgen --target web`). Caminho **oficial** (rotas estáticas do `leptos_router` + `cargo leptos`) também testado em `oficial\`: gera o HTML, mas depende de servidor axum e quebra a navegação num host estático (ver abaixo). Texto do cartão está no HTML (legível sem JS); Pagefind 1.3.0 indexou 4 páginas / 47 palavras. Clique e canvas conferidos no Edge headless e no WebView2 do Tauri. |
| 2 | Wasm (release, `opt-level="z"`, LTO, `codegen-units=1`, `panic=abort`) | saída do cargo: 2.099.937 B · depois do wasm-bindgen: 991.428 B (quase tudo seção de nomes/debug; só despido sem otimizar: 278.235 B) · **depois do `wasm-opt -Oz` (binaryen version_133): 244.113 B** · **gzip -9: 105.715 B** · JS de cola: 33.847 B (gzip 6.725 B). Caminho oficial (inclui router + cliente de server function, wasm-opt version_123 embutido no cargo-leptos): 445.344 B, gzip 178.763 B. |
| 3 | Compilação web | `rustc` rodando antes: 3 (outro agente). **Limpo** (wasm release, `--target-dir target-web`): **98 s**. **Incremental** (mudar um texto em `card`): **5,3 s** só o wasm; pipeline inteiro (`build-web.sh`: gerador nativo release + wasm + bindgen + wasm-opt) 20,8 s. Gerador nativo, primeira compilação (debug, com dependências): 120 s. Caminho oficial limpo (`cargo leptos build --release`, servidor + wasm): ~197 s, com um build Android rodando junto. |
| 4 | Windows (Tauri 2.12.7, `cargo tauri build --no-bundle`) | **exe 2.917.376 B (2,8 MB)**, mesmo perfil de release do workspace. Limpo: **199 s** (0 `rustc` alheios no início). Incremental (mudança de config, relink com LTO fat): 81 s. Nada novo instalado (MSVC + WebView2 já existiam). Instalador: NSIS e MSI (WiX 3) são baixados pelo próprio tauri-cli no `cargo tauri build`; o MSI precisa do recurso VBScript do Windows. Não gerado. |
| 5 | Android | **APK gerado.** `cargo tauri android init --ci` (instalou sozinho os alvos rustup armv7/i686/x86_64) e `cargo tauri android build --debug --apk --target aarch64`: **402 s** (inclui baixar o Gradle 9.6.1; o Rust compila duas vezes, uma no CLI e outra pela tarefa do Gradle). APK debug: **130.862.339 B** (a `.so` debug sem strip tem 123,6 MB). Bônus: `--apk` release (não assinado) aarch64: **10.272.255 B**, 252 s. Nenhum aceite de licença pedido. Não rodado em aparelho. |
| 6 | Canvas | Funciona (pixel (20,20) lido como `181,72,44,255`). Elemento via `NodeRef::<Canvas>` ligado com `node_ref=canvas`; desenho num `Effect`, que só roda no cliente. |
| 7 | Documentação e clareza | Docs oficiais 3/5 · mensagens de erro 3/5 · peças 3/5 · código do cartão 4/5 (detalhes abaixo). |

## Como a geração estática funciona

### Caminho principal (feito à mão, usado nas medidas)
- `card\` (lib `cdylib`+`rlib`): tipos com `serde(rename)` e componentes. Feature `ssr` → `leptos/ssr`; `hydrate` → `leptos/hydrate`; `entry` exporta a função `hydrate()` para o JS.
- `ssg\` (binário nativo): para cada página cria `Owner::new_root(Some(Arc::new(SsrSharedContext::new())))` e chama `view! { <App page/> }.to_html()`. O resultado já sai com os marcadores de hidratação (`<!>`). Grava `dist\index.html`, `dist\<id>.html` e copia as imagens.
- Os dados da página vão serializados num `<script id="page-data" type="application/json">` no `<head>`; o cliente lê, desserializa e chama `leptos::mount::hydrate_body`. Assim o wasm não carrega o bestiário inteiro (escala para centenas de criaturas).
- Escolhi este caminho porque o `cargo-leptos` não estava instalado no início (outro processo o compilava) e porque o caminho oficial exige um servidor; depois testei o oficial também.

### Caminho oficial (testado em `oficial\`)
- `Route ... ssr=SsrMode::Static(StaticRoute::new().prerender_params(...))` no `leptos_router`, mais um binário axum/tokio que chama `leptos_axum::generate_route_list_with_ssg` e `static_routes.generate(&options).await`. O exemplo oficial (`examples/static_routing`) gera os arquivos ao iniciar o servidor e continua servindo; aqui o binário grava e sai. Saída: `target\site\<id>\index.html`.
- Os dados vêm por `#[server]` + `Resource::new_blocking`; o valor é embutido no HTML (`__RESOLVED_RESOURCES`), então a primeira carga hidrata sem servidor.
- Problemas achados: (a) o `<Router>` intercepta todo `<a>` e faz navegação no cliente, que chama `POST /api/get_creature...`; num host estático (ou no Tauri) isso dá 501 e página em branco. Precisa de `rel="external"` nos links ou de um servidor de verdade. (b) O HTML sai com `crossorigin="<nonce>"` nos preloads e scripts depois de `</html>` (o navegador aceita). (c) Conflito: duas crates exportando `hydrate` derruba o wasm-bindgen ("the name `hydrate` is exported by multiple crates"); resolvido com a feature `entry`.
- Maturidade: o SSG do Leptos é pensado para "servidor com páginas pré-renderizadas e regeneração", não para exportar um site 100% estático. Funciona, mas o caminho à mão é mais simples e mais previsível para Pagefind + Tauri.

## Comandos exatos

Sessão (bash do Git): `export PATH="$USERPROFILE/.cargo/bin:$PATH" CARGO_BUILD_JOBS=6`

Web (script `build-web.sh`):
```
cargo run -p ssg --release
cargo build -p card --lib --target wasm32-unknown-unknown --release --features entry --target-dir target-web
wasm-bindgen --target web --no-typescript --out-dir dist/pkg target-web/wasm32-unknown-unknown/release/card.wasm
tools/binaryen-version_133/bin/wasm-opt.exe -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals --enable-reference-types --enable-multivalue -o dist/pkg/card_bg.wasm dist/pkg/card_bg.wasm
gzip -9 -c dist/pkg/card_bg.wasm | wc -c
npx -y pagefind@1.3.0 --site dist --output-path <pasta de teste>
```
Windows:
```
cargo tauri init --ci -A centelha-leptos -W "Bestiário" -D ../dist -P http://127.0.0.1:8731 --before-dev-command "" --before-build-command ""
cargo tauri build --no-bundle
```
Android (variáveis só na sessão):
```
export JAVA_HOME="C:\Program Files\Android\Android Studio\jbr" ANDROID_HOME="D:\Android\Sdk" NDK_HOME="D:\Android\Sdk\ndk\28.2.13676358"
cargo tauri android init --ci
cargo tauri android build --debug --apk --target aarch64
cargo tauri android build --apk --target aarch64
```
Caminho oficial: `cd oficial && cargo leptos build --release && ./target/release/oficial.exe`

Teste de hidratação: Edge headless com `--remote-debugging-port` e um script Node (CDP) que abre a página, clica três vezes no botão, lê o `<output>` e um pixel do canvas. No Tauri, o mesmo script via `additionalBrowserArgs: "--remote-debugging-port=9444"` posto temporariamente no `tauri.conf.json` (já retirado; a variável `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` não teve efeito).

## Código do cartão (`card\src\lib.rs`)

```rust
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
                {creature.attributes.rows().into_iter()
                    .map(|(label, value)| view! { <tr><th>{label}</th><td>{value}</td></tr> })
                    .collect_view()}
            </tbody></table>
            <h2>"Ataques"</h2>
            <ul class="attacks">
                {creature.attacks.into_iter().map(|attack| {
                    let details = [attack.die, attack.damage_type].into_iter().flatten()
                        .collect::<Vec<_>>().join(", ");
                    let details = (!details.is_empty()).then(|| format!(" ({details})"));
                    view! { <li>{attack.name}{details}</li> }
                }).collect_view()}
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
```
Tipos: `Creature { id, name←nome, threat←ameaca, spark←centelha, willpower←vontade, attributes←atributos, attacks←ataques }`, `Attack { name←nome, die←dado: Option, damage_type←tipoDano: Option }`, `Attributes` com os 9 campos renomeados.

## Avaliação (1 a 5)
- **Documentação oficial: 3.** O livro (book.leptos.dev) é bom para componentes, sinais e SSR com servidor, mas não tem capítulo de SSG/rotas estáticas; foi preciso ler o exemplo `static_routing` no GitHub e o código do `leptos` (`mount.rs`, `hydration/mod.rs`) para montar o caminho à mão. O docs.rs de `static_routes` lista os tipos quase sem explicação.
- **Mensagens de erro: 3.** As do `view!` são legíveis; as de tipo em views grandes ficam longas. O conflito de `hydrate` só aparece no wasm-bindgen, depois de 2,5 min de compilação.
- **Peças: 3.** À mão: cargo + wasm-bindgen + wasm-opt (binaryen baixado à parte, version_133) + cargo-tauri. Oficial: cargo-leptos (traz wasm-opt próprio) + axum + tokio. Tauri adiciona Gradle/JDK/NDK no Android, mas o CLI cuidou de tudo.
- **Código do cartão: 4.** `view!` parece HTML, `serde` direto nos tipos, `Option` nos ataques sem ruído.

## Problemas encontrados
- `wasm-opt --all-features` (version_133) liga "compact imports" e gera um wasm que o Chrome/Edge recusa (`Invalid import kind 127`). Use a lista explícita de features.
- `<tr>` direto em `<table>` quebra a hidratação (o navegador insere `<tbody>`); usar `<tbody>` explícito.
- Sem `wasm-opt`, o wasm do wasm-bindgen fica com 991 KB (seção de nomes); o tamanho real só aparece depois do `-Oz`.
- Incremental do Tauri com LTO fat e `codegen-units=1` é lento (81 s); para desenvolvimento, usar `cargo tauri dev` ou um perfil sem LTO.
- APK debug enorme (131 MB) por causa dos símbolos de debug da `.so`; o release não assinado tem 10 MB.
- Ao encerrar o teste, matei os processos `msedge.exe` com `taskkill /IM msedge.exe /F` para fechar o Edge headless; se havia um Edge aberto pelo usuário, ele também foi fechado.

## O que ficou de fora
- Ilhas (`islands`): reduziriam o wasm (só o botão hidrata), não medido.
- `--cfg erase_components` (prometido para wasm menor e compilação mais rápida), não medido.
- Instalador MSI/NSIS não gerado; APK não rodado em aparelho; só o alvo aarch64 compilado no Android.
- Verificação de avisos de hidratação em build debug (em release o clique e o canvas funcionaram, sinal de que a árvore casou).

## Arquivos
- `card\` componentes e tipos · `ssg\` gerador · `dist\` site gerado · `src-tauri\` app Tauri (gen\android incluso) · `oficial\` caminho oficial · `build-web.sh` · `tools\binaryen-version_133\` · `logs\` saídas de build.
