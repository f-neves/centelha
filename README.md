# Centelha

Centelha é um sistema de RPG de mesa em d6, do mortal ao semideus. Este repositório é a nova versão do site e do app do jogo, escrita em Rust: o livro de regras, o bestiário, os catálogos, a ficha de personagem e a mesa de jogo com o tabuleiro.

Estado: em construção (fase 0, a fundação). O site atual continua sendo o oficial até esta versão ficar pronta.

## Organização

| Pasta | O que tem |
|---|---|
| `crates/centelha-data` | leitura e validação dos dados do jogo |
| `crates/centelha-engine` | as regras: ficha, criaturas, combate, tempo, Artes e o tabuleiro, sem IO |
| `crates/centelha-supabase` | cliente do Supabase (contas, mesas, tempo real) |
| `crates/centelha-web` | o site |
| `crates/centelha-grid` | o tabuleiro da mesa |
| `crates/centelha-app` | o app de Windows e Android |
| `xtask` | tarefas do repositório (`cargo xtask ...`) |
| `content` | capítulos, dados e imagens do jogo, copiados da fonte por `cargo xtask sync-content` (não editar à mão) |
| `docs/adr` | registro das decisões de arquitetura |

Contratos entre os crates: [`CONTRACTS.md`](CONTRACTS.md). Termos do jogo em português e em inglês: [`GLOSSARY.md`](GLOSSARY.md).

## Como compilar

Requer o Rust da versão fixada em `rust-toolchain.toml` (o `rustup` instala sozinho).

```sh
cargo build
cargo test
```

Depois de clonar, ligue os ganchos do repositório:

```sh
git config core.hooksPath scripts/hooks
```
