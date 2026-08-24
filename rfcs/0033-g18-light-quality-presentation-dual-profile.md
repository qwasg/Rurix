# RFC-0033 — G18 光线画质表达纵深 + Presentation 双 Profile 出图协议

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0033（next_free=33） |
| 状态 | Agent Approved（决策程序） |
| 承接 | G18.2 M-a/M-b |

## 摘要

G18 在 G16plus GI 表达 18/18 达标基础上，纵深光线传输（天光/反射/软阴影/降噪）与 presentation 出图（夜/日双 profile、PNG 导出）走加性 profile，默认臂 Stage A digest 零漂移。

## 范围

- in: `g18_presentation_contract.json`、post_chain 接入、`--presentation-profile`、`--export-png`
- out: 修改 G13 冻结契约、默认 `--gi off` digest 锚
