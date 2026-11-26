# Contratos entre os crates

Versão 3 (fase 1; site e app pelo ADR 0003; a ficha como JSON pelo ADR 0004). Este arquivo diz o que cada crate expõe e o que consome. Mudança aqui só com ADR em `docs/adr/`. Quem implementa um crate não muda a API pública de outro: pede a mudança.

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
- Os dados de `content/data` têm chaves em português. A tradução acontece na leitura, com `#[serde(rename = "...")]`; o JSON nunca é reescrito. Exceção declarada: a ficha do jogador no `centelha-engine::sheet` (ADR 0004).
- `centelha-data` e `centelha-engine` compilam para `wasm32-unknown-unknown` e para nativo.
- Sem `unsafe` (o workspace proíbe).
- Erros com tipo próprio por crate (enum), não `String`, na API pública.

## centelha-data

**Faz:** lê e valida o conteúdo de `content/` (capítulos e JSON). Os tipos espelham o schema dos dados do site atual (o schema dos capítulos e a validação dos dados de lá).

**Expõe (o que já existe no `main`):**

- Módulos: `bestiary`, `rules`, `weapons`, `armor`, `shields`, `ancestry`. Os outros arquivos de dados (atributos, Habilidades, Virtudes, Técnicas, Caminhos, Artes, Efeitos, condições, glossário, raças) entram como módulo quando a API proposta for aprovada neste arquivo.
- Na raiz do crate: `Creature`, `Rules`, `Weapon`, `Armor`, `Shield`, `Ancestry`, `DataError`, os `parse_*` e os `load_*`. O resto mora no módulo de cada um (por exemplo `bestiary::Size`).
- `Creature`: uma criatura do bestiário, com `id: String` e os campos usados pelo motor.
- Leitura a partir de texto: `fn parse_creature(json: &str) -> Result<Creature, DataError>` e equivalentes. Funciona em WebAssembly (sem arquivo).
- Leitura a partir de pasta, só no nativo (`cfg(not(target_arch = "wasm32"))`): `fn load_bestiary(content_dir: &Path) -> Result<Vec<Creature>, DataError>`.
- `DataError`: `enum` com `Io`, `Json` e `Invalid`; métodos `file()`, `path()` (caminho dentro do JSON, como `atributos.forca`) e `reason()`.
- Regras e catálogos (o que os derivados da criatura leem): `rules::{Rules, parse_rules}`, `weapons::{Weapon, parse_weapons}`, `armor::{Armor, parse_armors}`, `shields::{Shield, parse_shields}`, e no nativo `load_rules`, `load_weapons`, `load_armors`, `load_shields` (recebem `content_dir`). `Weapon`, `Armor` e `Shield` têm `stats: Option<...>` (o bloco de números pode ser `null` na fonte).
- Listas do bestiário como `enum` em `bestiary::`, com os nomes do `GLOSSARY.md`: `Size` (com `Size::ALL` em ordem e `id()`), `CarapaceProfile`, `HpLine`, `AttackClass`, `Category` (aberta: categoria nova na fonte entra como `Category::Other(String)`; tem `id()` e não é `Copy`). A Velocidade do ataque é o campo `timing: Option<Timing>` (`windup`, `blow`, `recovery`), lido de `pgr`. `Attributes::get(key)` pela chave dos dados.
- Tipos auxiliares públicos em `bestiary::`: `Attributes`, `Body`, `Attack`, `Timing`, `Form`, `MainForm`, `Swarm`, `SecondSize`, `Equipment`, `ArsenalItem`, `WornArmor` e as versões parciais das formas (`BodyPatch`, `AttributesPatch`, `EquipmentPatch`). As Habilidades continuam texto (chave do mapa de Habilidades); `RANGED_SKILLS` lista as de tiro aceitas no ataque natural.
- Raças: `ancestry::{Ancestry, ConditionalBonus, parse_ancestries}` e, no nativo, `load_ancestries` (de `racas.json`). `Ancestry` com `id`, `name`, `size: Option<Size>`, `attributes`, `appearance_mod`, `mass_kg: Option<[f64; 2]>`, `movement_fraction`, `conditional_bonuses` e `adds_stamina_to_hp()`.
- Em `Rules`, para o personagem: `XpRules`, `XpTrack`, `XpKind` (fechado de propósito: tipo de trilha desconhecido em `regras.json` faz a leitura falhar, onde o TypeScript somaria como `acum` sem avisar), `XP_NAMED`, `AppearanceRules`, `DeathRules`, `DeathRounding`, `DamageRules`, `PierceGate`, `StrengthRules`, `SizeAccuracyReform`.
- Em `Rules`, para a ficha: `rules::{MovementRules, Ladder, NearMissRules, NearMissWeapon, NearMissArmor, NearMissThresholds, CombatMovement, Charge}` (`derived.movement`, `near_miss`, `combat.movement.charge`). As escadas do Deslocamento ficam na ordem do JSON, porque a soma é feita nessa ordem.
- `serde_json` com `float_roundtrip` (a leitura de `f64` tem de ser exata para a paridade).

