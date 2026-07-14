# Mini-RFC MR-0008 — 语言 1.0 stable channel 最小清单（channel_manifest.json + Release 层子门延伸）

| 字段 | 值 |
|---|---|
| Mini-RFC 标识 | **MR-0008**（Mini-RFC 序列；独立于 Full-RFC 的 `RFC-####` 命名空间，不复用 RFC 编号，10 §9.5。Mini-RFC = 单页提案 + 失败测试先行，10 §3。**编号说明:MR-0005 已用于 G1.5 fatbin（rfcs/README §5 台账滞后随本 PR 修正）;MR-0006 / MR-0007 已被 GRX showcase 分支（未合 main）claim——避撞取 MR-0008,对齐 MR-0005 避撞 MR-0003/0004 的既有教训,编号永不复用**） |
| 标题 | 语言 1.0 首个 stable 发行的 channel 身份锚:`rurixup release` 追加产出确定性 `channel_manifest.json`（channel=stable）,清单一致性纳入 Release 层 hard-block 第 8 子门 |
| 档位 | **Mini-RFC**（10 §3:工具行为 + 发布产物清单形态量级;**不触** UB / 内存模型映射 / FFI ABI / 安全包络禁区，见 §3）。agent 自主裁为 Mini-RFC（2026-07-14,用户同日 AskUserQuestion 裁决范围 = 最小 stable channel 清单,V1_CONTRACT §7 ④） |
| 状态 | Approved — 2026-07-14（agent 自主批准并记录,AGENTS v3.0 硬规则 1） |
| 承接里程碑 | V1.2（V1_CONTRACT D-V1-3 / G-V1-3;V1 = 语言 1.0 正式发布稳定化收尾） |
| 关联条款 | 拟落 spec/release.md **RXS-0185 ~ RXS-0186**（延伸既有发布产物语义面,不新建文件,沿 MR-0005 落点先例;**RXS-0181~0184 已被 GRX showcase 分支 claim,避撞续号**） |
| 依据决策 | D-241（rurixup + 按版号原子分发,08 §9）· RXS-0135~0139（发布链路五语义面,M8.4）· RXS-0150~0152（fatbin 分发,G1.5/MR-0005 先例）· RXS-0180（stable 面与 edition 关系,RFC-0008/G2.5）· RD-008（stable 快照已激活）· 10 §6（工具链发布门）· V1_CONTRACT §7 ④（用户裁决:最小清单,不做 rustup 前端） |
| Provenance | `Assisted-by: claude-code:claude-fable-5`。agent 自主决策，批准后推进下游 PR |
| 失败测试先行 | `ci/channel_manifest_smoke.py`（CI 步骤 50）+ `src/rurixup/src/channel.rs` 单测——引用拟新增能力;**当前 main 上 RED**（脚本与模块均不存在,channel 身份锚能力缺失）;实现 PR 落地后转为有意义拦截:①漂移注入（`--simulate-channel-drift`）应阻断却放行即红;②未知 channel（`--channel nightly`）应拒却受理即红;③清单缺失/字段漂移/确定性破坏即红。10 §3 Mini「必须先有失败测试」 |

---

## 1. 摘要

语言 1.0 是 Rurix 首个 stable 发行。现有发布产物（`bundle.json` / SBOM 双视图 / `signing_manifest.json` / `gate_decision.json`,RXS-0135~0139）回答「产物是什么、是否完整、是否可信」,但**不回答「产物处于哪个发行渠道」**。rustup 式工具链前端（install/update/channel 切换,08 §9 r6「MVP 后期」）未来需要一个机器可消费的 channel 身份锚。本提案只落**最小清单**:`rurixup release` 追加产出确定性 `channel_manifest.json`（channel=stable,含 rurix_version / bundle 清单 digest 引用 / 组件清单）,并把清单一致性纳入 Release 层 hard-block 门集第 8 子门 `channel-manifest`。**最大化复用**:digest 复用 `rurix_pkg::sha256`（RXS-0093 内容寻址口径）,序列化复用 `crate::json_escape` + 字典序确定性纪律（RXS-0138 同模）,门集复用 `gate::release_decision` 既有枚举形态（RXS-0139,既有 7 门相对顺序 0-byte）。**不实现** install/update/channel 切换,**不建** nightly channel,不引入任何网络端点。

