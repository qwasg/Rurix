---
# 里程碑契约模板(从 M0 实例提炼,14 §1 四要素;M1 起复用)
# 用法:复制为 milestones/mX/MX_CONTRACT.md,填全部 <> 占位;
# 开工时 status: active;close-out 只追加 §8,既有条款 0-byte 修改。
contract: <MX>
title: <里程碑标题>
status: active            # active → closed
version: v1.0
date: <YYYY-MM-DD>
timebox: "<M+n(约 n 周)>"
rfc_required: <none | RFC 编号列表>   # 语义变更必须先有 RFC(10 §3)
upstream_docs:
  - "11 §3 (<MX> 定义)"
  - "<其他依据文档编号>"
in_scope:
  - <范围项标识符>
out_of_scope:
  - <排除项标识符>        # 与 deferred_refs 对应
deferred_refs: [<RD-###>]
deliverables:
  - id: D-<MX>-1
    name: <交付物>
acceptance_gates:
  - id: G-<MX>-1
    check: "<可脚本提取的验收判据,含数字与证据等级要求>"
guardrails:
  - "<字节级核对项(git diff 语义)>"
---

# <MX> 契约 — <标题>

> 所属:[../../11_ROADMAP.md](../../11_ROADMAP.md) §3 <MX> / 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1

---

## 1. 目标

<一段话:本里程碑结束时项目获得什么能力>

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | 对应交付物 |
|---|---|---|

### 2.2 out-of-scope(显式排除)

<逐项列出并引用 RD-### / SG-###;11 §2 红线不触碰>

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|

## 4. 验收门(完整版,YAML 头为可提取摘要)

<逐条展开;性能门必须注明证据等级(measured_local)与采样协议引用>

## 5. Guardrails(字节级,机器核对)

见 YAML 头 `guardrails` 字段。核对方式:`ci/check_guardrails.py <上一里程碑 close tag>`。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源,本表仅引用。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | <YYYY-MM-DD> | 初版契约固化 |

---

## 8. Close-out(只追加区 — 开工时为空)

<!-- 验收记录、guardrail 核对输出、deferred 继承/关闭记录追加于此;上方条款 0-byte 修改。 -->
