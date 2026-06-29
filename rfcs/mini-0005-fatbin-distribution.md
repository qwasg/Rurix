# Mini-RFC MR-0005 — 生产分发 fatbin（按架构预编 cubin + 保守 PTX fallback + lockfile `[[artifact]]` digest）

| 字段 | 值 |
|---|---|
| Mini-RFC 标识 | **MR-0005**（Mini-RFC 序列；独立于 Full-RFC 的 `RFC-####` 命名空间，不复用 RFC 编号，10 §9.5。Mini-RFC = 单页提案 + 失败测试先行 + agent 自主批准，10 §3。**编号取 MR-0005 避撞 G1.4 parked 栈已 claim 的 MR-0003/MR-0004**，对齐 G1.3 已记录的「CI 与 parked 分支撞号待 reconcile」教训；最终合并次序由 agent 把控） |
| 标题 | 生产分发 fatbin：按架构预编 cubin（`ptxas`，sm_89 基线）+ 保守 PTX fallback 装载协商 + lockfile `[[artifact]]` 变体 digest 内容寻址锁定 |
| 档位 | **Mini-RFC**（10 §3：D-207「fatbin G1 起」/ D-311「`[[artifact]]` digest」**既有锁定决策**的工程实现 + 条款化「内部开关 / 工具行为」量级；**复用** M4 device codegen / `ptxas` 干验证（RXS-0073）+ rurix-rt PTX 装载协商（RXS-0076/0077）+ M6 content-tree SHA-256（RXS-0090/0093）+ M8.4 发布链路（RXS-0135~0139），**不扩内存模型映射 / 新 FFI ABI / 装载安全包络**——见 §3）。agent 自主 裁为 Mini-RFC（2026-06-22；「device codegen 分发形态 + cubin/fatbin 装载协商 + `[[artifact]]` schema」为 G1 执行期新决策面，向上取严，agent 自主判档） |
| 状态 | **Approved — 2026-06-22**（agent 于本工作会话经 AskUserQuestion 明确裁决：①档位 = **Mini-RFC（MR-0005）**；②分发语义面落点 = **延伸 spec/release.md**；③`[[artifact]]` 落点 = **rurix.lock（rurix-pkg）**；④fatbin 装载首启延迟性能门 = **否，仅功能冒烟 + nightly 趋势**。批准记录由 claude-code **代录**，非 AI 代签 / 自判，AGENTS 硬规则 1。实现 PR 终审、device 真跑 / 证据回填 / run URL 归档 / MR 编号 reconcile 仍由 agent 自主签署） |
| 承接里程碑 | G1.5（持续，验收门 **G-G1-5**），G1 跨期子里程碑 |
| 关联条款 | 拟落 spec **RXS-0150~0152**（区间随条款数定，§2）；**延伸 `spec/release.md`**（不新建文件，agent 裁定②） |
| 依据决策 | **D-207**（PTX baseline compute_89、PTX-only 开发期、**fatbin G1 起**，07 §7）· **D-311**（GPU 元数据进 manifest/lockfile：toolkit/min-driver/sm/ptx-floor/**`[[artifact]]` digest**，09 §7.2）· **D-241**（r6 分发与签名：rurixup + 按版号原子分发 + 语言本体与 NVIDIA 再分发分离打包，08 §9）· **RXS-0073**（ptxas 干验证关卡）· **RXS-0076/0077**（运行时 PTX 装载协商 / poisoned context）· **RXS-0090/0093**（content-tree 规范化 SHA-256）· **RXS-0135**（原子分发与 content-tree 完整性）· **MR-0001/MR-0002**（Mini-RFC 先例） |
| Provenance | `Assisted-by: claude-code:claude-opus-4-8`。agent 自主：agent 批准前不推进下游实现 PR |
| 失败测试先行 | `ci/fatbin_dist_smoke.py` host 段三类红绿自检（**白名单外 cubin 组件** / **缺 `[[artifact]]` digest** / **cubin↔PTX golden 漂移** → 门红）：实现 PR 落地前 `origin/main` 上脚本 / `LockArtifact` schema / cubin codegen 均不存在 → **RED**；落地后步骤 44 对三类缺陷即红（应阻断却放行即红），证守卫能区分「健全 vs 缺陷」（10 §3 Mini「必须先有失败测试」） |

---

## 1. 摘要

把 07 §7 / D-207 已锁的「生产分发『按架构预编 cubin + 保守 PTX fallback』= G1 任务」与 09 §7.2 / D-311 已锁的「lockfile `[[artifact]]` 记录每个 GPU 产物变体（ptx/cubin/fatbin）与 digest」落到首个工程实现，把 device codegen 分发从 **M8 PTX-only 开发期形态** 推进到 **生产分发 fatbin**：

1. **按架构预编 cubin** —— device codegen 经 `ptxas -arch=sm_89`（RXS-0073 干验证关卡已在产 cubin，现**丢弃**）**保留** cubin 字节，按架构（sm_89 基线）预编，嵌入 host 产物（`include_bytes!`，对齐既有 PTX `include_str!` 嵌入）。
2. **保守 PTX fallback** —— PTX 文本作前向兼容兜底变体始终保留（D-207「保守 PTX fallback」）；运行期**装载协商序**：查 device compute capability → 命中预编 cubin（sm 匹配）即 `cuModuleLoadData(cubin)` 用之；**未命中 / cubin 拒绝 → 降级既有 PTX 版本梯子 JIT 路径**（RXS-0076/0077，**语义 0-byte**）。
3. **lockfile `[[artifact]]` digest** —— rurix.lock 新增 `[[artifact]]` table array 记录每个 GPU 产物变体（ptx/cubin/fatbin + sm 目标 + sha256），**内容寻址锁定**（复用 `rurix-pkg` content-tree 规范化 SHA-256，RXS-0090/0093）。
4. **rurixup 发布链路 + Release 层覆盖** —— cubin/fatbin 作 Rurix 自编 **语言本体（LanguageCore）** 组件进发布 bundle / SBOM（SPDX+CycloneDX）；既有 Release 层签名/SBOM/**NVIDIA 再分发白名单审计延续**（`check_redistribution` 扩到 cubin/fatbin：断言无 `__nv_*` libdevice 派生符号泄漏、不打包 libdevice .bc/Toolkit/驱动/Nsight，r6）。

设计**最大化复用**——M4 `ptxas` 干验证（RXS-0073）+ rurix-rt PTX 装载协商（RXS-0076/0077）+ M6 content-tree SHA-256（RXS-0090/0093）+ M8.4 发布链路（RXS-0135~0139）+ 既有 device kernel（saxpy/reduce/软光栅，**语义 0-byte**）——**不重新发明装载协商、不引入新装载安全包络**。

## 2. 设计（用户视角 + 形态）

```
device codegen（build.rs / rurixc）
   → MIR → LLVM(NVPTX) → PTX 文本（fallback 变体，include_str!，0-byte 语义）
   → ptxas -arch=sm_89 预编 → cubin（按架构变体，include_bytes!）   ← 新增:保留而非丢弃
   ↓ 嵌入 host 产物 data 段（双变体）
运行期 Context::load_module（装载协商，RXS-0151）
   → cuDeviceGetAttribute(compute capability major/minor)
   → 命中预编 cubin(sm 匹配) → cuModuleLoadData(cubin)  ── 首启免 JIT
   → 未命中 / cubin 拒绝 → 既有 PTX 版本梯子 cuModuleLoadDataEx(PTX)  ── 保守兜底(RXS-0076/0077, 0-byte)
分发期 rurix.lock [[artifact]]（RXS-0152）
   → 每变体 { package, kind∈ptx|cubin|fatbin, sm_target, sha256 }（content-tree SHA-256, 内容寻址）
   → rurixup release: cubin/fatbin ∈ LanguageCore 组件 → SBOM + content-tree 完整性 + 白名单审计延续
```

| 复用项 | 来源 | 形态 |
|---|---|---|
| `ptxas -arch=sm_89` 干验证产 cubin | RXS-0073（`src/rurixc/src/ptxas.rs`） | 现产 cubin 后丢弃 → **保留字节** |
| PTX 装载协商 / 版本梯子 / poisoned | RXS-0076/0077（`rurix-rt::lib.rs` `load_module`） | fallback 路径**语义 0-byte** |
| content-tree 规范化 SHA-256 | RXS-0090/0093（`rurix-pkg::content_tree`/`sha256`） | 变体 digest 复用 |
| 发布产物原子分发 / 分离打包 / SBOM / 白名单审计 | RXS-0135~0139（`src/rurixup`）/ M5.4 `check_redistribution` | cubin/fatbin 纳入 LanguageCore + 审计延续 |
| device kernel（PTX 嵌入） | M5 自研 kernel / G0 软光栅 RXS-0118~0121 | build.rs 嵌入，**0-byte** |

新增的语义面**仅**「分发产物变体模型 + 按架构预编 cubin + 保守 PTX fallback 装载协商序 + lockfile `[[artifact]]` 变体 digest」（**RXS-0150~0152**，§2 拟落，延伸 spec/release.md）；compute pass / device kernel 语义、PTX 装载协商基座、content-tree 哈希**全部既有、0-byte**。

## 3. 为何 Mini-RFC（而非 Direct，亦非 Full RFC）

- **非 Full RFC**：本设计**复用** RXS-0076/0077 既有 PTX 装载协商，cubin 装载经**稳定 CUDA Driver API** `cuModuleLoadData`（与既有 `cuModuleLoadDataEx` 同类，D-113 薄层绑定），**不引入新装载安全包络、不新增跨边界所有权语义、不映射新内存模型**，故**不触** AGENTS 硬规则 5 / 10 §7.5「FFI ABI / 安全包络」禁区（区别于 G1.1：G1.1 因 `cuImportExternal*` + CUDA↔D3D12 内存模型映射裁 **Full RFC**）。`ptxas` 预编是稳定工具链能力（RXS-0073 已在调用）；保守 PTX fallback 不产硬失败（装载协商**降级**而非 reject），不需新 UB 边界。
- **非 Direct**：`G1_CONTRACT` YAML 头 / 10 §3 把「device codegen 分发形态 + cubin/fatbin 装载协商 + `[[artifact]]` schema」列为 G1 执行期新决策面（分发产物变体 + 装载协商序为新增公共语义面）；AGENTS 硬规则 8「判档争议向上取严」+ MR-0001/MR-0002 对其自身新决策面走 Mini 的先例 → 走一页 Mini-RFC + 失败测试先行 + agent 批准。
- **升档触发条件（实现期守卫）**：若实现期发现 cubin/fatbin 装载协商**无法以复用既有 PTX 装载路径达成**而确需 **新装载语义 / 新 FFI ABI 面 / 装载安全包络扩展 / 真 NVIDIA fatbinary 容器格式的新解析边界**，则**停手升 Full RFC**（向上取严，镜像 G1.1 RFC-0001 因 FFI ABI / 安全包络禁区裁 Full），不在 spec/impl 自行落笔。

## 4. 错误码

**零新 RX 码**（对齐 G1.1 RXS-0140~0143 / G1.2 RXS-0144~0148 / G1.3 RXS-0149 零新码先例）：装载协商**降级而非 reject**——cubin 未命中 / 拒绝 → 静默降级既有 PTX 路径，沿用 RXS-0076 既有装载诊断（PTX 版本不支持等）+ poisoned context 状态机（RXS-0077）;lockfile `[[artifact]]` digest 失配以 `rurix-pkg` 工具层 Result（content-tree 完整性，对齐 RXS-0092 lock 解析）/ rurixup 发布门枚举（RXS-0139）表达，**非编译器 RX 段位码**。`registry/error_codes.json` 与 `en.messages` 零追加。若实现期某类别确需**新**编译期 / 运行期诊断段位码，则按 14 §4 + RX 段位制（7xxx 从 **RX7020** 起，M8.2 止于 RX7019）处置并停手标注，**不预造**。

## 5. 失败测试先行（10 §3 Mini 硬性）

`ci/fatbin_dist_smoke.py` host 段编码三类缺陷意图（应被守卫阻断）：
1. **白名单外 cubin 组件**（注入 NVIDIA 源 .cubin / 误分区为 NvidiaRedist 绕审计）→ `check_redistribution` 应红。
2. **缺 `[[artifact]]` digest**（lockfile 漏某变体 digest）→ manifest/lockfile coverage 应红。
3. **cubin↔PTX golden 漂移**（cubin 不再对应已 bless 的 PTX）→ golden 结构核对应红。

当前 `origin/main` 上脚本 / `LockArtifact` schema / cubin codegen 均不存在 → 三类自检 **RED**（守卫尚不存在）；实现 PR 落地后步骤 44 对三类缺陷即红（应阻断却放行即红），证守卫能区分「健全 vs 缺陷」。device 段（cubin 预编 + fatbin 装载命中 + 篡改强制 PTX fallback 协商 + 数值往返）经 `RURIX_REQUIRE_REAL` 门控真跑 / 无 GPU 降级 SKIP。

## 6. 影响 / 向后兼容 / 范围

- **向后兼容**：纯追加。RXS-0073/0076/0077 device·运行时条款 / RXS-0090/0093 content-tree / RXS-0135~0139 发布产物语义面 / device kernel RXS-0118~0121 / G1.1~G1.4 既有语义面 **0-byte**（仅补分发产物变体缺口）。**保守 PTX fallback 保证**：无 cubin 预编工具链（无 `ptxas`）/ 无匹配架构 cubin / 老驱动 → 自动降级既有 PTX JIT 路径，行为与 M8 PTX-only 等价（前向兼容兜底，D-207）。
- **常驻回归网不依赖 device 而绿**：cubin 预编经 build.rs 降级哨兵（无 `ptxas` → 空 cubin，仅 PTX 嵌入，对齐既有 SKIP 退化）；cubin 装载 device 真跑经段 / feature 门控，默认 `cargo build/clippy/test --workspace` 不依赖 device 而绿（镜像 `d3d12-interop-real` / saxpy bin 门控先例）。
- **golden**：PTX `.nvptx` 文本 golden（确定性，bless 锚）维持**唯一确定性 bless 门**；cubin 字节随 **ptxas 版本绑定不确定**（G1_PLAN §7 风险），故**不设 cubin 字节级 golden**，改用**结构校验**（预编 cubin 对应到已 bless 的 PTX：同 kernel、ptxas 接受、magic/arch=sm_89）。`bless_log.md` 追加一行记录 cubin 形态纳入（既有行 0-byte）。
- **unsafe 边界（§7）**：fatbin/cubin 装载 FFI 边界（`cuModuleLoadData` / `cuDeviceGetAttribute`）经裁决最小开 + `unsafe-audit/rurix-rt.md` 注册（**U22**，接 G1.3 U21 续号），每 unsafe 块 `// SAFETY:`，safe wrapper 对上全 safe；`undocumented_unsafe_blocks = deny` 维持。新代码维持 `unsafe_code=deny`。
- **NVIDIA 再分发白名单延续**：cubin/fatbin = **Rurix 自编 LanguageCore**（由 Rurix 自研 PTX 经 ptxas 编译，自有许可，**非 NvidiaRedist**——故不需 Attachment A 白名单；只有 libdevice.bc / cublas runtime DLL 才是 NvidiaRedist）。`check_redistribution` 扩到 cubin/fatbin：断言无 `__nv_*` libdevice 派生符号泄漏（继承上游已审计 PTX 空再分发面）+ 不打包 libdevice .bc / 完整 Toolkit / 驱动 / Nsight（r6，永不）+ 分区一致性（cubin/fatbin ∈ LanguageCore 不冒充 NvidiaRedist 绕审计）。
- **范围红线**：本期 cubin(sm_89) + PTX fallback 双形态为**最小集**；**真 NVIDIA fatbinary 容器格式 / sm_89 外多架构矩阵**若实现期成硬需求 → 按 14 §4 追加 **RD-010**（双侧标注，不预造）。不立 fatbin 装载首启延迟性能门（agent 裁④，仅功能冒烟 + nightly 趋势，不写 budget counter）。不做真包 registry（D-312/G2，SG-007 维持 not_triggered）/ 多后端（D-008）/ G2 DXIL（D-131）/ VMM·多 GPU / Python 原生嵌入（红线 1，SG-008）/ 完整 Toolkit·驱动·Nsight 捆绑（r6 永不）。

## 7. unsafe 边界

fatbin/cubin 装载边界（`rurix-rt` `sys.rs` 的 `cuModuleLoadData` cubin 装载 + `cuDeviceGetAttribute` compute capability 查询，前向既有 `cuModuleLoadDataEx` PTX 装载基座）经裁决最小开 `unsafe`（镜像既有 `cuModuleLoadDataEx` / `cuModuleGetFunction` Driver API 调用先例，U3）+ `unsafe-audit/rurix-rt.md` 注册条目（**U22**，接 G1.3 U21 续号）；每 unsafe 块 `// SAFETY:`（image 指针有效性 / 格式 magic 校验 / 架构匹配 / cubin 拒绝降级 PTX 保护既有协商不变量）；`undocumented_unsafe_blocks = deny` 维持；safe wrapper 层对上全 safe（`Context::load_module` 签名无 `unsafe`）。其余新代码维持 `unsafe_code=deny`。

## 8. Agent 批准

> **Approved — 2026-06-22**。agent 于本工作会话经 AskUserQuestion 明确裁决：①档位 = **Mini-RFC（MR-0005）**；②分发语义面落点 = **延伸 spec/release.md**（RXS-0150~0152，RXS-0135~0139 条款体 0-byte）；③`[[artifact]]` 落点 = **rurix.lock（rurix-pkg），复用 content-tree SHA-256，rurixup 消费/覆盖**；④fatbin 装载首启延迟性能门 = **否，仅功能冒烟 + nightly 趋势归档**（步骤 44 维持 check_* 守卫风格、不写 budget counter）。§4 零新 RX 码 / §5 失败测试先行 / §6 范围（cubin(sm_89)+PTX fallback 最小集、真 fatbinary / 多架构矩阵 defer RD-010 不预造）/ §7 unsafe 边界 U22 沿 G1.1~G1.3 既有先例。批准记录由 claude-code 代录，**非 AI 代签 / 自行裁决**（AGENTS 硬规则 1）。实现 PR 终审、device 真跑（cubin 预编 + fatbin 装载 + PTX fallback 协商）/ 证据回填 / `evidence/fatbin_dist_smoke.json` / run URL 归档 / MR-0005 编号与 G1.4 parked 栈 reconcile / 2-PR 栈按序合并仍由 agent 自主签署。

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-22 | 初版 Mini-RFC（MR-0005，G1.5 生产分发 fatbin）：§1 摘要（按架构预编 cubin + 保守 PTX fallback 装载协商 + lockfile `[[artifact]]` digest + rurixup/Release 层覆盖）/ §2 设计（复用 RXS-0073 ptxas 干验证产 cubin·RXS-0076/0077 PTX 装载协商·RXS-0090/0093 content-tree·RXS-0135~0139 发布链路，新增仅补分发产物变体缺口）/ §3 判档 Mini（复用既有装载协商不触 FFI ABI/安全包络禁区，升档触发条件守卫）/ §4 零新 RX 码（降级而非 reject，确需则 RX7020 续接停手不预造）/ §5 失败测试先行（三类 host 红绿自检）/ §6 影响（0-byte、保守 fallback 兜底、cubin 不设字节 golden 改结构校验、RD-010 不预造）/ §7 unsafe 边界（U22）/ §8 agent 批准（四项裁定留痕）。agent 2026-06-22 经 AskUserQuestion 批准全文，claude-code 代录非 AI 代签 | Mini-RFC（MR-0005） |
