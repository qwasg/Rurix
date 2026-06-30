# stable API 快照 bless 审批记录(只追加)

> 任何 `tests/stable/stable_api.snapshot` 的新增/修改/删除必须同 PR 在本表追加一行
> (RD-008 stable API 快照冻结机制,G2.5 语言 1.0 激活;RFC-0008 §9 Q-RD008 /
> spec/edition.md RXS-0180;`ci/check_guardrails.py` `check_stable_snapshot_bless`
> 机器核对:既有行 0-byte)。bless 纪律对齐 UI/MIR/PTX/DXIL golden snapshot
> (`RURIX_BLESS=1 py -3 ci/stable_snapshot.py` 重写 + 本表追加留痕)。
>
> stable 面(RXS-0180):稳定语言面的**存在性 + 含义**——
> - `editions` / `edition_anchor`:合法 edition 值集与首个 edition 版本锚(src/rurix-pkg VALID_EDITIONS);
> - `rx_cli_subcommands`:rx CLI 广告的稳定子命令面(src/rx/src/main.rs USAGE);
> - `spec_clauses`:spec/*.md 全部 `### RXS-####` 条款 ID;
> - `error_codes`:registry/error_codes.json 错误码 ID → message_key(含义冻结,10 §6)。
>
> 🔒 **快照不冻结二进制 ABI**:register/字节布局/工具版本不进 stable(RXS-0180 L3,
> 对齐 RXS-0162 / RXS-0165 先例)。同一 edition 内 stable 面**只增不破坏**(RXS-0180 L2);
> 加性变更(新增条款/错误码/子命令)经本表追加行 bless,破坏性变更须经新 edition 隔离。

| 日期 | 范围 | 理由 | 批准 |
|---|---|---|---|
| 2026-06-30 | tests/stable/stable_api.snapshot 首份 bless（语言 1.0 stable 面基准，edition_anchor=2026） | RD-008 stable API 快照冻结机制经 G2.5 语言 1.0（首个 stable 发布触发点）**激活**（RFC-0008 §9 Q-RD008，agent 完全自主裁决，AGENTS v3.0 硬规则 1）。首份快照定型语言 1.0 stable 面基准：editions=`["2026"]` / edition_anchor=`2026` / rx_cli_subcommands=`[bench,build,check,doc,fmt,run,test,vendor]`（src/rx USAGE）/ spec_clauses=180（spec/*.md 全部 RXS-0001~0180）/ error_codes=N（registry/error_codes.json id→message_key，含义冻结 10 §6）。机制：`ci/stable_snapshot.py`（确定性重算 stable 面 + 比对，含 red 自检）+ `RURIX_BLESS=1` 重写路径 + `ci/check_guardrails.py` `check_stable_snapshot_bless` 守卫分支（镜像 UI/MIR/PTX/DXIL golden bless）。命令：`RURIX_BLESS=1 py -3 ci/stable_snapshot.py` 写快照后 `py -3 ci/stable_snapshot.py --check` PASS；`ci/edition_smoke.py` 篡改快照 → 比对红 → 复原绿（真实红绿）。🔒 快照仅锚定 stable 面存在性+含义，不冻结二进制 ABI（RXS-0180 L3）。RD-008 status open→closed（registry/deferred.json append-only）。 | qwasg/白栀（agent 完全自主签署，AGENTS v3.0 硬规则 1） |
