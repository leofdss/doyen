# AGENTS.md — nestjs-consumer

Siga também o `AGENTS.md` da raiz.

## Finalidade

Este app é um consumidor de integração do **tarball npm** do `doyen`, não um segundo workspace npm da biblioteca.

## Regras

- Preserve `apps/` fora de `workspaces` no `package.json` da raiz.
- Não substitua a dependência `file:../../dist/packages/doyen/doyen-0.1.0.tgz` por link para `packages/doyen`.
- Execute a instalação e os testes por `npm run test:nestjs` na raiz; o Nx cuida de build, pack e instalação.
- Ao alterar a versão do pacote, atualize o nome do `.tgz` na dependência e na tarefa `install-local-package`.
- Não versione `node_modules`, o tarball nem o lockfile deste consumidor. O lockfile é desabilitado no `.npmrc` porque o tarball local muda a cada build.
- O app usa CommonJS com Jest porque o loader NAPI-RS é CommonJS. O NestJS 12 é carregado via `require(esm)`, suportado pelas versões de Node.js declaradas na raiz.
- Prefira pacotes oficiais do NestJS e ferramentas populares open source antes de adicionar abstrações ou código de automação.
