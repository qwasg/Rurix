# tests/ui/ — 诊断 golden 测试

> 通道:`src/rurixc/tests/ui_golden.rs`(契约 D-M1-4 / G-M1-2;14 §6 受控 bless)。
> 黄金路径 1 = 解析错误(07 §5 四路径之首,M1.4 起填充)。

## 形态(compiletest 风格)

- 每用例一对文件:`*.rx`(首行 `//@ spec: RXS-####` 锚定)+ `*.stderr`(snapshot)。
- `//~ ERROR RX####` 行注释:注释所在行必须产出同码 error 诊断,且数量一致。
- snapshot 内路径规范化为 `$DIR/...`,LF 行尾。

## 受控 bless(14 §6:审批动作,不是日常操作)

1. `RURIX_BLESS=1 cargo test -p rurixc --test ui_golden` 重写 snapshot;
2. **任何 `.stderr` 新增/修改/删除必须同 PR 在 [bless_log.md](bless_log.md) 追加一条审批记录**(既有行 0-byte),否则 `ci/check_guardrails.py` FAIL(M1 CI_GATES §4 第 6 项);
3. bless 记录须写明文件、理由与批准人。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v0.1 | 2026-06-11 | 占位 |
| v1.0 | 2026-06-11 | UI golden 通道落地(M1.4,D-M1-4) |
