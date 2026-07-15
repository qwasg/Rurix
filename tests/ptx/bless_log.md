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
| 2026-06-14 | tests/ptx/ 新增 3 条 golden(shared_reduce / thread_index_2d / device_math_sqrt) | M5.3 review fix:shared addrspace(3) + 2D sreg + libdevice `__nv_*` IR golden;既有 2 条 snapshot 随 device_codegen 演进重 bless(reqntid metadata 等) | pending-human-review |
| 2026-06-16 | tests/ptx/ 新增 4 条 golden(sr_binning / sr_raster_tile / sr_depth / sr_tonemap) | M7.3 G0 软光栅 kernel codegen 形态纳入 NVPTX IR golden(spec/softraster.md RXS-0118~0121,D-M7-3;M7_PLAN §3 任务 4)。四类代表:binning(1D global_id → tile,图元包围盒标量 min/max + 区间相交 device fn + while 遍历 + 桶 agent 独写,atomics-free)/ tile 光栅(2D global_id,边函数二维叉积 device fn + 重心权重 + 退化三角形分支)/ 深度(1D,less 深度测试 + 固定片元序串行合成,atomics-free)/ tonemap(标量量化 device fn,clamp[0,1]+NaN→0+as usize 截断 floor,不依赖 libdevice)。基线 = `device_codegen::build_and_emit` 文本逐字节(全 safe、零 unsafe;不引入 device 原子,D-406/RD-008 禁区);经 `rurixc --emit=ptx` ptxas 干验证(RXS-0073)best-effort | pending-human-review |
