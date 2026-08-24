# conformance/ — 语义验收测试

唯一语义验收边界(10 §4):每条 spec 条款 ≥1 测试锚定,traceability matrix 工具生成。

> 状态勘误(2026-08-24,只追加):原标题「(占位)」与「M1 起填充」为 M0 期快照——本目录自 M1 起已按波次持续填充,截至本日 spec `### RXS-####` 条款头共 389 个全锚定(实测 `py -3 ci/check_number_ledger.py` 输出「spec RXS 头 389 个零同号碰撞」;锚定源 = `conformance/**/*.rx` + `tests/ui/**/*.rx` + `src/rurixc`·`src/rurix-rt` 单测 `//@ spec` 注释行)。入库矩阵 = [`traceability_matrix.md`](traceability_matrix.md) / [`traceability_matrix.json`](traceability_matrix.json)(CRLF 字节纪律,生成器 `ci/trace_matrix.py`);新鲜度门禁 `py -3 ci/trace_matrix.py --check`(零未锚定/零幽灵锚定/入库矩阵与现状一致,blocking)。
