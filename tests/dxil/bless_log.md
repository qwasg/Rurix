# DXIL golden bless 审批记录(只追加)

> 任何 `tests/dxil/**/*.dxil-ll` 或 `*.dxil-disasm` 的新增/修改/删除必须同 PR 在本表
> 追加一行(14 §2 常驻集 DXIL 第二后端 golden;RFC-0003 §9 Q-Golden;G2.2 PR-C2
> 分片1 激活,`ci/check_guardrails.py` `check_dxil_bless` 机器核对:既有行 0-byte)。
> bless 纪律对齐 MIR/UI/PTX snapshot(`RURIX_BLESS=1` 重写 + 本表追加留痕)。
>
> 两层 golden(RXS-0157):
> - **`.dxil-ll`**:rurixc 自有 **DirectX 三元组 LLVM IR 文本**(`dxil_codegen` 产物,
>   确定性、无外部工具依赖,对齐 ptx_golden 取 IR 层纪律);
> - **`.dxil-disasm`**:经 patched llc `-filetype=obj` 产 DXIL 容器 + dxc validator
>   **接受后**的文本反汇编(RFC-0003 §9 Q-Golden;不合规 DXIL 不得入 golden)。
>   patched llc(`RURIX_LLC`,RD-011 dev 偏差)/ dxc validator(`RURIX_DXC_DIR`)缺失
>   → 反汇编关卡 SKIP(真实红绿在带工具链环境,对齐 RXS-0073 ptxas 干验证 SKIP)。

| 日期 | 范围 | 理由 | 批准 |
|---|---|---|---|
| 2026-06-24 | tests/dxil/ 初始 golden(cs_noop:`.dxil-ll` + `.dxil-disasm`) | G2.2 PR-C2 分片1 DXIL 第二后端最小 compute 端到端形态定型(spec/dxil_backend.md RXS-0157,D-G2-2)。代表:空体 compute kernel(无 ABI 形参)→ DirectX 三元组 `dxil-unknown-shadermodel6.0-compute` LLVM IR(`hlsl.shader`=compute / `hlsl.numthreads`=1,1,1 入口属性)。`.dxil-ll` 基线 = `dxil_codegen::build_and_emit_dxil` 文本逐字节;`.dxil-disasm` 经 patched llc(RURIX_LLC,RD-011)`-filetype=obj` 产 DXIL 容器 + dxc validator(1.9.2602.24)`Validation succeeded.` 接受后 `dxc -dumpbin` 反汇编入 golden | pending-human-review |
| 2026-06-25 | tests/dxil/ 新增 vertex/fragment golden(vs_noop / ps_noop:各 `.dxil-ll` + `.dxil-disasm`) | G2.2 PR-C2 分片2 RXS-0158 着色阶段着色 → DXIL 着色器类型降级对应:vertex 着色阶段 → DXIL vertex shader(`dxil-unknown-shadermodel6.0-vertex` + `hlsl.shader`=vertex,无 numthreads)、fragment 着色阶段 → DXIL pixel shader(`shadermodel6.0-pixel` + `hlsl.shader`=pixel)。`.dxil-ll` 基线 = `dxil_codegen::build_and_emit_dxil` 文本逐字节;`.dxil-disasm` 经 patched llc(RURIX_LLC,RD-011)`-filetype=obj` 产 DXIL 容器 + dxc validator(1.9.2602.24)`Validation succeeded.` 接受后 `dxc -dumpbin` 反汇编入 golden(disasm 证 Vertex Shader / `vs` SM6.0、Pixel Shader / `ps` SM6.0)。既有 cs_noop golden 0-byte 不变。compute fn / mesh / task / RT 阶段:compute 沿用 cs_noop golden;mesh/task/RT 映射登记实现 deferred(RD-012),无 golden | pending-human-review |
| 2026-06-25 | tests/dxil/ 新增 vertex/fragment I/O golden(vs_io / ps_io:各 `.dxil-ll`) | G2.2 PR-C2 分片3 RXS-0159 阶段 I/O → DXIL 签名/系统值语义降级(类型面):vertex 入口 vertex_id→SV_VertexID(输入)/ position→SV_Position + 透视插值 varying uv→interp:linear(输出);fragment 入口 frag_coord→SV_Position + 透视插值 varying(输入)/ 渲染目标颜色→SV_Target(输出)。`.dxil-ll` 基线 = `dxil_codegen::build_and_emit_dxil` 文本逐字节(DirectX 三元组 + `hlsl.shader` + 类型面签名元数据 `!rurix.dxil.sig.in`/`.out`,**仅 SV 语义名/插值限定符,无寄存器/偏移/component mask**——二进制布局属 RFC-0003 §4.6/§9 Q-Builtin 🔒 FFI ABI 禁区,由 LLVM DirectX 后端 emit,Rurix 不定义)。**`.dxil-disasm` 本片不入**:dev 环境 patched llc(RURIX_LLC,RD-011)/ dxc validator(RURIX_DXC_DIR)缺失 → 反汇编 + validator 真验证关卡 SKIP(per-file),disasm golden 待带工具链环境经 RURIX_BLESS=1 录入 + 人工 bless(对齐 RXS-0073/RXS-0157 干验证 SKIP 纪律)。入口 body 数据流降级 deferred(RD-013,本片仅签名 + void stub)。既有 cs_noop/vs_noop/ps_noop golden 0-byte 不变 | pending-human-review |
| 2026-06-25 | tests/dxil/ vs_io/ps_io `.dxil-disasm` 补证（**不入 golden**，disasm 关卡 SKIP→measured 收紧） | G2.2 PR-C2 分片3 RXS-0159 签名映射真达产物补证（RD-011 patched llc 环境，round-8）。vs_io/ps_io 的既有 `.dxil-ll`（含 `!rurix.dxil.sig.*`）经 patched llc（RURIX_LLC，SHA256 `BF6C0868…745261`）`-filetype=obj` 产 DXContainer，IDxcValidator ×25 = 25/25 accept（`{0x0:25}`）+ dxv.exe（1.9.2602.24）×20 = 20/20 `Validation succeeded.`。**但产物 ISG1/OSG1 签名 part `elemcount=0`**——无 SV_Position/SV_Target/SV_VertexID 任何 SV 元素：根因 LLVM DirectX 后端 `addSignature()` 对图形着色器无条件写空签名（`// FIXME: support graphics shader`，上游 issue #90504），`!rurix.dxil.sig.*` 自有元数据被忽略，且任何元数据形态当前都无 lower 路径。让 SV 真达产物需后端调 `Signature::addParam(Register/Mask/ExclusiveMask)` = 二进制 ABI 布局（RFC-0003 §4.6/§9 Q-Builtin 🔒 禁区，越出 RXS-0159 类型面）→ 硬规则 5 **需人工升档**。**故 vs_io/ps_io `.dxil-disasm` 本轮仍不入 golden**（签名空，录入即伪造 SV 真达；分片1/2 cs/vs/ps_noop disasm 与本片 `.dxil-ll` 0-byte 不变）。真发现详见 evidence/dxil_slice3_rxs0159_sig_disasm_round8.md；RD-011 history + RD-013 backfill 承接 | pending-human-review |
