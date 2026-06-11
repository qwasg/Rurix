# M0 CI 门禁规范

> 所属契约:[M0_CONTRACT.md](M0_CONTRACT.md)
> 版本:v1.0(2026-06-11)
> 三层结构来源:14 §8(照搬上一项目 H03 §5);本文规定 M0 期各层的实际内容与建成顺序。
> 铁律:**第一周即真跑**——任何工作流必须在真实 PR 上以真实失败/通过路径验证过(H06 D11.8-2:YAML 语法检查 ≠ CI)。

---

## 1. Runner

- 自托管 runner = 开发机(RTX 4070 Ti),14 §8:"GPU runner 就是开发机"。
- 要求:runner 常驻在线;GPU 任务串行队列(并发=1,避免基准互扰);无 GPU 可用时 GPU 步骤显式 fail 而非静默 skip(strict-only 精神,P-01)。
- 安全:闭门期私有仓库,runner 不接受 fork PR(开源后重审,登记于 close-out 提醒项)。

## 2. 三层门禁在 M0 的内容

| 层 | M0 状态 | 内容 |
|---|---|---|
| **PR Smoke** | M0.1 建成 | 见 §3 |
| **Nightly** | M0.3 起最小集 | L0 探测器全量跑 + SAXPY/bandwidthTest 短协议(1 trial 冒烟,不更新预算)+ 预算 evaluator 合并加载 |
| **Release** | **不建** | 签名/SBOM/许可审计无发布产物可挂 → **RD-001**(承接 M8) |

## 3. PR Smoke 步骤清单(M0 版)

| # | 步骤 | 失败即红 |
|---|---|---|
| 1 | 仓库结构核对(10 §4 一等公民目录存在性) | 是 |
| 2 | 注册表 schema 校验:`registry/deferred.json`、`registry/spike_gating.json`、`milestones/*/m*_budget.json`、证据 JSON(对 `evidence_schema.json`) | 是 |
| 3 | guardrail 脚本(§4) | 是 |
| 4 | harness 单测(统计函数:trimmed mean/IQR/CV 对合成数据复算) | 是 |
| 5 | 基准冒烟:SAXPY 装载 + 单次执行 + 正确性比对(不计时、不进预算;GPU 步骤,runner 专属) | 是 |
| 6 | 预算 evaluator 加载 `m0_budget.json`(M0 关闭前允许 `estimated` skip,但 skip_reason 必须输出留痕,14 §3) | 加载失败即红 |

## 4. Guardrail 核对脚本清单(M0 版,14 §2 子集)

机器执行、字节级核对(`git diff --stat`),非人工 checklist:

1. 历史预算 JSON 既有条目 0-byte(对比基准:上一次 close-out 的 tag;M0 期内为该文件首个合入版本)。
2. `registry/*.json` 既有条目只追加——对既有 `id` 的字段修改触发 FAIL(AI 修改既有条目自动转人工审查,14 §10)。
3. 规划文档集(`00_*.md` … `14_*.md`)在执行 PR 中无 diff;勘误必须是独立 PR 且只追加修订记录行(00 §6.3)。
4. 已关闭契约文件(`status: closed`)的非 close-out 区 0-byte。
5. `evidence/` 目录只增不删不改(证据不可篡改)。

> 14 §2 常驻集中的 stable API 快照/spec 档位标记/conformance 绿/unsafe-audit 等项在 M0 无对应实体,随 M1+ 逐项激活;激活记录追加于本文修订记录。

## 5. 验证程序(对应契约 G-M0-2)

1. 提交一个**故意违反 guardrail** 的 PR(如修改 `deferred.json` 既有条目)→ 必须红。
2. 修复后同 PR 转绿 → 合入。
3. close-out 附两次 run URL 与关键命令输出。

## 6. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版 |
