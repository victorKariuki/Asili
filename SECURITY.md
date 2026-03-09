# Security

## Reporting a vulnerability

If you believe you have found a security vulnerability in the Asili implementation or tooling (e.g. parser, evaluator, CLI, LSP, or build pipeline), please report it in a way that allows maintainers to address it before public disclosure.

**Do not** open a public GitHub issue for security-sensitive bugs.

**Preferred:** Contact the maintainers privately (e.g. via the repository’s listed contact or a dedicated security contact, if one is published). Include:

- A short description of the issue.
- Steps or code to reproduce.
- Impact (e.g. denial of service, unexpected code execution, information exposure).
- Any suggested fix or mitigation, if you have one.

We will acknowledge the report and work with you on a fix and disclosure timeline. We do not have a formal bug bounty program at this time.

## Scope

- **In scope:** Asili compiler, evaluator, CLI (Pata), LSP (Mwalimu), drivers, and standard library implementation (Rust code and build artifacts).
- **Out of scope:** Third-party dependencies are covered by their own security policies; please report issues in those projects to the respective maintainers. Specification or documentation mistakes that do not directly cause exploitable behavior are best reported as normal issues or PRs.

## Security-related design

- The language and runtime are designed so that standard execution is memory-safe within the evaluator (no unsafe execution of user Asili code as native code). Use of optional FFI or syscall modules (e.g. `kiungo`, `syscall`) may expose platform behavior and should be used with care.
- Build and test pipelines (e.g. CI) should not consume untrusted Asili source or config without isolation appropriate to your environment.
