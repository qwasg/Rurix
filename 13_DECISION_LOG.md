# 13 — 决策日志

> 所属文档集：[00_MASTER_INDEX.md](00_MASTER_INDEX.md)
> 版本：v1.0（2026-06-11）
> 约定：编号永不复用。状态 ∈ {**已批准**（所有者拍板）、**已选定**（规划期技术决策，所有者可否决）、**待决**（未来所有者决策点）}。每条含一行理由摘要；完整论证在"出处"列指向的文档章节。
> 编号段：D-0xx 战略 / D-1xx 语言 / D-2xx 编译器与运行时 / D-3xx 标准库与生态 / D-4xx 治理。

---

## 1. 战略决策（所有者已拍板，2026-06-11）

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
| D-131 | G2 DXIL 生成路径（LLVM DirectX 后端 vs SPIR-V 转译） | **待决**（G2 启动时按当时后端成熟度评估，所有者批准） | [06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §8.2 |

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
| D-312 | registry 启动与形态（sumdb 透明日志方向） | **待决**（社区规模触发，所有者批准） | [09](09_STDLIB_AND_ECOSYSTEM.md) §7.3 |
| D-313 | NVIDIA 再分发白名单 CI 审计 | r6 合规工程化；防 AI 随手打包 | [09](09_STDLIB_AND_ECOSYSTEM.md) §8 |

## 6. 治理决策（D-4xx，已选定）

| 编号 | 决策 | 理由摘要 | 出处 |
|---|---|---|---|
| D-401 | 闭门期角色帽（质量角色机器化）→ 开源后三人组实体化 + FCP-lite | 单人自我放水风险的结构性对策 | [10](10_GOVERNANCE.md) §2 |
| D-402 | 三档变更门（Direct/Mini-RFC/Full RFC），争议上取严 | r7 骨架 | [10](10_GOVERNANCE.md) §3 |
| D-403 | spec/rfcs/conformance/ui/unsafe-audit/agents 一等公民目录；规范领导实现 | r7 + FLS 反向教训 | [10](10_GOVERNANCE.md) §4 |
| D-404 | feature gate → tracking → stabilization report 生命周期；edition 预留（span 带 edition 第一天实现） | Rust 机制裁剪；r1 的 span-edition 低成本预埋 | [10](10_GOVERNANCE.md) §5 |
| D-405 | SemVer + 0.x 期诊断 schema 早稳 + 6 周 train（开源后）+ 发布机器门 | 工具生态依赖 schema；质量角色机器化的发布形态 | [10](10_GOVERNANCE.md) §6 |
| D-406 | AI 贡献政策全集（provenance/验证强制/UB-内存模型-ABI 禁区/unsafe 审计） | H06 实证 + Linux/LLVM 先例 | [10](10_GOVERNANCE.md) §7 |
| D-407 | 抗混乱永久条款（表面积预算/编号永不复用/P-01、P-13 准永久） | 五年后的自己是假想敌 | [10](10_GOVERNANCE.md) §9 |

## 7. 待决清单（未来所有者决策点汇总）

| 编号 | 决策点 | 触发时机 | 默认建议 |
|---|---|---|---|
| D-005 | MVP 验收后的 G1 优先级与协作者引入 | MVP 验收 | 先 G1-1（interop 出图）后社区 |
| D-006 | 12 个月评审点：AI 集群有效性与节奏调整 | M+12 | 数据说话（里程碑燃尽 + 质量门统计） |
| D-007 | 开源执行细节（时点/仓库形态/公告策略） | MVP 验收前 1–2 月 | 验收即开源，附 conformance 与三 demo |
| D-131 | G2 DXIL 生成路径 | G2 启动 | 届时按 LLVM DirectX 后端成熟度评估 |
| D-312 | registry 启动 | 生态包 >50 或社区强需求 | sumdb 透明日志模型 |
| D-008 | 多后端红线解除（红线 3） | G2 完成后 | 维持红线直至 NVIDIA 纵深完成 |

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版：D-001~007、D-1xx~D-4xx 首批登记 |
