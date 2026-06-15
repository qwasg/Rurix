# Contributing to Rurix

Thank you for your interest in contributing to Rurix! This document describes the contribution process and requirements.

## Development Environment

### Required Tools

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.93.1 (pinned) | Compiler & toolchain implementation |
| clang | 22.1.x | Codegen backend |
| MSVC link.exe | | Windows linker |
| CUDA Driver | 525+ | GPU runtime |
| Python | 3.10+ | CI scripts & benchmarks |
| NVIDIA GPU | compute_89+ | GPU testing |

### Setup

```bash
git clone https://github.com/rurix/rurix.git
cd rurix

# Install Python dependencies
py -3 -m pip install -r requirements.txt

# Build
cargo build --workspace

# Run tests
cargo test --workspace
```

## Change Classification

All changes fall into one of three tiers. You **cannot** self-classify a change as Direct PR; when in doubt, escalate to the stricter tier.

| Tier | Scope | Process |
|------|-------|---------|
| **Direct PR** | Bug fixes, typo fixes, refactoring with no semantic change | PR with description |
| **Mini-RFC** | New features within existing architecture, new error codes, new conformance tests | PR + `rfcs/` document |
| **Full RFC** | New language semantics, UB clauses, memory model changes, FFI ABI, safety envelope boundaries | PR + full RFC with rationale, alternatives, and impact analysis |

## Spec-First Discipline

Rurix follows a **spec-first** development process:

1. Language semantics are defined in `spec/` (the single source of truth)
2. Semantic PRs **must** reference spec clause numbers (RXS-####)
3. If a spec clause doesn't exist for a change, write the spec first
4. Conformance tests must include `//@ spec: RXS-####` traceability anchors

## CI Requirements

All PRs must pass the **PR Smoke** CI pipeline (~30 min), which includes:

- **Structure check** — `py -3 ci/check_structure.py`
- **Schema validation** — `py -3 ci/check_schemas.py`
- **Guardrails** — `py -3 ci/check_guardrails.py`
- **Traceability freshness** — `py -3 ci/trace_matrix.py --check`
- **NVIDIA redistribution audit** — `py -3 ci/check_redistribution.py`
- **Python tests** — `py -3 -m pytest tests/ -q`
- **GPU smoke** — SAXPY end-to-end
- **Format check** — `cargo fmt --all --check`
- **Clippy** — `cargo clippy --workspace --all-targets -- -D warnings`
- **Rust tests** — `cargo test --workspace`
- **Conformance batch** — borrowck, MIR golden, const eval
- **CLI smoke** — rx build/run/check/fmt/bench
- **Package manager** — offline resolve, reproducible rebuild

### Running CI Checks Locally

```bash
# Format
cargo fmt --all --check

# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Tests
cargo test --workspace

# Guardrails
py -3 ci/check_guardrails.py

# Schema validation
py -3 ci/check_schemas.py
```

## Code Style

- **Rust**: Follow `rustfmt` (enforced by CI). Edition 2024.
- **Unsafe code**: Globally denied (`unsafe_code = "deny"`). The only exception is `rurix-rt` FFI boundary, where every `unsafe` block must have a `// SAFETY:` comment and an entry in `unsafe-audit/`.
- **Error codes**: Follow the segmented scheme (0xxx lexical, 1xxx names, 2xxx types, 3xxx coloring, 4xxx borrow, 5xxx const eval, 6xxx codegen, 7xxx toolchain/pkg). Error code semantics are frozen once introduced.

## Testing

### Test Categories

| Category | Location | Description |
|----------|----------|-------------|
| Unit tests | `src/*/src/` (`#[test]`) | Per-module Rust unit tests |
| Conformance | `conformance/` | `.rx` files with spec traceability anchors |
| UI golden | `tests/ui/` | `.rx` + `.stderr` snapshot pairs |
| MIR golden | `tests/mir/` | `.rx` + `.mir` snapshot pairs |
| PTX golden | `tests/ptx/` | `.rx` + `.nvptx` snapshot pairs |
| GPU roundtrip | `src/rurix-rt/tests/` | End-to-end GPU execution with host verification |

### Updating Golden Files

Golden snapshots can only be updated with explicit opt-in:

```bash
RURIX_BLESS=1 cargo test -p rurixc --test ui_golden
```

Updates must be accompanied by a `bless_log.md` entry explaining the reason. CI verifies this via `check_guardrails.py`.

## Evidence Discipline

- Performance claims must cite evidence JSON paths from `evidence/`
- Evidence files are append-only (never modify or delete)
- Evidence levels: `measured_local` (real run) > `unlocked` (envelope) > `estimated` (placeholder)
- No numbers from memory — all data must come from command output

## AI Contributions

AI-assisted contributions are welcome but must follow the policy in `agents/AGENTS.md`:

- **Human-in-the-loop**: All AI output must be approved by a human before merging
- **Provenance**: Substantive AI content must include `Assisted-by: <tool>:<model>` in commit messages
- **Verification required**: Claims of completion must include real command output
- **Restricted zones**: UB clauses, memory model mapping, FFI ABI, and safety envelope boundaries are human-only (Full RFC required)

## Questions?

- Open an issue for bugs or feature requests
- Start a discussion for design questions
- Check `spec/` for language semantics questions
- Check `deep-research/` for technical background
