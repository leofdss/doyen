# Governance

doyen is a one-maintainer project. This file says who decides, who has access to what, and
how secrets are kept, as the OpenSSF
[Security Baseline](https://baseline.openssf.org/) asks (OSPS-GV-01, OSPS-BR-07).

## Roles

- **Maintainer:** Leonardo Farias de Souza Silva (GitHub `leofdss`). Approves the OpenSpec
  proposals, reviews and merges pull requests, merges the release pull request and approves
  each publish, and handles the security reports.
- **Contributors:** anyone who opens an issue or a pull request. They follow
  [CONTRIBUTING.md](CONTRIBUTING.md) and the [AI policy](AI_POLICY.md).
- **AI agents:** tools that a person uses. They never merge, publish, sign or approve
  ([AI_POLICY.md](AI_POLICY.md), rule 7).

## Access to sensitive resources

| Resource | Who has access |
|---|---|
| Admin of `leofdss/doyen` and of the `main` ruleset | the maintainer |
| Approval of the GitHub environment `release` | the maintainer |
| Owner of the crate `doyen` and of the npm packages `@leofdss/doyen*` | the maintainer; CI publishes through Trusted Publishing from `release.yml` |
| Write access to `leofdss/doyen-go` | the maintainer; the CI deploy key (environment `release` only) |
| GitHub App `doyen-release` | opens and updates the release pull request (contents and pull requests: write) |
| Dependabot | opens dependency update pull requests |

New collaborators get the lowest access first (read or triage), and more only after the
maintainer has worked with them.

## Decisions

- Behaviour changes go through an OpenSpec proposal. The maintainer approves it; open
  points are marked "to confirm (owner)" until then.
- A release happens only when the maintainer merges the release pull request.

## Review

Every change reaches `main` through a pull request that the maintainer reviews; an AI review
is advice only. A second human reviewer (OSPS-QA-07) is not possible with one maintainer.
When a second maintainer joins, every pull request will need the approval of a person who is
not its author.

## Secrets

- Secrets live only in GitHub secrets, in the environment `release` when a job there needs
  them: `RELEASE_APP_ID`, `RELEASE_APP_KEY`, `DOYEN_GO_DEPLOY_KEY`. There are no registry
  tokens: crates.io and npm use Trusted Publishing.
- The app key and the deploy key are rotated once a year, and at once when a leak is
  suspected or a person with access leaves.

## Changes to this file

By pull request, approved by the maintainer.
