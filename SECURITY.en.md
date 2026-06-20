# Security Policy

[English](SECURITY.en.md) · [简体中文](SECURITY.md)

## Supported versions

Rurix is at 0.x (the MVP phase); only the latest release line accepts security fixes. From 1.0 (the first LTS-quality release after open-sourcing) onward, a defined security-support window will be provided.

| Version | Security support |
|---|---|
| Latest `0.x` / `main` | ✅ |
| Earlier `0.x` | ❌ |

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
