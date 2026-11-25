# Spike B · Dioxus 0.7.10 (25/11/2026)

Pasta: `centelha-rs\spike\dioxus\`. Um único crate (`centelha-dx`) com as features `web`, `server`, `desktop` e `mobile`; o mesmo código serve a web estática, o Windows e o Android. Os logs completos estão em `logs\`.

## Tabela de medidas

| # | Item | Resultado |
|---|------|-----------|
| 1 | Web estática com hidratação | **Funciona, recurso oficial**, com dois defeitos do 0.7.10 (abaixo). Comando que funciona: `dx build --web --ssg --release --force-sequential --debug-symbols false`. Gera `index.html`, `criatura/{aboleth,aasimar,ankheg}/index.html` com o texto inteiro do cartão no HTML (conferido com JavaScript desligado no Chrome headless) e hidrata: o botão "Rolar 1d6" responde na página estática (cliques deram 5, 2, 2, 6). Maturidade: média. |
| 2 | WebAssembly (release) | Saída do cargo: 2 385 593 B. Depois do wasm-bindgen, sem wasm-opt: 937 755 B (gzip -9: 280 631 B). **Final, com o wasm-opt que o próprio dx roda (-Oz): 575 181 B, gzip -9: 232 971 B.** Conferência manual com `wasm-opt -Oz` (binaryen baixado pelo dx em `~\.dx\tools\binaryen-129`, `wasm-opt --version` responde "version 127"): 575 551 B, gzip 232 652 B. JS de cola: 43 011 B (gzip -9: 12 065 B). Os 3 JSON entram no wasm por `include_str!` (cerca de 29 KB crus). |
| 3 | Compilação web | Limpo (`rm -rf target`, release, cliente wasm + servidor nativo em paralelo, inclui ~20 s de "Waiting for cargo-metadata"): **236 s**. `rustc` rodando antes: 2 (na 1ª tentativa, 1). Incremental (troca de um texto no `rsx!`, cliente + servidor + SSG em sequência): **67 s**. Só regerar o SSG sem mudança de código: 6 a 8 s. |
| 4 | Windows | `dx build --platform desktop --release`: **213 s** (com 6 `rustc` rodando na máquina, o outro agente compilando junto). `centelha-dx.exe` **2,96 MB** (opt-level z, LTO, sem strip) + pasta `assets\` (imagens e CSS soltos ao lado do exe). Nada a instalar (WebView2 já existe). Abriu, mostrou o índice, o clique em "Aboleth" abriu o cartão com a imagem (capturas em `logs\desktop_indice.png` e `logs\desktop_cartao.png`); 22 MB de memória. Instalador: `dx bundle --desktop` gera NSIS por padrão (MSI com `--package-types msi`); o dx baixa sozinho NSIS 3.11 e WiX 3.14 e o instalador do WebView2 para `~\.dx\tools`. Não gerei. |
| 5 | Android | **APK gerado.** Depuração (`dx build --platform android`, x86_64 por padrão): 449 s, APK 90,6 MB (`libmain.so` de depuração com 306 MB crus). Release arm64 (`--release --target aarch64-linux-android --fullstack false`): **APK 19,9 MB**, `libmain.so` 24,7 MB crus / 8,4 MB comprimidos, assinado com a chave de depuração (variante `debug` do Gradle). Nada pediu licença nem clique. Defeito: com `lto = true` o link do release Android falha no Windows (o dx perde as barras invertidas dos caminhos ao repassar para o `clang`); resolvido com `[profile.android-release] lto = false`. |
| 6 | Canvas | Funciona (pixel conferido no Chrome: 181,72,42 = `#b5482a`). O elemento vem do evento `onmounted`: `evt.as_web_event()` (trait `dioxus::web::WebEventExt`) devolve o `web_sys::Element`. Só na web; no desktop/Android o mesmo `onmounted` não dá elemento DOM (teria de ser por `document::eval` com JS). |
| 7 | Documentação e clareza | Documentação oficial **3/5**; mensagens de erro **3/5**; peças **4/5** (só `dx`, que baixa sozinho wasm-bindgen, esbuild, wasm-opt, NSIS/WiX e chama o Gradle); código do cartão **4/5**. Detalhes abaixo. |

## Como o Dioxus faz a geração estática

Recurso oficial do 0.7 ("Static Site Generation" em dioxuslabs.com/learn/0.7/essentials/fullstack/static_site_generation):

