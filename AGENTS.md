# AGENTS.md

## Migração para Nx

A migração foi validada localmente em filesystem Linux nativo e em contêiner. Os resultados e o histórico da retomada estão em [`docs/agents/SESSION_HANDOFF.md`](docs/agents/SESSION_HANDOFF.md). Não repita as tentativas de build no DrvFs registradas nesse histórico; use o fluxo em contêiner quando o checkout estiver em um filesystem compartilhado.

## Objetivo do repositório

Monorepo do `doyen` com:

- `packages/doyen`: crate Rust e binding Node.js mantido pelo NAPI-RS;
- `apps/nestjs-consumer`: aplicação NestJS usada para validar o tarball npm real;
- Nx para orquestração e Cargo workspace para o Rust.

## Ferramentas e workflows — regra obrigatória

**Sempre construa e evolua os workflows com ferramentas populares, consolidadas e open source. Não crie scripts caseiros para tarefas que o ecossistema já resolve.**

- Use preferencialmente Nx, npm, Cargo, Rustfmt, Clippy, NAPI-RS, Nest CLI, Jest, Supertest, Oxlint, Prettier, Podman/Docker e GitHub Actions.
- Use executores e geradores existentes antes de criar código de automação.
- Todos os fluxos devem funcionar pelo terminal e pela CI, sem depender de um agente de IA, serviço de IA ou Nx Cloud.
- Aliases em `package.json` e tarefas declarativas do Nx são permitidos; não crie shells ou programas próprios para build, pack, instalação ou testes.
- Se uma necessidade parecer exigir automação própria, procure primeiro uma ferramenta popular open source e justifique qualquer exceção.

## Regras do pacote nativo

- Não edite manualmente `packages/doyen/index.js` nem `packages/doyen/index.d.ts`; ambos são gerados pelo NAPI-RS.
- Preserve o crate em `packages/doyen` e o Cargo workspace na raiz.
- Não versione `*.node`, `*.tgz`, `dist/`, `target/`, journals ou arquivos temporários do NAPI-RS.
- O tarball precisa conter loader, declarações, licenças e o binário nativo da plataforma.

## Regra da integração NestJS

- O consumidor deve testar `dist/packages/doyen/doyen-0.1.0.tgz`, nunca um symlink para o código-fonte.
- Por essa razão, `apps/` permanece fora de `workspaces` no `package.json` da raiz, embora o Nx gerencie o app.
- Não mova `apps/nestjs-consumer` para o npm workspaces.
- Ao mudar a versão, sincronize `packages/doyen/package.json`, `packages/doyen/Cargo.toml`, a dependência no app e o caminho do tarball na tarefa `install-local-package`.
- O lockfile do consumidor é desabilitado propositalmente porque a integridade do tarball local muda a cada build; o lockfile da raiz deve ser versionado.

## Comandos principais

Execute da raiz:

```bash
npm ci
npm run test:nestjs
npm test
npm run test:rust
npm run lint
npm run lint:rust
npm run format:check
cargo fmt --all -- --check
```

Outros comandos:

```bash
npm run build
npm run pack
npm run start:nestjs
npm run test:container
npm run graph
```

## Mudanças e validação

- Faça alterações incrementais e preserve trabalho existente; não use reset ou checkout para descartar mudanças.
- Não inclua `apps` no workspace nem troque a dependência local por `doyen: "*"` para contornar falhas.
- Depois de mudar build ou empacotamento, execute a integração NestJS e confira o conteúdo do tarball.
- Consulte o `AGENTS.md` dentro de `apps/nestjs-consumer` ao trabalhar nesse app.
