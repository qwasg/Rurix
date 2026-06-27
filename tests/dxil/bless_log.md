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
| 2026-06-27 | tests/dxil/graphics/ 图形=B golden(gfx_vs_min:`.dxil-disasm`) | **NOT BLESSED (local)**。G2.2 PR-D2 图形=B DXIL 第二后端 golden(spec/dxil_backend.md RXS-0162,任务10)。代表:最小 vertex 着色阶段(单 interpolate varying 输出)→ B 路 `dxil_spirv::emit_spirv`→SPIRV-Cross→dxc(Vulkan SDK 1.3.296.0,1.8.0.4739)→`dxc -dumpbin` 反汇编入 golden。**本机产物,非 owner pin 环境 bless**:① 本机 dxc 无签名 validator(dxil.dll/dxv),validator gate 仅结构性 dxc 编译,完整签名验证归 owner pin 环境;② 平凡 passthrough(入口 body 数据流降级 RD-013 deferred)经 spirv-cross DCE → 用户语义名 `uv` 退化为通用 `TEXCOORD`(by-construction UserSemantic 未被 spirv-cross 1.3.290 保真,机制① 待 owner pin spirv-cross / RD-013),签名退化形态如实入 golden;③ 版本噪声行(shader hash / dxc ident)已规范化为占位,不写死工具版本布局为语言保证(RXS-0162 IR5)。owner 在 pin 环境(签名 validator + 保真 spirv-cross + RD-013)重 bless 真签名保真 golden | pending-human-review |
