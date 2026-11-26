# Contratos entre os crates

Versão 2 (fase 0; site e app pelo ADR 0003). Este arquivo diz o que cada crate expõe e o que consome. Mudança aqui só com ADR em `docs/adr/`. Quem implementa um crate não muda a API pública de outro: pede a mudança.

## Dependências permitidas

```text
centelha-data      <- centelha-engine <- centelha-grid
       ^                  ^                    ^
       |                  |                    |
       +---- centelha-web (site) --------------+
centelha-supabase  <- centelha-web, centelha-app
centelha-app       (casca Tauri: carrega o dist/ gerado pelo centelha-web, sem depender dele como crate)
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

**Expõe (o que já existe no `main`):**

- Módulos: `bestiary`, `rules`, `weapons`, `armor`, `shields`. Os outros arquivos de dados (atributos, Habilidades, Virtudes, Técnicas, Caminhos, Artes, Efeitos, condições, glossário, raças) entram como módulo quando o Backend propuser a API e o Arquiteto aprovar.
- Na raiz do crate: `Creature`, `Rules`, `Weapon`, `Armor`, `Shield`, `DataError`, os `parse_*` e os `load_*`. O resto mora no módulo de cada um (por exemplo `bestiary::Size`).
- `Creature`: uma criatura do bestiário, com `id: String` e os campos usados pelo motor.
- Leitura a partir de texto: `fn parse_creature(json: &str) -> Result<Creature, DataError>` e equivalentes. Funciona em WebAssembly (sem arquivo).
- Leitura a partir de pasta, só no nativo (`cfg(not(target_arch = "wasm32"))`): `fn load_bestiary(content_dir: &Path) -> Result<Vec<Creature>, DataError>`.
- `DataError`: `enum` com `Io`, `Json` e `Invalid`; métodos `file()`, `path()` (caminho dentro do JSON, como `atributos.forca`) e `reason()`.
- Regras e catálogos (o que os derivados da criatura leem): `rules::{Rules, parse_rules}`, `weapons::{Weapon, parse_weapons}`, `armor::{Armor, parse_armors}`, `shields::{Shield, parse_shields}`, e no nativo `load_rules`, `load_weapons`, `load_armors`, `load_shields` (recebem `content_dir`). `Weapon`, `Armor` e `Shield` têm `stats: Option<...>` (o bloco de números pode ser `null` na fonte).
- Listas do bestiário como `enum` em `bestiary::`, com os nomes do `GLOSSARY.md`: `Size` (com `Size::ALL` em ordem e `id()`), `CarapaceProfile`, `HpLine`, `AttackClass`, `Category`. A Velocidade do ataque é o campo `timing: Option<Timing>` (`windup`, `blow`, `recovery`), lido de `pgr`. `Attributes::get(key)` pela chave dos dados.
- Tipos auxiliares públicos em `bestiary::`: `Attributes`, `Body`, `Attack`, `Timing`, `Form`, `MainForm`, `Swarm`, `SecondSize`, `Equipment`, `ArsenalItem`, `WornArmor` e as versões parciais das formas (`BodyPatch`, `AttributesPatch`, `EquipmentPatch`). As Habilidades continuam texto (chave do mapa de Habilidades); `RANGED_SKILLS` lista as de tiro aceitas no ataque natural.
- `serde_json` com `float_roundtrip` (a leitura de `f64` tem de ser exata para a paridade).

**Vai expor (entra com a tarefa 001 do Frontend, que o implementa por exceção, com esta API):** módulo `chapters`, com `Chapter { slug, order, numeral, title, summary, chapter_name, body }`. `chapter_name: Option<String>` é o campo opcional `capitulo` do schema do site atual (o nome do capítulo de várias páginas quando a primeira página tem outro título); os outros quatro campos do cabeçalho são obrigatórios. `body` é o Markdown sem o cabeçalho. Funções: `parse_chapter(slug: &str, text: &str) -> Result<Chapter, DataError>` e, no nativo, `load_chapters(content_dir: &Path) -> Result<Vec<Chapter>, DataError>` em ordem de `order`. O `slug` é o nome do arquivo sem `.md`. Sem dependência nova de YAML: o cabeçalho é uma linha por campo, `chave: valor`, separado no primeiro dois-pontos, com valor entre aspas duplas (texto) ou número inteiro (`ordem`); o que não seguir isso, ou chave desconhecida, dá erro `Invalid`.

**Consome:** `content/` (nada mais).

## centelha-engine

**Faz:** as regras puras do jogo. Mesma entrada, mesma saída, idêntica ao motor TypeScript do site atual (paridade).

**Expõe (primeira versão):**

- `js::to_uint32(f64) -> u32`: o `>>> 0` do JavaScript (conversão da semente).
- `rng::Mulberry32`: o gerador aleatório. `Mulberry32::new(seed: u32)`, `next_u32(&mut self) -> u32`, `next_f64(&mut self) -> f64` (no intervalo [0, 1), igual ao TypeScript bit a bit). Nenhum outro gerador aleatório no motor.
- `js::round` (`Math.round`, inclusive o `-0`), `js::log10` e `js::pow` (pela `libm`, ADR 0002).
- `creature`: os derivados da criatura (port de `criatura()` e `comForma()`).
  - Tudo em `creature::`. `Tables::new(rules, weapons, armors, shields)` e `Tables::rules()`: regras e catálogos já lidos; busca como o TypeScript (arma pela primeira peça com o id; armadura e escudo só com bloco de números, id repetido fica com o último).
  - `derive(&Tables, &Creature) -> CreatureDerived` e `with_form(&Creature, &Form) -> Creature` (devolve `id` e `name` da forma, como o `comForma()`).
  - `CreatureDerived` com `swarm`, `hp`, `lifted_weight`, `mass_band`, `size`, `second_size`, `dodge`, `block`, `mental_defense` e `social_defense` (`Defense::Value(f64)` ou `Defense::Immune`), `initiative`, `energy`, `mana`, `soak` (`Soak { impact, cut, pierce }`), `attacks: Vec<AttackDerived>`, `with_equipment: Option<WithEquipment>`. Tipos públicos: `CreatureDerived`, `Defense`, `Soak`, `SwarmState`, `AttackDerived`, `WithEquipment`. Números de saída em `f64`, como no TypeScript.
  - Contas pequenas públicas (a ficha e a mesa vão usar): `spark_in_roll`, `size_from_area`, `mass_band`, `hp_from_mass`, `hp_from_size`, `lifted_weight`, `average_damage`, `carapace_soak`.
- Daqui em diante, por fase: derivados da ficha e das criaturas (fase 1), combate e tempo em Ticks, Artes, lógica do Grid (fase 3). Cada módulo entra no contrato quando o Backend propõe a API e o Arquiteto aprova.

**Aritmética:** os números do TypeScript são `f64`. O port usa `f64` onde o TypeScript faz conta com fração e reproduz `Math.round` (o .5 sobe, inclusive nos negativos: `Math.round(-2.5) == -2`), `Math.floor` e `Math.trunc` exatamente.

**Consome:** tipos de `centelha-data` e a `libm` (com `default-features = false`, ADR 0002). Nenhuma outra dependência normal.

## centelha-supabase

**Faz:** fala com o Supabase do jogo (tabelas, funções RPC, arquivos e tempo real). Continua o mesmo banco do site atual (R2).

**Expõe:** a definir pelo Backend em ADR (fase 3), incluindo como fazer o tempo real no WebAssembly.

**Consome:** a URL e a chave anon por configuração (nunca no código). Chave de serviço nunca.

## centelha-web

Framework e versões: ADR 0003 (Leptos com ilhas; site estático gerado por nós).

**Faz:** o site. Páginas do livro geradas em HTML no build, com o texto inteiro; partes interativas como ilhas em WebAssembly.

**Expõe:**

- Sem features padrão. As features `ssr` e `hydrate` nunca se combinam (o Leptos não aceita as duas juntas); clippy e testes rodam por conjunto: nativo com `--features ssr`, `wasm32-unknown-unknown` com `--features hydrate`.
- Feature `ssr` (só nativo): os componentes que desenham as páginas e o binário gerador (`required-features = ["ssr"]`), que grava `dist/` (uma pasta por página, `index.html` dentro). O Markdown vira HTML aqui, e só aqui.
- Feature `hydrate` (só `wasm32-unknown-unknown`): a biblioteca do navegador, que exporta a função `hydrate` chamada pelo script de carga das ilhas. Nada de Markdown, de leitura de `content/` nem de roteador no cliente.
- Dados de uma ilha: pequenos vão nas propriedades da ilha (com `serde`, gravadas no HTML pelo gerador); grandes (ficha, catálogos) vão num `<script type="application/json">` da própria página, que a ilha lê. Ilha não busca conteúdo do livro pela rede.
- O caminho base vem de `SITE_BASE_PATH` no build (padrão `/centelha/`).

**Consome:** `centelha-data` (leitura de `content/`, no gerador), `centelha-engine` (toda conta que uma página mostra), `leptos`, `leptos_meta`, `wasm-bindgen`, `web-sys`, `js-sys`, `serde`, e só na feature `ssr` o `pulldown-cmark` (Markdown) e o que a chave dos diagramas pedir (SHA-1). Versões no ADR 0003. Depois, `centelha-grid` e `centelha-supabase`.

**Não faz:** conta de regra (pede ao `centelha-engine`), mudança no texto do livro (o que está em `content/` sai como está), corte ou recodificação de imagem.

## centelha-grid

**Faz (fase 3):** o tabuleiro da mesa, desenhado em canvas, como ilha Leptos (`#[island]`: no modo ilhas só a ilha roda no navegador) usada pelo `centelha-web` (e, pelo WebView, no app). O elemento é pego por `NodeRef::<Canvas>` e o desenho roda só no cliente.

