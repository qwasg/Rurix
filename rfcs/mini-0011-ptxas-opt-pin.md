# Mini-RFC MR-0011 — `RURIXC_PTXAS_OPT` 环境开关:ptxas 预编 cubin 优化档注入(RD-027 护栏)

| 字段 | 值 |
|---|---|
| Mini-RFC 标识 | **MR-0011**(Mini-RFC 序列;独立于 Full-RFC 的 `RFC-####` 命名空间,不复用 RFC 编号,10 §9.5。Mini-RFC = 单页提案 + 失败测试先行,10 §3;号位 = G3_CONTRACT §7 ⑥「MR-0011 留 RD-027 处置」claim 兑现) |
| 标题 | `RURIXC_PTXAS_OPT ∈ {0,1,2,3}` 注入 `compile_cubin` 的 ptxas `-O<n>` 旗标;缺省 0-byte 不注入;非法值确定性拒 |
| 档位 | **Mini-RFC**(10 §3:内部开关/工具行为变更;不触 UB/内存模型映射/FFI ABI/安全包络禁区,见 §3)。agent 自主裁为 Mini-RFC(2026-07-18,G3_CONTRACT §7 ⑤「改工具行为判 Mini」) |
| 状态 | Approved — 2026-07-18(agent 自主批准并记录) |
| 承接里程碑 | G3.1 RD-027 处置尾项(G-G3-1 ③(b) 护栏决定留痕;close-out 前置) |
| 关联条款 | **零新 RXS**(纯工具行为;RXS-0150 预编语义只增不改——缺省路径逐字节同前) |
| 依据决策 | RD-027 spike 归因(evidence/rd027_pt_poison_spike_report.md §6:ptxas -O0 下毒径构型正确终止 0.7s、完整生产档 256spp/4 弹射 9.5s/帧,而 -O1+ SASS `CALL.REL.NOINC` latch 协议死锁)· G3_CONTRACT §7 ② · 先例 MR-0005(G1.5 fatbin 预编面) |
| Provenance | `Assisted-by: claude-code:claude-fable-5`。agent 自主决策,批准后推进下游 PR |
| 失败测试先行 | `src/rurixc/src/ptxas.rs::tests::{opt_flag_accepts_levels_0_to_3_and_defaults_to_none, opt_flag_rejects_invalid_levels_deterministically}` —— 引用拟新增 `opt_flag_from_env`;本 Mini 落笔时点 main 上该函数不存在 = RED(能力尚不存在),实现 commit 落地后转绿。10 §3 Mini「必须先有失败测试」 |

---

## 1. 摘要

给 `ptxas.rs::compile_cubin`(RXS-0150 按架构预编 cubin,G1.5/MR-0005)加一个环境开关 `RURIXC_PTXAS_OPT`:取值 `0|1|2|3` 时向 ptxas 注入对应 `-O<n>` 旗标;未设置时**不注入任何旗标**(ptxas 自身默认,现行为逐字节 0-byte);非法值**确定性拒**(`CubinOutcome::Toolchain` 报文携 env 名与 RD-027 锚,不静默回落)。动机 = RD-027 spike 实证护栏:NVIDIA 优化后段(ptxas -O1+/驱动 JIT)对本仓合法 PTX 的特定构型产死锁 SASS,`-O0` 正确终止且使此前不可测的完整生产档(256spp/4 弹射)以 9.5s/帧可测。开关使受影响应用(如 apps/ruridrop 取证链)可显式 pin 档位,主线默认零变化。

## 2. 设计(用户视角 + 形态)

```text
RURIXC_PTXAS_OPT=0 rx build apps/ruridrop/src/offline.rx -o offline.exe
# → compile_cubin 调 ptxas 时前置 -O0;嵌入产物的 cubin 为 -O0 编译
# 未设置          → 不注入(ptxas 默认 -O3),现行为 0-byte
# 非法值(如 "4") → CubinOutcome::Toolchain(确定性报文),构建侧如实失败
```

复用项:`locate_ptxas`/ASCII 临时目录/RXS-0150 预编与嵌入链全部 0-byte;新增面 = 纯函数 `opt_flag_from_env(Option<&str>) -> Result<Option<String>, String>`(可测;**空串/纯空白 = 视同未设**,评审 F1)+ `compile_cubin` 头部一次 env 读取与 `cmd.arg` 条件注入 + driver 侧构建前预检(非法值 = 构建确定性拒)。`dry_gate`(RXS-0073 干验证)**不受影响**——验证关卡维持默认档,只有**保留分发**的 cubin 受开关控制(验证面与分发面解耦,验证不放宽;注意 gate 验的是 PTX 良构性,**不对所发档位的 ptxas codegen 背书**——RD-027 类运行期优化器缺陷本就在 gate 结构覆盖之外,评审 F4)。**护栏效力限定(评审 F2):本开关是通用 ptxas -O pin;仅 `=0` 具 RD-027 护栏效力,`=1/2/3` 为语法合法但会精确复现毒径死锁的档位(=3 ≡ 缺省)**。

