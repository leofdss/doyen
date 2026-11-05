# Security policy

## Supported versions

doyen is in early development (0.x). Only the latest release gets fixes; older 0.x versions
get none. There is no fixed support period before 1.0.

## Security contacts

The maintainer, Leonardo Farias de Souza Silva (GitHub `leofdss`), handles every report. A
private report through GitHub reaches the maintainer only.

## Report a vulnerability

Report in private. On GitHub, open the **Security** tab of this repository and choose
**Report a vulnerability**. Do not open a public issue.

Please include:

- the version or commit, the database (Redis, MongoDB…) and its setup;
- the steps, or better a test, that show the problem;
- what an attacker gains, in the threat model below;
- a fix and a regression test, if you have them;
- whether you used AI tools, and how you checked their output (see
  [AI_POLICY.md](AI_POLICY.md)).

This is a one-person project, so answers are best effort. A confirmed problem gets a fix, a
regression test and a GitHub security advisory (GitHub can give it a CVE). For the crate, the
advisory also goes to the [RustSec advisory database](https://rustsec.org/), which feeds
[OSV](https://osv.dev/). You get credit if you want it.

## Verify a release

From 0.1.0, every release can be checked:

- **Crate:** the GitHub release has `doyen-<version>.crate` and its SBOM
  (`doyen-<version>.cdx.json`). Check where the file was built:
  `gh attestation verify doyen-<version>.crate -R leofdss/doyen`.
- **npm:** the packages are published with provenance: `npm audit signatures` checks it.
- **Go:** each library in `leofdss/doyen-go` has an attestation:
  `gh attestation verify <library> -R leofdss/doyen`.

## Threat model

**What doyen is:** a library inside each instance of a service. It talks only to a database
that the team already runs. It has no server and opens no port.

**What doyen promises:**

- At most one master per group at a time, as long as the database keeps the guarantees that
  its documentation describes and the setup passes doyen's checks without a warning.
- The fencing token (`token()`) never goes back, also after the database loses its data,
  except in the cases that the README documents for each database (*Using the token*).
- An instance that cannot reach the database stops being master when its local lease ends,
  before another instance can take its place.

**In scope (please report):**

- Two masters, or a token that goes back, in a setup that doyen accepts without a warning.
- A secret, such as the password in a database URL, in an error, a warning, a log line or a
  status.
- A crash or a hang caused by a database reply or by a config.
- A memory-safety bug in the Node.js or Go bindings (the C ABI, napi-rs).

**Not in scope:**

- Anyone who can write doyen's keys, documents or rows. They can take the master role or stop
  the election. Protect the database with passwords, access rules (ACL) and TLS.
- An instance of the group that does not follow the rules: every member of a group is
  trusted.
- The known limits that doyen documents and reports at connect with a warning (codes
  `DOYEN_REDIS_*` and `DOYEN_MONGODB_*`), for example two masters during a Redis network
  partition without `min-replicas-to-write`. The fencing token protects the writes in these
  cases.
- Setups that doyen refuses (`DOYEN_UNSUPPORTED`), when someone forces them anyway.
- Bugs in the database server or in its client library: report them to that project.
