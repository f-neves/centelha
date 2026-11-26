# Glossário · português para inglês

Fonte única dos nomes dos termos do jogo no código (R8): o identificador em inglês sai daqui. Os dados continuam em português; a tradução fica na leitura (`serde(rename)`). Termo que não está aqui: pedir a inclusão antes de inventar.

Estado: **aprovado em 25/11/2026; é regra.** Identificador de termo do jogo sai desta tabela; nome fora dela, só depois de entrar aqui. Os acréscimos das seções de criaturas e derivados seguem a mesma regra. A coluna "nome próprio" marca o que NÃO se traduz.

`gm_compendium_entry` é provisório até se decidir o nome da tabela do banco (fase 3). `melee` (Armas) foi conferido no livro: a Habilidade Armas não cobre lançar a arma, que é Arremesso (`throwing`).

## Termos centrais

| Português | Inglês no código | Nome próprio | Nota |
|---|---|---|---|
| Centelha (o jogo) | `Centelha` | sim | nome do jogo e dos crates |
| Centelha (o nível de poder) | `spark` | | `escalaCentelha` vira `spark_scale` |
| Proeza | `feat` | | |
| Técnica | `technique` | | |
| Caminho | `path` | | a trilha do Caminho: `track` (`trilha`) |
| Arte | `art` | | as Artes mágicas; `artes-grid` vira `art_grid` |
| Efeito | `effect` | | |
| Desgaste | `strain` | | |
| Absorção | `soak` | | |
| Defesa | `defense` | | Defesa Mental: `mental_defense`; Social: `social_defense`; de agarrão: `grapple_defense` |
| Tick | `tick` | | unidade de tempo do combate |
| Margem | `margin` | | |
| Habilidade | `skill` | | Habilidade Secundária: `secondary_skill` |
| Atributo | `attribute` | | |
| Virtude | `virtue` | | |
| Força de Vontade | `willpower` | | |
| Iniciativa | `initiative` | | |
| Pool (parada de dados) | `pool` | | |
| Dificuldade | `difficulty` | | |
| Modos de dano | `damage_modes` | | |
| Nível de Perfuração | `pierce_level` | | |
| Penetração | `penetration` | | |
| Energia | `energy` | | |
| Mana | `mana` | | |
| Integridade | `integrity` | | também é Habilidade |
| Especialidade | `specialty` | | |
| Nível | `level` | | |
| Firula | `flourish` | | |
| Valor Passivo | `passive_value` | | |
| Tamanho | `size` | | |
| Velocidade (custo em Ticks) | `speed` | | |
| Condição | `condition` | | |
| Ferimento | `wound` | | |
| Criatura | `creature` | | |
| Bestiário | `bestiary` | | |
| Ameaça | `threat` | | campo `ameaca` da criatura |
| Couraça | `carapace` | | `perfilCouraca`: `carapace_profile` |
| Ficha | `character_sheet` | | `ficha-engine`: `sheet` no módulo |
| Personagem | `character` | | |
| Mesa | `table` | | a sessão de jogo; no banco, o nome da tabela fica como está |
| Mestre | `game_master` | | |
| Jogador | `player` | | |
| Grid | `grid` | | o tabuleiro da mesa |
| Arena | `arena` | | |
| Peça (token) | `token` | | |
| Encontro | `encounter` | | |
| Combatente | `combatant` | | |
| Lance (resolver golpe) | `strike` | | `lance.ts` |
| Golpe | `blow` | | |
| Arma | `weapon` | | |
| Armadura | `armor` | | |
| Escudo | `shield` | | |
| Equipamento | `equipment` | | |
| Raça | `ancestry` | | `racas.json` |
| Antecedente | `background` | | |
| Horda | `horde` | | |
| Sangramento | `bleeding` | | |
| Quase acerto | `near_miss` | | |
| Acaso (gerador aleatório) | `rng` | | `Mulberry32` |
| Códice do Mestre | `gm_compendium_entry` (provisório) | | o nome da tabela do banco é pergunta em aberto (plano, seção 8, item 7) |

## Atributos

