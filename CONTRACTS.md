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

- Módulos: `bestiary`, `rules`, `weapons`, `armor`, `shields`, `ancestry`, `techniques`, `paths`, `arts`, `effects` (e `chapters` e `glossary`, abaixo). Os outros arquivos de dados (atributos, Habilidades, Virtudes, condições) entram como módulo quando a API proposta for aprovada neste arquivo.
- Na raiz do crate: `Creature`, `Rules`, `Weapon`, `Armor`, `Shield`, `Ancestry`, `Technique`, `Art`, `Effect`, `Chapter`, `GlossaryEntry`, `DataError`, os `parse_*` e os `load_*` (de todos os módulos). `paths::Path` fica fora da raiz. O resto mora no módulo de cada um (por exemplo `bestiary::Size`).
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
- Catálogos da fase 2, no molde do `glossary` (`parse_*(&str) -> Result<Vec<_>, DataError>`, `load_*(content_dir)` só no nativo; campo desconhecido não dá erro; id vazio ou repetido dá `Invalid`):
  - `techniques::{Technique, TechniqueKind, TechniqueCost}`, de `tecnicas.json`, com `parse_techniques`/`load_techniques`;
  - `paths::{Path, Track}`, de `caminhos.json` (as Proezas), com `parse_paths`/`load_paths`. O tipo `Path` não é reexportado na raiz (o nome coincide com `std::path::Path`): use `paths::Path`;
  - `arts::{Art, ArtCategory, ArtLevel, ArtCost, ArtGrid}`, de `artes.json`, com `parse_arts`/`load_arts`;
  - `effects::{Effect, EffectArt, EffectParameter, ParameterKind, EffectGrid}`, de `efeitos.json`, com `parse_effects`/`load_effects`;
  - em `Rules`: `rules::{FeatScales, FeatScaleTrack}` e `Rules::feat_scales` (`escalasProeza.trilhas`);
  - combate e força: `rules::{CombatRules, PgrRules, WindupRule, PgrReform, PgrRow, FlurryRules, DualWieldRules}` (`combate.pgr`, `rajada`, `dupla`) e `StrengthRules::throw: ThrowRules`;
  - para a ficha interativa: `Ancestry::cost: Option<f64>` (o `custo` da Raça) e `Rules::arcane: Option<rules::ArcaneRules>`, com `ArcaneRules { art_level_cap: Option<ArtLevelCap> }` e `ArtLevelCap { by_spark: Vec<f64> }` (`arcano.tetoNivelArte.porCentelha`);
  - em `Rules`: `Rules::spark_scale` (`escalaCentelha`) e `Rules::willpower_scale` (`escalaVontade`), `Vec<rules::ScaleStep>` na ordem do arquivo; `ScaleStep { level, label, text, provisional }` (`provisional` é falso quando a chave falta);
  - cada `enum` desses tem `id()`, que devolve o texto do JSON.
- `serde_json` com `float_roundtrip` (a leitura de `f64` tem de ser exata para a paridade).

**Expõe também (feito pelo Frontend por exceção):** módulo `chapters`, com `Chapter { slug, order, numeral, title, summary, chapter_name, body }`. `chapter_name: Option<String>` é o campo opcional `capitulo` do schema do site atual (o nome do capítulo de várias páginas quando a primeira página tem outro título); os outros quatro campos do cabeçalho são obrigatórios. `body` é o Markdown sem o cabeçalho. Funções: `parse_chapter(slug: &str, text: &str) -> Result<Chapter, DataError>` e, no nativo, `load_chapters(content_dir: &Path) -> Result<Vec<Chapter>, DataError>` em ordem de `order`. O `slug` é o nome do arquivo sem `.md`. Sem dependência nova de YAML: o cabeçalho é uma linha por campo, `chave: valor`, separado no primeiro dois-pontos, com valor entre aspas duplas (texto) ou número inteiro (`ordem`); o que não seguir isso, ou chave desconhecida, dá erro `Invalid`.

**Expõe também (feito pelo Frontend por exceção):** módulo `glossary`, com `GlossaryEntry { id: String, term: String, aliases: Vec<String>, definition: String }` lido de `glossario.json` (`termo`, `aliases`, `definicao`), `parse_glossary(json: &str) -> Result<Vec<GlossaryEntry>, DataError>` e, no nativo, `load_glossary(content_dir: &Path)`. Id vazio ou repetido dá `Invalid`.

**Consome:** `content/` (nada mais).

## centelha-engine

**Faz:** as regras puras do jogo. Mesma entrada, mesma saída, idêntica ao motor TypeScript do site atual (paridade).

**Expõe (primeira versão):**

