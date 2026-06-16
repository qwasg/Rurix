# spec/ — Rurix 语言规范

> 地位:进入实现期后的**唯一语义事实源**(10 §4,D-403);规范领导实现——先写条款再写实现,缺条款的语义 PR 必须先补 spec。
> 本 README 规定条款体例与变更纪律,自身变更同样适用 §3 档位标记。

---

## 1. 条款编号

- 编号 `RXS-####`(四位,分配制递增),**永不复用**(10 §9.5);弃用条款标注 deprecated 并保留原文,不删除。
- 编号在全 spec 范围内唯一,不按文件分段——文件只是组织载体,条款号是稳定锚点。
- 已分配区间登记于 §4 文件清单。

## 2. 条款格式(FLS 风格,10 §4)

每条款一个三级标题:`### RXS-#### 标题`,按需分节:

| 节 | 内容 |
|---|---|
| Syntax | 文法产生式(EBNF 式记法)或词法形式定义 |
| Legality | 静态合法性规则;违例引用错误码 `RX####`([../registry/error_codes.json](../registry/error_codes.json)) |
| Dynamic Semantics | 运行期语义(词法/纯语法条款不适用) |
| UB | 未定义行为边界(**仅人类经 Full RFC 落笔**,10 §7.5 禁区) |
| Implementation Requirements | 对实现的强制要求(错误恢复/诊断质量/平台行为) |

**测试锚定**(10 §4):每条款 ≥1 测试,测试侧以 `//@ spec: RXS-####` 注释(conformance 样例)或单测注释引用条款号;traceability 矩阵由工具生成(M1.4 起)。

## 3. 变更纪律与档位标记

- spec 文件变更必须走对应档位(10 §3):初版条款化已选定决策(D-1xx 等)为 **Direct**;规范内 bug fix 为 **Mini-RFC**;新语法/语义变更为 **Full RFC**(RFC 合入后才可改 spec)。判档争议向上取严;AI 无权自行判档为 Direct。
- **机器核对**(M1 CI_GATES §4 第 7 项):每个 spec 文件末尾维护修订记录表,含"档位"列;任何对 spec 文件的变更必须**新增修订行**(既有行 0-byte),否则 guardrail FAIL。
- 错误码语义可加不可改(10 §6);条款引用的错误码一经发布含义冻结。

## 4. 文件清单与编号区间