## 2. 设计（用户视角 + 形态）

`rurixup release` 新增 flag `--channel <name>`（**缺省 `stable`**,发布链路默认即 stable,CI 无需显式传参;非法值 → 用法错误退出码 1）与 `--simulate-channel-drift`（故障注入,仅供真实红绿自检,镜像 `--simulate-missing-sbom`）。输出目录追加 `channel_manifest.json`:

```json
{
  "schema_version": 1,
  "channel": "stable",
  "rurix_version": "1.0.0",
  "bundle_manifest_sha256": "<64hex = sha256(bundle.json 字节流)>",
  "components": [
    { "name": "rurixup.exe", "version": "1.0.0", "partition": "language-core", "sha256": "<64hex>" }
  ],
  "sbom": { "spdx": "sbom.spdx.json", "cyclonedx": "sbom.cdx.json" },
  "signing_manifest": "signing_manifest.json"
}
```

形态要点（→ 条款 RXS-0185/0186）:

- **channel 合法集首版 = `{"stable"}`**（`channel::VALID_CHANNELS`;未来 nightly 只需扩集 + 条款修订,编号面预留但不预造）。
- **确定性**:components 按干名字典序;**日期/时间戳不进清单**——同一 bundle 两次生成逐字节一致（发布日期归 GitHub Release 元数据与 evidence `timestamp` 字段承载）;手写确定性 JSON,零外部依赖。
- **内容寻址引用**:`bundle_manifest_sha256` = `bundle.json` 字节流 SHA-256（`rurix_pkg::sha256::hex_digest`,对齐 RXS-0093 口径）——channel 清单锚定的正是同目录写出的那份 bundle 清单。
- **一致性判据**:channel manifest `rurix_version` == bundle `rurix_version`（RXS-0135 同版号判据延续）;components 与 bundle 组件全集一一对应（干名/版号/分区/digest 逐项一致）。
- **Release 层第 8 子门 `channel-manifest`**:清单生成失败 / 组件集漂移 / 版号不符任一 → 子门红 → `allow_upload=false` + 退出码 2（hard-block 语义不变,RXS-0139 既有 7 门相对顺序 0-byte,追加末位）。
- **实现落点**:新模块 `src/rurixup/src/channel.rs`（结构 + 纯函数 + 确定性序列化 + 单测,镜像 `sbom.rs` 形态）;`bundle_json` 序列化从 `main.rs` 上移为 `BundleManifest::to_json()`（main 与 channel 共用同一字节流,digest 才有唯一锚,语义 0-byte 纯搬移）;`gate::GateInputs` 增 `channel_manifest_ok`;`lib.rs::run_release` 编排接线 + `ReleaseReport` 扩展;摘要行追加 `channel=<name> channel_ok=<bool>` token（既有 token 0-byte,纯追加,冒烟脚本按 `key=value` 解析安全）。

## 3. 为何 Mini-RFC（而非 Direct，亦非 Full RFC）

