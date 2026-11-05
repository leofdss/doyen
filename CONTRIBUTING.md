# Contributing to doyen

## Before you start

- Read [AGENTS.md](AGENTS.md): how work is done here, for people and for AI agents.
- AI tools are welcome under the [AI policy](AI_POLICY.md).
- **Questions and ideas:** open an issue with the form *Idea or question*.
- **Bugs:** open an issue with the form *Bug report*. Never put a password or a token in it.
- **Security problems:** never in a public issue. Follow [SECURITY.md](SECURITY.md).
- Who decides and who has access: [GOVERNANCE.md](GOVERNANCE.md).

## How a change is made

1. **Spec first.** New or changed behaviour starts as an OpenSpec change in
   `openspec/changes/` (`/opsx:propose` in Claude Code, or by hand). The maintainer approves
   it before the code. Small fixes (a typo, a broken link, a dependency update) need no
   change.
2. **Branch:** `<type>/<change-name>`, for example `feat/add-core-election`.
3. **Tests first (TDD).** Write the tests for the scenarios, see them fail, then write the
   code until they pass. Each test names its scenario: `// spec: <capability> / <scenario>`.
   Never change a test only to make it pass.
4. **One task group, one commit:** its tests, its code, the spec changes and the ticks in
   `tasks.md`. Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/)
   (`feat(core): add the local lease`); the type sets the next version.
5. **Pull request** with the template. The CI must be green. The maintainer reviews it and
   merges it with a rebase merge.

### Commit messages

The CI checks the subject of every commit with `scripts/check-commits.sh`. Run the same
check before you push: `scripts/check-commits.sh origin/main HEAD`. The form is
`type(scope)!: subject`; the types are `feat`, `fix`, `docs`, `style`, `refactor`, `perf`,
`test`, `build`, `ci`, `chore` and `revert`, and the scope and `!` are optional. Two good
subjects:

- `feat(core): add the local lease`
- `fix(redis): renew on time`

A subject such as `Added lease` fails. Merge commits made by Git are skipped.

## Build and test

The commands are in [AGENTS.md](AGENTS.md) (*Commands*). The development container is
described in [distrobox/README.md](distrobox/README.md). Tests that need a database run only
when its URL is set (`DOYEN_TEST_REDIS_URL`, `DOYEN_TEST_MONGODB_URL`); without it they are
skipped.

## Dependencies: how we choose and track them

- Add a dependency only when the standard library and the current dependencies cannot do
  the job. Check that it is the real package (name, owner, repository, downloads, recent
  releases): AI tools invent names. Say why in the commit message.
- Add it with the official tool (`cargo add`, `npm install`) and commit the lock file.
- Dependabot proposes updates every week, 7 days after each release.
- `cargo deny` runs in CI: a known vulnerability (RustSec), a license that is not allowed,
  or a source other than crates.io blocks the merge and the release. An exception needs an
  `ignore` entry in `deny.toml` with the reason and a link, approved by the maintainer.

## Code quality

- `clippy` warnings are errors. `zizmor` findings in the workflows block the merge.
- The code style rules are in [AGENTS.md](AGENTS.md) (*Code style*): code that TypeScript
  programmers can follow, easy to read, neither too short nor too long.
- Each source file starts with `SPDX-License-Identifier: MIT OR Apache-2.0` in a comment, the
  Linux kernel way.

## License

doyen is licensed under MIT OR Apache-2.0. Unless you say otherwise, your contribution is
licensed the same way (see the *License* section of [README.md](README.md)).
