# AGENTS.md

How to work in this repo, for AI agents and for people.

## What this is

**doyen** picks one master among the instances of a service, through a database the team
already runs (Redis and MongoDB first, PostgreSQL later). One core in Rust; Node.js and Go
use it through bindings. Status: planning. The plan is a set of OpenSpec changes; their
order is in [`TODO.md`](TODO.md).

## Layout

| Path | What |
|---|---|
| `crates/doyen` | The Rust crate: core and adapters (cargo features) |
| `bindings/node` | The npm package `@leofdss/doyen` (napi-rs, v0.2) |
| `bindings/go` | The Go code (v0.3), published from `github.com/leofdss/doyen-go` |
| `openspec/` | Specs (`specs/`), open changes (`changes/`), project rules (`config.yaml`) |
| `distrobox/` | The development image and container |
| `scripts/` | Repo checks (`check-specs.py`) |

## Environment

- Rust, Node.js and Go run **inside the Distrobox container `doyen`**:
  `distrobox enter doyen -- cargo test`. See [`distrobox/README.md`](distrobox/README.md).
- Test databases run as Podman containers on the host (`distrobox-host-exec podman …`).
- The `openspec` CLI (1.15.0) is in the image.

## Work with OpenSpec

1. Take the next change in [`TODO.md`](TODO.md). Read its `proposal.md`, `design.md`, the
   spec files and `tasks.md` in `openspec/changes/<name>/`.
2. Apply it: `/opsx:apply <name>` in Claude Code, or the skill `openspec-apply-change` in
   `.agents/skills/`. Tick each task in `tasks.md` when it is done and verified.
3. After the merge: `/opsx:archive <name>` (skill `openspec-archive-change`); the specs move to
   `openspec/specs/`. Update the row in `TODO.md`.
4. A new idea: `/opsx:propose <idea>` (skill `openspec-propose`), then add it to `TODO.md`.
   To think first: `/opsx:explore`.

Before a commit that changes `openspec/` or `TODO.md`:

```bash
openspec validate --all --strict
python3 scripts/check-specs.py
```

The rules for proposals (the owner's decisions) are in `openspec/config.yaml`. Do not
change them in a proposal; ask the owner.

## Tests first (TDD)

The tests are the source of truth. The specs say what doyen must do; the tests check it; the
code only makes the tests pass.

1. **Red:** write the tests for the scenarios of the task group, and see them fail. Each test
   names its scenario in a comment: `// spec: election-core / Master dies` (`#` in Python).
2. **Green:** write the code until the tests pass.
3. **Refactor:** clean up with every test green. No test changes in this step.

- Commit the red tests first (`test(core): add the tests for the local lease`), then the code
  (`feat(core): add the local lease`). `main` stays green at every merge.
- To change a behaviour, change its test first, and the spec in the same change.
- Never change a test only to make it pass. A test that looks wrong goes back to the spec:
  ask the owner.
- `python3 scripts/check-scenario-tests.py` (from `add-ci-baseline`) fails when a scenario has
  no test, or a test names a scenario that no longer exists. A scenario that can only be
  checked by hand (for example a CI failure path) goes in `openspec/manual-checks.md`.

## Choosing the model

Use the skill `model-tiering` (`.agents/skills/model-tiering/`, also in `.claude/skills/`):
the tests and the safety code at the strongest everyday model, the work that makes existing
tests pass at a cheaper one, and when to escalate.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/): `type(scope): subject`.

- Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`,
  `revert`.
- Scopes: `core`, `redis`, `mongodb`, `postgresql`, `node`, `go`, `ci`, `release`, `docs`
  (or none).
- `!` after the type or scope for a breaking change, for example `feat(core)!: …`.
- Subject in English, imperative, lower case, no period: `feat(core): add the local lease`.
- Branch: `<type>/<change-name>`, for example `feat/add-core-election`.

## Rules

- **The owner publishes.** Never run `cargo publish` or `npm publish`, and never push a
  release tag (`v*`, `node-v*`, `go-v*`) without the owner's go.
- Do not edit the generated OpenSpec files (`.claude/commands/opsx/`,
  `.claude/skills/openspec-*`, `.agents/skills/openspec-*`): `openspec update` writes them.
- The repo's own skills live in `.agents/skills/<name>/SKILL.md`; `.claude/skills/<name>` is a
  relative symlink to that folder.
- Keep the English simple: short sentences, common words.
