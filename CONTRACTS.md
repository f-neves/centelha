# Contratos entre os crates

Versão 1 (fase 0). Este arquivo diz o que cada crate expõe e o que consome. Mudança aqui só com ADR em `docs/adr/`. Quem implementa um crate não muda a API pública de outro: pede a mudança.

## Dependências permitidas

```text
centelha-data      <- centelha-engine <- centelha-grid
       ^                  ^                    ^
       |                  |                    |
       +---- centelha-web (site) --------------+
                  ^
                  |
            centelha-app
centelha-supabase  <- centelha-web, centelha-app
xtask              (sozinho; não é dependência de ninguém)
```

- `centelha-engine` depende só de `centelha-data` (para os tipos) e de bibliotecas sem IO. Nunca de `centelha-supabase`, de framework de interface, de `web-sys` ou de relógio do sistema.
- `centelha-data` não depende de nenhum outro crate do workspace.
- `centelha-supabase` não depende de `centelha-engine`: transporta dados, não aplica regra.
- Nenhuma dependência circular. Um crate de interface (`web`, `grid`, `app`) nunca refaz uma conta que o `engine` já faz.

## Regras de todos os crates

- Identificadores em inglês, comentários e documentação em português (R8). Termos do jogo pelo `GLOSSARY.md`.
- Os dados de `content/data` têm chaves em português. A tradução acontece na leitura, com `#[serde(rename = "...")]`; o JSON nunca é reescrito.
- `centelha-data` e `centelha-engine` compilam para `wasm32-unknown-unknown` e para nativo.
- Sem `unsafe` (o workspace proíbe).
- Erros com tipo próprio por crate (enum), não `String`, na API pública.

## centelha-data

**Faz:** lê e valida o conteúdo de `content/` (capítulos e JSON). Os tipos espelham o schema dos dados do site atual (o schema dos capítulos e a validação dos dados de lá).

**Expõe (primeira versão, a fase 1 completa):**

- Tipos dos dados, um módulo por arquivo de dados: `bestiary` (criaturas), `attributes`, `skills`, `virtues`, `techniques`, `paths`, `arts`, `effects`, `weapons`, `armor`, `shields`, `conditions`, `glossary`, `rules`.
- `Creature`: uma criatura do bestiário, com `id: String` e os campos usados pelo motor.
- Leitura a partir de texto: `fn parse_creature(json: &str) -> Result<Creature, DataError>` e equivalentes. Funciona em WebAssembly (sem arquivo).
- Leitura a partir de pasta, só no nativo (`cfg(not(target_arch = "wasm32"))`): `fn load_bestiary(content_dir: &Path) -> Result<Vec<Creature>, DataError>`.
- `DataError`: `enum` com `Io`, `Json` e `Invalid`; métodos `file()`, `path()` (caminho dentro do JSON, como `atributos.forca`) e `reason()`.
- Tipos auxiliares públicos do bestiário: `Attributes`, `Body`, `Attack`, `Form` (e as versões parciais das formas). As listas fechadas (Tamanhos, perfis de Couraça, classes do ataque, Habilidades de tiro) viram `enum` com os nomes do `GLOSSARY.md`; enquanto forem `String` validada, as constantes `SIZES`, `CARAPACE_PROFILES`, `ATTACK_CLASSES` e `RANGED_SKILLS` ficam públicas.
- `serde_json` com `float_roundtrip` (a leitura de `f64` tem de ser exata para a paridade).

**Consome:** `content/` (nada mais).

## centelha-engine

**Faz:** as regras puras do jogo. Mesma entrada, mesma saída, idêntica ao motor TypeScript do site atual (paridade).

**Expõe (primeira versão):**

- `js::to_uint32(f64) -> u32`: o `>>> 0` do JavaScript (conversão da semente).
- `rng::Mulberry32`: o gerador aleatório. `Mulberry32::new(seed: u32)`, `next_u32(&mut self) -> u32`, `next_f64(&mut self) -> f64` (no intervalo [0, 1), igual ao TypeScript bit a bit). Nenhum outro gerador aleatório no motor.
- Daqui em diante, por fase: derivados da ficha e das criaturas (fase 1), combate e tempo em Ticks, Artes, lógica do Grid (fase 3). Cada módulo entra no contrato quando o Backend propõe a API e o Arquiteto aprova.

**Aritmética:** os números do TypeScript são `f64`. O port usa `f64` onde o TypeScript faz conta com fração e reproduz `Math.round` (o .5 sobe, inclusive nos negativos: `Math.round(-2.5) == -2`), `Math.floor` e `Math.trunc` exatamente.

**Consome:** tipos de `centelha-data`.

## centelha-supabase

**Faz:** fala com o Supabase do jogo (tabelas, funções RPC, arquivos e tempo real). Continua o mesmo banco do site atual (R2).

**Expõe:** a definir pelo Backend em ADR (fase 3), incluindo como fazer o tempo real no WebAssembly.

**Consome:** a URL e a chave anon por configuração (nunca no código). Chave de serviço nunca.

## centelha-web, centelha-grid, centelha-app

Só o nome até a decisão do framework (spike da fase 0). O contrato de cada um entra com o ADR da decisão.

## xtask

- `cargo xtask sync-content`: copia o conteúdo da fonte (pasta em `RPG_SYSTEM_DIR`) para `content/`, só arquivos versionados lá, limpa o metadado das imagens e varre o resultado; com qualquer achado, não copia. Grava `content/ORIGEM.txt`.
- `cargo xtask scan`: varre arquivos versionados (conteúdo e nome) e mensagens de commit. Sai com erro se achar termo proibido. `--no-commits` pula as mensagens.

## Testes de paridade

- As referências moram em `tests/parity/` (JSON), geradas por `tools/parity/` rodando o próprio TypeScript da fonte, na máquina do autor. O CI não busca a fonte: confere o Rust contra as referências gravadas.
- Referência não se edita para passar teste.
