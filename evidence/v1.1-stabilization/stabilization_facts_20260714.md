# V1.1 stabilization report 机器事实归档(2026-07-14)

> 所属:milestones/v1/STABILIZATION_REPORT.md(D-V1-1 / G-V1-1)。本文件为命令真实输出原文归档(measured_local),锚定基线 `0ceca0d9`(= `g2-closed`)。evidence/ 只增不删不改。

## 1. stable 快照核对(本机真跑)

```
$ py -3 ci/stable_snapshot.py --check
[stable_snapshot] PASS(stable 面与入库快照一致:spec_clauses=180,error_codes=88,editions=['2026'],subcommands=['bench', 'build', 'check', 'doc', 'fmt', 'run', 'test', 'vendor'])
```

快照文件摘要:

```
tests/stable/stable_api.snapshot
sha256 = 08e2e26423579bc933faae2168a4bfc63810d640dfe46e1110a5c062a97e91e0
bytes  = 7527
首 bless = 2026-06-30(tests/stable/bless_log.md)
```

## 2. traceability 核对(本机真跑)

```
$ py -3 ci/trace_matrix.py --check
[trace_matrix] PASS (180/180 clauses anchored, 453 test files scanned)
```

## 3. 观察期 git 实证

main 自 g2-closed 零提交(全路径与 stable 面路径过滤均为空):

```
$ git rev-parse origin/main g2-closed
0ceca0d9f9d36b08648d4b6e9938fe6f81871b7d
0ceca0d9f9d36b08648d4b6e9938fe6f81871b7d

$ git log g2-closed..origin/main --oneline
(空)

$ git log g2-closed..origin/main --oneline -- spec/ registry/error_codes.json
(空)
```

里程碑 1 = G2.6 close-out(2026-06-30,零 spec/error_codes/src):

```
$ git show --stat f659f57a --format='%H %ad %s' --date=short
f659f57a63790c43db591cfddb652b152202edb9 2026-06-30 chore(g2.6): G2 整体 close-out — status active→closed + 基准 g1-closed→g2-closed(agent 自主签署)

 ci/check_guardrails.py       |  8 +++----
 milestones/g2/G2_CONTRACT.md | 56 +++++++++++++++++++++++++++++++++++++++++++-
 registry/deferred.json       |  9 ++++---
 registry/spike_gating.json   | 15 ++++++++----
 4 files changed, 75 insertions(+), 13 deletions(-)
```

里程碑 2 = GRX showcase close-out(2026-07-13,独立分支未合 main):

```
$ git log 95d5af43 -1 --format='%H %ad %s' --date=short
95d5af436b118c1a19f84d52816a5c71048dc009 2026-07-13 chore(grx): GRX 里程碑 close-out 签署 — status active→closed(诚实天花板收官修订版)

$ git diff g2-closed 95d5af43 --stat -- spec/ registry/error_codes.json
 spec/README.md       |   3 +-
 spec/dxil_backend.md | 101 ++++++++++++++++++++++++++++++++++++++++++++++++++-
 2 files changed, 102 insertions(+), 2 deletions(-)
```

GRX 分支 spec 改动定性:逐行核对 `git diff g2-closed 95d5af43 -- spec/` 的删除/修改行,仅 2 行为索引/编号登记行更新(spec/README.md §4 dxil_backend 表行 + dxil_backend.md「编号区间」登记段);其余 ~100 行全为新增(加性 spike 条款 RXS-0181~0184 等)。**既有 RXS-0001~0180 条款体零删改;registry/error_codes.json 零触碰;全部未合入 main。**

## 4. 佐证的证明力边界(诚实标注)

「main 自 g2-closed 零提交」同时意味着 main 上零活动,故该佐证为「无修订」的**必要非充分**证据;主证据为上述两个里程碑的实证(两者均有实质工作发生且未产生对既有 stable 面的修订需求)。残余弱点(日历窗口约两周 / 里程碑 1 与定型同日 / 里程碑 2 unmerged)与对冲条款见 STABILIZATION_REPORT §3.3。
