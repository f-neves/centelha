# 0003 · Framework do site e do app: Leptos e Tauri 2

Data: 25/11/2026. Estado: aceita (decidida pelo relatório do spike da fase 0).

## Contexto

O site precisa de páginas estáticas (o livro tem de ser lido e indexado pela busca sem WebAssembly), de partes interativas em WebAssembly (ficha, bestiário, catálogos, rolador, mesa) e de um tabuleiro em canvas que é o objetivo número um do projeto: desempenho do Grid e da mesa. O app tem de rodar no Windows e no Android.

O spike da fase 0 comparou dois caminhos com a mesma página de prova (o cartão de uma criatura com hidratação, canvas, Windows e APK do Android). Os números de tamanho e de compilação web saíram com o perfil do spike (`opt-level = "z"`, LTO completo, `codegen-units = 1`, `panic = "abort"`), não com o perfil de release deste workspace:

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
2. **Versões fixadas** no `Cargo.toml` do workspace (`[workspace.dependencies]`, com `=`) e no `Cargo.lock`, conferidas no crates.io em 25/11/2026: `leptos` 0.8.23, `leptos_meta` 0.8.7, `wasm-bindgen` 0.2.130, `web-sys` e `js-sys` 0.3.107, `pulldown-cmark` 0.13.6 (Markdown, só no gerador), `tauri` 2.12.7 (com `tauri-build` 2.7.6 e o `tauri-cli` 2.12.6). Fora do Cargo: o `wasm-bindgen-cli` tem de ser exatamente a versão do crate `wasm-bindgen` (0.2.130) e o `wasm-opt` é o do binaryen `version_133`; o `cargo xtask site` confere as duas versões antes de montar e para com mensagem clara se não baterem. Dependência nova (por exemplo `serde` nas propriedades das ilhas, SHA-1 para a chave dos diagramas) entra com a versão citada no `CONTRACTS.md`. Subir qualquer versão é commit próprio, com a versão nova citada aqui.
3. **Site estático gerado por nós, sem servidor.** Cada página sai pronta em HTML no build, com o texto inteiro. O gerador é um binário nativo do `centelha-web` (feature `ssr`), que desenha os componentes com `to_html()` e grava `dist/`. O `cargo xtask site` só orquestra (gerador, build do WebAssembly, `wasm-bindgen --target web`, `wasm-opt -Oz`) chamando programas, para o `xtask` continuar sem dependências externas (ADR 0001). Não usamos o roteador do Leptos no cliente nem as rotas estáticas oficiais: elas pressupõem servidor e quebram num host estático e no app. A navegação entre páginas é por link comum.
4. **Ilhas de interatividade** (feature `islands` do Leptos). O texto do livro é HTML que o WebAssembly não toca; só as partes interativas (o seletor de tema, depois a ficha e o Grid) são ilhas (`#[island]`) e hidratam no navegador. O esperado é que a hidratação não mexa no texto e que o WebAssembly fique menor que no spike, porque não carrega o conteúdo das páginas; o spike não mediu ilhas, então quem confere é a prova do item 7 e a medida da primeira tarefa do Frontend. O gerador próprio precisa, no modo ilhas, do que o spike não exercitou: o contexto de render `SsrSharedContext::new_islands()` (não `new()`), a biblioteca do navegador exportando `hydrate` (o script de carga chama essa função e depois hidrata as ilhas), o script de carga das ilhas (`HydrationScripts` com `islands: true`, ou cópia fiel do que ele grava) e propriedades de ilha serializáveis (`serde`). Se as ilhas falharem no gerador próprio, a alternativa provada no spike é a hidratação da página inteira com os dados num `<script type="application/json">`; trocar exige nota neste ADR.
5. **O Markdown vira HTML só no gerador** (nativo); o `pulldown-cmark` não entra no WebAssembly. O que se reproduz do site atual (Astro, com o padrão do Astro e os plugins do `astro.config.mjs`):
   - os três plugins: bloco `mermaid` trocado pelo SVG já desenhado em `content/data/diagramas.json` (mesma chave do site atual), o caminho base nos links que começam com `/`, tabelas envolvidas para rolar no celular;
   - o que o Astro faz por padrão e muda o texto ou os links: GFM (tabelas, riscado, links automáticos), pontuação tipográfica (aspas curvas e traços, que o Astro aplica por padrão) e o `id` de cada título igual ao do site atual (as âncoras `#` dos links entre capítulos dependem disso);
   - não se reproduz o realce de código (o livro não tem bloco de código além dos diagramas). Diferença que sobrar contra o HTML do site atual vai listada no relatório da tarefa, e o site atual é a referência.