| Português | Inglês | Grupo |
|---|---|---|
| Força | `strength` | físico (`physical`) |
| Destreza | `dexterity` | físico |
| Vigor | `stamina` | físico |
| Influência | `influence` | social (`social`) |
| Perspicácia | `insight` | social |
| Compostura | `composure` | social |
| Percepção | `perception` | mental (`mental`) |
| Inteligência | `intelligence` | mental |
| Raciocínio | `wits` | mental |

## Habilidades

| Português | Inglês | Português | Inglês |
|---|---|---|---|
| Briga | `brawl` | Persuasão | `persuasion` |
| Armas | `melee` | Oratória | `oratory` |
| Arremesso | `throwing` | Sociabilidade | `socializing` |
| Atirador | `marksmanship` | Empatia | `empathy` |
| Esquiva | `dodge` | Manha | `streetwise` |
| Bloqueio | `block` | Política | `politics` |
| Atletismo | `athletics` | Investigação | `investigation` |
| Furtividade | `stealth` | Conhecimentos Gerais | `general_knowledge` |
| Prontidão | `alertness` | Ocultismo | `occult` |
| Resistência | `resilience` | Burocracia | `bureaucracy` |
| Integridade | `integrity` | Sobrevivência | `survival` |
| Prestidigitação | `sleight_of_hand` | Ofícios Gerais | `general_crafts` |

## Virtudes

| Português | Inglês |
|---|---|
| Compaixão | `compassion` |
| Convicção | `conviction` |
| Temperança | `temperance` |
| Bravura | `bravery` |

## Livro e páginas (acréscimo de 25/11, ADR 0003)

| Português | Inglês | Nota |
|---|---|---|
| capítulo | `chapter` | arquivo de `content/chapters` |
| ordem | `order` | `ordem` no cabeçalho |
| numeral | `numeral` | o algarismo romano do capítulo |
| título | `title` | `titulo` |
| resumo | `summary` | `resumo` |
| nome do capítulo (de várias páginas) | `chapter_name` | `capitulo`, opcional |
| corpo (do capítulo) | `body` | o Markdown sem o cabeçalho |
| diagrama | `diagram` | `diagramas.json` vira `diagrams` |
| tema (`classico`, `escuro`, `legivel`) | `theme` (`Classic`, `Dark`, `Readable`) | o valor gravado continua o português |
| ilha (parte interativa) | `island` | termo do Leptos |

## Nomes próprios

Nomes de Caminhos, Técnicas, Artes, criaturas e lugares são texto do jogo: ficam em português, nos dados, e não viram identificador. No código eles aparecem só como `id` (a string do JSON, como `punho-de-ferro`).

## Criaturas e ataques (acréscimo de 25/11, pedido do Backend)

| Português | Inglês | Português | Inglês |
|---|---|---|---|
| corpo | `body` | classe (do ataque) | campo `class`, tipo `AttackClass` |
| enxame | `swarm` | dado | `dice` |
| forma (de criatura) | `form` | multiplicador de Força | `strength_multiplier` |
| forma principal | `main_form` | tipo de dano | `damage_type` |
| ataque | `attack` | bloqueia como arma | `blocks_as_weapon` |
| acerto | `accuracy` | arsenal | `arsenal` |
| PV (pontos de vida) | `hp` | vestida | `worn` |
| bônus de PV | `hp_bonus` | veneno | `poison` |
| linha de PV | `hp_line` | indivíduos por quadrado | `individuals_per_square` |
| massa | `mass` | Tamanho equivalente | `equivalent_size` |
| sem massa | `massless` | Tamanho escolhido | `chosen_size` |
| não ergue | `cannot_lift` | segundo Tamanho | `second_size` |
| categoria | `category` | | |

**P/G/R** (as três parcelas da Velocidade, `pgr`): o campo vira `timing`, com `windup` (Preparo), `blow` (Golpe) e `recovery` (Recuperação). Na leitura, `#[serde(rename = "pgr")]`.

**Tamanhos:** `minusculo` `Minuscule`, `miudo` `Tiny`, `pequeno` `Small`, `medio` `Medium`, `grande` `Large`, `enorme` `Huge`, `imenso` `Immense`, `colossal` `Colossal`.