**Expõe também (feito pelo Frontend por exceção):** módulo `chapters`, com `Chapter { slug, order, numeral, title, summary, chapter_name, body }`. `chapter_name: Option<String>` é o campo opcional `capitulo` do schema do site atual (o nome do capítulo de várias páginas quando a primeira página tem outro título); os outros quatro campos do cabeçalho são obrigatórios. `body` é o Markdown sem o cabeçalho. Funções: `parse_chapter(slug: &str, text: &str) -> Result<Chapter, DataError>` e, no nativo, `load_chapters(content_dir: &Path) -> Result<Vec<Chapter>, DataError>` em ordem de `order`. O `slug` é o nome do arquivo sem `.md`. Sem dependência nova de YAML: o cabeçalho é uma linha por campo, `chave: valor`, separado no primeiro dois-pontos, com valor entre aspas duplas (texto) ou número inteiro (`ordem`); o que não seguir isso, ou chave desconhecida, dá erro `Invalid`.

**Vai expor (entra com a tarefa 003 do Frontend, por exceção, com esta API):** módulo `glossary`, com `GlossaryEntry { id: String, term: String, aliases: Vec<String>, definition: String }` lido de `glossario.json` (`termo`, `aliases`, `definicao`), `parse_glossary(json: &str) -> Result<Vec<GlossaryEntry>, DataError>` e, no nativo, `load_glossary(content_dir: &Path)`. Id vazio ou repetido dá `Invalid`.

**Consome:** `content/` (nada mais).

## centelha-engine

**Faz:** as regras puras do jogo. Mesma entrada, mesma saída, idêntica ao motor TypeScript do site atual (paridade).

**Expõe (primeira versão):**

- `js::to_uint32(f64) -> u32`: o `>>> 0` do JavaScript (conversão da semente).
- `rng::Mulberry32`: o gerador aleatório. `Mulberry32::new(seed: u32)`, `next_u32(&mut self) -> u32`, `next_f64(&mut self) -> f64` (no intervalo [0, 1), igual ao TypeScript bit a bit), `draws(&self) -> u64` (quantos números já entregou, para os testes de paridade acharem a primeira divergência). Nenhum outro gerador aleatório no motor.
- `js::round` (`Math.round`, inclusive o `-0`), `js::log10` e `js::pow` (pela `libm`, ADR 0002).
- `creature`: os derivados da criatura (port de `criatura()` e `comForma()`).
  - Tudo em `creature::`. `Tables::new(rules, weapons, armors, shields)` e `Tables::rules()`: regras e catálogos já lidos; busca como o TypeScript (arma pela primeira peça com o id; armadura e escudo só com bloco de números, id repetido fica com o último).
  - `derive(&Tables, &Creature) -> CreatureDerived` e `with_form(&Creature, &Form) -> Creature` (devolve `id` e `name` da forma, como o `comForma()`).
  - `CreatureDerived` com `swarm`, `hp`, `lifted_weight`, `mass_band`, `size`, `second_size`, `dodge`, `block`, `mental_defense` e `social_defense` (`Defense::Value(f64)` ou `Defense::Immune`), `initiative`, `energy`, `mana`, `soak` (`Soak { impact, cut, pierce }`), `attacks: Vec<AttackDerived>`, `with_equipment: Option<WithEquipment>`. Tipos públicos: `CreatureDerived`, `Defense`, `Soak`, `SwarmState`, `AttackDerived`, `WithEquipment`. Números de saída em `f64`, como no TypeScript.
  - Contas pequenas públicas (a ficha e a mesa vão usar): `spark_in_roll`, `size_from_area`, `mass_band`, `hp_from_mass`, `hp_from_size`, `lifted_weight`, `average_damage`, `carapace_soak`.
