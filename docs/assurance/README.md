# Assurance documents

The Asili toolchain's contribution to the regulatory file of a product that uses it (IEC 62304
for medical device software, ISO 14971 for risk management, and tool-qualification practice from
ISO 26262-8 and DO-330):

| Document | Contents |
|---|---|
| [requirements.md](requirements.md) | The toolchain's safety requirements, each with an ID and how it is verified. |
| [traceability.md](traceability.md) | Requirement → tests, generated; a test fails when it is stale or a requirement has no test. |
| [risk-register.md](risk-register.md) | Hazards the toolchain can contribute to, their controls, and the risk left to the manufacturer. |
| [tool-qualification.md](tool-qualification.md) | Evidence for qualifying the compiler as a tool and the runtimes as SOUP, limits, and how to use them in a product. |

These are evidence, not a certificate. Compliance with IEC 62304 is a property of a
manufacturer's product and its development process — a quality management system, software
development and maintenance plans, the product's own requirements, architecture, verification
and risk management, configuration management and problem resolution — assessed by the
manufacturer and, where the regulation requires, a notified body or other auditor. Nothing here
can stand in for that; it gives the manufacturer documented, checkable facts about the
language and toolchain to build on.

## Keeping them current

- A new safety-relevant behaviour gets a requirement in `requirements.md` and a test with a
  `// Verifies: REQ-…` comment; regenerate the matrix with
  `ASILI_TRACE=write cargo test -p asili-evaluator --test traceability`.
- A new hazard, or a change in how one is controlled, updates `risk-register.md`.
- A change to the evidence or limits updates `tool-qualification.md`.