**Consome:** `centelha-engine` (posições, alcance, tempo em Ticks), `centelha-data`, `leptos`, `web-sys`. Nunca o `centelha-supabase` direto: a mesa passa os dados ao Grid.

**Expõe:** a definir quando o Grid começar. A bancada de desempenho (ganchos de medida do modo de teste) entra nesse contrato.

## centelha-app

**Faz (fase 4, entra depois da primeira página do Frontend):** a casca Tauri 2 do app de Windows e Android. Carrega o mesmo `dist/` que o `cargo xtask site` gera, com caminho base `/`.

**Expõe:** só os comandos do Tauri que a web não tem (conteúdo offline, sincronização com o Supabase quando há rede, atualização). Cada comando entra aqui quando o App propuser.

**Consome:** `tauri`, `tauri-build`, `centelha-supabase` (depois). Nunca duplica página nem componente do `centelha-web`.

## xtask

- `cargo xtask sync-content`: copia o conteúdo da fonte (pasta em `RPG_SYSTEM_DIR`) para `content/`, só arquivos versionados lá, limpa o metadado das imagens e varre o resultado; com qualquer achado, não copia. Grava `content/ORIGEM.txt`.
- `cargo xtask scan`: varre arquivos versionados (conteúdo e nome) e mensagens de commit. Sai com erro se achar termo proibido. `--no-commits` pula as mensagens.
- `cargo xtask site` (entra com a tarefa 001 do Frontend): monta `dist/` chamando o gerador do `centelha-web`, o build do WebAssembly, o `wasm-bindgen` e o `wasm-opt` como programas, nas versões do ADR 0003. O `xtask` continua sem dependências externas.

## Testes de paridade

- As referências moram em `tests/parity/` (JSON), geradas por `tools/parity/` rodando o próprio TypeScript da fonte, na máquina do autor. O CI não busca a fonte: confere o Rust contra as referências gravadas.
- Referência não se edita para passar teste.
