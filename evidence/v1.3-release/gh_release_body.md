# Rurix 1.0.0 — 首个 stable 发行

Rurix 是面向 NVIDIA GPU 纵深的实验性系统级 GPU 语言(Rust 风格所有权/借用 + kernel/着色阶段进语言 + CUDA PTX 与 D3D12/DXIL 双路径)。本发行为**语言 1.0 首个 stable 版本**(开源后第一个 LTS 质量版本,治理口径 10 §6/D-405)。

## 语言 1.0 stable 面承诺

| 构成 | 值 |
|---|---|
| spec RXS 条款 | **182**(RXS-0001~0186 区间内 182 条现存条款,FLS 体例,100% conformance 锚定) |
| 错误码 ID+含义 | **88**(含义冻结:可加不可改义,双语诊断) |
| edition | **"2026"**(首个 edition;同 edition 内 stable 面只增不破坏,破坏性变更只能走未来 edition) |
| rx CLI 子命令 | bench / build / check / doc / fmt / run / test / vendor |

- **稳定化依据**:[Stabilization Report](https://github.com/qwasg/Rurix/blob/main/milestones/v1/STABILIZATION_REPORT.md)(stable 面盘点 / 观察期判定 / 已知缺口诚实列举)· FCP-lite 公示(advisory,保持开放):https://github.com/qwasg/Rurix/issues/121
- **非 ABI 边界**:stable 快照锚定条款/错误码/edition/CLI 面的存在性+含义,**不冻结** register/字节布局/PTX·DXIL 产物形状/工具链版本(RXS-0180 L3)。
- **channel=stable 身份锚**(MR-0008):附件 `channel_manifest.json` 为发行渠道清单(确定性,bundle 内容寻址引用)。**注意:rurixup 尚无 install/update 前端**,清单为未来工具链前端预留的机器可消费锚点。

## 发布门(机器门,全绿)

本发行经 release workflow 全量硬门:Authenticode 签名验签 + SBOM 双视图(SPDX 2.3 + CycloneDX 1.5)+ NVIDIA Attachment A 白名单审计 + budget --strict + conformance / UI golden / 全 workspace 测试全绿。Run:https://github.com/qwasg/Rurix/actions/runs/29328321309

## ⚠️ 签名状态(诚实标注)

- 本 Release 附件二进制为**自签测试证书**的真实 Authenticode 签名(时间戳齐备)——**非生产证书**:Windows SmartScreen 会告警,证书链不被系统信任。
- of-record 生产签名(Azure Artifact Signing)维持 secret + 人工门控(spec/release.md §4),生产签名版待 owner 人工门放行后另行更新。
- 校验:附件 `SHA256SUMS` + `signing_manifest.json`(逐产物验签状态)+ `gate_decision.json`(发布门决策)。

## 许可与再分发

- 双许可 **MIT OR Apache-2.0**。
- NVIDIA 再分发组件仅 Attachment A 白名单最小集;本 Release 附件为语言本体(LanguageCore)分区,**不含** NVIDIA Toolkit/驱动/Nsight(许可红线 r6);EULA 白名单逐项法律核对状态 = `pending-human-review`(SBOM/审计附件可查)。

## 已知缺口(摘要)

mesh/task/RT 着色器 DXIL 降级(RD-012)、`#[export(c)]`(RD-009)、纹理采样 RFC-0007 子集外(RD-022/023/024)、bindless(RD-018)、窗口 present(RD-019)等为**加性未实现项**,详见 [Stabilization Report §4](https://github.com/qwasg/Rurix/blob/main/milestones/v1/STABILIZATION_REPORT.md)。反馈请评论 FCP-lite issue 或开新 issue。

---
*Provenance: Assisted-by: claude-code:claude-fable-5(agent 自主发布,治理留痕 V1_CONTRACT §7/§8)*
