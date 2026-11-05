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

(none yet)