- **非 Full RFC**:不触 AGENTS 硬规则 5 / 10 §7.5 禁区——无 UB（清单一致性以确定性机器事实定义,沿 spec/release.md「严禁 UB 节」体例）、无内存模型映射、无 FFI ABI 二进制布局（纯 host 工具层 JSON 产物）、无安全包络边界;不触语言语法/类型系统/运行时语义。stabilization/edition 的 Full RFC 面已由 RFC-0008 承载,本提案是其收尾期的发布工具面延伸。
- **非 Direct**:触及**执行期新决策面**（发布产物新增一类清单 + 发布门集扩充 = 工具行为变更,10 §3 Mini-RFC 明列「工具行为变更」）;且需落 spec 新条款(RXS-0185~0186)。对齐先例:MR-0002（引擎集成工具面）/ MR-0005（fatbin 分发产物面）同量级同档。硬规则 8 判档争议向上取严。
- **升档触发条件（实现期守卫）**:若实现期发现需 install/update 前端、网络端点、多 channel 语义、或触 FFI ABI / 安全包络,则**停手升档**（rustup 前端为后续里程碑独立判档,08 §9),不在本 PR 自行落笔。

## 4. 错误码 / 影响 / 范围

- **零新 RX 码**:channel 清单失败以工具层表达——未知 channel = 用法错误退出码 1(镜像既有未知参数路径);清单漂移/缺失/版号不符 = 第 8 子门红 → `failed_gates` 含 `channel-manifest` + 退出码 2。沿 spec/release.md §3 既定口径（rurixup 工具层 Result/退出码,不引 RX 段位;确需则 RX7021 起停手升档,不预造）。`registry/error_codes.json` 与双语 messages **零追加**（bilingual 88/88 不变）。
- **零新 unsafe**:纯 host 确定性 JSON(`unsafe_code=deny` 维持,无 U 续号)。
- **stable 快照联动**:RXS-0185~0186 使 `spec_clauses` 180→182 → `tests/stable/stable_api.snapshot` 同 PR 重 bless + `bless_log.md` 追加(RXS-0180 L2 加性演进;check_stable_snapshot_bless 守卫;pr-smoke 步骤 49 硬红故不可分 PR)。

## 5. 失败测试先行（10 §3 Mini 硬性）

- **路径**:`ci/channel_manifest_smoke.py`(CI 步骤 50,纯 host/CPU-only)+ `src/rurixup/src/channel.rs` 单测(`//@ spec: RXS-0185/0186` 锚定)。
- **编码意图**:channel 身份锚存在性 + 字段语义 + 确定性(两次生成逐字节一致) + 一致性判据 + 第 8 子门 hard-block。
- **当前 main 上 RED**:脚本与模块均不存在——`py -3 ci/channel_manifest_smoke.py` 无此文件;`rurixup release` 不产 channel_manifest.json;发布门只有 7 门,channel 漂移场景**无门可拦**(能力缺失即 RED)。
- **实现落地后转绿/转有意义拦截**:green(清单产出+字段+确定性断言)+ red→绿闭合(`--simulate-channel-drift` → exit 2 且 failed_gates 含 channel-manifest;`--channel nightly` → exit 1;复原绿,反 YAML-only)。

## 6. 影响 / 向后兼容 / 范围

- **向后兼容**:纯追加——既有 5 类输出文件字节流 0-byte(bundle_json 搬移为方法后序列化字节不变);摘要行 token 纯追加;`--channel` 缺省 stable 使既有调用 0 改动;既有 7 门相对顺序 0-byte。默认回归网纯 host,无 device 依赖。
- **范围红线**:不实现 install/update/channel 切换(rustup 前端,08 §9 后续按档);不建 nightly channel;不触 registry/sumdb(D-312/SG-007 not_triggered);不触多后端(D-008/SG-003);零网络端点;第二 edition 不引入(RFC-0008 §8)。

## 7. Agent 批准

> **Approved — 2026-07-14**。agent 自主批准本 Mini-RFC（§2 形态 + §3 判档 + §4 错误码/快照联动 + §6 范围）并记录（AGENTS v3.0 硬规则 1;用户 2026-07-14 AskUserQuestion 裁决「最小 stable channel 清单」为范围输入,V1_CONTRACT §7 ④ 留痕）。条款先行(commit 序条款在前) / 快照重 bless / CI 步骤 50 真实红绿 / 合入均由 agent 自主签署。