- `js::to_uint32(f64) -> u32`: o `>>> 0` do JavaScript (conversão da semente).
- `rng::Mulberry32`: o gerador aleatório. `Mulberry32::new(seed: u32)`, `next_u32(&mut self) -> u32`, `next_f64(&mut self) -> f64` (no intervalo [0, 1), igual ao TypeScript bit a bit), `draws(&self) -> u64` (quantos números já entregou, para os testes de paridade acharem a primeira divergência). Nenhum outro gerador aleatório no motor.
- `js::round` (`Math.round`, inclusive o `-0`), `js::log10` (pela `libm`, ADR 0002) e `js::pow` (com arredondamento correto, ver abaixo).
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
- Ficha interativa, em `sheet` (implementada em `sheet/editor.rs`, módulo privado; os itens são reexportados em `sheet`): port das contas de `src/lib/ficha-engine.ts`, que na fonte moram dentro de `montarFicha`. A ficha entra e sai em `Value` na forma de leitura (ADR 0004).
  - `EditorTables::new(SheetTables, EditorCatalogs<'_>)`, com `EditorCatalogs { attributes, skills, secondary, virtues, backgrounds: &Value, techniques: Vec<Technique>, arts: Vec<Art>, effects: Vec<Effect> }` (os JSON crus nos cinco primeiros), e `EditorTables::sheet_tables()`.
  - `fresh_sheet(&EditorTables) -> Value` (`fresh()`), com `FRESH_BUDGET = 1500` (o padrão; quem chama pode trocar o orçamento) e `FREE_SECONDARY` (as três Secundárias livres).
  - `spent_xp(&EditorTables, &Value) -> SpentXp` (`recompute()`), com as parcelas na ordem da soma (`attributes`, `skills`, `specialties`, `virtues`, `willpower`, `appearance`, `spark`, `secondary`, `techniques`, `arts`, `effects`, `backgrounds`, `ancestry`) e o `total`.
  - Tetos e piso: `trait_cap(&EditorTables, &Value, kind, key: Option<&str>) -> f64` (`capFor`) e `attribute_floor(&EditorTables, &Value, key) -> f64` (`pisoAttr`).
  - `change_ancestry(&EditorTables, &mut Value, ancestry, mass_chosen: bool)`: a troca de Raça; `mass_chosen` é o estado da ficha ao abrir (o `massaEscolhida` do `normalize`), que a tela guarda.
  - `mass_chosen(&Value) -> bool`: a conta do `normalize` (`ficha-engine.ts:305`) sobre a ficha na forma de leitura.
  - Especialidades: `clamp_specialties(&Value, skill_level) -> Vec<Specialty { name, level }>` (`clampSpecs`) e `specialty_cap(skill_level) -> f64` (metade do nível, para baixo).
  - `technique_status(&Value, &Technique) -> TechniqueStatus { owned, spark_ok, missing, ok }`: Centelha mínima e os pré-requisitos **diretos** que faltam (como a ficha da fonte; não o fecho).
  - `effects_in_reach(&EditorTables, &Value) -> Vec<&Effect>` (`efeitosAoAlcance`), por nível e depois por nome na ordem `pt`.
  - Tempo e movimento da ficha: `sheet_anatomy(&Rules, &[Weapon], &Value) -> Anatomy` (`anatomiaDaFicha`), `extra_lines(&Rules, &Value, &Value) -> Vec<ExtraLine>` (`linhasComExtra`), `sheet_flurry_cap(&Rules, &[Weapon], &Value) -> f64`; `half_movement_penalty(f64) -> f64` e `penalized_movement(&Movement, f64) -> PenalizedMovement` (a metade da penalidade física em cada medida, uma conta só para os dois lugares da ficha); `throw_range(&Rules, reserve, weight_kg, max_kg) -> f64` (`alcanceArremesso`).
  - As migrações do `normalize()` ainda não estão aqui (esperam a função pura pedida à equipe das regras).
- `character`: as contas do personagem fora da ficha.
  - XP: `xp_floor(&Rules, &str) -> Option<f64>`, `level_price(&Rules, &str, f64) -> Option<f64>`, `xp_cost(&Rules, &str, from: Option<f64>, to: f64) -> Option<f64>`; custos `feat_cost` (a chave do dado é `tecnica`, mas a conta é a da Proeza), `art_cost`, `effect_cost`, `specialty_cost(&Rules, f64, secondary: bool)`.
  - `appearance_mod`, `DamageMode` (`Cut`, `Pierce`, `Impact`, com `id()` e `from_id()`), `natural_soak`, `mode_soak`, `pierce_gate_opens`, `death_limit`, `past_death_limit`, `size_from_label`, `size_modifier`, `art_lifting`, `maneuver_roll`.
  - O total de XP gasto (o `recompute()` da tela) e as migrações da ficha salva ainda não estão aqui.
