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
| `CONTRIBUTING.md`, `GOVERNANCE.md` | How to contribute; who decides and who has access |
| `AI_POLICY.md` | How AI tools may be used here (people and agents) |
| `SECURITY.md` | How to report a vulnerability, and the threat model |

## Environment

- Rust, Node.js and Go run **inside the Distrobox container `doyen`**:
  `distrobox enter doyen -- cargo test`. See [`distrobox/README.md`](distrobox/README.md).
- Test databases run as Podman containers on the host (`distrobox-host-exec podman …`).
- The `openspec` CLI (1.15.0) is in the image.

## Commands

Run them inside the container (`distrobox enter doyen -- <command>`).

| What | Command |
|---|---|
| Format | `cargo fmt --all --check` |
| Lint | `cargo clippy --workspace --all-features --all-targets -- -D warnings` |
| Tests | `cargo test --workspace --all-features` |
| Specs | `openspec validate --all --strict` and `python3 scripts/check-specs.py` |
| Scenario tests (from `add-ci-baseline`) | `python3 scripts/check-scenario-tests.py` |

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
   Stage them (`git add`).
2. **Green:** write the code until the tests pass. The staged tests do not change: `git diff`
   on the test files stays empty.
3. **Refactor:** clean up with every test green, following *Code style*. No test changes in
   this step.
4. **Commit** the task group as one commit: its tests, its code, the spec changes and the
   ticks in `tasks.md` (see *Commits*). Every commit is green.

- To change a behaviour, change its test first, and the spec in the same commit.
- Never change a test only to make it pass. A test that looks wrong goes back to the spec:
  ask the owner.
- `python3 scripts/check-scenario-tests.py` (from `add-ci-baseline`) fails when a scenario has
  no test, or a test names a scenario that no longer exists. A scenario that can only be
  checked by hand (for example a CI failure path) goes in `openspec/manual-checks.md`.

## Code style

Owner rules (2026-11-05). Most readers know TypeScript, and every reader is a person.

**1. Familiar to TypeScript programmers.** Write Rust and Go with the parts a TypeScript
programmer already knows:

| TypeScript | Rust | Go |
|---|---|---|
| class with methods | `struct` + `impl` | `struct` + methods |
| `interface` | `trait` (like `Backend`) | `interface` |
| union type + `switch` | `enum` + `match` | typed constants + `switch` |
| `T \| null` + `if (x !== null)` | `Option<T>` + `if let Some(x)` | a pointer, or a second `ok` value |
| `throw` / `try` | `Result` + `?` | `if err != nil { return err }` |
| `async` / `await` | `async fn` / `.await` | a goroutine + a channel, only where the spec asks |
| `array.map` / `filter` / `find` | iterator `map` / `filter` / `find` | a `for` loop |

- Use a Rust- or Go-only feature only when the code needs it, with a comment that says why:
  lifetimes (a clone or an owned value often removes them), your own macros
  (`macro_rules!`), generics with many bounds, type-level tricks, `unsafe` (only in the
  C ABI).
- Use the names of the specs and the API, in full words: `renew_interval_ms`, `member`,
  `generation`; not `ri`, `m`, `g`.
- The Node.js binding is plain TypeScript for its users: camelCase, Promises, string unions,
  `Error` with a `code`.

**2. Easy to read: not too short, not too long.** A person reads the code once and
understands it.

- Too short (avoid): one-letter names, clever one-liners, long call chains, a closure inside
  a closure, `map_or_else` where an `if let` or a `match` reads better.
- Too long (avoid): a comment that repeats the code, a layer "for later" (a trait, generic,
  builder or module with one user), a function that only calls another one, checks for
  cases the types already rule out, the same block copied three times.
- One function does one thing and fits on one screen. Return early instead of nesting deep.
- Give each step a name: a well-named variable or small function beats a clever expression.
- Comments say *why* (the rule, the spec, the risk), not *what*. Each public item has a doc
  comment.
- Write the type when a reader would have to guess it (a function's return, a `let` from a
  long expression).
- Each source file starts with `// SPDX-License-Identifier: MIT OR Apache-2.0` (`#` in Python
  and shell, after the shebang line), the Linux kernel way.
- Tests follow these rules too. The refactor step of TDD and the review before the merge
  check them.

## Security

