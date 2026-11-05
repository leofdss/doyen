# AI policy

doyen accepts contributions made with AI tools: code assistants, agents and chatbots. This
policy follows the
[Linux Foundation guidance on generative AI](https://www.linuxfoundation.org/legal/generative-ai),
the OpenSSF and CNCF guide
[Securing Open Source in the Age of AI](https://openssf.org/resources/securing-open-source-in-the-age-of-ai-a-practical-guide/)
and the Linux kernel rules for
[AI coding assistants](https://docs.kernel.org/process/coding-assistants.html). It applies to
people and to AI agents. Agents also follow [AGENTS.md](AGENTS.md).

## A person is the author

- A person is the author of every contribution and is responsible for all of it. An AI tool
  is a tool, not an author.
- Understand every line you send and be able to explain it. If you cannot, do not send it.
- Review and test AI output like code from a new colleague. It can look right and be wrong:
  bugs, old APIs, packages that do not exist.
- AI changes follow the same rules as any other change: the specs, tests first (TDD), the
  code style and the CI checks.

## Say how you used AI

- **Commits:** when an AI tool wrote a meaningful part of a commit, end the message with the
  trailer `Assisted-by: <tool> (<model>)`, for example
  `Assisted-by: Claude Code (claude-opus-5-5)`. One trailer for each tool.
- **No AI sign-off:** an AI agent never adds `Signed-off-by` or `Co-authored-by` for itself.
  Only a person can certify the [Developer Certificate of Origin](https://developercertificate.org/).
- **Pull requests:** fill in the section *AI use* of the template: the tools and models,
  which parts they made, and how you checked them.

## Licenses

- doyen is MIT OR Apache-2.0. Use only AI tools whose terms let you contribute their output
  under these licenses.
- Do not send output that copies code from another project, unless its license allows it.
  Then keep its copyright notice and say where it comes from.
- Follow your employer's rules on AI tools.

## Dependencies

- AI tools invent package names, and attackers publish packages with those names
  ("slopsquatting"). Before you add a crate or a package, check that it is the real one:
  name, owner, repository, downloads, recent releases.
- Add it with the official tool (`cargo add`, `npm install`) and commit the lock file.

## Security reports

- Follow [SECURITY.md](SECURITY.md): report in private, never in a public issue.
- Check AI findings before you send them. A report needs proof (a test, or steps that show
  the problem in doyen, inside the threat model), not only the output of a tool.
- Do not trust the severity that a tool gives. Explain the impact in doyen's threat model.
- Say if and how AI helped.

## What may be closed

The maintainer may close a pull request, issue or report without a detailed review when:

- its author cannot explain it;
- it has no tests, or it changes tests only to make them pass;
- it adds code that no spec asks for;
- it is the output of a tool that no person checked.

This policy will change as the tools change.
