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

Choose a tier by semantic impact (details in 10 §3). Find your row, then act per the "Handled by" column:

| Your change | Tier | Requires | Handled by |
|---|---|---|---|
| Doc wording / pure refactor / added test coverage / semantics-preserving bugfix | **Direct** | Green CI | Straight to PR; nothing lands in `rfcs/` |
| In-spec bugfix / diagnostic-wording policy / internal switches / tool-behavior changes / rule-file (`agents/AGENTS.md`) edits | **Mini-RFC** | **Failing test first** + a one-page proposal | Land [`rfcs/mini-NNNN-*.md`](rfcs/TEMPLATE-MINI-RFC.md) first |
| New syntax / type-system change / runtime semantics / `unsafe` boundary / FFI ABI / memory-model mapping / stabilization / edition / design-principle change / touching a dead-route | **Full RFC** | RFC merged before implementation + feature gate + tracking issue + spec diff + conformance tests + stabilization report | Land [`rfcs/NNNN-*.md`](rfcs/TEMPLATE-RFC.md) first, then the feature gate |
| **Tier unclear** | → **round up to stricter** (a self-restraint guideline) | — | an agent may self-classify as Direct and records its rationale |

Templates and the proposal intake channel live in [`rfcs/README.md`](rfcs/README.md); the FCP-lite review window (public waiting window, 6-week train, promotion path) is in [`rfcs/README.md`](rfcs/README.md) §3 — advisory, with no required human-approval count; agents may proceed autonomously.

## AI-contribution policy (D-406, in force from day one; full agent autonomy)

1. **Full autonomy**: AI agents may autonomously draft / implement / verify / adjudicate / merge / bless / close out / flip statuses. **There is no agent approval gate or human sign-off checkpoint** — the agent is the decision-maker, rules on its own, and records its rationale.
2. **Provenance**: substantive AI-authored content is tagged `Assisted-by: <tool>:<model>`; the commit message states the scope of impact and how it was verified.
3. **Anti-extractive contribution**: do not push the verification cost onto reviewers with a "submit first, sort it out later" approach.
4. **High-sensitivity surfaces**: AI agents may autonomously draft, implement, and merge UB clauses, memory-model mappings, FFI ABIs, and safety-envelope boundaries — via a Full RFC as the record-and-traceability mechanism, with no separate approval required.

> After open-sourcing, CI automatically blocks PRs that lack provenance / verification output / a clause number — enforced by [`ci/check_contribution.py`](ci/check_contribution.py) in the PR Smoke guard step (10 §7 first-year roadmap landed).

### PR self-check (`ci/check_contribution.py` blocking items)

`ci/check_contribution.py` scans every non-merge commit in the PR range (`base..HEAD`); any of the three missing items turns CI red — self-check before submitting:

1. **Provenance**: every commit carries one of the provenance trailers (D-406 / hard rule 2): `Assisted-by: <tool>:<model>` (machine-readable colon form) / `Assisted-by: <name> (<model>)` (human parenthetical form, semantically mapped as tool=claude-code, e.g. `Assisted-by: Claude (Fable 5)`) / `Co-Authored-By:`.
2. **Clause number**: a commit touching `src/**/*.rs` or `spec/**/*.md` cites a clause number in the commit body / an added `//@ spec: RXS-####` comment line / an associated `rfcs/*.md` (or a deferred/RFC number; pure-docs / pure-tests commits are exempt, hard rule 7).
3. **Mandatory verification**: a commit with functional changes in `src/` carries a verification marker in its body (`Validation:` / `验证:` / a reference to `ci/*.py` / a `cargo test` command; numbers must come from command output, hard rules 3/10).

Local self-check: `py -3 ci/check_contribution.py` (PASS = exit 0 / blocking = non-zero exit).

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
