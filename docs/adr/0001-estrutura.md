# 0001 · Estrutura do repositório

Data: 25/11/2026. Estado: aceita.

## Contexto

O Centelha tem hoje um site em Astro e TypeScript, com dados em JSON, motor de regras em TypeScript e Supabase para contas e mesas. A versão em Rust tem três objetivos: desempenho do tabuleiro e da mesa, app instalável (Windows e Android) e um código claro para estudo. O conteúdo do jogo continua nascendo na fonte atual, que muda todo dia; aqui ele só é lido.

## Decisão

1. **Um workspace Cargo** com crates separados por responsabilidade: `centelha-data` (dados), `centelha-engine` (regras puras, sem IO), `centelha-supabase` (cliente do banco), `centelha-web` (site), `centelha-grid` (tabuleiro), `centelha-app` (app) e `xtask` (tarefas). A direção das dependências está no `CONTRACTS.md`.
2. **O motor não faz IO.** Ele compila para WebAssembly e para nativo, e o mesmo código serve ao site, ao app e aos testes.
3. **Conteúdo por cópia.** `cargo xtask sync-content` copia da fonte só o que o site usa (capítulos, dados, imagens prontas e ícones) para `content/`, só arquivos versionados lá, com o metadado das imagens removido e uma varredura de termos proibidos; `content/ORIGEM.txt` guarda o commit de origem. Sem submódulo e sem link. Atualizar a cópia é um commit próprio.
4. **Paridade com o TypeScript.** As referências de teste são geradas rodando o motor TypeScript da fonte e gravadas em `tests/parity/`. O CI confere o Rust contra elas.
5. **Toolchain fixada** em `rust-toolchain.toml` (Rust 1.100.0, edição 2024, alvo `wasm32-unknown-unknown`).
6. **`xtask` sem dependências externas:** a cópia, a limpeza de WebP e PNG (só blocos, sem recodificar) e a varredura usam só a biblioteca padrão, para compilar rápido e não depender de biblioteca de imagem.
7. **Ganchos do git** em `scripts/hooks` (`commit-msg` e `pre-commit`), ligados com `git config core.hooksPath scripts/hooks`. O CI repete a varredura com `cargo xtask scan`.

## Alternativas

- **Um crate só:** mais simples no começo, mas mistura regra com interface e impede o motor de ir para WebAssembly sem arrastar o resto.
- **Submódulo da fonte:** traria histórico e arquivos que não podem entrar aqui; descartado.
- **Ler o JSON da fonte direto no build:** prenderia o CI à máquina do autor; a cópia versionada deixa o repositório completo sozinho.
- **Biblioteca de imagem para limpar metadado:** recodificaria as imagens (perda e risco de corte); a limpeza por blocos mantém os pixels idênticos.

## Consequências

- Duas fontes de verdade temporárias: a fonte manda, e `content/` é uma foto datada dela. A rotina de paridade avisa quando a fonte mudou.
- O framework do site e do app fica para o ADR do spike; os crates `centelha-web`, `centelha-grid` e `centelha-app` existem só com o nome até lá.