| 文件 | 内容 | 已分配条款 | 起始里程碑 |
|---|---|---|---|
| [lexical.md](lexical.md) | 词法结构 | RXS-0001 ~ RXS-0010 | M1.2 |
| [syntax.md](syntax.md) | 语法结构 | RXS-0011 ~ RXS-0031 | M1.3 |
| [names.md](names.md) | 名称与模块语义 | RXS-0032 ~ RXS-0038 | M2.1 |
| [types.md](types.md) | 类型与检查语义 | RXS-0039 ~ RXS-0047 | M2.2 |
| [borrow.md](borrow.md) | 所有权与借用语义(desugar/穷尽性/drop scope 首批;move/init/Drop 执行语义;借用/生命周期主体) | RXS-0048 ~ RXS-0061 | M3.1 |
| [consteval.md](consteval.md) | const 求值语义(const fn 子集 / const item 求值 / const 泛型 / 求值失败) | RXS-0062 ~ RXS-0065 | M3.4 |
| [device.md](device.md) | device 语义(函数着色与跨着色调用 / 地址空间类型与一致性 / barrier uniform 可达性保守骨架 / 着色与地址空间诊断要求 / NVPTX codegen 目标与调用约定 / 地址空间 codegen 建模 / 线程索引与 launch bounds / ptxas 干验证关卡 / launch 类型契约与诊断要求 / PTX 装载协商 / poisoned context 状态机 / views 算子集语义与子 view 不相交证明 / shared+barrier 一致性数据流 / scoped atomics 类型契约与 PTX 映射 / device 数学函数 intrinsic 集与求值语义 / libdevice bitcode 链接流程与 codegen 诊断) | RXS-0066 ~ RXS-0082 | M4.1 |
| [toolchain.md](toolchain.md) | 工具链语义(rx CLI 子命令语义面与退出码约定 / 包管理 rurix.toml·rurix.lock 格式与依赖三来源解析 / rx test 子进程隔离 / workspace 多包 / 离线重建复现门 / LSP 能力面契约) | RXS-0083 ~ RXS-0103（rx CLI 首批 + 包管理 manifest/lock/vendor + rx test/workspace/repro + LSP MVP M6.4） | M6.1 |
| [stdlib.md](stdlib.md) | 标准库语义(core 数学库类型面:Vec `VecN<T>` N∈{2,3,4} / Mat `MatRxC<T>` / swizzle / 几何原语 Point·Vector·Normal·AABB·Ray 的构造·分量访问与 swizzle·逐元素算术·点积/叉积/范数·矩阵乘·几何谓词;全 safe、host+device 双路径同义) | RXS-0104 ~ RXS-0113（M7.1 Vec/Mat/swizzle 类型面 RXS-0104~0109:Vec 构造·分量访问与 swizzle·逐元素算术·点积/叉积/范数 + Mat 构造·逐元素算术与矩阵乘;几何原语 / 谓词 RXS-0110~0113:Point3·Vector3·Normal3 语义区分与互转 / AABB·Ray 类型与构造 / Point∈AABB 包含与点到 AABB 距离 / Ray–AABB 相交） | M7.1 |
| [imageio.md](imageio.md) | image-io 接口语义(host-only 确定性图像序列输出:图像缓冲与像素类型面 Rgb·Rgba·ImageBuffer / 无损格式优先与格式选择 PPM P6 优先·PNG 次 / 确定性字节布局与 header 规范化 / 图像序列落盘接口逐帧 content SHA-256 可核对;全 safe) | RXS-0114 ~ RXS-0117（M7.2 image-io 接口语义面:RXS-0114 图像缓冲与像素类型面 / RXS-0115 无损格式优先与格式选择 / RXS-0116 确定性字节布局与 header 规范化 / RXS-0117 图像序列落盘接口） | M7.2 |
| [softraster.md](softraster.md) | 软光栅 kernel 语义面(G0 compute 软光栅:图元分桶到 tile binning / tile 光栅覆盖判定·重心坐标·边函数 / 深度 z-buffer 写入与深度测试 / tonemap HDR→LDR 像素量化;全 safe 代码目标·device 路径 NVPTX 标量子集·确定性帧像素,对接 M7.1 数学库与 M7.2 image-io 像素口径) | RXS-0118 ~ RXS-0121（M7.3 G0 软光栅 kernel 语义面:RXS-0118 图元分桶到 tile binning / RXS-0119 tile 光栅覆盖判定·重心坐标·边函数 / RXS-0120 深度 z-buffer 写入与深度测试 / RXS-0121 tonemap HDR→LDR 像素量化） | M7.3 |
| [interop.md](interop.md) | 互操作语义面(`rx build --emit=pyd` PYD 产出约定 / `__cuda_array_interface__` v3 + DLPack 双协议零拷贝接入 PyTorch/CuPy / C ABI 边界;C ABI·PYD 唯一通道·永不 Python 原生嵌入·PTX-only·affine 所有权不设 UB) | RXS-0122 ~ RXS-0125（M8.1：RXS-0122 `rx build --emit=pyd` PYD 产出约定 / RXS-0123 `__cuda_array_interface__` v3 零拷贝消费 / RXS-0124 DLPack 双协议零拷贝与设备指针所有权 / RXS-0125 C ABI 边界，每条 ≥1 `src/rurix-interop` 单测锚定） | M8.1 |
## 5. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-11 | 占位 README 升级为正式体例(编号规则/条款格式/档位标记约定);依据 10 §3 §4 既定治理决策,无新语义 | Direct |
| v1.1 | 2026-06-11 | §4 文件清单追加 syntax.md(RXS-0011 ~ RXS-0031,M1.3;D-M1-3 条款化登记,无体例变更) | Direct |
| v1.2 | 2026-06-11 | §4 文件清单追加 names.md(RXS-0032 ~ RXS-0038,M2.1 names 条款化登记,无体例变更) | Direct |
| v1.3 | 2026-06-12 | §4 文件清单追加 types.md(RXS-0039 ~ RXS-0047,M2.2 types 条款化登记,无体例变更) | Direct |
| v1.4 | 2026-06-12 | §4 文件清单追加 borrow.md(RXS-0048 ~ RXS-0052,M3.1 desugar/穷尽性/drop scope 首批条款化登记,无体例变更) | Direct |
| v1.5 | 2026-06-13 | §4 borrow.md 行区间更新至 RXS-0056(M3.2 move/init/Drop 执行语义条款追加登记,无体例变更) | Direct |
| v1.6 | 2026-06-13 | §4 borrow.md 行区间更新至 RXS-0061(M3.3 借用/生命周期主体条款追加登记,无体例变更) | Direct |
| v1.7 | 2026-06-13 | §4 文件清单追加 consteval.md(RXS-0062 ~ RXS-0065,M3.4 const eval 首批条款化登记,无体例变更) | Direct |
| v1.8 | 2026-06-13 | §4 文件清单追加 device.md(RXS-0066 ~ RXS-0069,M4.1 device 着色/地址空间首批条款化登记;codegen/launch 条款随 M4.2/M4.3 续写本文件,无体例变更) | Direct |
| v1.9 | 2026-06-13 | §4 device.md 行区间更新至 RXS-0073(M4.2 NVPTX codegen 目标与调用约定 / 地址空间 codegen 建模 / 线程索引与 launch bounds / ptxas 干验证关卡条款追加登记;launch 类型契约条款随 M4.3 续写,无体例变更) | Direct |
| v1.10 | 2026-06-13 | §4 device.md 行区间更新至 RXS-0075(M4.3 launch 类型契约与诊断要求条款追加登记;运行时对象/装载协商/poisoned 状态机随 rurix-rt 实现 PR,无体例变更) | Direct |
| v1.11 | 2026-06-13 | §4 device.md 行区间更新至 RXS-0077(M4.3 运行时 PTX 装载协商 / poisoned context 状态机条款追加登记;rurix-rt 运行时落地,无体例变更) | Direct |
| v1.12 | 2026-06-14 | §4 device.md 行区间更新至 RXS-0078(M5.1 views 算子集语义与子 view 不相交证明条款追加登记;MIR 借用检查 device 扩展,条款 PR 先于实现 PR,无体例变更) | Direct |
| v1.13 | 2026-06-14 | §4 device.md 行区间更新至 RXS-0080(M5.2 shared+barrier 一致性数据流 / scoped atomics 类型契约与 PTX 映射条款追加登记;MIR 借用检查 device 扩展数据流 + scoped atomics 映射 D-406 人工落笔,条款 PR 先于实现 PR,无体例变更) | Direct |
| v1.14 | 2026-06-14 | §4 device.md 行区间更新至 RXS-0082(M5.3 device 数学函数 intrinsic 集与求值语义 / libdevice bitcode 链接流程与 codegen 诊断条款追加登记;libdevice 按需引入 06 §7 + 编译流程 07 §7 D-205/D-207 条款化,条款 PR 先于实现 PR,gpu 基元 kernel codegen 接通随实现 WP,无体例变更) | Direct |
| v1.15 | 2026-06-15 | §4 文件清单追加 toolchain.md（RXS-0083 起续号预留，起始里程碑 M6.1：rx CLI 子命令语义面 / 包管理 rurix.toml·rurix.lock 格式与三来源解析 / LSP 能力面契约）。M6 开工脚手架仅登记新文件名 + 预留区间，**不落裸条款头**——条款体与测试锚定随 M6.1+ 实现 PR 同落（条款 PR 先于实现 PR，trace_matrix 维持全锚定），无体例变更 | Direct |
| v1.16 | 2026-06-15 | §4 toolchain.md 行区间更新至 RXS-0088（M6.1 rx CLI 子命令语义面首批条款化登记：总入口与子命令分发 + 退出码约定 / build / run / check / fmt 收编 RD-005 / bench 收编 RD-003；07 §2 §6 §9 单一前端 + 08 §7 D-239 + BENCH_PROTOCOL §3 已锁定决策的条款化，条款 PR 先于实现 PR，每条 ≥1 测试锚定，无体例变更）。包管理 manifest/lock 格式条款（M6.2）与 LSP 能力面条款（M6.4）续写 toolchain.md | Direct |
| v1.17 | 2026-06-15 | §4 toolchain.md 行区间更新至 RXS-0094（M6.2 包管理 manifest/lock/vendor 条款化登记：rurix.toml 清单格式与声明式无 build.rs / 依赖三来源 path·git·archive 解析规则 / 依赖解析图与 feature additive-v1 加性合一 + 冲突检测 / rurix.lock 精确解析图格式 / 内容树规范化 SHA-256 / vendor 与离线解析路径；09 §7.1/§7.2 已锁定决策 D-308~D-311 的条款化，条款 PR 先于实现 PR，每条 ≥1 测试锚定，无体例变更）。LSP 能力面条款（M6.4）续写 toolchain.md | Direct |
| v1.18 | 2026-06-15 | §4 toolchain.md 行区间更新至 RXS-0097（M6.3 rx test 子进程隔离 / workspace members 多包 / G-M6-1 三包离线重建逐字节复现门条款化登记：`#[test]`/`#[test(gpu)]` 逐测试子进程 harness、workspace members 进入单根 lock 图、`rx build --locked --offline` reproducible profile 两次 host EXE SHA-256 一致且 lock/vendor 不改写；14 §6 + 09 §7.1/§7.2 + M6 契约 D-M6-3/G-M6-1 的条款化，条款 PR 先于实现 PR，每条 ≥1 测试锚定，无体例变更）。LSP 能力面条款（M6.4）续写 toolchain.md | Direct |
| v1.19 | 2026-06-15 | §4 toolchain.md 行区间更新至 RXS-0103（M6.4 LSP MVP 条款化登记：`rurixc --tooling-server` 常驻 query 层 / publishDiagnostics 诊断 JSON / completion / definition+references / documentHighlight / rename；07 §9 D-210 单一前端 + RD-004 无损语法树通道接通，条款 PR 先于实现 PR，每条 ≥1 测试锚定，无体例变更） | Direct |
| v1.20 | 2026-06-15 | §4 文件清单追加 stdlib.md（RXS-0104 起续号预留，起始里程碑 M7.1：core 数学库类型面 Vec `VecN<T>` N∈{2,3,4} / Mat `MatRxC<T>` / swizzle / 几何原语 Point·Vector·Normal·AABB·Ray 的构造·分量访问与 swizzle·逐元素算术·点积/叉积/范数·矩阵乘·几何谓词，全 safe、host+device 双路径同义）。M7 开工脚手架仅登记新文件名 + 预留区间，**不落裸条款头**——条款体与测试锚定随 M7.1 实现 PR 同落（条款 PR 先于实现 PR，trace_matrix 维持全锚定；01 §6 UC-03 + 08 §5 stdlib 充实 + 05 §1 device⊂host + 11 §3 M7，M7_CONTRACT D-M7-1 `rfc_required: none` 授权），无体例变更 | Direct |
| v1.21 | 2026-06-15 | §4 stdlib.md 行区间更新为 RXS-0104 ~ RXS-0109（M7.1 core 数学库 Vec/Mat/swizzle 类型面首批条款体落地：Vec 类型与构造 / 分量访问与 swizzle / 逐元素算术 / 点积·叉积·范数 / Mat 类型与构造 / Mat 逐元素算术与矩阵乘，每条 ≥1 conformance 锚定 `conformance/stdlib/**`，host 结构体 API 真跑 + device 标量分量原语 codegen，trace_matrix 维持全锚定）。几何原语 Point/Vector/Normal/AABB/Ray + 几何谓词保留为 stdlib.md §3 预留（不落裸条款头），随 M7.1 后续小步续号。Legality 仅引用既有 2xxx 类型类诊断（RX2001/RX2002/RX2003/RX2004），不新增错误码，无体例变更 | Direct |
| v1.22 | 2026-06-16 | §4 stdlib.md 行区间更新为 RXS-0104 ~ RXS-0113（M7.1 几何原语 / 谓词条款体落地：RXS-0110 几何向量类语义区分与互转 Point3·Vector3·Normal3 / RXS-0111 AABB·Ray 类型与构造 / RXS-0112 Point∈AABB 包含与点到 AABB 距离 / RXS-0113 Ray–AABB 相交,每条 ≥1 conformance 锚定 `conformance/stdlib/{host,device,reject}/**`,host 结构体 API 真跑 + device 标量分量谓词原语 codegen + 类型互斥 reject → RX2001,trace_matrix 维持全锚定）。stdlib.md §3 预留骨架升格为「几何原语条款落地说明」。Legality 仅引用既有 2xxx 类型类诊断（RX2001/RX2002/RX2003），不新增错误码,零编译器改动,无体例变更 | Direct |
| v1.23 | 2026-06-16 | §4 文件清单追加 imageio.md（RXS-0114 ~ RXS-0117,起始里程碑 M7.2：image-io 接口语义面条款体落地——RXS-0114 图像缓冲与像素类型面 Rgb·Rgba·ImageBuffer（复用 M7.1 标量 f32 像素口径）/ RXS-0115 无损格式优先与格式选择（PPM P6 优先·PNG 次）/ RXS-0116 确定性字节布局与 PPM P6 header 规范化（行主序·通道序·f32→u8 确定量化）/ RXS-0117 图像序列落盘接口（逐帧 content SHA-256 可核对·两次落盘逐字节一致），每条 ≥1 锚定（`src/image-io` crate 内确定性单测,trace_matrix 维持全锚定）。实现裁决：**host-only 单路径**（不引入 device codegen,区别于 stdlib 双路径）,纯函数确定性编码 + 标准库落盘,全 safe（`unsafe_code=deny`）;维度以具体像素类型名 + 运行期宽高编码,不用 const 泛型（RD-007 不触碰）。错误码：Legality 仅引用既有 2xxx 类型类诊断（RX2001/RX2003）,不新增 / 不预造错误码、不改 error_codes.json 与 en.messages;运行期失败（格式不支持 / 写入失败）以库层 Result 错误值表达,若确需编译器侧 RX 诊断则停手升档。授权:01 §6 UC-03 + 08 §5 stdlib 充实 + 09 §5/§7 image-io 包形态 + 11 §3 M7,M7_CONTRACT D-M7-2 / G-M7-1 子集 / G-M7-5 `rfc_required: none`,无体例变更 | Direct |
| v1.24 | 2026-06-16 | §4 文件清单追加 softraster.md（RXS-0118 ~ RXS-0121,起始里程碑 M7.3：G0 compute 软光栅 kernel 语义面条款体落地——RXS-0118 图元分桶到 tile binning（确定性遍历序·每桶 owner 线程独写·atomics-free）/ RXS-0119 tile 光栅覆盖判定·重心坐标·边函数 / RXS-0120 深度 z-buffer 写入与深度测试（less 约定·确定性深度合成序）/ RXS-0121 tonemap HDR→LDR 像素量化（对接 imageio RXS-0116 f32→u8 确定量化与 M7.1 标量 f32 像素口径），每条 ≥1 锚定（`conformance/soft_raster/**` device codegen 样例 + `src/soft-raster` crate 确定性单测,trace_matrix 维持全锚定）。实现裁决：device 路径 NVPTX codegen 标量子集（`kernel`/`device fn` + `View<global, f32>` + `shared` + `ThreadCtx` + `block.sync()`,复用 M7.1 标量分量原语），host 路径全 safe CPU 参考（`src/soft-raster`,复用 imageio `ImageBuffer`/`Rgb`/PPM 确定编码）;全 safe 代码目标（`unsafe_code=deny`）,本轮零 unsafe。确定性：固定输入两次运行帧像素逐字节一致（固定归约 / tile 调度 / 分桶 / 深度合成序）。维度以运行期 usize/View 长度编码,不用 const 泛型（RD-007 不触碰）;不引入 device 原子（D-406/RD-008 禁区）。错误码：Legality 仅引用既有 6xxx codegen 段位诊断（RX6003）,不新增 / 不预造错误码、不改 error_codes.json 与 en.messages;运行期边界以确定性返回值/哨兵表达,若确需编译器侧 RX 诊断则停手升档。授权:01 §6 UC-03 + 06 GPU 模型 + 07 §7 device codegen + 05 §1 device⊂host + 11 §3 M7,M7_CONTRACT D-M7-3 / G-M7-3 / G-M7-1 子集 / G-M7-5 `rfc_required: none`,无体例变更 | Direct |
| v1.25 | 2026-06-16 | §4 文件清单追加 interop.md（RXS-0122 起续号预留，起始里程碑 M8.1：互操作语义面——`rx build --emit=pyd` PYD 产出约定 / `__cuda_array_interface__` v3 + DLPack 双协议零拷贝接入 PyTorch/CuPy / C ABI 边界；C ABI·PYD 唯一通道、永不 Python 原生嵌入（红线 1，SG-008）、device 维持 PTX-only、设备指针所有权以 affine 所有权 + 确定性诊断定义不设 UB 节）。M8.1 开工脚手架**仅登记新文件名 + 预留区间，不落裸条款头**——条款体（RXS-0122 起）与每条 ≥1 测试锚定随 M8.1 实现 PR 同落（条款 PR 先于实现 PR，trace_matrix 维持全锚定；09 §6 + 02 §U1 + 01 §6 + 05 §FFI + 07 §7 + 11 §3 M8 已锁定决策的条款化，M8_CONTRACT D-M8-1 / G-M8-1 / G-M8-7 `rfc_required: none` 授权）。识别新决策面 PYD/C ABI 边界 unsafe 策略带档位标记 **Mini**（口径已由 M8_CONTRACT §5 guardrail 锁定），AI 不自判 Direct，判档争议向上取严，无体例变更 | Direct |
| v1.26 | 2026-06-16 | §4 interop.md 行区间更新为 RXS-0122 ~ RXS-0125（M8.1 实现 PR 条款体落地：RXS-0122 `rx build --emit=pyd` PYD 产出约定 / RXS-0123 `__cuda_array_interface__` v3 零拷贝消费 / RXS-0124 DLPack 双协议零拷贝与设备指针所有权 / RXS-0125 C ABI 边界，每条 ≥1 `src/rurix-interop` crate 单测锚定，trace_matrix 维持全锚定）。错误码新段位首批分配 RX7013~RX7015（互操作诊断：协议不支持 / 设备指针非法 / 形状不匹配，7xxx 续接、含义冻结，registry/error_codes.json v1.20 + en.messages）。实现裁决:rurix-interop FFI 边界 crate unsafe_code 经裁决豁免 + unsafe-audit 注册（对齐 rurix-rt）;PTX-only、不触 RD-007 / D-406 / 红线 1。PYD/C ABI 边界 unsafe 策略档位 **Mini**，AI 不自判 Direct，判档争议向上取严，无体例变更 | Direct |