- `sheet`: a ficha do jogador (port de `ficha-formato.ts`, `equip.ts`, da parte de personagem de `motor.ts`, de `mesa-ficha.ts` e do que eles usam de `quase-acerto.ts`). A ficha entra como `&serde_json::Value` (ADR 0004).
  - Tabelas: `Catalog::new(&Value, &Value, &Value)` e `Catalog::from_json(&str, &str, &str)`, que devolvem `Result<Catalog, SheetError>` (armas, armaduras e escudos crus; o `creature::Tables` usa os tipos do `centelha-data`, então quem usa os dois lê os catálogos duas vezes até a unificação), `SheetTables::new(Rules, Vec<Ancestry>, Catalog)`, `SheetTables::{rules, catalog, ancestry_of, near_miss}`, `SheetError` (`CatalogNotList`, `CatalogJson`).
  - Forma da ficha: `for_reading(&Catalog, &Value) -> Value`, `for_saving(&Catalog, &Value) -> Value`, `is_saved_form`. Aceita os nomes antigos que o TypeScript aceita. As migrações do `normalize()` da tela (com o `RENOMES`) ainda não estão aqui.
  - Equipamento: `piece_kind` (`PieceKind`, `ItemKind`, com `ItemKind::block`), `flatten_item`, `Catalog::{piece_item, piece_origin, catalog_copy, worn_armor, weapon, armor, shield, weapon_by_id_or_name}`.
  - Números: `sheet_numbers(&SheetTables, &Value) -> SheetNumbers`, `combat_summary(...) -> CombatSummary` (espera a ficha já passada por `for_reading`, como o `resumoFicha` faz), `sheet_summary(...) -> SheetSummary`, `db_summary(&SheetSummary) -> DbSummary`; partes públicas `hand_item`, `active_hand_set`, `hand_set`, `player_mass`, `stack_armor`, `movement` (o traço vem pela chave da regra, como `forca` ou `atletismo`), `average_damage_of_expr`.
  - Tipos de saída: `SheetNumbers`, `HandSet`, `HandItem`, `HandKind`, `DamageVersion`, `DualWield`, `ArmorStack`, `Initiative`, `CombatSummary`, `NearMiss`, `Step`, `Movement`, `SheetSummary`, `Passive`, `ArtLevel`, `DbSummary`. Campos `Value` com chaves em português, no formato do item do catálogo ou da ficha: `HandItem.weapon/shield/base/vs_fast_projectile`, `HandSet.attack_weapon`, `DualWield.off_die`, `CombatSummary.attributes/skills/class`, `SheetSummary.attributes/skills/secondary_skills/virtues`, `ArtLevel.level`.
  - Entrada de tipo errado (texto onde vai número): o port segue o `Number()` do JavaScript e não lança erro, onde o TypeScript às vezes lança. Ficha gravada pela tela não tem isso.
