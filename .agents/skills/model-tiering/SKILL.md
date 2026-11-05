---
name: model-tiering
description: Choose the model and the effort level for each kind of work in doyen (OpenSpec proposals, writing tests, implementing tasks, reviews, CI logs, docs), and when to escalate to a stronger model. Use before you delegate work to a subagent, when you start an OpenSpec change (/opsx:propose, /opsx:apply), when a task fails twice, or when the user asks which model to use or how to spend less.
metadata:
  updated: "2026-11-04"
---

# Model tiering

Use the cheapest model **that keeps the quality**. In doyen the quality gate is the tests:
they are the source of truth (TDD, see `AGENTS.md`). A cheaper model can implement a task
safely only when failing tests already say what "done" means. So the expensive thinking goes
into the specs and the tests; the cheaper models make the tests pass.

## The tiers

Prices are per million tokens (input / output), Anthropic API, as of 2026-10. Check the
current prices before you quote them to the owner.

| Tier | Alias (Claude Code) | Model ID | Price | Effort | Use it for |
|---|---|---|---|---|---|
| T0 · mechanical | `haiku` | `claude-haiku-5-5` | $0.10 / $0.50 | `low` | Read-only search and exploration, read a CI log and say which step failed, run commands and sum up the output, rename, format, update a row in `TODO.md`, tick tasks |
| T1 · standard | `sonnet` | `claude-sonnet-5-5` | $2 / $10 | `medium` (`high` for hard tasks) | Make failing tests pass when the tests already exist, docs and READMEs, CI workflow files, small fixes with a clear cause, the Node.js and Go wrappers once their tests exist |
| T2 · judgment | `opus` | `claude-opus-5-5` | $4 / $20 | `high` (`xhigh` for long tasks) | OpenSpec proposals and `design.md`, **writing the tests from the spec scenarios**, the safety code (lease, renew schedule, membership tag, generation, join/leave races), code review, a failure you cannot explain |
| T3 · hardest | `fable` | `claude-fable-5-1` | $10 / $50 | `high` | Only after T2 failed twice on the same problem, or when the owner asks: a release review of the safety rules (two masters, a token that goes back), a hard distributed bug |

Session default in this repo: T2 (`opus`). Move down for clear, test-guarded work; move up
only by the escalation rules.

## Work in doyen → tier

| Work | Tier |
|---|---|
| `/opsx:explore`, `/opsx:propose`, editing a `design.md` | T2 |
| Test-first step of a task group: write the tests from the scenarios, see them fail | T2 for `election-core`, `adapter-contract`, `status-reporting` and the adapters' generation and join rules; T1 for config, types, docs examples, CI |
| Implementation step: make the failing (staged) tests pass, without changing them | T1; T2 for the engine (`crates/doyen/src/engine/`) and each adapter's join / renew / generation code |
| Conformance suite and the in-memory adapter | T2 (they define what every adapter must do) |
| Bindings (`bindings/node`, `bindings/go`, `crates/doyen-ffi`) | T2 for the C ABI and the thread rules; T1 for the rest, after the tests exist |
| CI red: find the failing step | T0, then the tier of the failing code to fix it |
| Review of a change before the merge | T2 in a fresh-context subagent; T3 before a release |
| `/opsx:archive`, `TODO.md` rows, changelog | T0 |
| `scripts/check-specs.py` errors | T1 |

## Escalation rules

1. **Two failed attempts → one tier up.** The same failing test after two honest attempts
   means the tier is too low, not that the test is wrong.
2. **Never change a test to make it pass.** The tests are the contract. If a lower tier
   thinks a test is wrong, it stops and hands the question to T2 (or the owner), with the
   spec scenario the test comes from.
3. **Safety code is never below T2**: anything that decides who is master, when the lease
   ends, what the token is, or how a number or a generation is given.
4. **A review is never below T2.** Use a fresh-context subagent for it (a clean context
   found issues that the author's context missed in this project).
5. **Unsure → go up.** If the model says it is not sure about a concurrency or failover
   detail, escalate instead of guessing.

## Effort before model

Lower the effort on the same model before you switch to a cheaper model. On the current
models, a stronger model at a lower effort often matches a weaker model at a high effort, and
the prompt cache belongs to one model: switching models in the main session loses the cache.
So:

- Stay on one model in the main session. Change `/effort` for easy stretches.
- Use another tier through a **subagent**, not by switching the main session's model.

## How to apply it in Claude Code

- **Main session:** the owner picks the model with `/model <alias>` and the effort with
  `/effort <level>`. Suggest a change; do not assume it.
- **One subagent call:** the Agent tool takes `model` as an alias only (`haiku`, `sonnet`,
  `opus`, `fable`) and an `effort` parameter (Claude Code 2.1.292 or later). The per-call
  value wins over the subagent file; then the file's `model:`; then
  `CLAUDE_CODE_SUBAGENT_MODEL`; then the main session's model.
- **A subagent file** (`.claude/agents/<name>.md`) can fix its tier with `model:` and
  `effort:` in the frontmatter (`model:` also takes a full ID or `inherit`).
- **A skill** can set `model:` and `effort:` in its frontmatter; they last for the rest of
  that turn only.
- **When not to delegate:** a subagent starts cold and reads the context again. For a small
  edit, the extra reading costs more than the cheaper model saves. Delegate read-heavy work
  (exploration, logs, many files), work with a clear test-guarded goal, and fresh-context
  reviews. Spawn subagents only when the owner asks or the workflow says so.

## Tell the owner

When you choose a tier for a task group, say it in one line (for example "tests at T2,
implementation at T1"). When you escalate, say why (which test, how many attempts).

## Other agents

The tiers are classes, not brand names. An agent without Claude models maps them to its own
models: T0 = the smallest fast model, T1 = the everyday coding model, T2 = the strongest
general model, T3 = the strongest reasoning model at a high cost.