6. **Caminho base por configuração** (`SITE_BASE_PATH`, padrão `/centelha/` na prévia do GitHub Pages; `/` com o domínio próprio e no app).
7. **Duas provas obrigatórias no CI**, porque o gerador próprio é o ponto frágil desta escolha:
   - cada página gerada tem o texto completo sem WebAssembly: o teste compara o texto do HTML gravado (sem tags) com o texto dos eventos do mesmo parser de Markdown sobre o capítulo de origem, depois das mesmas trocas do item 5, página por página. Ele prova que o gerador não perde texto; a igualdade com o site atual é conferida à parte (item 5);
   - a hidratação não duplica nem apaga conteúdo: num navegador sem janela (Node e Chrome ou Edge no CI), o texto visível de cada página antes de carregar o WebAssembly e depois de as ilhas hidratarem é o mesmo, sem exceção de JavaScript (o teste ouve as exceções da página, porque em release um descasamento de hidratação vira pânico sem mensagem própria) e, de preferência, com o WebAssembly de debug, que escreve o aviso de hidratação no console.
8. **Imagens passam como estão** (R10): o gerador copia `content/public/` sem recodificar. Nunca `object-fit: cover`; o bestiário fica em 3:2 com `width: 100%; height: auto`.
9. **Perfil do WebAssembly:** a primeira tarefa do Frontend mede tamanho e velocidade com o perfil de release do workspace e com `opt-level = "z"`; o perfil do site entra por nota aqui depois da medida. O `"z"` encolhe o arquivo mas pode custar velocidade ao Grid, que é a meta número um.
10. **`unsafe_code = "forbid"`** continua valendo no workspace. O código que `#[wasm_bindgen]` e `#[island]` geram ainda não foi testado com esse lint; a primeira tarefa confere cedo e, se falhar, a saída vem por nota neste ADR (nunca tirando o lint do workspace inteiro).

## Alternativas

- **Dioxus 0.7:** uma ferramenta só e site estático oficial, mas o canvas fora da web passaria por JavaScript (o Grid seria dois códigos), a ferramenta regrava as imagens e o WebAssembly sai com o dobro do tamanho. O site estático oficial estava com defeito na versão do dia.
- **Rotas estáticas oficiais do Leptos (`cargo leptos`):** testadas no spike; geram o HTML, mas o roteador do cliente pede ao servidor cada página seguinte e dá página em branco num host estático.
- **Hidratação da página inteira:** provada no spike e mantida como alternativa. Com ilhas espera-se WebAssembly menor e o texto do livro fora do alcance dele; a primeira tarefa mede.

## Consequências

- Mais peças que o Dioxus: cargo, `wasm-bindgen-cli`, `wasm-opt` e `tauri-cli`. A montagem fica no `cargo xtask site` e no CI (Infraestrutura), com as versões deste ADR. A prova de hidratação põe Node e um navegador no CI.
- O gerador é nosso e precisa de cuidado: as duas provas do item 7 entram com a primeira página e não saem mais.
- O app é o site dentro do WebView: o que o app tem a mais (conteúdo offline, sincronização, atualização) fica no `centelha-app` e nos comandos do Tauri, sem duplicar página.
- O `wasm-opt --all-features` do binaryen `version_133` gera um WebAssembly que o navegador recusa; a lista de features vai explícita no `xtask`.
