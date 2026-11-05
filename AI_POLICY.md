# AI policy

Version 1.0, 2026-11-05. The format follows the Fedora
[AI-Assisted Contributions Policy](https://docs.fedoraproject.org/en-US/council/policy/ai-contribution-policy/)
(v1.0, 2025-10-24). The rules follow the
[Linux Foundation guidance on generative AI](https://www.linuxfoundation.org/legal/generative-ai),
the OpenSSF and CNCF guide
[Securing Open Source in the Age of AI](https://openssf.org/resources/securing-open-source-in-the-age-of-ai-a-practical-guide/)
and the Linux kernel rules for
[AI coding assistants](https://docs.kernel.org/process/coding-assistants.html).

The rules apply to people and to AI agents. Agents also follow [AGENTS.md](AGENTS.md).

1. **Use:** You *MAY* use AI tools (code assistants, agents, chatbots) to contribute to
   doyen, as long as you follow the rules below.
2. **Accountability:** You *MUST* take responsibility for your contribution: its quality,
   its license and its use.
   - The contributor, a person, is always the author and is accountable for all of the
     contribution. An AI tool is a tool, not an author.
   - You *MUST* understand every line you send and be able to explain it. If you cannot, do
     not send it.
   - You *MUST* review and test AI output. It can look right and be wrong: bugs, old APIs,
     packages that do not exist. AI changes follow the same rules as any other change: the
     specs, tests first (TDD), the code style and the CI checks.
3. **Transparency:** You *MUST* say that you used AI tools when a significant part of the
   contribution came from a tool without changes. You *SHOULD* say it for other uses, when
   it helps. Fixing grammar or spelling, or making the wording clearer, needs no note.
   - Say it where the authorship is normally written. In a commit, end the message with the
     trailer `Assisted-by: <tool> (<model>)`, one for each tool. In a pull request, fill in
     the section *AI use* of the template: the tools and models, which parts they made,
     and how you checked them.
   - Examples:
     - `Assisted-by: Claude Code (claude-opus-5-5)`
     - `Assisted-by: generic LLM chatbot`
   - An AI agent *MUST NOT* add `Signed-off-by` or `Co-authored-by` for itself. Only a
     person can certify the [Developer Certificate of Origin](https://developercertificate.org/).
4. **Licenses:** doyen is MIT OR Apache-2.0.
   - You *MUST* use only AI tools whose terms let you contribute their output under these
     licenses.
   - You *MUST NOT* send output that copies code from another project, unless its license
     allows it. Then you *MUST* keep its copyright notice and say where it comes from.
   - You *SHOULD* follow your employer's rules on AI tools.
5. **Dependencies:** AI tools invent package names, and attackers publish packages with
   those names ("slopsquatting").
   - You *MUST* check that a new crate or package is the real one: name, owner, repository,
     downloads, recent releases.
   - You *MUST* add it with the official tool (`cargo add`, `npm install`) and commit the
     lock file.
6. **Security reports:** You *MUST* report in private, as [SECURITY.md](SECURITY.md) says,
   never in a public issue.
   - You *MUST* check AI findings before you send them. A report needs proof (a test, or
     steps that show the problem in doyen, inside the threat model), not only the output of
     a tool.
   - You *SHOULD NOT* trust the severity that a tool gives. Explain the impact in doyen's
     threat model.
   - You *MUST* say if and how AI tools helped.
7. **Review:** AI tools *MAY* help a reviewer with analysis and suggestions.
   - AI *MUST NOT* be the only or the final judge of a contribution: a person decides the
     merge.
   - Automated checks of facts, such as the CI, the tests and spam filters, are fine.
   - The person who approves an automated action is accountable for it.
8. **Closing:** The maintainer *MAY* close a pull request, issue or report without a
   detailed review when:
   - its author cannot explain it;
   - it has no tests, or it changes tests only to make them pass;
   - it adds code that no spec asks for;
   - it is the output of a tool that no person checked.

This policy will change as the tools change.

The key words *MAY*, *MUST*, *MUST NOT*, *SHOULD* and *SHOULD NOT* in this document are to
be read as described in [RFC 2119](https://datatracker.ietf.org/doc/html/rfc2119).
