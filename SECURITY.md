# 安全政策

## 支持的版本

Rurix 已发行 1.0(2026-07-14,tag `v1.0.0`),仅最新发布线接受安全修复。

| 版本 | 安全支持 |
|---|---|
| 最新 `1.x` / `main` | ✅ |
| MVP 期 `0.x` | ❌ |

## 报告漏洞

**请勿在公开 issue 中报告安全漏洞。**

请通过以下任一**私下**渠道提交:

- 邮件:**25890346@qq.com**(安全联系)
- GitHub 私有漏洞报告:仓库 **Security → Report a vulnerability**(Private vulnerability reporting)

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

## 渲染器面(Rurix 渲染器 SDK)

渲染器面安全政策全文 = [`docs/renderer/support_policy.md`](docs/renderer/support_policy.md) §3(本段为入口指针,渲染器特有面不复制;渠道/时间线/披露纪律同上文)。渲染器面特有报告要素:

- **驱动交互**:Vulkan 能力链协商 / device lost / 驱动 TDR 边界问题——附 `RURIX_VK_VALIDATION=1` 全程输出。
- **shader 供应链**:canonical SPV 四件套 / 分发 fatbin 的篡改,或确定性 digest 锚的绕过。
- **vendor SDK**:DLSS Streamline / FSR FidelityFX / BasisU / Jolt 动态加载链(装载劫持、再分发白名单面越界)。
