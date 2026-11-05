# Tasks

## 0. Decision

- [ ] 0.1 (owner) Review this change (it is the napi-rs proposal of the design notes: tick, process kept alive from join to leave, leave never rejects, error codes, platforms, Node.js versions) and approve it or ask for changes

## 1. Binding crate

- [ ] 1.1 Replace the placeholder in `bindings/node` with the napi-rs v3 crate `doyen-node` (workspace member, `publish = false`, `tokio_rt`) and the `@napi-rs/cli` setup; verify `distrobox enter doyen -- npm run build` makes a `.node` file
- [ ] 1.2 Write the types (`DatabaseConfig`, `Status` with `discriminant_case`, the string enums, `Token`); verify the generated `index.d.ts` matches the public API with `tsc --noEmit` on the README example
- [ ] 1.3 Commit as `build(node): add the napi-rs binding crate`

## 2. Methods and callbacks

- [ ] 2.1 Write `new Doyen()`, `status()`, `token()` on the core snapshot, and `join` with an `Unknown` argument converted inside; verify the "Runtime values match the enums" and "Config from JavaScript" scenarios with a Node.js test (`node --test`) against a local Redis
- [ ] 2.2 Write `onStatusChange` with one weak thread-safe function per callback; verify the "Status changes through callbacks" and "A throwing callback" scenarios
- [ ] 2.3 Write `leave` (never rejects on a network error, second call at once, pending join → `DOYEN_LEFT`) and the error codes; verify the "Leave never rejects…" and "Error codes in JavaScript" scenarios
- [ ] 2.4 Write the warnings through `process.emitWarning`; verify the "Warnings through process.emitWarning" scenario with a Redis `volatile-lru` container
- [ ] 2.5 Commit as `feat(node): add the Doyen class`

## 3. Event loop check and process life

- [ ] 3.1 Write the tick with the strong thread-safe function and `__binding::set_liveness`; verify the "A blocked event loop stops the renews" scenarios (a 20 s and a 5 s busy loop in a child process)
- [ ] 3.2 Write the process life rules, the GC reference and the worker cleanup hook; verify the "Process life" and "Worker threads and garbage collection" scenarios (a script that exits after `leave`, a worker that ends)
- [ ] 3.3 Check the token type; verify the "Token numbers" scenario
- [ ] 3.4 Commit as `feat(node): stop the renews when the event loop is blocked`

## 4. CI and release

- [ ] 4.1 Add the CI job `node` (matrix: the three platforms × the Node.js LTS lines; Redis and MongoDB services on Linux x64); verify the job passes and runs the type test and the runtime tests
- [ ] 4.2 Add the workflow for tags `node-v*` (build all platforms, pack, upload artifacts); verify it on a throwaway tag, then delete the tag
- [ ] 4.3 Write the Node.js README (`bindings/node/README.md`: install, quick start, status, token, `SIGTERM`, warnings, platforms) and link it from the main README; verify the quick start runs against a local Redis
- [ ] 4.4 Commit as `ci(node): build and pack the npm packages`
- [ ] 4.5 Set the version 0.2.0, update `CHANGELOG.md`, tag `node-v0.2.0`; verify the workflow passes
- [ ] 4.6 (owner) Publish `@leofdss/doyen` 0.2.0 and the platform packages (`npm publish`, 2FA); verify `npm install @leofdss/doyen` works in a clean folder on Linux x64

## Workflow follow-up

- Archive the change with `/opsx:archive add-node-binding` after the release.
