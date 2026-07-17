# Contributing to Rurix

[English](CONTRIBUTING.en.md) · [简体中文](CONTRIBUTING.md)

Thank you for your interest in Rurix. Rurix is a GPU systems-programming language that makes *resource ownership, address spaces, and the parallel execution hierarchy* first-class citizens of the type system; from day one it builds **a testable specification + `conformance/` as the sole acceptance boundary + enforced provenance** into its governance backbone (see [`10_GOVERNANCE.md`](10_GOVERNANCE.md)). This guide is how those rules land for outside contributors.

> Governance overview: [`10_GOVERNANCE.md`](10_GOVERNANCE.md) §7–§9. Engineering-discipline mechanisms: [`14_ENGINEERING_DISCIPLINE.md`](14_ENGINEERING_DISCIPLINE.md). Mandatory context for every AI session: [`agents/AGENTS.md`](agents/AGENTS.md). (These documents are currently Chinese-only.)

## Core principle: the spec ↔ test ↔ PR triangle

Rurix's sole acceptance boundary is `conformance/`, not the PR description.

- **Spec first**: before touching `src/`, read the relevant `spec/*.md` clauses. A semantic PR **must cite a clause number `RXS-####`**; a semantic change that lacks a clause must add the spec first (with the matching change tier + revision row), and **the clause PR precedes the implementation PR**.
- **Every spec clause is anchored by ≥1 test** (`ci/trace_matrix.py` checks this globally).
- **Verification is mandatory**: a completion claim must carry the **real output** of the conformance / UI / unit-test commands; **numbers must come from command output** — filling them in from memory or inference is forbidden.

## Change tiers (the three-tier gate)

Choose a tier by semantic impact (details in 10 §3):

- **Direct** — engineering work that does not change the semantic surface (bugfix / refactor / docs / added test coverage). No new clause required.
- **Mini-RFC** — a small semantic surface, or edits to a rule file (`agents/AGENTS.md`).
- **Full RFC** — a new language feature / semantic surface / expansion direction. Template: motivation / design / alternatives / diff against the spec / open questions (see [`rfcs/`](rfcs/)).

**When in doubt, round up to stricter**: if the tier is unclear, take the stricter one; do not self-classify as Direct.

## AI-contribution policy (D-406, in force from day one, for everyone including the owner)

1. **Human-in-the-loop**: AI output must be approved by a human before merge; AI may not sign off on any attribution commitment on its own.
2. **Provenance**: substantive AI-authored content is tagged `Assisted-by: <tool>:<model>`; the commit message states the scope of impact and how it was verified.
3. **Anti-extractive contribution**: do not push the verification cost onto reviewers with a "submit first, sort it out later" approach.
4. **Off-limits**: AI may not define or modify UB clauses, memory-model mappings, FFI ABIs, or safety-envelope boundaries — those may only be written by a human via Full RFC.

> After open-sourcing, CI will automatically block PRs that lack provenance / verification output / a clause number.

## `unsafe` discipline

- Every `unsafe` block carries a `// SAFETY:` comment referencing a registry entry in [`unsafe-audit/`](unsafe-audit/); one operation per block.
- **An `unsafe` block with no registry entry is a CI error.** The whole repo defaults to `unsafe_code = deny`; at FFI boundaries (PYD / C ABI / DLPack / cublas), any `unsafe` requires an adjudicated minimal opening + registration.

## Pre-submission self-check

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
py -3 ci/trace_matrix.py --check        # spec ↔ test anchoring all green
py -3 ci/budget_eval.py --strict        # performance/diagnostics budgets measured_local (zero estimated)
py -3 ci/check_guardrails.py && py -3 ci/check_schemas.py && py -3 ci/check_structure.py
```

Performance numbers must follow [`milestones/m0/BENCH_PROTOCOL.md`](milestones/m0/BENCH_PROTOCOL.md) (L0 clock lock + three process-level independent runs + trimmed mean), with evidence written to `evidence/` (append-only — never deleted or modified).

## Upstream policy

Patches to LLVM are upstreamed first; a pinned fork patch must carry an upstream issue link (to guard against fork drift).

## Code of conduct

By participating in this project you agree to abide by the [`CODE_OF_CONDUCT.en.md`](CODE_OF_CONDUCT.en.md).

## License

By submitting a contribution you agree that it is dual-licensed under **MIT OR Apache-2.0** (see [`LICENSE-MIT`](LICENSE-MIT) / [`LICENSE-APACHE`](LICENSE-APACHE)), consistent with the project.
