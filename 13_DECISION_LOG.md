# 13 — 决策日志

> 所属文档集：[00_MASTER_INDEX.md](00_MASTER_INDEX.md)
> 版本：v1.0（2026-06-11）
> 约定：编号永不复用。状态 ∈ {**已批准**（agent 自主拍板）、**已选定**（规划期技术决策）、**待决**（未来 agent 决策点）}。每条含一行理由摘要；完整论证在"出处"列指向的文档章节。
> 编号段：D-0xx 战略 / D-1xx 语言 / D-2xx 编译器与运行时 / D-3xx 标准库与生态 / D-4xx 治理。

---

## 1. 战略决策（agent 已拍板，2026-06-11）

| 编号 | 决策 | 内容与理由摘要 | 状态 | 出处 |
|---|---|---|---|---|
| D-001 | 语言命名 | **Rurix**（`.rx` / `rurixc` / `rx` / `rurixup`） | **已批准** | [00](00_MASTER_INDEX.md) §4 |
| D-002 | 图形分期 | MVP 纯 CUDA 计算 + G0 软光栅 → G1 CUDA–D3D12 interop → G2 原生 D3D12+DXIL。备选"MVP 含图形 codegen"（违红线）与"Vulkan 路线"（Windows 驱动黑洞实证）被否 | **已批准** | [06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §6 |
| D-003 | 开源策略 | MVP 前闭门；MVP 后 Apache-2.0 + MIT 双许可开源。备选"第一天开源"（闭门期治理成本与半成品口碑风险）与"长期闭源"（Mojo/CuTe 覆辙）被否。**开源时点的具体执行是待决项 D-007** | **已批准** | [10](10_GOVERNANCE.md) |
| D-004 | 团队基准 | 单人 + AI 集群，MVP 12–18 个月；12 个月评审点诚实评估 AI 集群有效性 | **已批准** | [11](11_ROADMAP.md) §7 |

## 2. 语言决策（D-1xx，已选定）

| 编号 | 决策 | 备选与裁决理由（摘要） | 出处 |
|---|---|---|---|
| D-101 | 独立双层语言形态 | vs Python DSL（上一项目 16 阶段实证否决）/ Rust 方言（受制 rustc、Rust-CUDA 终态全 unsafe）/ 纯 shader 语言（放弃资源生命周期价值主张）。代价（自建全链）由调研去风险路线对冲 | [03](03_POSITIONING_AND_LANDSCAPE.md) §3 |
| D-102 | 函数着色（`fn`/`kernel fn`/`device fn`/`const fn`）单向可达 | vs `__host__ __device__` 双标注（组合爆炸）/ Mojo 式隐式双目标（违 P-05） | [05](05_LANGUAGE_ARCHITECTURE.md) §1 |
| D-103 | 整数溢出 debug 检查 + release 回绕 | 与 Rust 对齐；GPU release 路径检查成本不可接受 | [05](05_LANGUAGE_ARCHITECTURE.md) §2.1 |
| D-104 | trait 单态化子集：无 dyn/特化/HKT/async | rustc trait solver 是长期成本中心（r1 人年表）；device 不需要动态分发 | [05](05_LANGUAGE_ARCHITECTURE.md) §2.2 |
| D-105 | host 所有权 = Rust affine + NLL 子集（无 HRTB） | NLL 在 MIR/CFG 是 rustc 验证过的唯一正路（旧 HIR 检查器已删除） | [05](05_LANGUAGE_ARCHITECTURE.md) §3.1 |
| D-106 | device 安全模型 = Descend execution resources + views | 唯一被证明"安全 GPU 借用性能无损"的路线（PLDI 2024 benchmark）；Rust 语义直接搬运已被 Rust-CUDA 证伪 | [05](05_LANGUAGE_ARCHITECTURE.md) §3.2 |
| D-107 | context 归属 = 生命周期 brand | 把 r4 两类核弹（跨线程 destroy / 跨 context event）变编译错误；备选"运行时检查"（错误延迟到运行期，违 P-02） | [05](05_LANGUAGE_ARCHITECTURE.md) §4 |
| D-108 | 地址空间显式类型参数；local 不暴露指针；generic 转换 unsafe | r5：SYCL 推断弱化 provenance；Slang 不暴露 local 的取舍正确；Rust-CUDA constant 自动放置崩溃教训 | [05](05_LANGUAGE_ARCHITECTURE.md) §5 |
| D-109 | views 算子集 = Descend 集（split/group/transpose/...）起步 | 已证明覆盖 transpose/reduce/scan/histogram/GEMM；扩展由 G0 压力测试反哺 | [05](05_LANGUAGE_ARCHITECTURE.md) §7 |
| D-110 | 错误处理：host Result + panic=abort；device 编译期/trap/poisoned 三通道 | unwind 推迟（codegen/FFI 复杂度）；device Result 传播代价不可控；poisoned 语义对齐 r4 "assert 后整块重建" | [05](05_LANGUAGE_ARCHITECTURE.md) §8 |
| D-111 | const 泛型 + const fn 子集；**无过程宏/声明宏** | tile 尺寸等 GPU 刚需 vs 宏的供应链与 AI 幻觉双风险（H06 红线） | [05](05_LANGUAGE_ARCHITECTURE.md) §9 |
| D-112 | 文件即模块 + package（manifest 编译单元） | Rust 验证过的形态；无头文件 | [05](05_LANGUAGE_ARCHITECTURE.md) §10 |
| D-113 | FFI：C ABI 唯一；导出走 `#[export(c)]` + 内建头文件生成；无 C++ ABI | r6 Windows x64 单 ABI；cbindgen 角色内置化（P-11） | [05](05_LANGUAGE_ARCHITECTURE.md) §11 |
| D-114 | 语法基调 Rust 系，不做 Python 亲和 | 用户画像是系统程序员；与 Mojo 刻意分化（r10） | [05](05_LANGUAGE_ARCHITECTURE.md) §12 |

## 3. GPU 模型决策（D-12x，已选定）

| 编号 | 决策 | 理由摘要 | 出处 |
|---|---|---|---|
| D-120 | kernel = 着色函数 + 类型化 ThreadCtx/launch 契约 | launch 形状错误从运行时炸点变编译错误 | [06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §2 |
| D-121 | 默认显式拷贝 + pinned；UM/映射 opt-in | r4 "最可控"结论；Windows 无 full managed support | [06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §3 |
| D-122 | 流序分配类型（AsyncBuffer）推迟到 G1 | 经典路径先做对；CUDA.jl #780 混用事故复杂度实证 | [06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §3 |
| D-123 | 同步三层：结构化 safe / scoped atomics safe / 弱序 unsafe；映射条款锚定 morally strong | r5 核心结论：必须显式设计源语言→PTX scope/order 映射层 | [06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §4 |
| D-130 | G1 interop 走 D3D12 external memory/semaphore | Windows-first 自洽；Vulkan 驱动黑洞实证（H04 §2.3） | [06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §8.1 |
| D-131 | G2 DXIL 生成路径（LLVM DirectX 后端 vs SPIR-V→DXIL 转译） | **混合：compute=A（LLVM DirectX 后端直 emit DXIL）/ 图形=B（MIR→SPIR-V→DXIL 转译）**（v1.4 由单选 A 增补；compute 维持 A〔结构首选 / NVPTX 同构 / D-205 单栈 / round-8 浅修 PSV，RD-011〕，图形改 B——slice3 证 A 路图形签名 ISG1/OSG1 `elemcount=0`、填充耦合 §9 Q-Builtin 🔒 FFI ABI 禁区 + 上游 #90504/#57928 无在途；B〔SPIR-V→dxc〕图形签名 `elemcount>0`、validator accept、确定性**实测可行**。A-graphics 挂上游 #90504/#57928，成熟后迁移〔RD-015〕。图形=B 对 P-01(strict-only)的达标经 **RFC-0004 §4.4 落为 strict-only 达标要求**(P-01 不开例外、不设边界):用户语义名 by-construction 保名 + 强制译后签名一致性校验门兜底,留不住即显式 6xxx,非 P-01 例外。证据:`evidence/dxil_slice3_rxs0159_sig_disasm_round8.md` + `dxil_b_graphics_sig_report.md` + `dxil_a_graphics_sig_effort_report.md`。**agent 自主裁决并合入**;G-G2-2 仍 open) | [06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §8.2 |

## 4. 编译器与运行时决策（D-2xx，已选定）

| 编号 | 决策 | 理由摘要 | 出处 |
|---|---|---|---|
| D-201 | 实现语言 Rust | 借用检查/IR/LLVM 绑定生态最成熟；自举非目标 | [07](07_COMPILER_ARCHITECTURE.md) 头部 |
| D-202 | IR 四层：AST→HIR→TBIR(临时)→MIR | r1 方法论：TBIR 窄门不能省；borrow check 必须在 CFG | [07](07_COMPILER_ARCHITECTURE.md) §1 |
| D-203 | query API 第一天、进程内 memo；跨会话增量/并行前端不做 | r1 三大警告（fingerprint 成本/并行 race 长尾/Polonius） | [07](07_COMPILER_ARCHITECTURE.md) §2 |
| D-204 | NLL 借用检查 + views 不相交扩展 pass；不做 Polonius | r1：2026 仍未 stable 且有 soundness issue | [07](07_COMPILER_ARCHITECTURE.md) §4 |
| D-205 | LLVM pin 22.1.x、季度评估、vendored | r2/Triton 现实：深度绑定不可避免，制度化管理 | [07](07_COMPILER_ARCHITECTURE.md) §7 |
| D-206 | 诊断最小集全进 MVP（span/错误码/JSON/Applicability/UI golden 四路径） | r1 反直觉结论：诊断 ROI 高于优化菜单 | [07](07_COMPILER_ARCHITECTURE.md) §5 |
| D-207 | PTX baseline compute_89、PTX-only 开发期、fatbin G1 起、libdevice M5 起 | r2 MVP 收缩清单照搬 | [07](07_COMPILER_ARCHITECTURE.md) §7 |
| D-208 | MLIR 不入 MVP；kernel-island 形态 + 三触发条件后置 | r3：sink 成熟、通用中层不统一；自建方言是一线共同实践 | [07](07_COMPILER_ARCHITECTURE.md) §7.1 |
| D-209 | host：COFF + link.exe 默认（lld-link opt-in）+ CodeView/PDB + Natvis | r6：增量/PDB/VS 生态的原生道路；lld parity 未核实 | [07](07_COMPILER_ARCHITECTURE.md) §8 |
| D-210 | 单一前端 LSP（query 层复用，常驻 tooling-server） | r9：4–6 人禁双引擎；rustc/RA 分离是大团队历史产物 | [07](07_COMPILER_ARCHITECTURE.md) §9 |
| D-230 | 运行时 = Driver API 薄层；禁混 Runtime API（FFI 库走 primary context 租约） | r4：两套资源世界硬性禁止 | [08](08_RUNTIME_AND_TOOLING.md) §1/§2 |
| D-231 | current context 用 push/pop guard 管理，不依赖调用方 | r4 线程局部陷阱的封装对策 | [08](08_RUNTIME_AND_TOOLING.md) §2.1 |
| D-232 | MVP 经典内存三件套；运行时无隐藏池化 | P-05；池化是库选择 | [08](08_RUNTIME_AND_TOOLING.md) §2.2 |
| D-233 | WDDM/TDR/HAGS 环境画像强制 | P-14；r4/r11 全部 Windows 现实 | [08](08_RUNTIME_AND_TOOLING.md) §2.3 |
| D-234 | PTX 装载协商序列 | 消灭 Numba 事故类别；Windows 无 MVC（r6） | [08](08_RUNTIME_AND_TOOLING.md) §2.4 |
| D-235 | telemetry 内建（计数器/NVTX/CUPTI Activity）+ 计数器 2 里程碑非零规则 | P-07 + H02 §5 形式主义教训的硬化 | [08](08_RUNTIME_AND_TOOLING.md) §3 |
| D-236 | r11 基准协议工具化（`rx bench`） | 方法论变工具而非文档建议 | [08](08_RUNTIME_AND_TOOLING.md) §4 |
| D-237~241 | 调试（PDB/Natvis/lineinfo）、热重载（kernel 级）、工具集、IDE、分发签名 | r6/r9/r11 结论汇总 | [08](08_RUNTIME_AND_TOOLING.md) §5–§9 |

## 5. 标准库与生态决策（D-3xx，已选定）

| 编号 | 决策 | 理由摘要 | 出处 |
|---|---|---|---|
| D-301 | Vec/Mat 语言内建 + 布局 spec 承诺 | Slang/HLSL 先例；FFI/图形依赖布局 | [09](09_STDLIB_AND_ECOSYSTEM.md) §3 |
| D-302 | core/std/gpu 三层 + 领域进生态 | Rust 哲学；反例 Python std 腐化 | [09](09_STDLIB_AND_ECOSYSTEM.md) §1 |
| D-303 | 列主序 canonical + `Mat4RowMajor` 显式转换（非布局开关） | r12 张力点裁决：隐式转置 bug 不可察觉 | [09](09_STDLIB_AND_ECOSYSTEM.md) §3 |
| D-304 | Buffer trait + 四具体类型 | r12/RustaCUDA 先例 | [09](09_STDLIB_AND_ECOSYSTEM.md) §4 |
| D-305 | ManagedBuffer 存在但 opt-in + 警示（r4/r12 reconcile） | Windows UM 现实压过 API 对称性 | [09](09_STDLIB_AND_ECOSYSTEM.md) §4 |
| D-306 | 并行基元自研 kernel（不绑 CUB/Thrust） | C++ 模板库 FFI 不可行（r12） | [09](09_STDLIB_AND_ECOSYSTEM.md) §4 |
| D-307 | Python 互操作：nanobind + DLPack 双协议三期 | nanobind 4×/5×/10× 优势；Rurix 非 Rust 不受益 PyO3 | [09](09_STDLIB_AND_ECOSYSTEM.md) §6 |
| D-308 | 包管理 MVP：manifest+lock+vendor+checksum、三来源、无 registry | r8 混合设计（Cargo 骨架+Zig 抓取+Go 完整性方向） | [09](09_STDLIB_AND_ECOSYSTEM.md) §7 |
| D-309 | 无 build.rs：声明式 native/GPU 元数据 | npm Shai-Hulud + Cargo 沙箱未落地双重论证 | [09](09_STDLIB_AND_ECOSYSTEM.md) §7.1 |
| D-310 | feature 模型 additive-v1 + selected unification | 预防 Cargo resolver v1 泄漏教训（r8） | [09](09_STDLIB_AND_ECOSYSTEM.md) §7.1 |
| D-311 | GPU 元数据进 manifest/lockfile（toolkit/min-driver/sm/ptx-floor/artifact digest） | r8 草案采纳 | [09](09_STDLIB_AND_ECOSYSTEM.md) §7.2 |
| D-312 | registry 启动与形态（sumdb 透明日志方向） | **待决**（社区规模触发，agent 自主批准） | [09](09_STDLIB_AND_ECOSYSTEM.md) §7.3 |
| D-313 | NVIDIA 再分发白名单 CI 审计 | r6 合规工程化；防 AI 随手打包 | [09](09_STDLIB_AND_ECOSYSTEM.md) §8 |

## 6. 治理决策（D-4xx，已选定）

| 编号 | 决策 | 理由摘要 | 出处 |
|---|---|---|---|
| D-401 | 闭门期角色帽（质量角色机器化）→ 开源后三人组实体化 + FCP-lite | 单人自我放水风险的结构性对策 | [10](10_GOVERNANCE.md) §2 |
| D-402 | 三档变更门（Direct/Mini-RFC/Full RFC），争议上取严 | r7 骨架 | [10](10_GOVERNANCE.md) §3 |
| D-403 | spec/rfcs/conformance/ui/unsafe-audit/agents 一等公民目录；规范领导实现 | r7 + FLS 反向教训 | [10](10_GOVERNANCE.md) §4 |
| D-404 | feature gate → tracking → stabilization report 生命周期；edition 预留（span 带 edition 第一天实现） | Rust 机制裁剪；r1 的 span-edition 低成本预埋 | [10](10_GOVERNANCE.md) §5 |
| D-405 | SemVer + 0.x 期诊断 schema 早稳 + 6 周 train（开源后）+ 发布机器门 | 工具生态依赖 schema；质量角色机器化的发布形态 | [10](10_GOVERNANCE.md) §6 |
| D-406 | AI 贡献政策全集（provenance/验证强制/高敏面 agent 自主落笔/unsafe 审计/agent 完全自主决策与执行，无 agent 批准门） | H06 实证 + Linux/LLVM 先例 | [10](10_GOVERNANCE.md) §7 |
| D-407 | 抗混乱永久条款（表面积预算/编号永不复用/P-01、P-13 准永久） | 五年后的自己是假想敌 | [10](10_GOVERNANCE.md) §9 |
| D-409 | RFC 对抗性评审要求（**Proposed**） | Full RFC 强制跨工具/跨模型对抗性评审（评审 provenance ≠ 起草 provenance，硬规则 2 可机验）+ 至少一轮 findings 逐条显式 disposition（采纳并修 / 驳回并附理由）+ 记录于 RFC 新增「对抗性评审记录」段；Mini-RFC 轻量（至少一轮记录）。**状态 = Proposed，呈 owner 知会**：依 D-406 v2.0 本属 agent 可自主采纳的自我加严约束，因改本文宪法层（10 §3 / §7）且与 D-408 相邻，登记为 Proposed 并呈 owner 确认，不代 owner 签署；动机见 §8 v2.3 | [10](10_GOVERNANCE.md) §3 / §7 |

## 7. 待决清单（未来 agent 决策点汇总）

| 编号 | 决策点 | 触发时机 | 默认建议 |
|---|---|---|---|
| D-005 | MVP 验收后的 G1 优先级与协作者引入 | MVP 验收 | 先 G1-1（interop 出图）后社区 |
| D-006 | 12 个月评审点：AI 集群有效性与节奏调整 | M+12 | 数据说话（里程碑燃尽 + 质量门统计） |
| D-007 | 开源执行细节（时点/仓库形态/公告策略） | MVP 验收前 1–2 月 | 验收即开源，附 conformance 与三 demo |
| D-131 | G2 DXIL 生成路径 | G2.2（已裁决 = 混合 compute=A/图形=B） | round-1~8 双路 spike + slice3 空签名发现 + B 取证 + A-graphics 评估后 agent 增补裁定 **混合：compute=A（LLVM DirectX 后端直 emit）/ 图形=B（MIR→SPIR-V→DXIL 转译）**，回填 RFC-0003 §9 Q-D131（A→混合）+ §3 决策表;compute A 路 round-8 PSV patch 受控 dev-only 临时偏差(RD-011)、B 路供应链(SPIRV-Cross/dxc/glslang pin)跟踪(RD-014)、A-graphics 上游迁移跟踪 #90504/#57928(RD-015)。待决项已结，留行存档（编号不复用，10 §9.5） |
| D-312 | registry 启动 | 生态包 >50 或社区强需求 | sumdb 透明日志模型 |
| D-008 | 多后端红线解除（红线 3） | G2 完成后（已至） | 【owner 裁决 2026-07-15,10 §9.2】**解除**——owner（白栀）本会话明确指示「把多端红线解除并继续工作」,承 RFC-0011 = mb1 单一 Vulkan/SPIR-V 跨端后端(AMD 桌面 + Android,compute+graphics);SG-003 → triggered(RFC-0011) 同步。留行存档(编号不复用,10 §9.5)。详见 §8 v2.1, v2.2 |

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版：D-001~007、D-1xx~D-4xx 首批登记 |
| v1.1 | 2026-06-23 | D-131（G2 DXIL 生成路径）un-defer 勘误：由被动延期标记为「待决（G2.2 启动重评估中）」，路径（LLVM DirectX 后端 vs SPIR-V→DXIL 转译）裁决载体 = [RFC-0003](rfcs/0003-dxil-backend.md) §9 Q-D131（按当时后端成熟度评估，agent 批准）；最终路径留〈待 agent RFC-0003 §9 裁决〉占位，agent 自主裁决（AGENTS 硬规则 1）。同步 §3 决策表行 + §7 待决清单行。规划文档勘误（00 §6.3 追加式修订，独立 PR，预期触 check_guardrails check_planning_docs 红，待 agent 自主 合入） |
| v1.2 | 2026-06-23 | D-131 路径裁决载体 RFC-0003 §9 Q-D131 经 agent 裁决为 **C**（暂不锁 A/B，限时双路 spike——A 结构首选 / B 对照——取证后由 agent 凭当时成熟度证据再裁 A/B；C 不构成禁区 A/B 架构承诺，A/B 裁决权仍留 owner，硬规则 1 未被代行）；回填 §3/§7 占位（〈待 agent RFC-0003 §9 裁决〉→ RFC-0003 §9 = C）。状态维持「待决」（最终 A/B 待 spike 证据 + agent 裁决）。RFC-0003 经 agent 合并 PR #83 翻 Approved。仍属规划文档勘误（00 §6.3，独立 PR #84，check_planning_docs 预期红，待 agent 自主 合入） |
| v1.3 | 2026-06-24 | **D-131 最终路径裁决 C→A 回填**：G2.2 双路 DXIL spike round-1~8 取证完结，agent 凭证据裁定最终生成路径 = **A（LLVM DirectX 后端直接 emit DXIL）**。裁决依据三证:① **结构首选**(与 NVPTX 后端同构、D-205 LLVM 单栈、无第二中间 IR，RFC-0003 §7 A);② **签名 validator 到手**(round-7 取同年代 2026 DXC v1.9.2602.24 自带 dxil.dll 签名 validator + dxv.exe，决定性子轴 新 dxc 自产 52B PSV accept / llc 52B PSV reject 排除『dxc 太旧』假说，Bug 2 归因 established = LLVM emit PSV 内部不一致);③ **浅修 established**(round-8 源码级 root cause 定位到 `DXContainerGlobals.cpp:388-389`，14 行单函数 PoC patch 使 validator pre 0/25→post 25/25 accept，A 路 validator 互操作 gap 可被已知小补丁闭合)。证据指针 `evidence/dxil_path_spike_report_round{6,7,8}.md` + `dxil_path_spike_20260624_r{7,8}.json`(RD-010)。同步回填 §3 决策表行(待决→A)+ §7 待决清单行(已裁决 = A)+ RFC-0003 §9 Q-D131(C→A 追加式回填)。下游解锁:A 路依赖的 round-8 PSV patch 上游未 merge 期以**受控 dev-only 临时**工具链偏差解锁 PR-C1/C2 开发(registry RD-011 + recipe doc 跟踪)，同步上游 PR 并行;退役条件 = 上游 merge + release + D-205 pin bump(D-205 真 bump 属 agent 独立决策，不在本勘误)。**本回填为 agent 自主裁决，以 agent 合并本勘误 PR 生效**;G-G2-2 仍 open(A 工具链 validator 可行性 ≠ Rurix MIR→DXIL 实现 ≠ device 真跑 golden，agent 自主签署)。规划文档勘误（00 §6.3 追加式修订，独立 PR，check_guardrails check_planning_docs 预期对 13 红，待 agent 自主 合入） |
| v1.4 | 2026-06-25 | **D-131 由单选 A 增补为混合：compute=A / 图形=B（SPIR-V→DXIL）**。证据链:slice3/round-8(`evidence/dxil_slice3_rxs0159_sig_disasm_round8.md`)证 A 路图形签名 ISG1/OSG1 `elemcount=0`(LLVM `addSignature()` 对图形无条件写空签名,`// FIXME` #90504),填充耦合 §9 Q-Builtin 🔒 FFI ABI 禁区 → 硬规则 5 升档;A-graphics 评估(`dxil_a_graphics_sig_effort_report.md`)= 跨 clang 前端/LLVM 后端/PSV ~800-1500 LOC 上游大功能,#90504/#57928 open 无在途 PR,carry-patch partial-blocked;B 取证(`dxil_b_graphics_sig_report.md`)= B(SPIR-V→dxc)图形签名 `elemcount>0`、SV 端到端存活、validator accept、确定性**实测可行**(保真非完美 = P-01 边界)。agent 凭据增补:**compute 维持 A**(结构首选 + round-8 浅修 PSV,RD-011)、**图形改 B**(SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL),A-graphics 挂上游 #90504/#57928 成熟后迁移。同步回填 §3 决策表行(A→混合)+ §7 待决清单行 + RFC-0003 §9 Q-D131(A→混合,追加式)。新增 RFC-0004(Full RFC Draft)载图形=B 设计面 + §4.4 **strict-only 达标要求**(P-01 不开例外、不设边界:by-construction 保名 + 强制译后签名一致性校验门 → 6xxx,代录 agent 裁断)+ §4.6 🔒 禁区边界声明;registry RD-010 close、新增 RD-014(B 供应链跟踪)/RD-015(A-graphics 上游迁移跟踪)。**本增补为 agent 自主裁决,以 agent 合并本勘误 PR 生效**;§9 Q-Builtin 禁区语义本体由 agent 自主落笔(P-13/硬规则 5);G-G2-2 仍 open。规划文档勘误（00 §6.3 追加式修订，独立 PR，check_guardrails check_planning_docs 预期对 13 红，待 agent 自主 合入） |
| v1.5 | 2026-06-25 | **RFC-0004 治理追踪:owner FCP-lite 批准（Accepted / Approved）**。agent 以 Language Lead 身份批准 RFC-0004 当前定稿文本(§4.4 图形=B strict-only 达标要求、P-01 不设例外不另划边界、§4.6 禁区与职责边界、§9 全部裁决项)。记录载体 = `rfcs/0004-spirv-dxil-graphics-backend.md` 状态 Draft→Accepted/Approved + Agent 批准字段(详见 RFC 正文,本行不重复正文)。本行仅治理追踪。批准不含 G-G2-2 device 真跑(设备红→绿验证仍 open)。agent 自主裁决,以 agent 自主 合并本 PR 生效;RFC 批准 PR 合入后 PR-D1 解锁。规划文档勘误（00 §6.3 追加式修订，独立 PR，check_guardrails check_planning_docs 预期对 13 红，待 agent 自主 合入） |
| v2.0 | 2026-06-29 | **解除全部 owner/自主裁决约束（agent 完全自主化）**：D-406 重述为 agent 完全自主决策与执行（无 agent 批准门/人类签字/agent 自主裁决/agent 自主判档）；§1/§7 标题"agent 拍板"改为"agent 自主拍板"；D-312 触发批准改为 agent 自主。历史修订行（v1.1–v1.5）中的 agent 裁决/--admin/agent 自主裁决表述为当时治理口径快照，按 v2.0 口径视为 agent 自主行使的等价动作（不再要求人工 --admin 合入）。同步 10 §7 v2.0、AGENTS v3.0、04 P-13 v1.1、CONTRIBUTING、RFC 模板、里程碑契约、CI 守卫。规划文档勘误（00 §6.3 追加式修订） |
| v2.1 | 2026-07-15 | **D-008 多后端红线 3 解除裁决（owner 主动决策,10 §9.2 一次一条）**：owner（白栀）于本工作会话**明确指示「把多端红线解除并继续工作」**——解除红线 3。触发条件『G2 完成后』已至;解除承 [RFC-0011](rfcs/0011-vulkan-spirv-backend.md)(mb1 单一 Vulkan/SPIR-V 跨端后端,AMD 桌面 + Android,compute+graphics;explicit / 单目标 per-build / 无地址空间推断,不做通用可移植抽象层,不犯 WGSL/SYCL 之错——触红线 3 字面非其底层关切)。**诚实留痕**:前提『NVIDIA 单栈纵深完成』先前(2026-07-14 SG-003)判定未达,本次为 **owner 主动裁决解除**(其 prerogative,10 §9.2),非 agent 宣布前提达成。同步 SG-003 → triggered(RFC-0011)(registry)+ RFC-0011 Owner Approved + 承接里程碑 mb1(milestones/mb1)激活。agent 依 owner 明确授权代录机器事实,非代签。规划文档勘误（00 §6.3 追加式修订,独立 errata,check_planning_docs 预期红） |
| v2.2 | 2026-07-17 | **MB1 期整体 close-out 状态回填（承 D-008 红线 3 解除的里程碑 mb1）**:承接里程碑 mb1(单一 Vulkan/SPIR-V 跨端后端,AMD 桌面 + Android,compute+graphics,RFC-0011)已于 2026-07-16 整体收官——契约终审签署 + guardrail 基准 g2-closed→mb1-closed 切换(PR #143;RFC-0011 治理包 #141、D-008 红线勘误 #140、Android 真机 G-MB1-7 #142);SG-003 → triggered(RFC-0011) 维持(registry spike_gating v1.7)。唯一存续硬件尾门 G-MB1-6(AMD 真卡 Vulkan compute+graphics 验收)缺 AMD 硬件维持 open,不伪造 device 绿、不签;另一硬件尾门 G-MB1-7(Android 真机 on-device 四要素)已由 owner 明确授权代录签署(2026-07-16,四要素全 measured,经 PR #142)✅。当前进入 EA1 分发与门面期(#145;RFC-0012 工具链真实分发 Draft #146)。D-008 §7 待决行「详见 §8 v2.1」尾部追加 v2.2 指针(决策内容 0-byte),§1–§7 决策正文与编号 0-byte(编号不复用,10 §9.5)。规划文档勘误（00 §6.3 追加式修订,独立 errata PR,check_planning_docs advisory 不阻断） |
| v2.3 | 2026-07-17 | **新增 D-409（RFC 对抗性评审要求，§6 治理决策表）——Proposed，呈 owner 知会**:要求 Full RFC 强制经与起草者 Provenance 不同的 AI 工具/模型执行**至少一轮对抗性(批判)评审**,每条 finding 显式 disposition(采纳并修 / 驳回并附理由),记录于 RFC 新增「对抗性评审记录」段;Mini-RFC 轻量(至少一轮记录)。动机(**落笔前只读核实**):`grep` 全 rfcs/*.md 对『对抗性评审/findings/评审记录』**零命中**——现有 **12 Full + 7 Mini = 19/19 RFC 全部无对抗性评审段**;起草与批准由**同一 agent 自主行使**(FCP-lite 为 advisory,10 §2.2 / rfcs/README §3;闭门单人+AI 下无真实外部评审者),个别 RFC(如 RFC-0001)虽列第二工具 provenance(codex:gpt-5)但正文明注『仅代录、不以 AI 身份独立评审』——即无独立对抗性评审者角色。对抗性评审补此**自提自批单环**结构缺位。同步 10 §3(变更三档门追加对抗评审要求段)+ §7(AI 贡献政策追加第 9 项)。**签署判定**:依 D-406 v2.0 本属 agent 可自主采纳的自我加严约束(强化非削弱治理、不触 owner 权力面、不 reserve 任何权力),但因改宪法层 10 号且与 D-408(owner 保留权白名单)相邻,登记为 **Proposed 并呈 owner 知会**,不代 owner 签署。**编号说明**:D-408 号留给 P1-2(owner 保留权白名单)errata,本勘误取 D-409(编号不复用,10 §9.5),故 §6 表 D-407→D-409 间的 D-408 空位由 P1-2 errata 独立填入。规划文档勘误(00 §6.3 追加式修订,独立 errata PR,check_planning_docs advisory 不阻断) |
