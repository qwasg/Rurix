# Mini-RFC MR-0009 — rurixup 本地工具链版本注册 + stable channel 消费(install / list / default 前端)

| 字段 | 值 |
|---|---|
| Mini-RFC 标识 | **MR-0009**（Mini-RFC 序列;独立于 Full-RFC 的 `RFC-####` 命名空间,编号永不复用,10 §9.5。MR-0006/0007 GRX showcase 分支 claim,MR-0008 = V1.2 channel 清单,续 MR-0009） |
| 标题 | rurixup 首个工具链管理前端切片:消费 stable channel 清单(MR-0008)的**离线、纯确定性** install / list / default——多版本注册 + 默认切换;**真实文件系统物化 + 网络拉取 defer(RD-025)** |
| 档位 | **Mini-RFC**（10 §3:工具行为变更;**不触** UB / 内存模型映射 / FFI ABI / 安全包络禁区,见 §3——纯 host 确定性状态,无网络端点、无真实 FS 物化、无 PATH/junction 活跃切换） |
| 状态 | Approved — 2026-07-14（agent 自主批准并记录,AGENTS v3.0 硬规则 1） |
| 承接里程碑 | **post-V1 / 无独立里程碑契约**（V1 已 close-out,post-V1 里程碑未定义;本 Mini-RFC 自承载,08 §9 D-241「rurixup = 工具链版本管理器(rustup/juliaup 模式),MVP 后期」locked 意图的首切片;直接兑现 MR-0008 §1「为未来 rustup 式前端预留的机器可消费锚点」的下游前端） |
| 关联条款 | 拟落 spec/release.md **RXS-0187 ~ RXS-0188**（延伸既有发布/分发语义面,不新建文件;RXS-0181~0184 GRX 分支占用跳号避撞,RXS-0185/0186 = MR-0008） |
| 依据决策 | D-241（rurixup 工具链版本管理器,08 §9 r6）· RXS-0135（原子安装与 content-tree 完整性,复用现有 `install.rs` kernel）· RXS-0185/0186（stable channel 清单,MR-0008)· 09 §7（用户产物分发指引） |
| Provenance | `Assisted-by: claude-code:claude-fable-5`。agent 自主决策,批准后推进下游 PR |
| 失败测试先行 | `ci/toolchain_frontend_smoke.py`（CI 步骤 51）+ `src/rurixup/src/toolchain.rs` 单测——引用拟新增能力;**当前 main 上 RED**（脚本与模块均不存在,rurixup 无 install/list/default 子命令);实现落地后转有意义拦截:①install 消费被篡改 bundle(digest 失配)应拒却受理即红;②`default` 指向未注册版号应拒却受理即红;③注册幂等/默认指针破坏即红 |

---

## 1. 摘要

MR-0008 落地了 stable channel 清单(`channel_manifest.json`),并明言它是「**为未来 rustup 式前端预留的机器可消费锚点**」,同时把 install/update/channel 切换前端 defer 到后续。本 Mini-RFC 兑现该前端的**首个切片**:让 `rurixup` 从 `rurixup release` 产出的 `channel_manifest.json` + `bundle.json` **消费 stable channel**,把版本**注册**进一个确定性工具链注册表(`toolchains.json`),支持**多版本共存 + 默认版本切换**——`rurixup install` / `list` / `default`。

**最大化复用**:安装完整性判据复用既有 `install.rs` 的 content-tree SHA-256 内核(RXS-0135,已 spec 化、已实现);channel 一致性判据复用 `channel.rs::consistent`(RXS-0186);内容寻址校验复用 `rurix_pkg::sha256`(RXS-0093)。**纯 host、纯确定性、零网络端点**——沿 rurixup 现有「纯函数 + 确定性 + `unsafe_code=deny`」纪律(lib.rs 头)。

**明确不做(defer)**:①**真实文件系统物化**(把工具链内容树写到磁盘版本目录 + 用 PATH/junction 切换活跃版本);②**网络拉取**(从 URL 下载 channel/bundle)。二者是真实 IO + 安全包络 + 网络端点面,单独 defer 为 **RD-025**,届时按 10 §3 判档(可能需 Full RFC);本切片只做**注册表逻辑 + 消费校验**,让 stable channel「可被机器登记与切换」而不触真实 IO/网络。

## 2. 设计（用户视角 + 形态）

`rurixup` 新增三子命令(既有 `release` 子命令 0-byte):

```
rurixup install --channel-manifest <path> --bundle <path> [--registry <path>]
                                       # 消费 stable channel → 校验 → 注册版本(幂等)
rurixup list [--registry <path>]       # 列出已注册版本 + 标注 default
rurixup default <version> [--registry <path>]   # 设默认版本(须已注册,否则退出 1)
```

`--registry` 缺省 `./toolchains.json`(确定性状态文件,镜像 `rurix.lock` 小状态文件先例)。核心逻辑落纯模块 `src/rurixup/src/toolchain.rs`:

```
InstalledToolchain ::= { version, content_digest }         // 一个已注册版本
ToolchainRegistry  ::= { installed: [InstalledToolchain], default: Option<version> }
ToolchainRegistry::install(&channel_manifest, bundle_json) -> Result<version, ToolchainError>
ToolchainRegistry::set_default(version) -> Result<(), ToolchainError>
ToolchainRegistry::list() -> &[InstalledToolchain]  /  default_version() -> Option<&str>
ToolchainRegistry::to_json() / from_json(&str)             // 确定性(版号字典序,无时间戳)
```

