---
name: Mini-RFC 提案
about: 规范内 bugfix / 诊断措辞 / 内部开关 / 工具行为变更 / 规则文件(agents/AGENTS.md)级修改(10 §3 Mini-RFC)
title: "Mini-RFC: <一句话标题>"
labels: ["mini-rfc", "needs-triage"]
---

<!-- 先按 CONTRIBUTING.md「变更分档(三档门)」自助判定。Mini-RFC = 单页提案 +
     失败测试先行。若触及 unsafe / FFI ABI / 内存模型 / 安全包络 → 升 Full RFC。
     判档不清 → 向上取严(自我约束建议)。agent 完全自主。 -->

## 摘要

<!-- 要做什么、复用了什么既有面(语义 0-byte 请标注)。 -->

## 为何 Mini-RFC(而非 Direct/Full)

<!-- 非 Direct:触执行期新决策面 / 工具行为 / 规则文件级修改。
     非 Full:不触 UB / 内存模型映射 / FFI ABI / 安全包络禁区。 -->

## 下一步

- [ ] 复制 [`rfcs/TEMPLATE-MINI-RFC.md`](../../rfcs/TEMPLATE-MINI-RFC.md) → `rfcs/mini-NNNN-<kebab-title>.md`(取下一个未用 `MR-####`,见 [`rfcs/README.md`](../../rfcs/README.md) §5,永不复用)
- [ ] **失败测试先行**(10 §3 Mini 硬性:当前 main 上 RED)
- [ ] 单页提案(agent 自主批准)
- [ ] 开 PR(`ci/check_contribution.py` 阻断缺 provenance/条款号/验证)