The threat model is in [`SECURITY.md`](SECURITY.md). These rules come from the OpenSSF
[guide for AI code assistant instructions](https://best.openssf.org/Security-Focused-Guide-for-AI-Code-Assistant-Instructions),
the parts that matter for doyen:

- **Secrets:** never put a password, token or key in code, tests, logs, errors or commit
  messages. Database URLs carry passwords: show a URL only with `***` in place of each
  secret (`client-config`, "Secrets are never shown"). Tests read their URLs from the
  `DOYEN_TEST_*` variables, never from a file in the repo.
- **Untrusted input:** a config and every database reply can be wrong. Check them and return
  an error or an offline status with a reason. Never panic and never `unwrap()` on them.
- **Safe defaults:** TLS (`rediss://`, MongoDB `tls=true`) must work. Never turn off the
  certificate check by default.
- **`unsafe`:** only in the C ABI and the napi-rs glue, each block with a `// SAFETY:`
  comment that says why it is sound. Go tests run with `-race`.
- **Negative tests:** each safety rule has a test that shows it fails safe: a lost place goes
  offline, a token never goes back, a secret never shows.
- **Dependencies:** add one only when the standard library and the current dependencies
  cannot do the job. Check that the crate or package is the real one (name, owner,
  repository, downloads): AI tools invent names. Add it with `cargo add` or `npm install`,
  commit the lock file, and say why in the commit message.
- **Downloads** in the image or in CI: check a checksum or a signature.
- **GitHub Actions:** pin each action to a full commit SHA, with the version in a comment.
  Give each workflow and job the least `permissions`. Pass `${{ … }}` values to `run:`
  scripts through `env:`. Secrets live only in GitHub secrets.
- **Review loop:** before you say a task is done, review your own change for problems (bugs,
  security, the specs, *Code style*), fix them, and run the checks again. Two rounds are
  usually enough.

## AI agents

Agents follow [`AI_POLICY.md`](AI_POLICY.md), like people. Also:

- **A person decides.** An agent never pushes, merges, publishes, creates a tag or a release, changes
  GitHub settings, or sends anything outside the repo (issues, comments, security reports,
  emails) without the owner's go. `.claude/settings.json` blocks `cargo publish`,
  `cargo yank`, `cargo owner`, `npm publish`, `npm unpublish` and `npm deprecate`, and asks
  before `git push`, `gh release` and `gh pr merge`.
- **Commits:** end the message with `Assisted-by: <tool> (<model>)`, for example
  `Assisted-by: Claude Code (claude-opus-5-5)`. Never add `Signed-off-by` or
  `Co-authored-by` for an agent: only a person can sign.
- **Say what you did not check:** in the final message and in the pull request, list what
  you could not build, test or reproduce.
- **Ask first** before a command that deletes data, writes outside the repo, or uses a
  database that is not a local test container.

## Choosing the model

Use the skill `model-tiering` (`.agents/skills/model-tiering/`, also in `.claude/skills/`):
the tests and the safety code at the strongest everyday model, the work that makes existing
tests pass at a cheaper one, and when to escalate.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/): `type(scope): subject`.

- Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`,
  `revert`.
- Scopes: `core`, `redis`, `mongodb`, `postgresql`, `node`, `go`, `ci`, `release`, `docs`,
  `deps` (Dependabot), or none.
- `!` after the type or scope for a breaking change, for example `feat(core)!: …`.
- Subject in English, imperative, lower case, no period: `feat(core): add the local lease`.
- **Code goes with its spec.** When code (tests included) belongs to an open change, the
  commit that adds or changes it also holds the spec changes it needs (`specs/`,
  `design.md`) and the ticks in that change's `tasks.md`. Never code in one commit and its
  spec in another. One task group = one commit. Code outside a change (a typo, a dependency
  update) has no spec. `scripts/check-commit-specs.py` (from `add-ci-baseline`) checks it on
  the change branches.
- Branch: `<type>/<change-name>`, for example `feat/add-core-election`.
- The commit type sets the next version (`release-process`): before 1.0, `!` or `feat` →
  minor, `fix` or `perf` → patch; other types make no release. Mark every breaking change
  with `!`: `cargo semver-checks` fails the release if you forget.
- AI help: the trailer `Assisted-by: <tool> (<model>)` (see *AI agents*).

## Rules

- **Branches and releases (GitHub Flow):** `main` is always green; short branches; pull
  request; rebase merge. One version for every package, one tag `v<version>`. release-please
  keeps the release pull request; **only the owner merges it and approves the environment
  `release`**, and then CI publishes with Trusted Publishing. An agent never runs
  `cargo publish` or `npm publish`, never creates or pushes a tag `v*`, and never merges the
  release pull request. If Trusted Publishing is down, the owner can publish by hand with
  2FA.
- Do not edit the generated OpenSpec files (`.claude/commands/opsx/`,
  `.claude/skills/openspec-*`, `.agents/skills/openspec-*`): `openspec update` writes them.
- The repo's own skills live in `.agents/skills/<name>/SKILL.md`; `.claude/skills/<name>` is a
  relative symlink to that folder.
- Keep the English simple: short sentences, common words.
