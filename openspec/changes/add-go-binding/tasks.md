# Tasks

Test first: in each group with behaviour, the first task writes the tests (C test program and
`go test -race`; each test names its scenario: `// spec: go-binding / <scenario>`) and shows
that they fail; the next tasks write the code until they pass. Never change a test only to
make it pass.

## 1. C ABI

- [ ] 1.1 Write the C test program (built in CI): join a local Redis, read the status, read a status change with `next_event`, cancel a blocked join, leave, `free` while another thread is in `status`; verify it fails to link or fails (red)
- [ ] 1.2 Create the crate `crates/doyen-ffi` (`staticlib`, `publish = false`) with the 9 functions, the `Arc` handle and the `DoyenError` out-parameter; verify the C test passes (green) under AddressSanitizer
- [ ] 1.3 Generate the header with cbindgen and check it in CI; verify the CI fails when the header is out of date
- [ ] 1.4 Commit the group in one commit (tests, code, spec changes, ticks): `feat(go): add the C ABI for the Go binding`

## 2. Go package

- [ ] 2.1 Write the Go tests against a local Redis: "Typical use", "Print a reason", "Join with a deadline", "Read changes", "Channel after Leave", "App stopped reading", "Forgotten object" (with `runtime.GC()`), "Concurrent calls", "Bad config", "PSA replica set" (MongoDB); verify they fail with `go test -race` (red)
- [ ] 2.2 Write `New`, `Join` with `cancel` on `ctx` end, `Status`, `Token`, `Leave`, the config structs and the `error` type with `Code`; verify the API, cancel and error tests pass
- [ ] 2.3 Write the event goroutine (`select` on stop), `StatusChanges()` (one channel per object), the warnings to `slog`, `AddCleanup` and the `Leave` steps; verify the remaining tests pass (green) with `go test -race`
- [ ] 2.4 Write the package docs (cgo needs, cross-compiling, platforms) and a README with a quick start; verify `go vet` and that the quick start runs
- [ ] 2.5 Commit the group in one commit (tests, code, spec changes, ticks): `feat(go): add the Go package`

## 3. CI and release

- [ ] 3.1 Add the CI job `go` (build the static library on Linux x64, the C test, `go test -race` with Redis and MongoDB services) and the `gomod` ecosystem (`/bindings/go`) to `.github/dependabot.yml`; verify the job passes
- [ ] 3.2 (owner) Create the repo `leofdss/doyen-go` and a deploy key with write access; add the private key as the secret `DOYEN_GO_DEPLOY_KEY` of the environment `release`
- [ ] 3.3 Write the release check test "Library too large" (the size check script fails on a file over 100 MiB); verify it fails, then write `scripts/check-go-libs.sh` and verify it passes
- [ ] 3.4 Add the jobs `go-build` and `go` to `release.yml` (three platforms, `llvm-strip --strip-debug`, the size check, Go tests, an attestation of each library with `actions/attest-build-provenance`, a CycloneDX SBOM per library uploaded to the GitHub release, the license files in the module, push and tag in `leofdss/doyen-go` in the environment `release`) and a `workflow_dispatch` dry run that pushes to a test branch of `doyen-go` without a tag; verify the dry run, then delete the test branch
- [ ] 3.5 Commit the group in one commit (tests, code, spec changes, ticks): `ci(go): build, strip and publish the Go module`
- [ ] 3.6 Run `python3 scripts/check-scenario-tests.py --change add-go-binding`; add "go get", "cgo off", "Release 0.3.0", "Library provenance" and "Module contents" to `openspec/manual-checks.md` once checked; verify every `go-binding` scenario is named by a test or a manual check
- [ ] 3.7 Write the Go quick start in the main README; then (owner) after the merge of this change, merge the release pull request (0.3.0) and approve the environment `release`; verify `go get github.com/leofdss/doyen-go@v0.3.0` and a build work in a clean module on linux/amd64

## Workflow follow-up

- Archive the change with `/opsx:archive add-go-binding` after the release.
