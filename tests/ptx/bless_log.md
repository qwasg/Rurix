# NVPTX IR golden bless 审批记录(只追加)

> 任何 `tests/ptx/**/*.nvptx` 的新增/修改/删除必须同 PR 在本表追加一行(14 §2 常驻集
> PTX/NVPTX IR 文本 golden;07 §11;M4 CI_GATES §4 第 3 项,`ci/check_guardrails.py`
> `check_ptx_bless` 机器核对:既有行 0-byte)。bless 纪律对齐 MIR/UI snapshot
> (`RURIX_BLESS=1` 重写 + 本表追加留痕)。
>
> golden 取 **device codegen 产出的 NVPTX 约束 LLVM IR 文本**(rurixc 自有产物,
> 确定性、无外部工具依赖);clang IR→PTX→ptxas 真跑关卡由 `rurixc --emit=ptx`
> (PR Smoke 步骤 17)覆盖,不入本字节 golden(PTX 字节稳定性绑定工具链版本)。

| 日期 | 范围 | 理由 | 批准 |
|---|---|---|---|
| 2026-06-13 | tests/ptx/ 初始 2 条 golden(saxpy / thread_index) | M4.2 NVPTX codegen 形态定型(RXS-0070~0072),PTX/NVPTX IR golden guardrail 激活(M4_PLAN §2 任务 4;M4 CI_GATES §4 第 3 项)。两类代表:SAXPY 雏形(global_id + View<global> 索引读写 + f32 算术 + 边界分支 + ptx_kernel 入口)/ 线程索引写回(global_id sreg 组合 + ViewMut<global> 索引写 + usize→u32 cast)。基线 = `device_codegen::build_and_emit` 文本逐字节,经 `rurixc --emit=ptx` clang 真跑产合法 PTX(`.entry` / `.target sm_89` / `ld.global`/`st.global`)验证 | pending-human-review |
