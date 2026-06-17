# 安全政策

## 支持的版本

Rurix 处于 0.x(MVP 期),仅最新发布线接受安全修复。1.0(开源后首个 LTS 质量版本)起将提供明确的安全支持窗口。

| 版本 | 安全支持 |
|---|---|
| 最新 `0.x` / `main` | ✅ |
| 更早 `0.x` | ❌ |

## 报告漏洞

**请勿在公开 issue 中报告安全漏洞。**

请通过 GitHub 的**私有漏洞报告**渠道提交:
仓库 **Security → Report a vulnerability**(Private vulnerability reporting)。

报告请尽量包含:

- 受影响的组件(`rurixc` / `rurix-rt` / `rx` / 发布链路 / FFI 边界等)与版本/commit。
- 复现步骤或 PoC。
- 影响评估(内存安全 / 资源生命周期绕过 / 供应链 / 签名链 等)。

## 处理时间线

- **确认**:3 个工作日内确认收到。
- **评估与修复**:依严重度排期;高危优先。
- **披露**:修复发布后协调公开(coordinated disclosure);报告者可署名致谢(可选)。

## 范围提示

Rurix 的安全模型核心是**编译期拦截资源生命周期错误**与**strict-only 工具链**(见 [`01_VISION_AND_MISSION.md`](01_VISION_AND_MISSION.md) §3、[`10_GOVERNANCE.md`](10_GOVERNANCE.md))。以下尤其欢迎报告:

- 借用/资源检查器**漏报**(本应编译期拦截却放行的 use-after-free / double-free / 跨线程 / 跨流未同步)。
- `unsafe` 边界(PYD / C ABI / DLPack / cublas FFI)的内存安全缺陷。
- 发布链路签名 / SBOM / 许可白名单审计的绕过。
- 包管理(lockfile + vendor + checksum)的供应链问题。