- `dice`: os dados (port de `rolagem.ts`). `DiceExpr { dice, flat }` com `DiceExpr::parse(&str)` (as leituras `baseDadosDeExpr` e `flatDeExpr`, com as mesmas expressões regulares), `Roll { dice, flat, rolls, total }`, `adjusted_dice(base, extra) -> f64` (`dadosAjustados`), `d6(&mut Mulberry32) -> f64` e `roll_expr(&str, extra_dice, extra_flat, &mut Mulberry32) -> Roll` (`rolarExpr`; os dados saem um por vez, na ordem).
- `strike`: o lance (port de `lance.ts`).
  - Fontes de dados: o trait `DiceSource` (`roll(&mut self, &str, extra_dice, extra_flat) -> (f64, Vec<f64>)`), `RolledDice<'a>(&'a mut Mulberry32)` (a `fonteRolada`) e `FixedDice::new(attack, damage)` (a `fonteFixa`, dados que já caíram).
  - Tipos: `StrikeAttacker`, `StrikeTarget`, `StrikeInput`, `StrikeOutcome`, `BlowDamage { gross, absorbed, net }`.
  - Funções: `effective_defense(&StrikeTarget) -> Option<f64>` (`defesaEfetiva`), `resolve_strike(&StrikeInput, &mut impl DiceSource) -> StrikeOutcome` (`resolverGolpe`; o dano só é rolado no acerto), `blow_damage(Option<AttackOutcome>, rolled, graze_damage, soak, slips) -> BlowDamage` e `blow_slips(&Rules, gate_on, weapon_pierce: Option<f64>, pierce_level: Option<f64>) -> bool`. O dano do golpe (piso do raspão e portão da Perfuração) mora só em `blow_damage`: no acerto, o rolado (nunca abaixo de 0) menos a Absorção, com piso no dano do raspão; no raspão, o dano fixo, sem Absorção; no erro, nada; quando o golpe resvala, nada, nem o piso (a conta do Grid que desce a Vida).
- `technique`: as contas das Técnicas que a árvore, o catálogo e a ficha usam.
  - `technique_modifier(&Rules, effect, level) -> Option<TechniqueModifier { value, title }>` (`modProeza` de `data.ts`; o título é o do `data.ts`, com a unidade; a árvore monta o texto curto dela com o `value` e o rótulo da escala). Dá `None`, como a fonte, na trilha `estado`, na trilha que não existe, no nível fora dos `values` (1 a 6; nível não inteiro também) e no valor vazio ou só com o traço.
  - `prerequisite_closure<'a>(&'a [Technique], id: &str) -> Vec<&'a str>`: os pré-requisitos diretos e indiretos da Técnica, sem ela mesma, na ordem do `closure` da árvore (o da fonte é a Técnica seguida desta lista); pré-requisito que não está na lista dada fica de fora.
  - `chain_xp(&Rules, &[Technique], id) -> f64`: o XP da Técnica mais o da cadeia, pelo `feat_cost` do nível (0 para id de fora).
- `bestiary_card`: o card do bestiário (port de `vistaPublicaCriatura` de `motor.ts`, do passo do `inimigos.json` e do `build` de `scripts/gen-monsters.mjs`). Entrada e saída em `serde_json::Value` (chaves em português, as do `monsters.json`), como extensão da ADR 0004 (o card copia boa parte da fonte; as mesmas regras de `preserve_order` e `shift_remove` valem).
  - `CardTables::new(creature::Tables, weapons: Value, rules: Value, skills: &Value, &[Technique], &[Art]) -> Result<CardTables, CardError>` e `CardTables::creature_tables()`; `bestiary_card(&CardTables, source: &Value) -> Result<Value, CardError>` devolve a entrada do `monsters.json`; `CardError` (`Creature`, `MissingReference`, `NoMovement`, `Tables`).
  - `form_entry(main: &Value, form: &Value) -> Value`: o `comForma` sobre o JSON cru (o `creature::with_form` é a mesma conta sobre os tipos; os dois têm de concordar).
  - Contas que o card faz na hora: `hp_line_in_zone(&Rules, kg, stamina, Option<HpLine>) -> Option<HpLineInZone { zone_kg, line, hp_above, hp_below }>` (`linhaPVNaZona`), `card_mass(&Rules, body: &Value) -> Option<f64>` (`massaDaConta`), `pool_with_penalty(&str, penalty) -> String` (`poolComPenalidade`), `card_appearance_mod(f64) -> Option<f64>` (a curva do próprio card, que **não** é a regra; a regra é `character::appearance_mod`; a diferença entre as duas foi levada à equipe das regras, e o card reproduz a curva dele até a resposta).
