# Security Policy

[English](SECURITY.en.md) · [简体中文](SECURITY.md)

## Supported versions

Rurix has released 1.0 (tag `v1.0.0`, 2026-07-14); only the latest release line accepts security fixes.

| Version | Security support |
|---|---|
| Latest `1.x` / `main` | ✅ |
| MVP-phase `0.x` | ❌ |

## Reporting a vulnerability

**Please do not report security vulnerabilities in public issues.**

Please use one of the following **private** channels:

- Email: **25890346@qq.com** (security contact)
- GitHub private vulnerability reporting: the repository's **Security → Report a vulnerability**.

Please try to include:

- The affected component (`rurixc` / `rurix-rt` / `rx` / the release pipeline / an FFI boundary, etc.) and the version/commit.
- Reproduction steps or a PoC.
- An impact assessment (memory safety / resource-lifetime bypass / supply chain / signing chain, etc.).

## Handling timeline

- **Acknowledgement**: receipt confirmed within 3 business days.
- **Assessment & fix**: scheduled by severity; high-severity issues are prioritized.
- **Disclosure**: coordinated disclosure after the fix ships; reporters may be credited by name (optional).

## Scope notes

Rurix's security model centers on **intercepting resource-lifetime errors at compile time** and a **strict-only toolchain** (see [`01_VISION_AND_MISSION.md`](01_VISION_AND_MISSION.md) §3 and [`10_GOVERNANCE.md`](10_GOVERNANCE.md); Chinese-only). The following reports are especially welcome:

- **False negatives** in the borrow/resource checker (a use-after-free / double-free / cross-thread / cross-stream-unsynchronized case that should have been intercepted at compile time but was let through).
- Memory-safety defects at `unsafe` boundaries (PYD / C ABI / DLPack / cublas FFI).
- Bypasses of the release pipeline's signing / SBOM / license-whitelist audit.
- Supply-chain issues in package management (lockfile + vendor + checksum).

## Renderer surface (Rurix renderer SDK)

The full renderer-surface security policy lives in [`docs/renderer/support_policy.md`](docs/renderer/support_policy.md) §3 (this section is an entry pointer only; channels, timeline, and disclosure rules are the same as above). Renderer-specific report items:

- **Driver interaction**: Vulkan capability-chain negotiation / device lost / driver TDR boundary issues — attach the full `RURIX_VK_VALIDATION=1` output.
- **Shader supply chain**: tampering with the canonical SPV quartet / distributed fatbins, or bypasses of the deterministic digest anchors.
- **Vendor SDKs**: DLSS Streamline / FSR FidelityFX / BasisU / Jolt dynamic-loading chains (load hijacking, redistribution whitelist violations).