- `sheet`, acréscimos da fase 1: Quase-Acerto (`SheetTables::{weapon_near_miss, armor_near_miss, near_miss_between}`, `encounter_near_miss`, `missed_by`, `grazes`, `attack_outcome`; tipos `WeaponNearMiss`, `ArmorNearMiss`, `NearMissAttacker`, `NearMissTarget`, `Graze`, `NearMissOutcome`, `AttackOutcome`); textos (`signed_number`, `damage_text`, `weapon_stats`, `shield_stats`, `armor_stats`, iguais byte a byte ao TypeScript, com o sinal de menos U+2212); troca de ids de arma (`migrate_weapon_refs`, `WEAPON_RENAMES`); ficha já lida (`sheet_numbers_read`, `sheet_summary_read`, `Catalog::worn_armor_read`), para quem chama a cada tecla.
- `character`: as contas do personagem fora da ficha.
  - XP: `xp_floor(&Rules, &str) -> Option<f64>`, `level_price(&Rules, &str, f64) -> Option<f64>`, `xp_cost(&Rules, &str, from: Option<f64>, to: f64) -> Option<f64>`; custos `feat_cost` (a chave do dado é `tecnica`, mas a conta é a da Proeza), `art_cost`, `effect_cost`, `specialty_cost(&Rules, f64, secondary: bool)`.
  - `appearance_mod`, `DamageMode` (`Cut`, `Pierce`, `Impact`, com `id()` e `from_id()`), `natural_soak`, `mode_soak`, `pierce_gate_opens`, `death_limit`, `past_death_limit`, `size_from_label`, `size_modifier`, `art_lifting`, `maneuver_roll`.
  - O total de XP gasto (o `recompute()` da tela) e as migrações da ficha salva ainda não estão aqui.
- `dice`: os dados (port de `rolagem.ts`). `DiceExpr { dice, flat }` com `DiceExpr::parse(&str)` (as leituras `baseDadosDeExpr` e `flatDeExpr`, com as mesmas expressões regulares), `Roll { dice, flat, rolls, total }`, `adjusted_dice(base, extra) -> f64` (`dadosAjustados`), `d6(&mut Mulberry32) -> f64` e `roll_expr(&str, extra_dice, extra_flat, &mut Mulberry32) -> Roll` (`rolarExpr`; os dados saem um por vez, na ordem).
- `strike`: o lance (port de `lance.ts`).
  - Fontes de dados: o trait `DiceSource` (`roll(&mut self, &str, extra_dice, extra_flat) -> (f64, Vec<f64>)`), `RolledDice<'a>(&'a mut Mulberry32)` (a `fonteRolada`) e `FixedDice::new(attack, damage)` (a `fonteFixa`, dados que já caíram).
  - Tipos: `StrikeAttacker`, `StrikeTarget`, `StrikeInput`, `StrikeOutcome`, `BlowDamage { gross, absorbed, net }`.
  - Funções: `effective_defense(&StrikeTarget) -> Option<f64>` (`defesaEfetiva`), `resolve_strike(&StrikeInput, &mut impl DiceSource) -> StrikeOutcome` (`resolverGolpe`; o dano só é rolado no acerto), `blow_damage(Option<AttackOutcome>, rolled, graze_damage, soak, slips) -> BlowDamage` e `blow_slips(&Rules, gate_on, weapon_pierce: Option<f64>, pierce_level: Option<f64>) -> bool`. O dano do golpe (piso do raspão e portão da Perfuração) mora só em `blow_damage`: no acerto, o rolado (nunca abaixo de 0) menos a Absorção, com piso no dano do raspão; no raspão, o dano fixo, sem Absorção; no erro, nada; quando o golpe resvala, nada, nem o piso (a conta do Grid que desce a Vida).
- `js::max`, `js::min`, `js::number_to_string`: `Math.max`, `Math.min` e `String(número)` do JavaScript.
- Daqui em diante, por fase: derivados da ficha e das criaturas (fase 1), combate e tempo em Ticks, Artes, lógica do Grid (fase 3). Cada módulo entra no contrato quando a API proposta é aprovada neste arquivo.

**Aritmética:** os números do TypeScript são `f64`. O port usa `f64` onde o TypeScript faz conta com fração e reproduz `Math.round` (o .5 sobe, inclusive nos negativos: `Math.round(-2.5) == -2`), `Math.floor` e `Math.trunc` exatamente.

**Consome:** tipos de `centelha-data`, a `libm` (com `default-features = false`, ADR 0002) e o `serde_json` (com `float_roundtrip` e `preserve_order`, ADR 0004). Nenhuma outra dependência normal.

## centelha-supabase

**Faz:** fala com o Supabase do jogo (tabelas, funções RPC, arquivos e tempo real). Continua o mesmo banco do site atual (R2).

**Expõe:** a definir pelo Backend em ADR (fase 3), incluindo como fazer o tempo real no WebAssembly.