## 3. 为何 Mini-RFC(而非 Direct,亦非 Full RFC)

- **非 Full RFC**:不触 AGENTS 硬规则 5 / 10 §7.5 禁区——无 UB 面、无内存模型映射变更、无 FFI ABI、无安全包络;PTX/cubin 语义等价(仅优化档),运行时装载协商(RXS-0151)0-byte。
- **非 Direct**:新增用户可见环境开关 = 工具行为决策面(档位集合、非法值语义、验证面是否随动三个决策点),按 10 §3「内部开关/工具行为」归 Mini;G3_CONTRACT §7 ⑤ 明记该判档。
- **升档触发条件(实现期守卫)**:若后续需按 kernel 粒度/语言面(attribute)控制优化档(触编译器语义面),停手升 Full RFC;本 Mini 仅进程级 env。

## 4. 错误码 / 影响 / 范围

零新 RX 码:非法值经 driver 构建前预检确定性拒(exit 1,报文携 `RURIXC_PTXAS_OPT` 与 `RD-027` 锚;`compile_cubin` 内同校验为纵深防御走 `CubinOutcome::Toolchain`);无编译期语义诊断面。**误设半径如实声明(评审 F1 部分驳回记录):非空非法值在无 ptxas 的纯 host 环境同样构建硬红——这是有意的 fail-closed(用户显式设置了坏值,静默忽略即护栏假生效);空串已归「未设」,CI 常见清空写法不受累。**

## 5. 失败测试先行(10 §3 Mini 硬性)

两条单测(见头表)编码开关全值域:合法档位映射 `-O0`~`-O3`、缺省 `None`、六类非法值确定性拒且报文携锚。本 Mini 落笔时点 `opt_flag_from_env` 不存在 = RED;实现 commit 落地后全绿(commit 序:Mini 文档+测试 → 实现,同 PR)。

## 6. 影响 / 向后兼容 / 范围

- **向后兼容**:缺省路径逐字节 0-byte(不注入旗标 = 现命令行原样);既有 fatbin/lockfile/装载链全不动;host 回归网不依赖 device。
- **范围红线**:不改 JIT fallback 腿(驱动侧无对应开关,RD-027 报告 §6 如实记限界);不做按 kernel 粒度控制(升档触发条件)——**进程级 = 对该进程全部 `compile_cubin` 一律施档,多 kernel 应用启用 `=0` 护栏会连带 de-opt 无辜 kernel(评审 F3 如实声明),直至升 kernel 粒度另裁**;不在 CI 默认开启(uc07 取证链按需显式设置);上游缺陷本体维持 RD-027 open,本开关是护栏非修复。

## 7. Agent 批准

> **Approved — 2026-07-18**。agent 自主批准本 Mini-RFC(§2 形态 + §3 判档 + §4 错误码 + §6 范围)并记录。device 真跑(护栏红绿:默认档毒径挂 → `RURIXC_PTXAS_OPT=0` 同源完成)/ 证据回填 / 合入均由 agent 自主签署。

## 7.1 对抗性评审记录(轻量,D-409 Mini-RFC 档 · [`../13_DECISION_LOG.md`](../13_DECISION_LOG.md) D-409)

> - **评审者 provenance**:`Assisted-by: claude-code:claude-opus-4-8`(≠ 起草 Provenance)。
> - **评审轮次**:第 1 轮,2026-07-18。结论:无阻断项,4 findings。
> - **Findings 与 disposition**:F1(MEDIUM,空串即拒+误设红爆炸半径)——**采纳(a)**:空串/纯空白归「未设」(`opt_flag_from_env` + 测试更新);**部分驳回(b)**:非空非法值在无 ptxas 环境仍硬红为有意 fail-closed,§4 如实声明。F2(LOW-MED,值域含毒档护栏错觉)——**采纳**:§2 补「仅 =0 具护栏效力,=1/2/3 复现毒径」限定。F3(LOW,进程级 blanket de-opt 后果未言明)——**采纳**:§6 补后果句。F4(LOW,gate 不背书所发档位 codegen)——**采纳**:§2 补 gate 覆盖面限界句;解耦方向安全性(「-O0 更保守+fail-closed 不静默发坏件」)经评审核验成立。
