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
