---
name: Full RFC 提案
about: 新语法 / 类型系统 / 运行时语义 / unsafe 边界 / FFI ABI / 内存模型映射 / 稳定化 / edition(10 §3 Full RFC)
title: "RFC: <一句话标题>"
labels: ["rfc", "needs-triage"]
---

<!-- 先按 CONTRIBUTING.md「变更分档(三档门)」自助判定。Full RFC 触发面 = 新语法 /
     类型系统变更 / 运行时语义 / unsafe 边界 / FFI ABI / 内存模型映射 / 稳定化 / edition /
     设计原则修改 / 死亡路线触碰。判档不清 → 向上取严,不自判 Direct。 -->

## 动机

<!-- 解决什么问题 / 落地哪条已锁决策(D-###) / 采纳判据。 -->

## 拟议范围

<!-- 大致设计方向 + 触及的禁区(UB / 内存模型映射 / FFI ABI / 安全包络须 owner 经 Full RFC 落笔)。 -->

## 下一步

- [ ] 复制 [`rfcs/TEMPLATE-RFC.md`](../../rfcs/TEMPLATE-RFC.md) → `rfcs/NNNN-<kebab-title>.md`(取下一个未用 `RFC-####`,见 [`rfcs/README.md`](../../rfcs/README.md) §5 编号台账,永不复用)
- [ ] **失败测试先行**(编码拟议意图,当前 main 上 RED)
- [ ] 下游 spec 条款映射(RXS-#### 续号;**条款 PR 先于实现 PR**)
- [ ] 开 PR,经 FCP-lite 评审窗(≥2/3 同意含语言负责人 + 5–7 天公开等待窗,见 `rfcs/README.md` §3)
