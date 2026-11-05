# Manual checks

The tests are the source of truth: every spec scenario is named by an automated test. A few
scenarios can only be checked by a person (for example a CI failure path, or a page on
crates.io). Each of them has one line here, and `scripts/check-scenario-tests.py` accepts the
line in place of a test.

Format of a line (one per scenario; say how it was checked and when):

```text
- <capability> / <scenario>: <how and when it was checked>
```

A line without the "how and when" part does not count. A line that names a scenario no spec
has fails the check, like a stale test. When a scenario changes, check it again and update its
line.

## Checks

The failure paths of `ci-checks` were checked on 2026-11-05 with a throwaway draft pull
request (#4, CI run 42108749159, branch deleted after): one fault per job in one commit. The
jobs with a fault failed with the message of that fault; `coverage`, `scenarios` and
`licenses`, which had none, passed. The green path is pull request #3 (CI run 42108494913).

- ci-checks / Pull request opened: pull request #3, 2026-11-05 (run 42108494913): the CI ran `fmt`, `clippy`, `test`, `msrv` and `commits` with the six other jobs, all green.
- ci-checks / A check fails: pull request #4, 2026-11-05 (run 42108749159): eight jobs failed and the run was marked `failure`.
- ci-checks / Unformatted file: pull request #4, 2026-11-05: a function with wrong spacing in `crates/doyen/src/lib.rs`; `fmt` failed with "Diff in crates/doyen/src/lib.rs".
- ci-checks / Clippy warning: pull request #4, 2026-11-05: a `&Vec<u32>` argument; `clippy` failed with `ptr_arg` ("writing `&Vec` instead of `&[_]`").
- ci-checks / A test fails: pull request #4, 2026-11-05: a unit test with a wrong sum; `test` failed (`tests::a_test_fails`), and `coverage` still passed (`--ignore-run-fail`).
- ci-checks / Code needs a newer Rust: pull request #4, 2026-11-05: `Vec::pop_if` (stable in Rust 1.86); `msrv` failed with E0658 (`vec_pop_if`), and the jobs on stable Rust built it.
- ci-checks / Report after the tests: pull request #3, 2026-11-05 (run 42108494913): the run has the artifact `coverage` (`lcov.info`, empty while the crate is the placeholder); run 42108749159 has one with data.
- ci-checks / Conflict between changes: pull request #4, 2026-11-05: a second open change with the capability `ci-checks`; `specs` failed with "capability `ci-checks` is in two open changes".
- ci-checks / Unpinned action: pull request #4, 2026-11-05: `actions/checkout@v6` in `.github/workflows/zz-unpinned.yml`; `workflows` failed with `unpinned-uses` and named that file.
- ci-checks / Known vulnerability: pull request #4, 2026-11-05: `smallvec = "=1.6.0"`; `deps` failed and named RUSTSEC-2021-0003.
- ci-checks / License not allowed: pull request #4, 2026-11-05: `webpki-roots = "=0.25.4"` (MPL-2.0); `deps` failed with "license is not explicitly allowed".
- ci-checks / Unknown source: pull request #4, 2026-11-05: `itoa` from a git repository; `deps` failed with "detected 'git' source not explicitly allowed".