要点(→ 条款 RXS-0187/0188):

- **消费校验**(install):channel 清单须 `channel ∈ {stable}` + 与 bundle **一致**(`channel.rs::consistent` 判据,RXS-0186)+ `bundle_manifest_sha256` == 实测 `sha256(bundle_json)`(内容寻址校验,RXS-0093/0135 口径)。任一不符 → `ToolchainError`,不注册(全有或全无,对齐 RXS-0135 原子性)。
- **幂等注册**:同一 `(version, content_digest)` 重复 install = no-op(不重复入表);首个 install 自动成为 default。
- **默认切换**:`set_default(v)` 仅当 `v` 已注册,否则 `ToolchainError::UnknownVersion`(退出 1)。
- **确定性**:`installed` 按版号字典序序列化,**不含时间戳**——同一操作序列产逐字节一致 `toolchains.json`(镜像 RXS-0138/0185 确定性纪律)。
- **错误码**:**零新 RX 码**——`ToolchainError`(`ManifestInconsistent` / `DigestMismatch` / `UnknownVersion`)以工具层 Result + 退出码表达,沿 spec/release.md §3 rurixup 工具层口径。

## 3. 为何 Mini-RFC（而非 Direct，亦非 Full RFC）

- **非 Full RFC**:不触 AGENTS 硬规则 5 / 10 §7.5 禁区——本切片是**纯 host 确定性状态管理**(注册表逻辑 + 消费校验),无 UB、无内存模型映射、无 FFI ABI、无安全包络;**无真实 FS 物化、无网络端点**(二者 defer RD-025)。触及语言语义/类型系统者零。
- **非 Direct**:引入 `rurixup` 新子命令面 + 新工具链注册状态 = **工具行为变更**(10 §3 Mini-RFC 明列);且需落 spec 新条款(RXS-0187/0188)。对齐先例 MR-0002/MR-0005/MR-0008 同量级。
- **升档触发条件(实现期守卫)**:实现期若发现需真实 FS 物化(写磁盘 + PATH/junction 活跃切换)、网络拉取(URL 下载)、或触安全包络,则**停手 defer 至 RD-025**(不在本 PR 落笔真实 IO / 网络),届时按 10 §3 判档(可能 Full RFC)。

## 4. 错误码 / 影响 / 范围

- **零新 RX 码**:`ToolchainError` 工具层 Result + 退出码 1(用法/校验失败);`registry/error_codes.json` 与双语 messages 零追加(bilingual 88/88 不变)。
- **零新 unsafe**:纯 host 确定性(`unsafe_code=deny` 维持)。
- **stable 快照联动**:RXS-0187~0188 使 `spec_clauses` 182→184 → `tests/stable/stable_api.snapshot` 同 PR 重 bless + `bless_log.md` 追加(RXS-0180 L2 加性演进,同 edition 2026 内只增不破坏)。rurixup 子命令**不进快照**(快照只锚 `rx` CLI 子命令面,MR-0008 已确认),故 subcommands 段不变。
- **新 deferred RD-025**:真实 FS 工具链物化(磁盘版本目录 + PATH/junction 活跃切换)+ 网络拉取(URL 下载 channel/bundle)——真实 IO + 安全包络 + 网络端点面,defer 至硬需求出现时按 10 §3 判档。

## 5. 失败测试先行（10 §3 Mini 硬性）

- **路径**:`ci/toolchain_frontend_smoke.py`(CI 步骤 51,纯 host)+ `src/rurixup/src/toolchain.rs` 单测(`//@ spec: RXS-0187/0188`)。
- **编码意图**:stable channel 消费校验 + 多版本注册幂等 + 默认切换 + 确定性序列化。
- **当前 main 上 RED**:脚本与模块均不存在——`rurixup install/list/default` 未知子命令;无 `toolchains.json` 语义;篡改 bundle 的 install 场景**无逻辑可拦**(能力缺失即 RED)。
- **实现落地后转绿/转有意义拦截**:green(install→list→default→幂等 re-install)+ red→绿闭合(篡改 bundle digest → install 拒 exit 1;`default` 未注册版号 → exit 1;复原绿,反 YAML-only)。

## 6. 影响 / 向后兼容 / 范围

- **向后兼容**:纯追加——`release` 子命令与既有 5+1 类输出字节流 0-byte;新增 install/list/default 与 `toolchains.json` 全新面,无既有调用改动。默认回归网纯 host,无 device/网络依赖。
- **范围红线**:不做真实 FS 物化 / 网络拉取(RD-025 defer);不建 nightly/beta channel(合法集维持 `{stable}`,MR-0008);不触 registry/sumdb(D-312/SG-007);零网络端点;第二 edition 不引入(RFC-0008 §8)。

## 7. Agent 批准

> **Approved — 2026-07-14**。agent 自主批准本 Mini-RFC（§2 形态 + §3 判档 + §4 错误码/RD-025 defer + §6 范围）并记录（AGENTS v3.0 硬规则 1;直接兑现 MR-0008 §1 预留的 rustup 式前端锚点,08 §9 D-241 locked 意图）。条款先行(commit 序条款在前) / 快照重 bless 182→184 / CI 步骤 51 真实红绿 / 合入均由 agent 自主签署。真实 FS 物化 + 网络拉取显式 defer RD-025,不在本切片落笔。
