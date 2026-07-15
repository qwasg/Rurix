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
| 2026-07-14 | tests/stable/stable_api.snapshot 重 bless(spec_clauses 180→182,其余三段 0 变化) | V1.2 最小 stable channel 清单(Mini-RFC/MR-0008,agent Approved 2026-07-14;V1_CONTRACT §7 ④)加性条款 RXS-0185(channel 清单存在性·字段语义·确定性序列化)/ RXS-0186(同版号一致性判据 + Release 层第 8 子门 channel-manifest)入 spec/release.md §2.6 → `spec_clauses` 180→182。**RXS-0180 L2 加性演进**(同一 edition 内 stable 面只增不破坏):既有 RXS-0001~0180 条款 ID 与含义 0 变化,error_codes=88 / editions=["2026"] / rx_cli_subcommands=8 均不变(零新 RX 码/零 CLI 面变更)。RXS-0181~0184 已被 GRX showcase 分支 claim,跳号避撞(编号永不复用 10 §9.5)。命令:`RURIX_BLESS=1 py -3 ci/stable_snapshot.py` 重写后 `py -3 ci/stable_snapshot.py --check` PASS;`ci/edition_smoke.py` 篡改红绿闭合复绿。条款+实现+重 bless 同 PR(check_stable_snapshot_bless 守卫,步骤 49 硬红约束)。 | qwasg/白栀(agent 完全自主签署,AGENTS v3.0 硬规则 1;V1.2/G-V1-3) |
| 2026-07-14 | tests/stable/stable_api.snapshot 重 bless(spec_clauses 182→184,其余三段 0 变化) | post-V1 rurixup 工具链前端首切片(Mini-RFC/MR-0009,agent Approved 2026-07-14)加性条款 RXS-0187(工具链版本注册表与默认切换)/ RXS-0188(stable channel 消费与 install 内容寻址校验)入 spec/release.md §2.7 → `spec_clauses` 182→184。**RXS-0180 L2 加性演进**(同一 edition 2026 内 stable 面只增不破坏):既有 RXS-0001~0186 条款 ID 与含义 0 变化,error_codes=88 / editions=["2026"] / rx_cli_subcommands=8 均不变(零新 RX 码;rurixup 子命令 install/list/default 不进快照——快照只锚 rx CLI 子命令面)。命令:`RURIX_BLESS=1 py -3 ci/stable_snapshot.py` 重写后 `py -3 ci/stable_snapshot.py --check` PASS;`ci/edition_smoke.py` 篡改红绿闭合复绿。条款+实现+重 bless 同 PR(check_stable_snapshot_bless 守卫,步骤 49 硬红约束)。 | qwasg/白栀(agent 完全自主签署,AGENTS v3.0 硬规则 1;post-V1/MR-0009) |
| 2026-07-14 | tests/stable/stable_api.snapshot 重 bless(spec_clauses 184→192 + error_codes 88→95,其余两段 0 变化) | MS1.2 single-source 宿主 GPU 编排 std::gpu(Full RFC/RFC-0009,agent Approved 2026-07-14;MS1_CONTRACT §7 ⑤/G-MS1-1)加性条款 RXS-0189~0196(std::gpu 类型面/方法签名与元素推断/launch lowering 与 marshalling/single-source 嵌入与装载协商/运行期错误与 poisoned/rxrt C ABI 边界/extern 保名与 #[link]/out-of-line 模块)入新建 spec/host_orchestration.md → `spec_clauses` 184→192;新错误码 RX1005/RX2010/RX3015/RX6024/RX6025/RX7021/RX7022(en/zh 成对,bilingual 95/95)→ `error_codes` 88→95。**RXS-0180 L2 加性演进**(同一 edition 2026 内 stable 面只增不破坏):既有 RXS-0001~0188 条款 ID 与含义 0 变化,editions=["2026"] / rx_cli_subcommands=8 不变。RXS-0197~0199(present typestate/backbuffer blit/图像落盘桥)为本文件预留区间,随 MS1.2b 落体再次重 bless。命令:`RURIX_BLESS=1 py -3 ci/stable_snapshot.py` 重写后 `py -3 ci/stable_snapshot.py --check` PASS。条款+实现+重 bless 同 PR(check_stable_snapshot_bless 守卫,步骤 49 硬红约束)。 | qwasg/白栀(agent 完全自主签署,AGENTS v3.0 硬规则 1;MS1.2/G-MS1-1) |
