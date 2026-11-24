# doyen

Monorepo do `doyen`, com o núcleo em Rust, o binding Node.js mantido pelo NAPI-RS e um
consumidor NestJS para validar o pacote npm real.

> O projeto ainda está em planejamento. As funções atuais são exemplos usados para validar o
> binding e a distribuição; elas não representam uma versão pronta para produção.

## Estrutura

```text
.
├── packages/doyen/          Crate Rust, binding NAPI-RS e pacote npm
├── apps/nestjs-consumer/    Consumidor NestJS do tarball npm
├── Cargo.toml               Cargo workspace
├── nx.json                  Orquestração e cache local do Nx
└── Containerfile.test       Validação reproduzível em contêiner
```

O npm workspaces contém somente `packages/*`. O app em `apps/` é gerenciado pelo Nx, mas fica
fora do workspaces para que sua dependência seja sempre o tarball físico, nunca um symlink para
o código-fonte.

## Pré-requisitos

- Node.js 22.13 ou superior; Node.js 24 é usado na CI e no contêiner.
- npm compatível com a versão do Node.js instalada.
- Rust 1.88 ou superior, com Cargo, Clippy e Rustfmt.
- Um compilador C compatível com a plataforma.
- Podman, somente para a validação opcional em contêiner.

Com rustup, os componentes de qualidade podem ser instalados com:

```bash
rustup component add clippy rustfmt
```

Instale as dependências JavaScript da raiz:

```bash
npm ci
```

### WSL e filesystems compartilhados

Mantenha o checkout no filesystem nativo do Linux, por exemplo abaixo de `$HOME`. Não use
`/mnt/*`: o NAPI-RS pode falhar na substituição atômica dos artefatos em DrvFs e em outros
compartilhamentos do Windows. Se o checkout não puder ser movido, use `npm run test:container`
para executar a validação em um filesystem nativo de contêiner.

## Comandos

Execute todos os comandos a partir da raiz do repositório.

| Comando                      | Finalidade                                                     |
| ---------------------------- | -------------------------------------------------------------- |
| `npm run build`              | Compila os projetos e suas dependências em modo release.       |
| `npm run build:debug`        | Compila somente o binding nativo em modo debug.                |
| `npm test`                   | Executa os testes unitários do binding Node.js e do NestJS.    |
| `npm run test:rust`          | Executa os testes do crate Rust.                               |
| `npm run test:nestjs`        | Gera o tarball e executa a integração NestJS de ponta a ponta. |
| `npm run lint`               | Executa o Oxlint no consumidor NestJS.                         |
| `npm run lint:rust`          | Executa o Clippy com warnings tratados como erros.             |
| `npm run format:check`       | Verifica a formatação com Prettier.                            |
| `npm run format`             | Formata os arquivos com Prettier.                              |
| `npm run format:rust`        | Formata o código Rust com Rustfmt.                             |
| `cargo fmt --all -- --check` | Verifica a formatação de todo o Cargo workspace.               |
| `npm run pack`               | Compila e empacota o pacote npm.                               |
| `npm run start:nestjs`       | Prepara o pacote local e inicia o consumidor NestJS.           |
| `npm run test:container`     | Executa toda a validação em uma imagem Podman.                 |
| `npm run graph`              | Abre a visualização do grafo de projetos do Nx.                |

## Fluxo do tarball npm

`npm run test:nestjs` executa o seguinte grafo:

```text
doyen:build
  -> doyen:pack
    -> nestjs-consumer:install-local-package
      -> nestjs-consumer:build
        -> nestjs-consumer:e2e
```

O NAPI-RS gera o loader CommonJS, as declarações TypeScript e o binário nativo da plataforma.
Em seguida, `npm pack` cria:

```text
dist/packages/doyen/doyen-0.1.0.tgz
```

O consumidor instala esse arquivo em seu próprio `node_modules`. Os testes e2e verificam o
caminho real do módulo e chamam as funções nativas pelas rotas HTTP, garantindo que o teste não
use o código-fonte por meio de um symlink do workspace.

Para conferir o conteúdo do pacote:

```bash
tar -tf dist/packages/doyen/doyen-0.1.0.tgz
```

O tarball deve conter `index.js`, `index.d.ts`, as licenças, o README e o binário `.node` da
plataforma atual. Não edite manualmente `index.js` nem `index.d.ts`; ambos são gerados pelo
NAPI-RS.

## Validação manual completa

Todo o fluxo funciona pelo terminal, sem agente de IA, serviço de IA, Nx Cloud ou conta externa:

```bash
npm ci
npm run test:nestjs
npm test
npm run test:rust
npm run lint
npm run lint:rust
cargo fmt --all -- --check
npm run format:check
```

Como alternativa reproduzível em Node.js 24 e Rust 1.88:

```bash
npm run test:container
```

A mesma sequência principal está configurada em `.github/workflows/ci.yml`. Consulte também o
[README do pacote](packages/doyen/README.md) e o
[README do consumidor](apps/nestjs-consumer/README.md).

## Licença

Distribuído sob licença dupla: [Apache License 2.0](LICENSE-APACHE) ou
[MIT](LICENSE-MIT), a critério do usuário.
