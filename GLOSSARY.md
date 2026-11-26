# Glossário · português para inglês

Fonte única dos nomes dos termos do jogo no código (R8): o identificador em inglês sai daqui. Os dados continuam em português; a tradução fica na leitura (`serde(rename)`). Termo que não está aqui: pedir ao Arquiteto antes de inventar.

Estado: **aprovado em 25/11/2026; é regra.** Identificador de termo do jogo sai desta tabela; nome fora dela, só depois de entrar aqui (pedido ao Arquiteto-RS). Os acréscimos das seções de criaturas e derivados seguem a mesma regra. A coluna "nome próprio" marca o que NÃO se traduz.

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

## Glossário do livro (acréscimo de 26/11, tarefa 003 do Frontend)

| Português | Inglês |
|---|---|
| verbete do glossário | `glossary_entry` (`GlossaryEntry`) |
| termo | `term` (`termo`) |
| sinônimo | `alias` (`aliases`) |
| definição | `definition` (`definicao`) |