- `collate`: a ordem alfabética do `localeCompare(b, 'pt')`, uma só para o motor e o site. `compare` (base, depois acento, depois caixa), `compare_base` (`sensitivity: 'base'`), `compare_numeric_base` (`numeric: true`, `sensitivity: 'base'`), `fold`, `primary_key` e `covers(char) -> bool`. A cobertura (ASCII imprimível, espaço e letras que se decompõem numa letra ASCII com acento) está no cabeçalho do módulo; fora dela a ordem pode diferir do ICU, e um teste reprova nome de catálogo com letra fora da cobertura.
- `timeline` (a parte do tempo que a ficha usa; o resto do tempo do combate fica para a fase 5): `TimeSystem`, `Physics`, `TimeClass`, `Maneuver`, `Anatomy`, `AnatomyInput`; `catalog_weapon`, `weapon_class`, `time_class`, `weapon_speed`, `windup_of`, `flurry_cap`, `dual_wield_extra_cycle`, `anatomy`.
- `js::pow` com arredondamento correto (ADR 0002, nota de 26/11); `js::array_length` e `js::or_zero` públicos.
- `js::max`, `js::min`, `js::number_to_string`: `Math.max`, `Math.min` e `String(número)` do JavaScript.
- Daqui em diante, por fase: derivados da ficha e das criaturas (fase 1), combate e tempo em Ticks, Artes, lógica do Grid (fase 3). Cada módulo entra no contrato quando a API proposta é aprovada neste arquivo.

**Aritmética:** os números do TypeScript são `f64`. O port usa `f64` onde o TypeScript faz conta com fração e reproduz `Math.round` (o .5 sobe, inclusive nos negativos: `Math.round(-2.5) == -2`), `Math.floor` e `Math.trunc` exatamente.

**Consome:** tipos de `centelha-data`, a `libm` (com `default-features = false`, ADR 0002) e o `serde_json` (com `float_roundtrip` e `preserve_order`, ADR 0004). Nenhuma outra dependência normal.

## centelha-supabase

**Faz:** fala com o Supabase do jogo (tabelas, funções RPC, arquivos e tempo real). Continua o mesmo banco do site atual (R2).

**Expõe:** a definir na primeira entrega (só leitura), pela ADR 0007: pedido montado sem IO, envio por alvo atrás de um `Transport` (`gloo-net` no WebAssembly, `reqwest` no nativo), toda leitura de dado de mesa com `Session`, `is_admin` só pela `eh_admin`. O tempo real vem depois, em PR próprio.

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

**Armazenamento do navegador (R26).** Até o domínio próprio, o site em Rust e o site atual moram na mesma origem (`f-neves.github.io`) e dividem o `localStorage`, o `sessionStorage` e a sessão do Supabase. Regras:
1. Em chave que o site atual também lê, o Rust grava só valores no mesmo formato e com o mesmo significado; nunca apaga, renomeia nem muda o formato.
2. Chave que só o Rust usa leva o prefixo `rs:` (por exemplo `rs:algo`).
3. Toda chave dividida está na tabela abaixo, com o formato; chave nova dividida entra aqui no mesmo PR que começa a usá-la. A Revisora-RS confere isso em todo PR que mexa em armazenamento.

| chave | onde | formato (o do site atual) | o que o Rust faz hoje |
|---|---|---|---|
| `tema` | `localStorage` | `classico`, `escuro` ou `legivel`; sem a chave vale `escuro` | lê no `<head>` e grava ao trocar o tema |
| `sidebar` | `localStorage` | `collapsed` ou `open` | lê no `<head>` e grava ao recolher ou abrir a barra |
| `centelha:config` | `localStorage` | objeto JSON das preferências de `src/lib/config-site.ts` (largura, fonte, entrelinha, margem, colunas, fundo, cores por tema...) | só lê, no `<head>` (gravar entra com a página de preferências) |
| `centelha:consulta` | `sessionStorage` | lista JSON de textos `tipo:id` (os termos fixados no cartão do glossário; `termo` para os do glossário) | lê e grava, no mesmo formato |
| sessão do Supabase | `localStorage` (a chave do `supabase-js`) | a do `supabase-js` | não usa (fase 3) |

**Preferências de leitura (R25):** largura, fonte, entrelinha, colunas e fundo são de cada leitor, como o tema; não são ação de admin e nenhum controle delas fica escondido por `eh_admin`. Sem conta, ficam no navegador (`centelha:config`, com `try/catch` como o tema); com conta, no `profiles.config` da própria conta (fase 3). Os valores têm limites que não quebram a leitura a 375 px.

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