**Perfis de Couraça:** `rigida` `Rigid`, `couro` `Leather`, `mole` `Soft`. **Linha de PV:** `acima` `Above`, `abaixo` `Below`.

**Classes do ataque:** `leve` `Light`, `media` `Medium`, `pesada` `Heavy`, `haste` `Polearm`, `haste-media` `MediumPolearm`, `haste-guerra` `WarPolearm`, `distancia` `Ranged`, `arremesso` `Thrown`.

**Categorias de criatura:** `aberracao` `Aberration`, `animal` `Animal`, `besta-magica` `MagicalBeast`, `celestial` `Celestial`, `construto` `Construct`, `corruptor` `Corruptor`, `dragao` `Dragon`, `elemental` `Elemental`, `fada` `Fey`, `gigante` `Giant`, `humanoide` `Humanoid`, `humanoide-monstruoso` `MonstrousHumanoid`, `limo` `Ooze`, `morto-vivo` `Undead`, `outsider` `Outsider`, `planta` `Plant`.

## Derivados, regras e catálogos (acréscimo de 25/11, tarefa 002 do Backend)

Oficializados como o Backend usou: `hp_line_reason` (`linhaPVMotivo`); sufixo `Patch` para a versão parcial que a forma aplica (`BodyPatch`, `AttributesPatch`, `EquipmentPatch`); `Timing`; `MainForm`; `ArsenalItem.reference` (`ref`); `WornArmor`; `lifted_weight` (peso erguido); `mass_band` (faixa de massa); `dodge` e `block` (Esquiva e Bloqueio como Defesa); `roll` (jogada); `penalty` (Penalidade); `against_shots` (`contraTiro`); `with_equipment` (`comEquipamento`); `Defense::Immune` (imune); `SwarmState` (`full_hp`, `individuals`, `minimum`, `still_swarm`); Absorção por tipo de dano `impact`, `cut`, `pierce` (Impacto, Corte, Perfuração); `pierce_level` também para o `resistPerf` da armadura.

Regras (`regras.json`): `size_accuracy` (`porteAcerto`), `derived` (`derivados`), `by_size` (`porte`), `tolerance` (`folga`), `floor` (`piso`), `strength_damage` (`danoForca`) com `one_hand`/`two_hands`, `scale_by_size` (`escalaPorTamanho`), `plus_wits`, `plus_willpower`, `traits` (`tracos`), `spark_mult`; os demais nomes técnicos das curvas seguem a tradução literal da chave.

Catálogos: `stats` (o bloco `arma`/`armadura`/`escudo`), `skill` (`pericia`), `damage_bonus`, `hands` (`maos`), `strength_cap`, `strength_min`, `melee_block` (`bloqCaC`), `vs_fast_projectile` (`vsProjetilRapido`), `blocks`.

Habilidades Secundárias de criatura (`musculatura`, `corrida`, `voo`, `halterofilismo`, `natacao`, `escalada`, `equilibrio` e outras) e as Habilidades de tiro do ataque natural (`disparo`, `atirador`, `arremesso`, ver `RANGED_SKILLS`) continuam texto: são chaves do mapa de Habilidades, não viram identificador. Quando uma delas aparecer como parâmetro de função, o nome em inglês é a tradução literal (`musculature`, `weightlifting`).

## Ficha do jogador (acréscimo de 26/11, tarefa 003 do Backend)

| Português | Inglês | Português | Inglês |
|---|---|---|---|
| Deslocamento | `movement` | conjunto (de armas nas mãos) | `hand_set` |
| passo (de Batalha) | `step`, campo `battle` | mão hábil / inábil | `main_hand` / `off_hand` |
| Arranque | `dash` | empunhadura dupla | `dual_wield` (`DualWield`) |
| Corrida | `run` | peça personalizada | `custom` |
| Natação | `swim` | armaduras vestidas | `worn_armor` |
| salto vertical / parado / com corrida | `vertical_jump` / `standing_jump` / `running_jump` | Defesa das mãos (soma) | `hands_defense` |
| Investida | `charge` | Força que falta (requisito) | `strength_missing` |
| Força a mais da Investida | `charge_strength` | arma do ataque | `attack_weapon` |
| Proezas (`tech` na ficha de leitura) | `feats` | Perfuração da arma | `weapon_pierce` |
| Aparência | `appearance` | Valor Passivo (o tipo) | `Passive` |
| Vigor somado aos PV (pela Raça) | `vitality` | | |