**Consome:** a URL e a chave anon por configuração (nunca no código). Chave de serviço nunca.

**Admin (R22):** admin é só quem a função `eh_admin` do banco aprova. Nenhum e-mail, id de usuário ou lista de admins no código, em constante, teste, comentário ou configuração do repositório. A conferência devolve sim ou não, e qualquer falha (rede, erro do banco, sessão vencida ou ausente) é não. Sem cache que mantenha alguém admin depois de a sessão cair: a resposta vale para a sessão em que foi pedida e some com ela. Toda ação de admin que grava é barrada no próprio banco (RLS ou função que confere `eh_admin`); o cliente não é proteção.

## centelha-web

Framework e versões: ADR 0003 (Leptos com ilhas; site estático gerado por nós).

**Faz:** o site. Páginas do livro geradas em HTML no build, com o texto inteiro; partes interativas como ilhas em WebAssembly.

**Expõe:**

- Sem features padrão. As features `ssr` e `hydrate` nunca se combinam (o Leptos não aceita as duas juntas); clippy e testes rodam por conjunto: nativo com `--features ssr`, `wasm32-unknown-unknown` com `--features hydrate`.
- Feature `ssr` (só nativo): os componentes que desenham as páginas e o binário gerador (`required-features = ["ssr"]`), que grava `dist/` (uma pasta por página, `index.html` dentro). O Markdown vira HTML aqui, e só aqui.
- Feature `hydrate` (só `wasm32-unknown-unknown`): a biblioteca do navegador, que exporta a função `hydrate` chamada pelo script de carga das ilhas. Nada de Markdown, de leitura de `content/` nem de roteador no cliente.
- Dados de uma ilha: pequenos vão nas propriedades da ilha (com `serde`, gravadas no HTML pelo gerador); grandes (ficha, catálogos) vão num `<script type="application/json">` da própria página, que a ilha lê. Ilha não busca conteúdo do livro pela rede.
- O caminho base vem de `SITE_BASE_PATH` no build (padrão `/centelha/`).
- Fontes (R13): Cinzel e EB Garamond servidas pelo próprio site, em `crates/centelha-web/fonts/` com o `OFL.txt` de cada família; só `woff2`, subconjunto latino, só os pesos usados, `font-display: swap` e `unicode-range`; pré-carga só da fonte do texto corrido. Nada de serviço externo de fontes.

**Consome:** `centelha-data` (leitura de `content/`, no gerador), `centelha-engine` (toda conta que uma página mostra), `leptos`, `leptos_meta`, `wasm-bindgen`, `web-sys`, `js-sys`, `serde`, e só na feature `ssr` o `pulldown-cmark` (Markdown) e o que a chave dos diagramas pedir (SHA-1). Versões no ADR 0003. Depois, `centelha-grid` e `centelha-supabase`.

**Admin na tela (R22):** esconder um controle de admin é conveniência, não proteção. A tela pergunta ao `centelha-supabase` (que pergunta à `eh_admin`) e mostra o controle só com um sim; enquanto não há resposta, ou com falha, mostra como não admin. Nada de e-mail fixo nem lista de admins no site. Ação de admin que grava só existe se o banco a barra por conta própria; uma ação do site atual protegida só no navegador vira aviso ao Coordenador-RS antes do port.

**Não faz:** conta de regra (pede ao `centelha-engine`), mudança no texto do livro (o que está em `content/` sai como está), corte ou recodificação de imagem.

## centelha-grid

**Faz (fase 3):** o tabuleiro da mesa, desenhado em canvas, como ilha Leptos (`#[island]`: no modo ilhas só a ilha roda no navegador) usada pelo `centelha-web` (e, pelo WebView, no app). O elemento é pego por `NodeRef::<Canvas>` e o desenho roda só no cliente. Texto no canvas: o canvas não troca a fonte sozinho quando ela chega; antes de desenhar texto, esperar `document.fonts.load(...)` da fonte usada e redesenhar quando ela chegar. Rótulos pequenos do tabuleiro podem usar a fonte do sistema se a legibilidade pedir (decisão de desenho na fase 3).

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
