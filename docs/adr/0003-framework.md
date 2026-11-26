# 0003 · Framework do site e do app: Leptos e Tauri 2

Data: 25/11/2026. Estado: aceita (decisão do Coordenador-RS pelo spike da fase 0).

## Contexto

O site precisa de páginas estáticas (o livro tem de ser lido e indexado pela busca sem WebAssembly), de partes interativas em WebAssembly (ficha, bestiário, catálogos, rolador, mesa) e de um tabuleiro em canvas que é o objetivo número um do projeto: desempenho do Grid e da mesa. O app tem de rodar no Windows e no Android.

O spike da fase 0 comparou dois caminhos com a mesma página de prova (o cartão de uma criatura com hidratação, canvas, Windows e APK do Android):

| | Leptos 0.8.23 + Tauri 2.12.7 | Dioxus 0.7.10 |
|---|---|---|
| HTML estático com o texto | gerador próprio, pequeno | recurso oficial, com defeitos contornados por bandeiras |
| WebAssembly final (gzip) | 106 KB | 233 KB |
| Compilação web: limpa / ao mudar um texto | 98 s / 5 s | 236 s / 67 s |
| Canvas | web e app (o app roda o mesmo WebAssembly no WebView) | só na web; no desktop e no Android o código roda nativo, sem DOM |
| Imagens | copiadas como estão | a ferramenta regrava todo WebP, sem opção para desligar |
| Android | APK release de 10 MB | APK de 20 MB |

## Decisão

1. **Leptos** para os componentes do site e do Grid; **Tauri 2** para a casca do app de Windows e Android. O app carrega o mesmo site gerado (as mesmas páginas e o mesmo WebAssembly) dentro do WebView. O Grid em canvas é um código só para a web, o Windows e o Android.
2. **Versões fixadas** no `Cargo.toml` do workspace (`[workspace.dependencies]`, com `=`) e no `Cargo.lock`, conferidas no crates.io em 25/11/2026: `leptos` 0.8.23, `leptos_meta` 0.8.7, `wasm-bindgen` 0.2.130, `web-sys` e `js-sys` 0.3.107, `tauri` 2.12.7 (com `tauri-build` 2.7.6 e o `tauri-cli` 2.12.6). Fora do Cargo: o `wasm-bindgen-cli` tem de ser exatamente a versão do crate `wasm-bindgen` (0.2.130); o `wasm-opt` é o do binaryen `version_133`. Subir qualquer uma delas é commit próprio, com a versão nova citada aqui.
3. **Site estático gerado por nós, sem servidor.** Cada página sai pronta em HTML no build, com o texto inteiro. O gerador é um binário nativo do `centelha-web` (feature `ssr`), que desenha os componentes com `to_html()` e grava `dist/`. O `cargo xtask site` só orquestra (gerador, build do WebAssembly, `wasm-bindgen --target web`, `wasm-opt -Oz`) chamando programas, para o `xtask` continuar sem dependências externas (ADR 0001). Não usamos o roteador do Leptos no cliente nem as rotas estáticas oficiais: elas pressupõem servidor e quebram num host estático e no app. A navegação entre páginas é por link comum.
4. **Ilhas de interatividade.** O texto do livro é HTML que o WebAssembly não toca. Só as partes interativas (o rolador, um cartão de ficha, depois o Grid) são ilhas (`#[island]`, feature `islands` do Leptos) e hidratam no navegador. Assim a hidratação não tem como duplicar nem apagar o texto, e o WebAssembly não carrega o conteúdo das páginas. Os dados de que uma ilha precisa vão nas propriedades da ilha, gravadas no HTML pelo gerador. Se as ilhas falharem no gerador próprio, a alternativa provada no spike é a hidratação da página inteira com os dados num `<script type="application/json">`; trocar exige nota neste ADR.
5. **O Markdown vira HTML só no gerador** (nativo). A biblioteca de Markdown não entra no WebAssembly. O gerador reproduz o pipeline do site atual: blocos de diagrama trocados pelo SVG já desenhado em `content/data/diagramas.json`, o caminho base nos links que começam com `/` e tabelas envolvidas para rolar no celular.
6. **Caminho base por configuração** (`SITE_BASE_PATH`, padrão `/centelha/` na prévia do GitHub Pages; `/` com o domínio próprio e no app).
7. **Duas provas obrigatórias no CI**, porque o gerador próprio é o ponto frágil desta escolha:
   - cada página gerada tem o texto completo sem WebAssembly: o teste compara o texto do HTML gravado com o texto que sai do Markdown de origem, página por página;
   - a hidratação não duplica nem apaga conteúdo: num navegador sem janela, o texto visível de cada página antes de carregar o WebAssembly e depois de as ilhas hidratarem é o mesmo, e o console não tem erro de hidratação.
8. **Imagens passam como estão** (R10): o gerador copia `content/public/` sem recodificar. Nunca `object-fit: cover`; o bestiário fica em 3:2 com `width: 100%; height: auto`.

## Alternativas

- **Dioxus 0.7:** uma ferramenta só e site estático oficial, mas o canvas fora da web passaria por JavaScript (o Grid seria dois códigos), a ferramenta regrava as imagens e o WebAssembly sai com o dobro do tamanho. O site estático oficial estava com defeito na versão do dia.
- **Rotas estáticas oficiais do Leptos (`cargo leptos`):** testadas no spike; geram o HTML, mas o roteador do cliente pede ao servidor cada página seguinte e dá página em branco num host estático.
- **Hidratação da página inteira:** provada no spike e mantida como alternativa; com ilhas o WebAssembly fica menor e o texto do livro não depende dele.

## Consequências

- Mais peças que o Dioxus: cargo, `wasm-bindgen-cli`, `wasm-opt` e `tauri-cli`. A montagem fica no `cargo xtask site` e no CI (Infraestrutura), com as versões deste ADR.
- O gerador é nosso e precisa de cuidado: as duas provas do item 7 entram com a primeira página e não saem mais.
- O app é o site dentro do WebView: o que o app tem a mais (conteúdo offline, sincronização, atualização) fica no `centelha-app` e nos comandos do Tauri, sem duplicar página.
- O `wasm-opt --all-features` do binaryen `version_133` gera um WebAssembly que o navegador recusa; a lista de features vai explícita no `xtask`.