O passo de Batalha é `battle` em todo lugar: na saída da ficha (`Movement.battle`, `Step.battle`) e nas regras lidas (`rules::MovementRules.battle`, lido da chave `derivados.deslocamento.normal`). A soma de Defesa das mãos é `hands_defense`.

## Glossário do livro (acréscimo de 26/11, tarefa 003 do Frontend)

| Português | Inglês |
|---|---|
| verbete do glossário | `glossary_entry` (`GlossaryEntry`) |
| termo | `term` (`termo`) |
| sinônimo | `alias` (`aliases`) |
| definição | `definition` (`definicao`) |

## Personagem, XP e Quase-Acerto (acréscimo de 26/11, tarefa 004 do Backend)

| Português | Inglês | Português | Inglês |
|---|---|---|---|
| raspão | `graze` | piso de XP | `xp_floor` |
| errou por | `missed_by` | preço do nível | `level_price` |
| saída do ataque | `attack_outcome` (`Hit`, `Graze`, `Miss`) | trilha de XP / tipo da trilha | `XpTrack` / `XpKind` (`acum` `Accumulated`, `flat` `Flat`, `gratis` `Free`) |
| limite da morte | `death_limit` | custo da Proeza (chave `tecnica`) | `feat_cost` |
| passou do limite | `past_death_limit` | levantamento (da Arte de força) | `lifting`, `art_lifting` |
| modo de ataque | `DamageMode` (`Cut`, `Pierce`, `Impact`) | jogada da manobra | `maneuver_roll` |
| Absorção natural | `natural_soak` | curva | `curve` |
| portão da Perfuração | `pierce_gate` | reforma (por categoria) | `reform`, `per_category` |

`DamageMode` segue o `pierce` da Absorção por tipo: a variante é `Pierce`.

## Dados e lance (acréscimo de 26/11, tarefa 006 do Backend)

| Português | Inglês | Português | Inglês |
|---|---|---|---|
| fonte de dados | `DiceSource` | dano líquido | `net` |
| dados que já caíram | `FixedDice` | resvalar (no portão da Perfuração) | `slips` |
| dados rolados | `RolledDice` | | |
| rolagem | `Roll` | Defesa perdida | `lost_defense` |
| expressão de dados | `DiceExpr` | condições da Defesa | `conditions_defense` |
| dados ajustados | `adjusted_dice` | penalidade de dados do golpe | `blow_penalty_dice` |
| Defesa efetiva | `effective_defense` | índice do golpe | `blow_index` |
| dano bruto | `gross` | modificador manual | `manual_mod` |
| dano absorvido | `absorbed` | margem do raspão / dano do raspão | `graze_margin` / `graze_damage` |

## Catálogos da fase 2 (acréscimo de 26/11, tarefa 008 do Backend)

Proeza, Caminho e Técnica: a fonte usa "Proeza" nos dois sentidos (às vezes o Caminho, como "Proeza Quebra-Muralhas"; às vezes a compra, como na ficha). No código: cada entrada de `caminhos.json` é um Caminho (`path`); cada Técnica comprada é uma `technique`, e a lista das compradas na ficha é `feats` (por isso `feat_cost`, que é o custo pelo nível da Técnica, e `feat_scales`, de `escalasProeza`, que dá a escala por trilha que cada Técnica usa); `technique_modifier` é o `modProeza`.

"Trilha" tem três sentidos no código, sempre com o tipo dizendo qual: a trilha de XP (`XpTrack`), a trilha de melhora da Técnica (`FeatScaleTrack`) e a trilha do Caminho (`Track`: `Body`, `Voice`, `Mind`, já na tabela de termos centrais).

