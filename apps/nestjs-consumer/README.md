# nestjs-consumer

Aplicação NestJS usada para validar o **tarball npm real** do pacote `doyen`. Ela não faz parte
do npm workspaces e não consome o binding por symlink.

O serviço importa `raiz()` e `saudacao()` do pacote instalado e expõe duas rotas:

| Rota                  | Resposta       |
| --------------------- | -------------- |
| `GET /`               | `Olá, mundo!`  |
| `GET /saudacao/:nome` | `Olá, <nome>!` |

## Preparar e testar

Execute os comandos pela raiz do monorepo, não por este diretório:

```bash
npm ci
npm run test:nestjs
```

O Nx executa o build release do binding, gera `dist/packages/doyen/doyen-0.1.0.tgz`, instala o
arquivo no `node_modules` deste app, compila o TypeScript e roda os testes e2e. Não é necessário
executar um `npm install` separado neste diretório.

Para rodar também os testes unitários do app e do binding:

```bash
npm test
```

Para iniciar o servidor depois de instalar as dependências da raiz:

```bash
npm run start:nestjs
```

## O que a integração verifica

- O pacote é resolvido a partir de `apps/nestjs-consumer/node_modules`.
- O caminho real do módulo não aponta para `packages/doyen`.
- As duas rotas executam as funções nativas e preservam as respostas esperadas.
- O app CommonJS compila com Nest CLI e é testado com Jest e Supertest.

Os testes usam `node --experimental-vm-modules` porque o NestJS 12 é carregado via `require(esm)`
nas versões de Node.js suportadas pelo repositório. O loader gerado pelo NAPI-RS permanece
CommonJS.

## Dependência local e lockfile

A dependência usa deliberadamente:

```json
"doyen": "file:../../dist/packages/doyen/doyen-0.1.0.tgz"
```

Não a substitua por `doyen: "*"`, por um caminho para `packages/doyen` nem por um workspace
npm. O lockfile deste consumidor é desabilitado em `.npmrc`, pois a integridade do tarball local
muda a cada build. O lockfile da raiz continua versionado.

Ao alterar a versão do pacote, sincronize o nome do tarball nesta dependência e na tarefa
`install-local-package` de `project.json`, além de `packages/doyen/package.json` e
`packages/doyen/Cargo.toml`.
