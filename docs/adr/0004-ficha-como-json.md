# 0004 · A ficha do jogador como JSON, com a ordem das chaves

Data: 26/11/2026. Estado: aceita.

## Contexto

A ficha do jogador é gravada no banco do site atual em formas de várias épocas, com chaves em português e chaves que o motor não conhece e que têm de voltar intactas quando a ficha é gravada de novo. O motor TypeScript a trata como objeto solto e depende da ordem das chaves em alguns resultados (o espalhamento `{...}`, a ordem em que soma e lista). A regra geral do `CONTRACTS.md` é traduzir as chaves na leitura, com tipos próprios.

## Decisão

1. **Exceção declarada:** no módulo `centelha-engine::sheet`, a ficha entra e sai como `serde_json::Value`, com as chaves em português, como está gravada. Os números que o motor calcula saem em tipos próprios, com nomes do `GLOSSARY.md`. Campos de saída que repetem um pedaço da ficha ou de um catálogo (o item da arma, os Atributos e as Habilidades como estão) continuam `Value` e são marcados no contrato como "chaves em português".
2. **`serde_json` é dependência normal do `centelha-engine`** (versão 1.0.154, a mesma do `centelha-data`), com `float_roundtrip` e `preserve_order`.
3. **`preserve_order` vale para todo build que inclua o `centelha-engine`**, porque o Cargo unifica as features: nesse build, todo `serde_json::Map` passa a guardar a ordem de inserção (`indexmap`). Nesse modo, `Map::remove` troca o último elemento para o lugar do removido e muda a ordem; o código usa `shift_remove`, e o `clippy.toml` do workspace (entra com a tarefa 004 do Backend) passa a proibir `serde_json::Map::remove` (`disallowed-methods`), para o workspace inteiro.
4. A cadeia nova no `Cargo.lock`: `indexmap` 2.14.2, `hashbrown` 0.17.2, `equivalent` 1.0.3 e `foldhash` 0.2.0 (as versões que o índice do crates.io deu em 25/11/2026).

## Consequências

- A paridade da ficha continua a comparar valores; comparar também a ordem das chaves de `for_reading` e `for_saving` fica como melhoria futura.
- Se a ilha da ficha precisar de um campo tipado, ele entra como tipo de saída novo do motor, sem trocar a entrada `Value`.