| Português | Inglês | Português | Inglês |
|---|---|---|---|
| tipo da Técnica: passiva, ativa, reflexiva | `TechniqueKind`: `Passive`, `Active`, `Reflexive` | categoria da Arte: elemental, universal | `ArtCategory`: `Elemental`, `Universal` |
| trilha de melhora (da Técnica) | `effect` (o campo), `FeatScaleTrack` | atributo de conjuração | `casting_attribute` |
| escalas da Proeza | `feat_scales` | nível da Arte | `ArtLevel` |
| modificador da Técnica | `technique_modifier`, `TechniqueModifier` | exemplos | `examples` |
| rótulo / unidade / valores | `label` / `unit` / `values` | sabor (do Efeito numa Arte) | `flavor` |
| fecho dos pré-requisitos | `prerequisite_closure` | parâmetro: padrão, substitui, fixo | `ParameterKind`: `Standard`, `Replaces`, `Fixed` |
| XP da cadeia | `chain_xp` | escala / régua / pontos / por nível / custa Mana | `scale` / `ruler` / `points` / `per_level` / `mana_cost` |
| | | escalonável | `scalable` |
| Habilidade âncora | `anchor_skill` | ação livre | `free_action` |
| dano bruto (da Arte) | `raw_damage` | dado por nível | `dice_per_level` |

No `grid` do Efeito: forma `shape`, âncora `anchor`, gatilho `trigger`, alvo `target`, persiste `persists`, matéria `matter`, condição `condition`, condição aparente `apparent_condition`, pega item `takes_item`, arena inteira `whole_arena`, dissipa `dispels`, fere `harms`, cura `heals`, teste `test`.

## Card do bestiário e escalas (acréscimo de 26/11, tarefas 009 e 009a do Backend)

| Português | Inglês | Português | Inglês |
|---|---|---|---|
| card do bestiário | `bestiary_card` | massa da conta (do card) | `card_mass` |
| tabelas do card | `CardTables` | pool com a penalidade | `pool_with_penalty` |
| entrada da forma | `form_entry` | modificador da Aparência do card | `card_appearance_mod` |
| linha de PV na zona (de transição) | `hp_line_in_zone`, `HpLineInZone` | escala de Força de Vontade | `willpower_scale` |
| zona / PV da linha de cima / de baixo | `zone_kg` / `hp_above` / `hp_below` | degrau (de uma escala) | `ScaleStep` |
| provisório | `provisional` | escala de Centelha | `spark_scale` |

## Ficha interativa (acréscimo de 26/11, tarefa 010 do Backend)

| Português | Inglês | Português | Inglês |
|---|---|---|---|
| tabelas da ficha interativa | `EditorTables` | troca de Raça | `change_ancestry` |
| ficha nova | `fresh_sheet` | massa escrita pelo jogador | `mass_chosen` |
| orçamento (de XP) | `budget`, `FRESH_BUDGET` | especialidade nomeada | `Specialty { name, level }` |
| Secundárias livres | `FREE_SECONDARY` | teto das especialidades | `specialty_cap`, `clamp_specialties` |
| XP gasto | `spent_xp`, `SpentXp` | estado da Técnica | `TechniqueStatus` |
| parcela dos Antecedentes (no XP gasto) | `SpentXp::backgrounds` | Centelha que basta / pré-requisitos que faltam | `spark_ok` / `missing` |
| teto (de uma trilha) | `trait_cap` | Efeitos ao alcance | `effects_in_reach` |
| piso do Atributo | `attribute_floor` | teto da Arte pela Centelha | `arcane.art_level_cap.by_spark` |
| custo da Raça | `Ancestry::cost` | | |

## Colação e ficha, fatia 2 (acréscimo de 26/11, tarefas 011 e 012 do Backend)

| Português | Inglês | Português | Inglês |
|---|---|---|---|
| colação (a ordem alfabética do `localeCompare`) | `collate` | linha com o item extra | `extra_lines`, `ExtraLine` |
| letra de base | `fold`, `primary_key` | teto da rajada do item | `sheet_flurry_cap` |
| comparação só pela base / com números | `compare_base` / `compare_numeric_base` | metade da penalidade no deslocamento | `half_movement_penalty` |
| catálogos da ficha interativa | `EditorCatalogs` | deslocamento com a penalidade | `penalized_movement`, `PenalizedMovement` |
| linha Preparo, Golpe e Recuperação da ficha | `sheet_anatomy` | alcance do arremesso / Força aplicada (FAA) | `throw_range` / `reserve` |
| tabela do livro (a reforma do P/G/R) | `PgrReform`, `PgrRow` (`melee`, `shot`) | regras do arremesso | `ThrowRules` |