1. `dioxus` com a feature `fullstack` (+ `router`); features do crate `web = ["dioxus/web"]` e `server = ["dioxus/server"]`.
2. `LaunchBuilder` com `server_only! { ServeConfig::builder().incremental(IncrementalRendererConfig::new().static_dir(<exe>/public).clear_cache(false)) }`.
3. Uma função de servidor com endpoint fixo que lista as rotas: `#[server(endpoint = "static_routes", output = server_fn::codec::Json)]`. `Route::static_routes()` só devolve rotas sem parâmetro; as rotas `/criatura/:id` eu acrescentei à mão a partir dos JSON.
4. O CLI compila o servidor, sobe o executável em `127.0.0.1:9999`, chama `/api/static_routes`, pede cada rota e grava o HTML em `target\dx\centelha-dx\release\web\public\`. O HTML sai com marcas `data-node-hydration` e um `window.initial_dioxus_hydration_data`; o wasm hidrata em cima dele.

Defeitos encontrados no 0.7.10 (conferidos no código-fonte da tag v0.7.10):

- **A doc manda usar `dx bundle --web --ssg`, mas o `bundle` ignora o `--ssg`.** No `packages/cli/src/cli/bundle.rs` há literalmente `// todo: make sure to run pre-render static routes! we removed this from the other bundling step`. Só o `dx build --ssg` pré-renderiza.
- **Corrida entre cliente e servidor:** sem `--force-sequential`, o cliente termina depois do SSG e sobrescreve o `index.html` pré-renderizado da raiz com o esqueleto vazio (as páginas das criaturas ficam certas). O ramo principal do Dioxus já força a ordem quando há `--ssg`; no 0.7.10 é preciso passar `--force-sequential`.
- **`--debug-symbols` vale `true` por padrão, mesmo em `--release`.** O wasm-opt roda, mas com `--debuginfo`, e o wasm fica com 936 KB em vez de 575 KB. Tem de passar `--debug-symbols false`.
- **Imagens WebP são reprocessadas e crescem 8 vezes.** O otimizador de assets do dx decodifica e regrava qualquer `.webp` (com `asset!`, com `AssetOptions::builder()`, com `AssetOptions::folder()`, até pelo `public_dir`) pelo crate `image`, que só grava WebP sem perda: 34 KB viraram 282 KB. Não achei opção para copiar sem tocar. No spike as imagens ficam numa pasta registrada com `AssetOptions::folder()` (sem hash no nome) e eu copio os originais por cima depois do build (`cp assets/img/*.webp <public>/assets/img/`). O desktop e o APK foram com as versões regravadas (248 a 315 KB cada).
- Assets antigos não são limpos: cada build deixa o wasm e o JS anteriores (com outro hash) em `public/assets`. Para publicar, apagar `public` antes ou copiar só o que o HTML referencia.
- `strip = true` no perfil release quebra o build web (o wasm-bindgen 0.2.130 recusa um wasm sem a seção `target_features`). Ficou `strip = false`; o wasm-opt tira o que precisa.
- O derive `Routable` emite um aviso "unreachable call" em toda compilação (inofensivo).

Maturidade: o caminho existe, é oficial e o resultado é bom (HTML legível sem JS, hidratação sem erro no console), mas pede quatro bandeiras e um contorno para imagens que a documentação não menciona. Para o Pagefind, o HTML gerado serve como está.

## Comandos exatos

Sessão (bash), sempre com `export PATH="$HOME/.cargo/bin:$PATH" CARGO_BUILD_JOBS=6`.

