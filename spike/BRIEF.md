# Spike do framework · Centelha em Rust (25/11/2026)

Objetivo: comparar dois caminhos para o site e o app do Centelha (RPG de mesa; site estático hoje em Astro): **(A) Leptos + Tauri 2** e **(B) Dioxus 0.7** (alvos próprios web, desktop e Android). Cada um é feito por um agente, em pasta própria; as medidas têm de sair comparáveis.

## Página de prova (igual nos dois)
O cartão de uma criatura do bestiário, lido dos JSON em `spike\dados\` (fora do ramo) (aboleth, aasimar, ankheg; imagem `<id>_criatura.webp` ao lado):
- nome (`nome`), Ameaça (`ameaca`), Centelha (`centelha`), Força de Vontade (`vontade`), os 9 atributos (`atributos`: forca, destreza, vigor, influencia, perspicacia, compostura, percepcao, inteligencia, raciocinio), a imagem (proporção 3:2, NUNCA cortada: nada de `object-fit: cover`; `width:100%; height:auto`), e a lista de ataques (`ataques[]`: `nome`, e `dado` e `tipoDano` quando existirem; os campos variam, use `Option`).
- Defesa é derivada pelo motor (fase seguinte): mostre "Defesa: (motor, fase 1)".
- Uma parte interativa mínima que prove a hidratação: botão "Rolar 1d6" que mostra o resultado (pode usar `js_sys::Math::random` ou um contador; o importante é o clique funcionar na página estática hidratada).
- Uma página por criatura (3 páginas) mais um índice com links.
- Identificadores em inglês, textos da página em português. Tipos com `serde(rename = "...")` para as chaves em português. Escrita sem travessão (—).

## O que medir (tabela no relatório)
1. **Web estática com hidratação:** gerar HTML estático (o texto do cartão tem de estar no HTML, legível com JavaScript desligado, para a busca Pagefind indexar) e hidratar no navegador. Diga exatamente como o framework faz isso (comando, recurso oficial ou feito à mão) e se é maduro.
2. **Tamanho do WebAssembly** em release, cru e depois de `wasm-opt -Oz` se houver (diga a versão do wasm-opt), e comprimido com gzip (`gzip -9 -c x.wasm | wc -c`). Tamanho do JS de cola.
3. **Tempo de compilação:** limpo (release, web) e incremental (mudar um texto e recompilar), em segundos. Antes de compilar limpo, anote quantos `rustc` estão rodando (outras instâncias podem estar compilando; não é para pará-las).
4. **Windows:** app de desktop abrindo o mesmo cartão. Tamanho do executável release, tempo de compilação, se precisou de algo instalado. Para o instalador (MSI/NSIS) só diga o que pede; não é obrigatório gerar.
5. **Android:** tente gerar um APK de depuração. Ambiente da máquina: Android SDK em `%LOCALAPPDATA%\Android\Sdk` (NDK 28.2.13676358, licenças já aceitas antes, pasta `licenses` existe), JDK do Android Studio em `C:\Program Files\Android\Android Studio\jbr` (use como JAVA_HOME só na sua sessão; não mude variável global do Windows). Alvos Rust do Android pode instalar com `rustup target add` (não pede licença). Se algo pedir aceite de licença ou instalação com clique, PARE nesse ponto e anote o que faltou. Não precisa rodar em aparelho; APK gerado e tamanho já basta.
6. **Canvas:** prove em 10 linhas que dá para desenhar num `<canvas>` (por exemplo um quadrado com `web-sys` `CanvasRenderingContext2d`) a partir de um componente; diga como se pega o elemento (node ref).
7. **Documentação e clareza:** sua avaliação curta (1 a 5) de documentação oficial, mensagens de erro, quantas peças/ferramentas são necessárias, e o código do cartão (cole o componente no relatório).

## Regras
- Versões: as estáveis do dia, conferidas no crates.io (já conferidas em 25/11: leptos 0.8.23, leptos_meta 0.8.7, leptos_router 0.8.17, cargo-leptos 0.3.12, tauri 2.12.7, tauri-cli 2.12.6, dioxus 0.7.10, dioxus-cli 0.7.10, trunk 0.22.2, wasm-bindgen 0.2.130, serde 1.0.229, serde_json 1.0.154). Use essas; se uma não funcionar, diga.
- Ferramentas já instaladas em `%USERPROFILE%\.cargo\bin` (ponha no PATH da sessão): `dx` 0.7.10, `cargo-tauri` 2.12.6, `trunk` 0.22.2, `wasm-bindgen` 0.2.130, `cargo-binstall`. `cargo-leptos` pode estar sendo compilado agora por outro processo; confira se existe antes de usar; não comece uma segunda instalação dele. Rust 1.100.0 (rustup), alvo wasm32-unknown-unknown instalado, MSVC Build Tools e WebView2 existem.
- Limite a compilação a `CARGO_BUILD_JOBS=6` (a máquina tem 16 GB e outro agente compila ao mesmo tempo).
- Trabalhe só na sua pasta (`spike\<leptos|dioxus>\`). Não toque na pasta do site atual nem nas árvores do repositório. Não faça commit nem push em lugar nenhum; não crie repositório git.
- Nada no código, comentário ou nome de arquivo pode citar assistente de IA, modelo de linguagem ou empresas do ramo (o código pode virar parte de um repositório público que proíbe isso).
- Saída longa de build vai para arquivo de log na sua pasta; no relatório só o resumo.
- Tempo: no máximo 2h30. Se um alvo travar (Android, por exemplo) por mais de 30 minutos, anote o motivo e siga.
- Relatório final em `spike\<leptos|dioxus>\RELATORIO.md`, em português, com a tabela de medidas, os comandos exatos usados, problemas encontrados e o que ficou de fora. Sua resposta final ao chamador: a tabela e 10 linhas de conclusão.
