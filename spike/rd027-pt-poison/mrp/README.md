# SPIKE(RD-027) MRP — 最小化挂起复现(E4 d6a,276 行)

> 纪律:探针语料,不入 src/ 生产路径,不随产品编译;上游提报**不由本仓发起**
> (DRAFT — do NOT file,owner 复核门;正式备包归 evidence/upstream-reports/)。

## 复现步骤(本机 RTX 4070 Ti / driver 620.02 / CUDA 13.2 驱动 + 13.3 工具链实录)

1. `cargo build -p rurixc -p rx`
2. 取 `apps/ruridrop/src` 整目录副本,以本目录 `render_pt_mrp_d6a.rx` 替换其中
   `render_pt.rx`;`params.rx` 打毒径参数:`SPP 32→8 / SPP_BATCH 32→8 /
   PT_BOUNCES 2→3 / REND_FRAMES 8→1`(切片值为 STUB(RD-027) 现值)。
3. `target\debug\rx.exe build <副本>/offline.rx -o poison.exe`
4. 运行(**必须带看门狗**,如 `py -3 bench/proc_guard.py --timeout 120 -- poison.exe`):
   - 默认构建(ptxas -O3 AOT cubin):**挂起**(util 100%/~63W/满频,BSYNC 死等)
   - `RURIXC_PTXAS` 失效强制 PTX JIT(驱动 620.02):**挂起**
   - ptxas 注入 `-O0`(cubin AOT):**0.4~0.7s 正确完成**
   - `PT_BOUNCES=2` 对照:任意档秒级完成

## 关键判别事实(全记录 = evidence/rd027_pt_poison_spike_20260718.json)

- 绿/挂分界精确 O0→O1(O2≡O3 SASS 逐字节);双装载路(AOT/JIT)一致挂
- O1 即引入 4 处 `@!P0 CALL.REL.NOINC` latch 出口(无 reconvergence 记账;
  O0 同环 = `@P0 BREAK` + `BRA→BSYNC` 正规记账)——SASS 证据模式 A–F 带行号
  见 spike 报告 §5(分析工件驻 build/spike-rd027/e5/,不入库)
- 全源循环硬封顶仍挂;对照 memcheck 0 errors;E4 阶梯非单调(形态叠加型陷阱)
- 数据依赖:需 4 子步 sim 后粒子分布(SUBSTEPS=0 初始排布不触发)——自包含
  合成数据版 MRP 归上游备包阶段(处置 PR)按需再最小化