```bash
# web: SSG + hidratação (o que funciona no 0.7.10)
dx build --web --ssg --release --force-sequential --debug-symbols false
cp assets/img/*.webp target/dx/centelha-dx/release/web/public/assets/img/
# servir para testar
cd target/dx/centelha-dx/release/web/public && python -m http.server 8765

# compilação limpa medida (a doc indica este comando; ele compila tudo mas NÃO roda o SSG)
rm -rf target && dx bundle --web --ssg --release

# medidas do wasm
wasm-bindgen --target web --out-dir target/medidas target/wasm32-unknown-unknown/wasm-release/centelha-dx.wasm
~/.dx/tools/binaryen-129/bin/wasm-opt.exe -Oz --enable-bulk-memory --enable-nontrapping-float-to-int \
  --enable-sign-ext --enable-mutable-globals --enable-reference-types --enable-multivalue \
  target/medidas/centelha-dx_bg.wasm -o target/medidas/oz.wasm
gzip -9 -c arquivo.wasm | wc -c

# desktop
dx build --platform desktop --release
# -> target/dx/centelha-dx/release/windows/app/centelha-dx.exe

# android (variáveis só na sessão)
export JAVA_HOME="C:\\Program Files\\Android\\Android Studio\\jbr" ANDROID_HOME="$LOCALAPPDATA\\Android\\Sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME\\ndk\\28.2.13676358" NDK_HOME="$ANDROID_HOME\\ndk\\28.2.13676358"
dx build --platform android                                   # depuração, x86_64
dx build --platform android --release --target aarch64-linux-android --fullstack false
# -> target/dx/centelha-dx/<debug|release>/android/app/app/build/outputs/apk/debug/app-debug.apk
```

Teste de hidratação: script Node que abre um Chrome headless próprio via CDP (o MCP do DevTools não conectou), carrega a página com JavaScript desligado e lê o `innerText`, depois recarrega com JavaScript, clica no botão e lê o `<output>`, e lê um pixel do `<canvas>` no índice.

## Código do cartão

Tipos (chaves em português pelo `serde(rename)`):

```rust
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
    pub attributes: Attributes, // 9 campos u8, mesmo padrão
    #[serde(rename = "ataques", default)]
    pub attacks: Vec<Attack>,
}
```

Componente:

```rust
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
```

`roll_d6()` usa `js_sys::Math::random` no wasm e os nanossegundos do relógio no nativo. CSS da imagem: `.retrato { width: 100%; height: auto; aspect-ratio: 3 / 2; }`, sem `object-fit`.

Canvas (`src/canvas.rs`):

```rust
canvas {
    width: "120",
    height: "120",
    onmounted: move |evt| {
        use dioxus::web::WebEventExt;
        use wasm_bindgen::JsCast;
        let canvas: web_sys::HtmlCanvasElement = evt.as_web_event().dyn_into().unwrap();
        let ctx: web_sys::CanvasRenderingContext2d =
            canvas.get_context("2d").unwrap().unwrap().dyn_into().unwrap();
        ctx.set_fill_style_str("#b5482a");
        ctx.fill_rect(20.0, 20.0, 80.0, 80.0);
    },
}
```

## Avaliação (1 a 5)

- **Documentação oficial: 3.** O guia do 0.7 tem a página de SSG com código que compila, mas o comando indicado (`dx bundle --ssg`) não pré-renderiza, nada fala de rotas com parâmetro, de `--force-sequential` nem de `--debug-symbols`. A página de mobile cita `NDK_HOME` e não diz onde sai o APK. Tive de ler o código-fonte do CLI em quatro pontos.
- **Mensagens de erro: 3.** Os erros do `rustc`/`rsx!` vêm limpos. Os do CLI são piores: o `strip = true` gerou uma cadeia de três erros onde o real (wasm-bindgen) ficou no meio; o link do Android com LTO só mostra caminhos sem barras, sem indicar que o defeito é do próprio dx. Os defeitos silenciosos (SSG ignorado no `bundle`, `index.html` sobrescrito, wasm com debuginfo, WebP inchado) não avisam nada.
- **Peças: 4.** Uma ferramenta (`dx`) para web, desktop e Android, um crate, um `Dioxus.toml` de 9 linhas. O dx baixa wasm-bindgen, esbuild, binaryen, NSIS/WiX e gera o projeto Gradle. Do lado de fora só ficam o JDK e o SDK/NDK do Android.
- **Código do cartão: 4.** `rsx!` com `for`, `if let` e `match` direto no markup é legível; o mesmo componente roda nos três alvos sem mudança. Perde um ponto pelas pegadinhas de alvo (`cfg(target_arch)` para random e canvas).

## O que ficou de fora

- Instalador do Windows (NSIS/MSI) não gerado; só conferido o que o `dx bundle` usa.
- APK não instalado em aparelho nem emulador. O release usou `lto = false` por causa do defeito do link.
- Canvas no desktop/Android (precisaria de `document::eval` com JS).
- O contorno das imagens WebP foi manual (cópia depois do build); um projeto real precisaria de um passo de pós-build ou de outra extensão de arquivo.
- Não medi tempo de carga no navegador nem Pagefind de fato (só que o texto está no HTML).
