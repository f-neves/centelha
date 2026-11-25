# Glossário · português para inglês

Fonte única dos nomes dos termos do jogo no código (R8): o identificador em inglês sai daqui. Os dados continuam em português; a tradução fica na leitura (`serde(rename)`). Termo que não está aqui: pedir ao Arquiteto antes de inventar.

Estado: **proposta da fase 0, em aprovação.** A coluna "nome próprio" marca o que se propõe NÃO traduzir.

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

## Nomes próprios

Nomes de Caminhos, Técnicas, Artes, criaturas e lugares são texto do jogo: ficam em português, nos dados, e não viram identificador. No código eles aparecem só como `id` (a string do JSON, como `punho-de-ferro`).