## Dados de texto e cliente do banco (acréscimo de 26/11, tarefas 014 e 014a do Backend)

| Português | Inglês | Português | Inglês |
|---|---|---|---|
| descrição de um nível | `LevelText` | tabela ou view do banco (PostgREST) | `Relation` (a mesa do jogo continua `Table`) |
| resiste (da Virtude) | `resists` | sessão (o token do usuário) | `Session` |
| conduta | `conduct` | | |

## Regras do Arcano (acréscimo de 26/11, tarefa 016 do Backend)

| Português | Inglês | Português | Inglês |
|---|---|---|---|
| improviso (da Arte) | `improvisation` | manifestação | `manifestation` |
| fonte do elemento (fonteElemental) | `elemental_source` | esticar | `stretch` |
| desvio (sair da área) | `detour` | Cura (a Arte) | `healing` (o campo `cura` do Efeito continua `heals`) |
| padrão (valor) | `standard` (no `SideState`, o campo do JSON `padrao` ficou `default`) | acima de (linha do esticar) | `above` |
| aberturas (graus do arco) | `openings` | estado no lado | `side_state` |
| fatias | `slices` | árvore de texto (de uma regra) | `Node` |

## Cliente do banco, só leitura (acréscimo de 26/11, tarefa 015 do Backend)

| Português | Inglês | Português | Inglês |
|---|---|---|---|
| configuração (URL e chave anon) | `Config` | transporte (quem envia o pedido) | `Transport` |
| consulta (select do PostgREST) | `Select` | está em (filtro `in`) | `is_in` (`in` é palavra reservada) |
| perfil | `Profile` | membro da mesa | `TableMember` |
| papel na mesa | `MemberRole` (`GameMaster`, `Player`) | situação da ficha | `CharacterStatus` (`Draft` rascunho, `Submitted` enviado, `Approved` aprovado) |
| colega do grupo (linha da `grupo_visivel`) | `PartyMember` | grupo visível | `visible_party` |
| é admin (pela `eh_admin`) | `is_admin` | migração (linha de `migracoes`) | `Migration` |
| código de convite | `invite_code` | XP inicial | `starting_xp` |
| revelar | `reveal` | gravar lances | `record_strikes` |
| dono | `owner` | conceito | `concept` |
| história | `history` | anotações | `notes` |
| nota de revisão | `review_note` | vaga / código da vaga | `slot` / `slot_code` |
| posição da imagem | `image_position` | à mão / aplicada em (migração) | `by_hand` / `applied_at` |
| resumo (da ficha na mesa: Vida, Energia) | `summary` (o mesmo termo do resumo do capítulo, em outro contexto) | | |

## Força, carga e mãos (acréscimo de 26/11, tarefas 013 e 014 do Backend)

| Português | Inglês | Português | Inglês |
|---|---|---|---|
| levantar acima da cabeça | `overhead_lift` | teto do arremesso | `throw_cap` |
| faixa de carga (Mínima, Leve, Média, Pesada, Máxima) | `LoadBand` | fração da velocidade com carga | `load_speed_fraction` |
| carga carregada / carga atual | `carried_load` / `current_load` | velocidade com a penalidade | `penalized_speed` |
| corte e expoente da carga | `cut` / `exponent` (`LoadRules`) | peso escrito como texto | `weight_from_text` |
| papel da peça nas mãos (nada, mão hábil, mão inábil) | `HandRole` (`None`, `Main`, `Off`) | marcar uso | `mark_hand_use` |
| ataque com as duas mãos | `DualAttack` | peça do arsenal para uma referência | `piece_for` |
| campo de edição da peça | `EditField` | sincronizar as mãos | `sync_slots` |

