# Tasks

Test first: in each group with behaviour, the first task writes the Node.js tests
(`node --test`, each one names its scenario: `// spec: node-binding / <scenario>`) and shows
that they fail; the next tasks write the binding until they pass. Never change a test only to
make it pass.

## 0. Decision

- [ ] 0.1 (owner) Review this change (the napi-rs design in design.md and the `node-binding` spec: tick, process kept alive from join to leave, leave never rejects, error codes, platforms, Node.js versions, every adapter in each binary) and approve it or ask for changes

## 1. Binding crate and types

- [ ] 1.1 Replace the placeholder in `bindings/node` with the napi-rs v3 crate `doyen-node` (workspace member, `publish = false`, `tokio_rt`) and the `@napi-rs/cli` setup; verify `distrobox enter doyen -- npm run build` makes a `.node` file
- [ ] 1.2 Write the type test (`tsc --noEmit` on the README example, scenario "Types of the README example") and the runtime tests "Compare with the enum", "Offline status object" and "Token type"; verify they fail (red)
- [ ] 1.3 Write the types (`DatabaseConfig`, `Status` with `discriminant_case`, the string enums, `Token`); verify the tests pass (green)
- [ ] 1.4 Commit the group in one commit (tests, code, spec changes, ticks): `feat(node): add the napi-rs binding types`

## 2. Methods and callbacks

- [ ] 2.1 Write the tests against a local Redis: "Setting read back", "Missing url", "Negative number", "Busy event loop", "Callback throws", "SIGTERM with the database down", "Join after leave", "Redis with volatile eviction"; verify they fail (red)
- [ ] 2.2 Write `new Doyen()`, `status()`, `token()` on the core snapshot, and `join` with an `Unknown` argument converted inside; verify the config tests pass
- [ ] 2.3 Write `onStatusChange` with one weak thread-safe function per callback; verify the callback tests pass
- [ ] 2.4 Write `leave` (never rejects on a network error, second call at once, pending join → `DOYEN_LEFT`), the error codes and the warnings through `process.emitWarning`; verify the remaining tests pass (green)
- [ ] 2.5 Commit the group in one commit (tests, code, spec changes, ticks): `feat(node): add the Doyen class`

## 3. Event loop check and process life

- [ ] 3.1 Write the tests in child processes: "Endless loop" (20 s busy loop), "Short block" (5 s), "Script ends after leave", "Object not kept by the app", and a worker that ends; verify they fail (red)
- [ ] 3.2 Write the tick with the strong thread-safe function and `__binding::set_liveness`; verify the event loop tests pass
- [ ] 3.3 Write the process life rules, the GC reference and the worker cleanup hook; verify the remaining tests pass (green)
- [ ] 3.4 Commit the group in one commit (tests, code, spec changes, ticks): `feat(node): stop the renews when the event loop is blocked`

## 4. CI and release

- [ ] 4.1 Add the CI job `node` (matrix: the three platforms × the Node.js LTS lines; Redis and MongoDB services on Linux x64); verify it runs the type test and every runtime test, and passes
- [ ] 4.2 Write the platform test "Unsupported platform" (import on Linux musl in a container fails with the platform name); verify it passes with the napi-rs loader, and add "Install on Linux x64" to `openspec/manual-checks.md` after a clean install
- [ ] 4.3 Add the job `npm` to `release.yml` (build all platforms, the license files in each package, a CycloneDX SBOM per binary uploaded to the GitHub release, `napi prepublish`, `npm publish --provenance` per package, environment `release`, `id-token: write`) and a `workflow_dispatch` dry run (`npm publish --dry-run`); verify the dry run builds and packs every package with its license files ("Package contents"), and zizmor passes
- [ ] 4.4 Write the Node.js README (`bindings/node/README.md`: install, quick start, status, token, no fixed order between the `join` Promise and the first callback, `SIGTERM`, warnings, platforms) and link it from the main README; verify the quick start runs against a local Redis
- [ ] 4.5 (owner) Add the new `ci.yml` jobs of this group to the required checks of the ruleset of `main` (a job that is not required can be red on a merged pull request)
- [ ] 4.6 Commit as `ci(node): build and publish the npm packages`
- [ ] 4.7 Run `python3 scripts/check-scenario-tests.py --change add-node-binding`; verify every `node-binding` scenario is named by a test or a manual check
- [ ] 4.8 (owner) Add the npm Trusted Publisher (repository `leofdss/doyen`, workflow `release.yml`, environment `release`) to `@leofdss/doyen` and to each platform package (design.md: a placeholder first if npm needs it); verify the npm settings page of each package shows it
- [ ] 4.9 (owner) After the merge of this change, merge the release pull request (0.2.0) and approve the environment `release`; verify `npm install @leofdss/doyen` works in a clean folder on Linux x64 and npm shows the provenance, and add "Release 0.2.0" to `openspec/manual-checks.md`
- [ ] 4.10 (owner) Deprecate the npm placeholders: `npm deprecate @leofdss/doyen@"<0.1.0" "placeholder: use 0.2.0 or newer"`; verify `npm view @leofdss/doyen@0.0.2` shows the note

## Workflow follow-up

- Archive the change with `/opsx:archive add-node-binding` after the release.
