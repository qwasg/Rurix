> **Status: DRAFT — do NOT file.** Owner review gate; agent does not file externally.

# PROVENANCE — RD-027 PT 自旋(BSYNC 死等)NVIDIA 上游报告备包

## 来源

- 本包起草于 `main` @ `24fff4b17b539238874569cb45f1f3a7c3b71111`(2026-07-18,merge PR #171
  "spike(g3.1): RD-027 毒径四层判别矩阵归因落地");事实源全部落于该 commit(源文件经
  PR #171 / `233920a9` 合入),起草零杜撰,一切数值取自下列机器证据。
- 事实源(入库,tracked):
  - `evidence/rd027_pt_poison_spike_report.md` — G3.1 取证报告本体(§3 实验矩阵 / §4 SASS
    证据 / §5 归因裁决 / §7 复现清单)。
  - `evidence/rd027_pt_poison_spike_20260718.json` — 33 项实验机器记录(过
    `ci/check_schemas.py`;环境串 / 逐 run exit+wall / 挂起签名 / attribution)。
  - `spike/rd027-pt-poison/attribution.json` — 归因裁决(verdict =
    `nvidia_optimizing_backends`, confidence = high)。
  - `spike/rd027-pt-poison/mrp/README.md` + `mrp/render_pt_mrp_d6a.rx` — 最小化复现
    (E4 d6a,276 行,全循环编译期有界)。
- 事实源(工作区工件,**不入库**,按需再生;正式提报时按 ISSUE_DRAFT 附件清单固化):
  - `build/spike-rd027/campaign.jsonl` — 逐 run 原始 JSONL(107 行;本包
    `repro_log_20260718.md` 的逐字输出即摘自此)。
  - `build/spike-rd027/bin/ctrl_b2.ptx` — 3838 行完整 PTX(sha256
    `85d597dd22e2d05f511a0cf8b2a27823bee78cf58523862eb06ccc67c738e315`);本包只入
    `ptx_excerpt.md` 摘录(≤60 行),完整文件提报时整体附上,不入库。
  - `build/spike-rd027/e5/analysis.md` + `aot_O{0,1,2,3}.sass` / `e7b_O{1,3}.sass` —
    SASS 静态分析(证据模式 A–F 带行号)与反汇编工件。

## 完整 PTX / SASS 再生命令(工件不入库的补偿)

```
cargo build -p rurixc -p rx
py -3 spike/rd027-pt-poison/run_e0a.py --build-only        # 产 build/spike-rd027/bin/ctrl_b2.ptx 等五变体
ptxas -arch=sm_89 -O0 build/spike-rd027/bin/ctrl_b2.ptx -o e5/aot_O0.cubin   # + nvdisasm -c → aot_O0.sass
ptxas -arch=sm_89 -O1 build/spike-rd027/bin/ctrl_b2.ptx -o e5/aot_O1.cubin   # + nvdisasm -c → aot_O1.sass
ptxas -arch=sm_89 -O2 build/spike-rd027/bin/ctrl_b2.ptx -o e5/aot_O2.cubin   # ≡ -O3 逐字节
ptxas -arch=sm_89 -O3 build/spike-rd027/bin/ctrl_b2.ptx -o e5/aot_O3.cubin
```

(ptxas/nvdisasm = CUDA v13.3;ptxas 输入须 ASCII 路径。PTX 再生须核 sha256 与上值一致
——rurixc 构建确定性,同 commit 同参数应逐字节同一。)

## 真因(一句话)

`rx_pt_render_176`(合法 PTX:全循环有界、可归约、零 spill)经 NVIDIA 优化后段
(ptxas -O1 及以上,驱动 620.02 JIT 同类变换)的 latch 出口协议重构——计数出口被编成
无 reconvergence 记账的 `@!P0 CALL.REL.NOINC` 谓词边(O0 同环为 `@P0 BREAK`+`BRA`→
`BSYNC` 正规记账,共 4 处,barrier id 9/9 零裕度)——非 uniform 触发时 warp 无记账切分、
同 id 并发再臂、参与者掩码破坏,任意下游 `BSYNC` 永等(机理 M1′),外观即
util 100% / ~63W / 满频 2745MHz 挂起;`ptxas -O0` 对同一 PTX 正确终止(毒径 0.66s,
完整生产档 9.49s/帧)。

## `<FILL>` 占位清单(4 处,全在 `ISSUE_DRAFT.md`,留 owner 补)

| # | 位置 | 内容 |
|---|---|---|
| 1 | 头部 note | 提报渠道裁定(NVIDIA Developer Forums CUDA 版 / developer.nvidia.com/bugs 开发者 bug 门户)+ owner 的 NVIDIA 开发者账号 |
| 2 | Environment | 驱动 620.02 的安装包分支确认(Game Ready / Studio;实测值 620.02 取自 NVML,分支信息 agent 侧无法核证) |
| 3 | Reproduction | 复现包附件上传(完整 ctrl_b2.ptx + O0–O3 cubin/SASS + 最小化源 + repro log) |
| 4 | Pre-filing checklist item 4 | 渠道模板若要求的系统信息 dump(nvidia-bug-report / DxDiag) |

另:草稿末尾整节 "Pre-filing checklist" 为 owner 自查用,粘贴到公开渠道前须整节删除
(E6 nvcc 对照与 cuda-gdb warp 停驻取证为**可选补强**,非提报前置——报告 §5 诚实限界
①②原文如此,不在本包升格)。

## DRAFT 标头口径

本包四文件(`PROVENANCE.md` / `ISSUE_DRAFT.md` / `repro_log_20260718.md` /
`ptx_excerpt.md`)均为散文/摘录文件,规范标头 `DRAFT — do NOT file` 已逐一落于各文件
首行,无结构化文件豁免项(对照:godot-buffer-clear 包的 `.tscn`/`project.godot` 豁免
口径,本包不适用)。

## 提报纪律

- agent 只负责备包(本目录即备包产物);**上游提报由 owner 亲自执行,AI 不对外提交**
  (EA1 G-EA1-7 先例;G3_CONTRACT 处置尾项 (b) 路线,报告 §6)。
- 本包在 G3 期内仅作证据归档,构成 G3 close-out 前置之一,不构成提报动作;RD-027
  诚实存续(不 force-close),backfill_condition 原文维持不预支。
- 提报时点须重跑草稿 §Pre-filing checklist(可选补强裁量、敏感信息清查、查重、
  填 `<FILL>`、打包附件)。
