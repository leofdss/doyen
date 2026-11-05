# Tasks

## 1. C ABI

- [ ] 1.1 Create the crate `crates/doyen-ffi` (`staticlib`, `publish = false`) with the 9 functions, the `Arc` handle and the `DoyenError` out-parameter; verify a C test program (built in CI) joins a local Redis, reads the status and leaves
- [ ] 1.2 Generate the header with cbindgen and check it in CI; verify the CI fails when the header is out of date
- [ ] 1.3 Commit as `feat(go): add the C ABI for the Go binding`

## 2. Go package

- [ ] 2.1 Write `bindings/go` (`New`, `Join` with `cancel` on `ctx` end, `Status`, `Token`, `Leave`, the config structs, the `error` type with `Code`); verify the "Go public API", "Go types", "Cancel a join with the context" and "Go errors" scenarios with `go test` against a local Redis
- [ ] 2.2 Write the event goroutine (`select` on stop), `StatusChanges()`, the warnings to `slog`, `AddCleanup` and the `Leave` steps; verify the "Status changes on a channel", "Nothing waits for the channel reader", "Object collected without Leave" (with `runtime.GC()`) and "Warnings through slog" scenarios, all with `go test -race`
- [ ] 2.3 Write the package docs (cgo needs, cross-compiling, platforms) and a README with a quick start; verify `go vet` and that the quick start runs
- [ ] 2.4 Commit as `feat(go): add the Go package`

## 3. CI and release

- [ ] 3.1 Add the CI job `go` (build the static library on Linux x64, `go test -race` with Redis and MongoDB services); verify it passes
- [ ] 3.2 (owner) Create the repo `leofdss/doyen-go` and a deploy key with write access; add the private key as the secret `DOYEN_GO_DEPLOY_KEY`
- [ ] 3.3 Add the workflow for tags `go-v*` (three platforms, `llvm-strip --strip-debug`, size checks, Go tests, push and tag in `leofdss/doyen-go`); verify it on a test tag against a test branch of `doyen-go`, then delete the tag and the branch
- [ ] 3.4 Commit as `ci(go): build, strip and publish the Go module`
- [ ] 3.5 Update `CHANGELOG.md` and the main README (Go quick start), tag `go-v0.3.0`; verify `go get github.com/leofdss/doyen-go@v0.3.0` and a build work in a clean module on linux/amd64

## Workflow follow-up

- Archive the change with `/opsx:archive add-go-binding` after the release.
