# 0005 · Nomes internos de papel nas notas de revisão do bestiário

Data: 26/11/2026. Estado: aceita (decisão do autor, R16).

## Contexto

O conteúdo chega do site atual por cópia fiel (`cargo xtask sync-content`). As notas de revisão e de procedência de algumas criaturas citam quem fez uma estimativa pelo nome interno de um papel da equipe que revisou o bestiário ("estimativa do ...", "leitura da ..."). No site atual esses nomes ficam; no repositório em Rust, não.

## Decisão

1. **Primeira exceção à cópia fiel:** na cópia, esses nomes viram "revisão", com a concordância ajustada ("do X" vira "da revisão", "pelo X" vira "pela revisão"; o número da rodada, como em "X-5", sai junto). Nenhum número muda.
2. **Só nos campos previstos:** `motivo` e `justificativa` das criaturas em `data/bestiario/`, `justificativa` e a justificativa do ataque "Pinça do enxame" em `data/monsters.json`, `notaNumeros` em `data/comando-barra.json`. A troca é por linha, pela chave JSON da linha.
3. **Fora disso, a cópia para:** nome em outro campo, em outro arquivo ou com concordância não prevista dá erro e `content/` fica como estava. Assim um texto do jogo que coincida com o nome (como a Habilidade Arquitetura, que não casa porque a busca é por palavra inteira) não é estragado sem ninguém ver.
4. **Varredura:** os nomes entram na lista de termos do `cargo xtask scan` e do gancho, como palavra inteira; nenhuma cópia futura os traz de volta, nem a documentação do repositório. Três mensagens de merge de antes desta decisão citam um nome de papel no nome do ramo; como o histórico não se reescreve, a varredura deixa só essas três fora da busca dos nomes de papel (lista `ROLE_WORDS_HISTORIC` no `xtask`), e os termos de rastro continuam valendo para elas. Ramo novo não usa esses nomes.
5. O site atual (`rpg-system`) não muda; o `content/ORIGEM.txt` diz quantas trocas a cópia fez. O histórico do repositório não é reescrito.

## Consequências

- As notas são texto: não entram em conta e as referências de paridade não mudam.
- Campo novo com o nome na fonte faz a cópia parar; quem cuida da cópia decide se o campo entra na lista (com nota aqui) ou se o caso vai ao autor.
