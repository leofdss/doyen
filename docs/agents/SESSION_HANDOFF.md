# Handoff — migração para monorepo Nx

Data da conclusão local: 2026-11-24

## Estado atual

A migração foi retomada em `/home/dev/private/doyen`, sobre ext4, e validada localmente e em
contêiner. Não há mais bloqueio conhecido no build, no empacotamento ou na integração NestJS.
A working tree continua contendo a migração ainda não commitada: não use reset, clean ou checkout
destrutivo.

A única validação configurada que ainda não foi observada é a execução do workflow no GitHub
Actions, pois isso depende de push ou pull request no repositório remoto.

## Validações concluídas

Os seguintes comandos passaram na raiz:

```bash
npm run test:nestjs
npm test
npm run test:rust
npm run lint
npm run lint:rust
cargo fmt --all -- --check
npm run format:check
npm run test:container
```

Resultados relevantes:

- O NAPI-RS compilou o binding release corretamente fora do DrvFs.
- Os testes unitários do binding Node.js e do consumidor NestJS passaram.
- Os dois testes unitários do crate Rust passaram.
- Os três testes e2e do consumidor passaram, incluindo a verificação de que o módulo veio do
  `node_modules` do app e não de um symlink do workspace.
- Clippy, Rustfmt, Oxlint e Prettier passaram.
- O fluxo completo também passou no Podman com Node.js 24 e Rust 1.88.

O conteúdo de `dist/packages/doyen/doyen-0.1.0.tgz` foi conferido com `tar -tf` e contém:

```text
package/LICENSE-APACHE
package/LICENSE-MIT
package/index.js
package/package.json
package/README.md
package/doyen.linux-x64-gnu.node
package/index.d.ts
```

## Correções feitas na retomada

- `doyen:pack` agora cria `dist/packages/doyen` antes de chamar `npm pack`; o npm não cria esse
  diretório automaticamente em um checkout limpo.
- `Containerfile.test` instala os componentes oficiais `clippy` e `rustfmt` com rustup, ausentes
  da imagem Rust mínima usada no build.
- Rust 1.88.0, Clippy e Rustfmt foram instalados no ambiente local com rustup. Essa instalação é
  um pré-requisito do ambiente, não uma alteração do repositório.

## Documentação concluída

- A raiz ganhou um `README.md` com arquitetura, pré-requisitos, comandos, fluxo do tarball, WSL,
  contêiner e validação manual sem agente.
- O README genérico do NestJS foi substituído por instruções específicas do consumidor.
- A seção de desenvolvimento de `packages/doyen/README.md` agora usa os comandos da raiz do
  monorepo.
- `Containerfile.test` e `.github/workflows/ci.yml` foram comparados com os resultados reais. A CI
  já instala Node.js 24, Rust 1.88, Clippy e Rustfmt; nenhuma alteração adicional foi necessária.

## Observações

- Durante `npm ci` e a instalação do consumidor, o npm relatou vulnerabilidades moderadas nas
  árvores de dependências. Nenhum `npm audit fix --force` foi aplicado, para não introduzir
  atualizações breaking fora do escopo da migração.
- O npm também exibiu aviso sobre o script de instalação do Nx não aprovado pelo novo mecanismo
  `allowScripts`; o build e todos os testes passaram mesmo assim. Reavalie esse aviso ao atualizar
  o npm ou o Nx, sem criar automação própria.
- As recomendações de Nx Cloud exibidas pelo CLI são opcionais e não foram adotadas.

## Histórico do problema de filesystem

Na raiz anterior, `/mnt/d/Private/doyen` (DrvFs), o Cargo compilava, mas o NAPI-RS falhava na
transação de filesystem com erros como `Failed to restore unexpected transaction entry` e
`Filesystem transaction failed and rollback was incomplete`.

Foram testadas sem sucesso as versões 3.11.1, 3.10.12 e 3.9.1 do `@napi-rs/cli`. Não repita esses
downgrades e não substitua o NAPI-RS por comandos próprios de cópia. O projeto permanece em
`@napi-rs/cli` `^3.11.1`; a compilação passou sem alteração de versão depois da mudança para ext4.

## Decisões permanentes

As regras que devem orientar as próximas alterações estão no `AGENTS.md` da raiz e, para o app, em
`apps/nestjs-consumer/AGENTS.md`. Em particular, preserve Nx + npm + Cargo sem Nx Cloud, NAPI-RS
como ferramenta do binding, o teste do tarball físico e `apps/` fora do npm workspaces.
