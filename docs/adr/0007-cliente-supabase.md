# 0007 · Cliente do Supabase: pedido montado sem IO, envio por alvo

Data: 26/11/2026. Estado: aceita.

## Contexto
O banco continua o do site atual (R2): PostgREST, GoTrue, Storage e Realtime do mesmo projeto. A segurança mora no banco (RLS, views que mascaram colunas, funções `security definer`). O site em Rust é estático, com ilhas em WebAssembly (ADR 0003), e o app Tauri roda o mesmo WebAssembly no WebView. `centelha-supabase` não depende do `centelha-engine` e nunca leva chave de serviço (CONTRACTS).

## Decisão
1. **Pedido como dado.** O crate monta cada pedido como valor puro: método, caminho, consulta, os cabeçalhos `apikey` e `Authorization: Bearer`, `Accept` e `Prefer`, e o corpo JSON. A resposta chega igual (situação, cabeçalhos, corpo) e o crate a interpreta: linhas, contagem pelo `Content-Range` e o erro do PostgREST (`code`, `message`, `details`, `hint`). Montar e interpretar têm teste sem rede, nos dois alvos.
2. **Envio por alvo, atrás de um trait** `Transport`: no `wasm32`, `gloo-net` (o `fetch` do navegador); no nativo, `reqwest` sem as features padrão, com `rustls` (para conferência manual e testes de integração fora do CI). Nos testes, um transporte que devolve respostas gravadas.
3. **Configuração de fora.** A URL e a chave `anon` chegam num `Config` montado por quem usa o crate (no site, das variáveis de build, como o site atual faz com `PUBLIC_SUPABASE_URL` e `PUBLIC_SUPABASE_ANON_KEY`). Nenhuma das duas fica no código, em teste ou em fixture.
4. **Dado de mesa só com sessão.** Toda leitura de tabela, view ou função pede um `Session` (o token de acesso do usuário) no tipo. A única leitura sem sessão é a de `migracoes`, a única tabela que o banco abre ao `anon`, num método próprio. O cliente não conta com o `anon` para nenhum dado de mesa.
5. **Admin (R22).** `is_admin` chama só a `eh_admin` com a sessão. Qualquer falha (rede, erro do banco, sessão ausente ou vencida, corpo que não é `true`) dá `false`. A resposta não fica guardada.
6. **Sessão do site atual (R26).** O crate não lê, não grava e não interpreta a sessão do `supabase-js` no `localStorage`: recebe o token pronto. Quando a sessão dividida entrar (GoTrue e a renovação), ela entra com a linha da tabela do R26 no mesmo PR.
7. **Fases.** A primeira entrega só lê: `select` em tabelas e views e as funções `stable` que o site chama. Gravação, GoTrue, Storage e tempo real (o WebSocket próprio do estudo) vêm depois, cada um no seu PR, sem mudar 1 e 2.

## Consequências
- As versões entram no `Cargo.toml` do crate, por `cfg(target_arch)`, e são conferidas no crates.io no dia.
- O crate ainda não entra no `centelha-web`, então o tamanho do WebAssembly não muda nesta entrega. A primeira ilha que o usar mede o acréscimo pelo método da ADR 0003.
- Os testes do CI não falam com o banco.

## Notas

- **26/11/2026, nomes.** No crate, a mesa do jogo é `Table` (o termo do glossário) e a tabela ou view do PostgREST é `Relation`. As fixtures de leitura com sessão são feitas à mão a partir das colunas das migrações, com uuids e nomes inventados; só o que o `anon` responde é gravado de verdade, com a chave lida na hora e nunca guardada.
