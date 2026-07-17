# RFC-0012 — rurixup 工具链真实分发（真实 FS 物化 + 活跃版本切换 + GitHub Releases 网络拉取 + 发布资产自动化）

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0012（4 位制，编号永不复用，10 §9.5） |
| 标题 | rurixup 工具链真实分发：把已校验 bundle 物化到磁盘版本目录、切换活跃版本、从 GitHub Releases 经四级内容寻址校验拉取签名 bundle，并建发布侧对称自动化——RD-025 兑现 |
| 档位 | **Full RFC**（10 §3：**真实 IO**(磁盘物化/原子落盘)+ **安全包络**(下载校验 fail-closed 信任链)+ **网络端点面**(本仓首段网络代码)——RD-025 backfill_condition 明记「按 10 §3 判档,可能需 Full RFC」；AGENTS 硬规则 5，判档争议向上取严 硬规则 8） |
| 状态 | **Draft（2026-07-16）**。§9 裁决 A~D（A 网络端点+信任根+载体 / B 活跃切换机制 / C 冷启动验收口径 / D bundle 自动发布确认）标 **owner-pending**，经 [milestones/ea1/OWNER_DECISION_PACKAGE.md](../milestones/ea1/OWNER_DECISION_PACKAGE.md) 呈 owner 勾选；**裁决 A、B 落地前本 RFC 不翻 Approved**（RXS-0216/0217 语义依赖 A，RXS-0215 依赖 B），不推进 EA1.1 实现 PR；C/D 可后置 pending（ODP §3.4 部分落地协议）。agent 起草并摊清，不预设裁决结果 |
| 承接里程碑 | EA1（[milestones/ea1/EA1_CONTRACT.md](../milestones/ea1/EA1_CONTRACT.md)，验收门 G-EA1-1 ~ G-EA1-8；裁决 A gate EA1.1b 网络面） |
| 关联条款 | 拟落 spec/release.md **延伸 §2.8**（G1.5/MR-0005「延伸 spec/release.md」先例），条款 **RXS-0214 ~ RXS-0219**（区间随条款数定，见 §5）；**零裸条款头**——条款体随 EA1.1a/1.1b/1.2 实现 PR 同落，脚手架与本 RFC 不动 spec/ |
| 依据决策 | **RD-025**（兑现对象：MR-0009 defer 的真实 FS 物化 + PATH/junction 切换 + URL 下载；backfill_condition =「网络拉取若引入须先裁 D-312 相关面」→ §9 Q-A）· **D-312**（包 registry sumdb 待决——本 RFC **拟**窄裁「工具链分发非 registry 激活」呈裁决 A，SG-007 现状维持 not_triggered）· D-308/D-309（包管理 MVP 无 registry / 无 build.rs——供应链姿态一致性）· MR-0009（工具链前端首切片范围红线，本 RFC 解除其 defer）· RXS-0135~0139 / RXS-0185~0188（既有分发条款底座：原子分发内容树 / 分离打包 / 签名 / SBOM / hard-block 发布门 / channel 清单 / 一致性 / 注册表 / 内容寻址消费——**全部只增不破坏**）· 12 R-202（供应链事故红线）· D-406 v2.0（agent 自主默认；裁决 A~D 为 RD-025 契约性前置 + outward-facing 惯例的 carve-out） |
| Provenance | `Assisted-by: claude-code:claude-fable-5`。agent 起草并把裁决 A~D 摊清呈 owner；拟裁全程标「拟」 |
| Agent 批准 | **未批准（Draft）**。翻 Approved 前置 = 裁决 A、B 经 OWNER_DECISION_PACKAGE 落地（裁决落地小 PR 内同步回填 §9 + 本表状态行 + §4.7 演练版号/Release 创建形态定案，agent 代录）；若裁决 A 为「触 D-312」，本 RFC 网络面章节（§4.4/§4.5/§4.7 网络半程）标 blocked，本地面（§4.1~§4.3）可独立 Approved 推进（EA1 按 ODP §1-A 备选后果收窄） |

---

## 1. 摘要

把 rurixup 从「纯确定性账面注册表」（MR-0009 首切片，RXS-0187/0188）升级为**真实分发闭环**，四件事：

1. **真实 FS 物化**：已校验 bundle 内容树写入磁盘版本目录，staging→全量校验→**同卷单次 rename** 原子提交，断电零半装（RXS-0214）。
2. **活跃版本切换**：拟 shim 目录一次入 PATH、按 argv0 干名转发 default 版本（裁决 B；junction 为备选 §7），切换 = 确定性注册表单写（RXS-0215）。
3. **网络拉取**：从本仓 GitHub Releases 经系统 curl.exe（固定参数集、https-only）拉取 channel/bundle/组件，**四级内容寻址信任链**（repo 锚 → channel 清单 → bundle 清单 → 组件字节）任一级失配 fail-closed 拒装（裁决 A；RXS-0216/0217）。
4. **发布侧对称自动化**：release.yml 全部 hard-block 门后构建 3 组件真发布件（rx.exe + rurixup.exe + **crt-static rurix_rt_cabi.lib**——v1.0.0 资产缺此件，无 Rust 环境时含 GPU 面的 `rx build` 必死，本期必修）、SHA256SUMS、`gh release upload`、上传后回读自校验（上传自动化 = 裁决 D;RXS-0218）、信任根登记 PR 流（登记流本体属裁决 A 的信任根面,D 只确认上传自动化——部分落地时以 A 为准）;冷启动 <10min 两段式 measured 验收（裁决 C;RXS-0219）。

```
[发布侧 release.yml,八门全绿后]                    [用户侧 rurixup]
build 3 组件(--release + crt-static .lib)
 → 自签 Authenticode(如实 selftest)                channels/stable.json(repo 锚,owner 合并 PR 入库)
 → rurixup release(8 门 hard-block)                  │ ① 锚:channel_manifest_sha256
 → SHA256SUMS + gh release upload                    ▼
 → 回读自校验(逐资产 digest,失配 job 红)       channel_manifest.json ←② bundle_manifest_sha256
 → 信任根登记 PR(owner 合并 = 人工门)               ▼
                                                  bundle.json ←③ 逐组件 sha256
                                                     ▼
                                                  rx.exe / rurixup.exe / rurix_rt_cabi.lib
                                                     │ staging → ④ tree_digest 全量校验 → rename
                                                     ▼
                                          %USERPROFILE%\.rurix\toolchains\<ver>\{bin,bin\lib,manifests}
                                                     │ toolchains.json(default 单写切换)
                                                     ▼
                                          %USERPROFILE%\.rurix\bin\rx.exe(shim,一次入 PATH)
```

**范围锁定**：单一逻辑分发端点（本仓 GitHub Releases;host 白名单三个——github.com / objects.githubusercontent.com / raw.githubusercontent.com,见 §5 RXS-0216）、单 channel（stable）、无镜像/代理/断点续传/自更新（§8，超界登 RD-033+）；rurix-pkg 侧（包生态）**零网络代码不变**——本 RFC 分发的是工具链本体，**拟**定性为非 D-312 registry 激活（裁决 A 待裁）。

## 2. 动机

**发行与可获得性脱节。** v1.0.0 已于 2026-07-14 正式发行（tag + GitHub Release + channel 清单),但外部用户今天获得 Rurix 的唯一路径仍是 clone 整仓 + `cargo build --workspace`（guide/00_install.md:19-30）——发行动作的外部价值为零兑现。01 §4 图景 3 承诺「从 `rurixup install` 到第一个 kernel 跑出 Nsight 时间线少于十分钟」（:70)；本 RFC 操作化其 install→首 kernel 段（Nsight 时间线段诚实标注为后续,不充数)。

**RD-025 的 backfill_condition 已触发。** 「出现『需真实磁盘物化 + 活跃版本切换』或『需远程 channel 拉取』的硬需求时接通」——EA1 立项（owner 2026-07-16 拍板）即该硬需求。既有底座万事俱备只欠 IO：channel 清单确定性序列化与一致性判据（RXS-0185/0186）、内容寻址校验（RXS-0188）、`atomic_install` 原子内核（install.rs:70-96，**内存**提交）、Authenticode 红绿链（m8.4）、八门 hard-block（RXS-0139+0186）全部在位；缺的恰是磁盘面、网络面与发布上传自动化。

**为何 Full RFC**：① 真实 IO（磁盘版本目录、原子落盘、断电语义）；② 安全包络（本仓第一次为外部用户建下载信任链，失败模式必须穷举 fail-closed）；③ 网络端点面（本仓第一段网络代码,且 RD-025 明记须先裁 D-312 相关面）。任一均达 Full RFC 门；三者并触,取严（硬规则 8）。

## 3. 指导级解释（用户视角）

```console
# 首次(bootstrap):从 Release 页下载 rurixup.exe(TLS + 手动核对 SHA256SUMS——诚实空窗,见 §4.5)
$ rurixup install 1.1.0        # 拉锚→channel→bundle→组件,四级校验,物化,注册
RURIXUP_INSTALL: version=1.1.0 channel=stable default=1.1.0 registered=1 components=3 digest_levels_verified=4 installed=%USERPROFILE%\.rurix\toolchains\1.1.0
# 既有字段(version/channel/default/registered)0-byte 保留,新字段尾部纯追加(RXS-0187 语义只增)
$ rurixup setup --add-path     # 显式 opt-in;缺省只打印 PATH 指令(裁决 B 拟案)
$ rx check hello.rx            # 经 shim 转发到 default 版本
$ rurixup list                 # 各版本 + (default) + --verify 重哈希核完整性
$ rurixup default 1.1.0        # 切换 = 注册表单写,已开 shell 即时生效
# 离线/隔离环境:
$ rurixup install 1.1.0 --from-dir D:\bundles\1.1.0     # 本地源,同一校验与物化路径
$ rurixup install 1.1.0 --expect-digest <64hex>          # 手动注锚(异构分发场景)
```

失败即拒、拒即干净：任何一级 digest 失配/截断/协议降级 → 非零退出 + `RURIXUP_INSTALL_ERROR: kind=integrity` + staging 清理 + toolchains/ 与注册表 0-byte。没有部分安装态。

## 4. 参考级设计

### 4.1 磁盘布局与 RURIX_HOME（RXS-0214 承载）

```
%USERPROFILE%\.rurix\            # 根,可由 RURIX_HOME 覆盖(测试缝 + 多用户)
├── bin\                         # shim 目录(一次入 PATH;裁决 B)
├── toolchains\<version>\
│   ├── bin\rx.exe  bin\rurixup.exe
│   ├── bin\lib\rurix_rt_cabi.lib  # 落点 = driver.rs:878-885 既有探测路径 <exe目录>\lib\(current_exe().parent().join("lib")),rurixc 零改动
│   └── manifests\{channel_manifest,bundle,sbom.spdx,sbom.cdx,signing_manifest}.json
├── tmp\.staging-<ver>-<nonce>\  # 与 toolchains 同卷 ⇒ rename 原子
└── toolchains.json              # 注册表(schema_version 1→2)
```

组件名 → 相对路径映射 = 确定性规则（`*.exe → bin/`、`*.lib → bin/lib/`、NvidiaRedist 分区 → `nvidia/`）,不给 Component 加 path 字段（schema 0-byte;组件面仅 3 件,规则一屏可审）。`.lib` 落 `bin\lib\` 而非兄弟 `lib\` 是**刻意对齐 driver.rs 既有探测语义**(`current_exe().parent().join("lib")` = exe 自身目录下的 lib 子目录)——shim 转发 spawn 的是 `toolchains\<ver>\bin\rx.exe`,其 current_exe 即该 bin 目录,探测命中,rurixc 零改动。

### 4.2 staged→rename 原子物化 + tree_digest 不变量（RXS-0214）

1. 全部组件落 `tmp\.staging-*`（toolchains\ 与注册表未触碰）;
2. **全量校验**:逐文件 sha256 == bundle 声明;同时计算 **`tree_digest = content_tree::hash_entries(components → (rel_path, sha256))`**——从 bundle.json 可预算、从磁盘经 `content_tree::collect_dir` 重哈希可复算的单一期望值,喂给 `atomic_install` 同款「期望==实测 → 一次性提交 / 失配 → 回滚」判据（**内核语义原样上盘,RXS-0135 复用而非重写**）;
3. **提交 = 同卷单次目录 rename**（staging → `toolchains\<ver>`)——提交点唯一,无逐文件半拷贝态;
4. 写 toolchains.json（先写 `.tmp` 再 rename,同样原子）;
5. **断电语义 = 「版本目录只经 rename 诞生」不变量**:staging 残留下次运行按名例清孤儿;rename 后注册前断电 → install 幂等重跑 `collect_dir` 重校验,匹配即补注册（修复而非报错）。

**注册表 schema v2**:`InstalledToolchain` 增 `install_path` + `tree_digest`（`toolchains.json` schema_version 1→2）;v1 旧条目（无路径账面项）读入标 `registered-only`,list 如实区分,不静默升格。`rurixup list --verify` 经重哈希标注 corrupted 条目并拒作 default 目标。

### 4.3 活跃版本切换（RXS-0215 承载;**裁决 B**,拟 shim）

`%USERPROFILE%\.rurix\bin\rx.exe` = rurixup.exe 的一份拷贝;rurixup main() 起始按 `current_exe()` file_stem 判定:≠"rurixup" → 代理模式,读 toolchains.json default → spawn `toolchains\<ver>\bin\<stem>.exe`（stdio inherit,退出码逐位透传;spawn 目标恒在 toolchains\ 下,防自递归）。切换 = 确定性注册表 JSON 单写:原子、免特殊权限、已开 shell 即时生效。代价 = 每调用一跳进程（毫秒级）+ 自更新换 shim 文件问题（defer RD-033+,§8）。PATH 接入默认**只打印指令**,`rurixup setup --add-path` 显式 opt-in（外呼 PowerShell `[Environment]::SetEnvironmentVariable(...,'User')`,免 setx 1024 截断坑)。junction 备选见 §7;若裁决 B 落地为 junction,本节与 RXS-0215 按 ODP §1-B 备选路径改写并修订留痕。

### 4.4 网络拉取载体（RXS-0216 承载;**裁决 A 子项**,拟系统 curl.exe）

固定参数集（条款锚定,逐参数可审）:

```
curl.exe --fail --silent --show-error --location
         --proto =https --proto-redir =https --max-redirs 5
         --max-time <n> --output <staging路径> <url>
```

- TLS 信任/代理/吊销全部委托 OS（schannel;Windows 10 1803+ 自带 curl.exe）——与本仓「重外部能力经受控外呼」既有模式同构（rurixc 外呼 clang/link.exe/ptxas,签名外呼 PowerShell）。
- `--proto-redir =https` 封死降级重定向;github.com → objects.githubusercontent.com 正常跳转保留。
- curl 只负责搬运,**完整性判定不信任传输层**——截断/部分下载由下游 digest 校验兜底（§4.6 #4）。
- curl.exe 缺失（spawn 失败）/非零退出（离线/DNS/TLS/HTTP≥400）→ 工具层错误 + stderr 透传 + `--from-dir` 手动路径指引。
- **协议例外唯一豁免**:host = 127.0.0.1 且 env `RURIXUP_TEST_ALLOW_LOOPBACK_HTTP=1` 时放行 `http://`（hermetic CI fixture 用;与契约 guardrail/CI_GATES 口径逐字对齐）,缺省 fail-closed 拒绝。
- 否决 FFI(WinHTTP/schannel——破 rurixup `unsafe_code=deny`)与外部 crate(reqwest——破零第三方依赖),见 §7;若裁决 A 改选 FFI,登记 unsafe-audit U29。

### 4.5 四级内容寻址信任链（RXS-0217 承载;**裁决 A**)

**repo 锚 `channels/stable.json`**（确定性 JSON,无时间戳）:

```json
{ "schema_version": 1, "channel": "stable",
  "releases": [ { "version": "1.1.0",
                  "channel_manifest_sha256": "<64hex>",
                  "base_url": "https://github.com/<owner>/<repo>/releases/download/v1.1.0/" } ],
  "latest": "1.1.0" }
```

锚获取通道三条:① `--channel-file <本地路径>` ② clone 仓内相对路径 ③ `https://raw.githubusercontent.com/<owner>/<repo>/main/channels/stable.json`（TLS）。另设 `--expect-digest <64hex>` 直接注锚。

**校验级联**（任一级失配 = 拒装/清 staging/退出 1/系统 0-byte）:

| 级 | 校验 | 代码现状 |
|---|---|---|
| ① 锚→channel | sha256(channel_manifest.json 字节流) == 锚声明 | **新增** |
| ② channel→bundle | sha256(bundle.json 字节流) == 清单声明 `bundle_manifest_sha256` | 既有（main.rs:95-100,RXS-0188） |
| ③ 一致性 | `channel::consistent`（channel 合法/版号/组件全集逐项） | 既有（RXS-0186） |
| ④ bundle→组件 | 逐组件 sha256 + tree_digest 全量判据 | 既有口径上盘（RXS-0135/§4.2） |

`latest` 解析自 repo 锚而非服务端 → 降级攻击(回喂旧版)在 ① 即失配。**无锚版本拒装**（锚 PR 合并前的过渡窗,如实文档化）。

**为什么 repo=信任根是诚实的**:用户信任 Rurix 源码即信任该 repo——clone 自建与下载安装同一信任面;不新增服务端信任设施(无 index/无 sumdb/无账号体系);digest 在版本控制历史里可审计。rustup 信任根(static.rust-lang.org 清单 over TLS)本质相同。

**诚实边界(三句,进条款与用户文档)**:
1. **Authenticode 现状 = 自签测试证书,对外部用户信任贡献为零**——生产签名(Azure,secret+人工门控,spec/release.md §4 禁区)接通前它只是纵深;signing_manifest `backend=selftest` 如实展示,不伪装。
2. **bootstrap 空窗**:用户首次获取 rurixup.exe 本身只有 TLS + 手动核对 SHA256SUMS 两道保护(与 rustup-init.exe 同构),文档如实写明,不假装闭环。
3. **威胁模型边界**:digest 链防「登记后资产篡改/传输篡改」,**不防 repo 级失陷**(锚与 Release 资产同一仓库权限域,非独立信任域);对冲 = 锚 PR owner 合并人工门 + git 历史可审计;离线签名的独立信任域价值留 D-312 真触发时重议(§7)。

### 4.6 失败模式表（RXS-0217 判据源,hermetic 红绿全覆盖)

| # | 失败模式 | 检出点 | fail-closed 行为 |
|---|---|---|---|
| 1 | channel 清单篡改/错版 | 级① 锚 digest 失配 | 拒装,清 staging,退出 1,注册表 0-byte |
| 2 | bundle.json 篡改 | 级②(既有) | 同上 |
| 3 | 组件 exe/lib 篡改 | 级④ 逐组件 sha256 | 同上(**任一组件失配 = 全量拒**,无部分安装) |
| 4 | 部分下载/截断 | 级④ 兜底(不信 content-length) | 同上 |
| 5 | 降级攻击(端点回喂旧版) | latest/URL 派生自 repo 锚 → 级① 失配 | 拒 |
| 6 | 重定向劫持/协议降级 | `--proto-redir =https` + digest 兜底 | curl 拒 或 digest 失配拒 |
| 7 | 离线/DNS/TLS 失败 | curl 非零退出 | 诚实错误 + 系统 0-byte + `--from-dir` 指引 |
| 8 | 磁盘满 | staging 写/rename 失败 | 错误 + 尽力清 staging;注册表未写 → 无半装 |
| 9 | 下载/物化中途断电 | staging 残留 + 注册表无该版 | 下次运行清孤儿;版本目录只经 rename 诞生 |
| 10 | rename 后注册前断电 | 目录在、注册表无 | 幂等重跑:collect_dir 重校验匹配即补注册 |
| 11 | 已装目录事后损坏 | `list --verify` 重哈希 | 标 corrupted,拒作 default 目标 |

### 4.7 发布侧对称自动化（RXS-0218 承载;**裁决 D**)

release.yml 既有八门(RXS-0139 七子门 + RXS-0186 第 8 门)与触发器(`v[0-9]+.[0-9]+.[0-9]+*`)**0-byte 沿用**,全绿后追加:

1. 真发布件构建:`cargo build --release -p rx -p rurixup` + crt-static `rurix-rt-cabi`(复用 driver.rs:904 同款命令行);
2. 自签 Authenticode(现状如实 selftest;生产签名门控不动);
3. `rurixup release` 以 **3 真组件**编排(全 LanguageCore;bundle 组件数 2→3 为新版本清单内容——v1.0.0 为 rx.exe+rurixup.exe 两件,老版本清单不动);
4. SHA256SUMS(固定字典序,同源两次逐字节一致);
5. Release 对象与资产上传:**Release/tag 一律由 run 内部 `gh release create` + `gh release upload` 以 `${{ github.token }}` 创建**——GITHUB_TOKEN 产生的事件不触发新 workflow,天然免二次触发;推真 tag 的正式流中 Release 对象同样由 run 内创建(推 tag 本身不产 Release);**禁止手工推演练 tag 或以 PAT 建 Release**(会再触发 tag 流与 dispatch run 抢资产——`v1.0.1-dist.1` 型 tag 会被触发器 `v[0-9]+.[0-9]+.[0-9]+*` 匹配,`*` 吞掉 `-dist.1` 后缀,此坑显式记录);
6. **上传后回读自校验**:curl 下载刚上传资产 → 逐件 sha256 复核 == bundle.json → 失配即 job 红(分发通路的真 HTTPS 红绿,每次发布必跑);
7. **信任根登记流**:生成 `channels/stable.json` 新条目 → 自动开 PR → **owner 合并 = 人工门**(每次发布用户可安装前 owner 须合一个 PR;合并前该版本处于「已发布未登记」过渡态,rurixup 拒装,文档写明)。

资产布局 = **裸文件 + SHA256SUMS 共 11 件**(现 10 件 + rurix_rt_cabi.lib):每资产字节 == bundle.json 组件 digest 的对象,一比一内容寻址无第二 digest 域;zip 备选见 §7。**首次演练走 workflow_dispatch**——注意「防误发」只防 tag 误触发,**演练产物即首个可安装公开资产**:演练以独立 pre-release 版号发布(拟 `v1.0.1-dist.1` 型,精确形态随裁决 D 落地定案回填本节),不覆写 v1.0.0 资产;演练失败的资产/Release 删除重来(GitHub Release 资产可变,与 evidence 只增纪律不冲突——evidence 记录尝试历史)。

### 4.8 测试三缝与 hermetic 红绿

1. **`--from-dir` 本地目录源**(主测缝,纯离线):staging/校验/物化/切换/回滚全逻辑复用同一路径;CI 步骤 59 前半跑此缝。
2. **环回 HTTP 例外**(见 §4.4):CI 步骤 59 后半以 Python `http.server`(stdlib,零第三方)起本地 fixture,真跑 curl 通路,可注入篡改/截断;**pr-smoke 零真实外呼**。
3. **真 HTTPS 端到端**只在 release.yml 回读自校验(每次发布必跑) + 冷启动 e2e 取证(evidence 面);pr-smoke 侧诚实 `[SKIP] real-network E2E → release workflow` token,不伪绿。

### 4.9 诊断与机器 token(零新 RX 码)

全部新错误(网络/digest/FS)由 rurixup 独立工具产生,rx.exe/rurixc 无一涉及——spec/release.md §3「确需编译器侧诊断才升档」触发条件不成立,**维持工具层 Result + 退出码**(0 成功/1 用法·IO·完整性/2 发布阻断,既有口径)。脚本可判性 = 机器 token 行纯追加:`RURIXUP_INSTALL: ...`(既有)+ **`RURIXUP_INSTALL_ERROR: kind=<integrity|network|io|usage>`**(新增)。条款 PR 顺手修正 §3 过期取号文字(「RX7021 起」→「RX7023 起」——RX7021/7022 已被 MS1 消费)。

### 4.10 冷启动验收协议(RXS-0219 承载;**裁决 C**,拟两段式)

- **A 段(VM,零预置证明)**:干净 Win11 VM 无 Rust/LLVM/VS;T0 = 文档首条命令(下载 rurixup.exe),T1 = `rx check hello_kernel.rx` 退出 0(含全部下载/校验/物化/PATH;`rx check` 零外部工具链依赖——driver.rs `--emit=check` 在 codegen 前返回)。判据 T1−T0 ≤ 10 min。
- **B 段(开发机干净用户账户,GPU 真跑证明)**:新建本地账户,用户环境无 Rust 工具链;系统级 LLVM 22.1.x + VS Build Tools + NVIDIA 驱动为**文档化前置,不计时**(rustup 同类口径,README/RFC 显著位诚实标注);T1 = 首 kernel device 真跑退出 0。判据 ≤ 10 min。样例 kernel 避 `__nv_*` 数学(免 libdevice,§8)。
- **不跨机加总**(物理上不可比);**计时重测 ≤3 次、全部尝试入 evidence、取 median**,超限如实记录走本 RFC §9 修订留痕,不静默放宽(反 measurement-shopping)。
- evidence:`evidence/ea1_install_e2e_<yyyymmdd>_<segment>.json` + schema(`milestones/ea1/install_e2e_evidence_schema.json`):`{segment, host{os,cpu,gpu,driver}, toolchain_version, t_start, t_end, duration_s, steps[{name,cmd,exit,duration_s}], digest_levels_verified, bytes_downloaded, bandwidth_note, attempt, pass}`;measured_local,**不进 CI 硬门**(带宽波动必 flaky);01 §4 图景 3 的 Nsight 时间线段标注为后续,不充数。

## 5. 下游 spec 条款映射(spec diff,10 §3 要件;全部落 spec/release.md 延伸 §2.8,随实现 PR)

| 条款 | 标题(草案) | 一句判据 | 测试锚定计划(每条 ≥1 `//@ spec:`) | 落地 PR |
|---|---|---|---|---|
| RXS-0214 | 真实 FS 物化与原子落盘 | 版本目录仅经「staging 全量校验→同卷单次 rename」诞生;任一校验失败 staging 不落 toolchains\、注册表 0-byte;tree_digest 双向独立复算必相等 | rurixup 单测(staging 失配回滚/断电孤儿/幂等补注册)+ 步骤 59 前半篡改 RED | EA1.1a |
| RXS-0215 | 活跃版本切换 | 机制按裁决 B(拟 shim:argv0 干名转发 default 同名 exe,退出码逐位透传);切换 = 确定性注册表单写,无系统状态改动;探针判据机制中立 | rurixup 单测(argv0 分派/default 单写)+ 步骤 59 切换探针与错向 RED | EA1.1a |
| RXS-0216 | 网络拉取载体与端点约束 | 拉取仅经 curl.exe 固定参数集,https-only(`--proto =https --proto-redir =https`),端点 host 白名单 = {github.com, objects.githubusercontent.com, raw.githubusercontent.com};唯一例外 = 环回 127.0.0.1 + 显式测试 env,缺省 fail-closed | rurixup 单测(参数集构造/默认态拒 http)+ 步骤 59 后半无 env 拒 http RED | EA1.1b |
| RXS-0217 | 下载校验 fail-closed 信任链 | 四级内容寻址任一级失配 → 拒装/清 staging/退出 1/零半装;无锚版本拒装;§4.6 表 11 条失败模式全 fail-closed | rurixup 单测(四级各失配/无锚拒装)+ 步骤 59 后半 hermetic 四路 RED | EA1.1b |
| RXS-0218 | 发布资产上传自动化与回读自校验 | 上传仅在八门全绿后;Release/tag 由 run 内 github.token 创建;回读逐资产 digest 复核失配即 job 红;信任根条目经 PR 门控入库 | 步骤 60(打包确定性/digest 闭环/3 组件完备)+ release.yml 演练 run | EA1.2 |
| RXS-0219 | 端到端安装时长(measured) | 两段式协议(§4.10,口径按裁决 C)各段 wall-clock ≤10min,evidence 按 schema 归档,measured 非 estimated | evidence schema 校验单测 + e2e evidence 实档(check_schemas 路由) | EA1.2 |

stable 快照随各条款 PR 加性重 bless(209→211→213→215,bless_log 同 diff,步骤 49 硬红不可分 PR)。

## 6. gate / tracking / 实现序(10 §3 要件)

- **无语言 feature gate**(rurixup 为独立工具非语言语义;既有 install/list/default/release 子命令语义**只增不破坏**——`--from-dir` 保留一等公民,纯账面 v1 注册条目兼容读入)。
- **实现序(栈式,各 PR 真实红绿)**:① 裁决落地小 PR(ODP §0 勾选 + 契约 §7 + 本 RFC §9 回填 + 翻 Approved)→ ② EA1.1a 条款 RXS-0214/0215 + FS 物化/切换 + 步骤 59 前半 → ③ EA1.1b 条款 RXS-0216/0217 + curl 拉取/四级校验 + 步骤 59 后半(hermetic)→ ④ EA1.2 条款 RXS-0218/0219 + release.yml 延伸 + 步骤 60 + workflow_dispatch 演练 + 信任根首 PR + 冷启动 e2e 取证。
- **失败测试先行声明**(10 §3):本 RFC 合入时点,`ci/rurixup_dist_smoke.py`、`ci/release_bundle_smoke.py`、rurixup 真实 IO/网络代码、`channels/` 目录在 main 上均不存在 = RED。
- tracking:EA1_CONTRACT G-EA1-1~8;RD-025 close-out 处置(关闭或收窄余项另立 RD-033+)。

## 7. 备选方案(为什么不)

- **junction `active` 链接切换**(RD-025 记载措辞):rmdir+mklink 两步窗口 + mklink 须外呼 cmd.exe;切换非单写原子。保留为裁决 B 备选,若裁定则 §4.3/RXS-0215 按 ODP §1-B 路径改写。
- **手写 ed25519 签清单**:签名实现错误 = 静默信任伪造(风险类别劣于 digest fail-closed);私钥引入 secret 保管面。独立信任域价值承认,留 D-312 真触发(透明日志/sumdb)重议。
- **FFI(WinHTTP/schannel)载体**:破 rurixup 唯一 crate 级纪律 `unsafe_code=deny`,TLS 状态机审计面远超收益;裁决 A 若选此登记 U29。
- **外部 HTTP crate(reqwest 等)**:传递依赖树破零第三方纪律(手写 SHA-256 是为此付过的成本)。不呈选项(ODP 已排除清单)。
- **纯 PATH 改写切换**:setx 1024 字符截断 + 已开 shell 不生效。不呈选项。
- **zip/tar 归档资产**:引入「归档字节 vs 内容字节」双 digest 域(zip 还带时间戳破确定性);~15MB 级产物压缩收益不关键。裸文件维持;若资产件数增长再议(RD-033+)。

## 8. 不做(范围红线;超界按 14 §4 登 RD-033+ 再议,不静默扩)

- rurixup **自更新**(shim 占用换文件问题)——手动重下载覆盖,登记 defer;
- **stable 外 channel**(beta/nightly 语义)、**多端点/镜像/代理/断点续传**;
- **包 registry/sumdb**(D-312/SG-007 维持 not_triggered;rurix-pkg 零网络不变);
- **生产签名(Azure)接通**(spec/release.md §4 禁区维持,secret+人工门控);
- **libdevice.10.bc 随包分发**(RXS-0136 白名单虽允许,EULA 签署面维持 pending-human-review;验收样例避 `__nv_*` 即可,§9 Q8);
- **Linux/macOS 安装器**(Windows 首期;跨 OS 面随后续多平台期);
- **Nsight 时间线集成**(01 §4 图景 3 后半句,诚实后续)。

## 9. 未决问题 / 关键裁决

**owner-pending(经 [OWNER_DECISION_PACKAGE.md](../milestones/ea1/OWNER_DECISION_PACKAGE.md) §0 勾选,裁决落地小 PR 回填本表)**:

| # | 问题 | agent 拟裁 | 状态 |
|---|---|---|---|
| Q-A | 网络端点+信任根+载体(gate EA1.1b;RD-025「先裁 D-312 相关面」兑现点) | 单端点本仓 Releases;repo 锚 `channels/stable.json` 四级 digest 链;curl.exe 固定参数集;**拟**定性非 D-312 激活,SG-007 现状维持;锚 PR owner 合并人工门 | **owner-pending** |
| Q-B | 活跃切换机制(偏离 RD-025「PATH/junction」记载措辞) | shim 目录 + argv0 转发;junction 降 §7 备选 | **owner-pending** |
| Q-C | 冷启动 <10min 验收口径 | 两段各 ≤10min measured(A 段 VM 到 rx check 含下载;B 段干净账户 GPU 真跑,系统级前置文档化不计时);重测 ≤3 次全入 evidence 取 median | **owner-pending** |
| Q-D | bundle 随 semver tag 自动发布公开资产(一次性确认) | 认可;首次演练 workflow_dispatch,演练产物即首个可安装资产(pre-release 版号,§4.7) | **owner-pending** |

**agent 拟裁自主项(D-406 v2.0;owner 可 veto,实现 PR 定案回填)**:

| # | 问题 | 拟裁 |
|---|---|---|
| Q5 | PATH 接入 | 默认只打印指令;`rurixup setup --add-path` 显式 opt-in |
| Q6 | 注册表 v1 条目兼容 | 读入标 registered-only,如实区分不静默升格 |
| Q7 | 资产布局 | 裸文件 + SHA256SUMS 共 11 件,不引 zip(§7) |
| Q8 | libdevice 随包 | 本期不带(EULA 面维持 pending-human-review),样例避 `__nv_*`;登后续 |
| Q9 | 真网络 E2E 位置 | 仅 release.yml 回读自校验 + e2e evidence;pr-smoke 诚实 SKIP token |
| Q10 | 自更新 | out_of_scope,执行期登 RD-033+,不预造 |

## 10. 稳定化与 provenance

- 条款 RXS-0214~0219 落 spec/release.md(stable 面,快照加性重 bless);rurixup CLI 面(install/list/default 语义)随条款进 stable 判据,`--from-dir`/`--expect-digest`/`setup` 为本 RFC 新增面。
- 全部代码 `Assisted-by` provenance 纪律沿用;签署/裁决留痕按 ODP §3(agent 代录,不代签 owner 裁决内容)。

## 11. 规范与实现依据

- 既有条款底座:spec/release.md §2.1~2.4(RXS-0135~0139)/ §2.6(RXS-0185/0186)/ §2.7(RXS-0187/0188)/ §3(零 RX 码口径)/ §4(禁区)。
- 既有代码挂钩:src/rurixup/src/install.rs:70-96(atomic_install 内存内核)/ toolchain.rs(注册表)/ channel.rs:78-124(清单序列化)/ gate.rs:56-71(八门)/ main.rs:79-126(install 流);src/rurix-pkg content_tree(hash_entries/collect_dir);src/rurixc/src/driver.rs:861-932(rt_cabi 定位序,exe 旁 lib/ 分支);.github/workflows/release.yml(八门 + RURIXUP_SIGN 链)。
- 治理依据:RD-025(registry/deferred.json v1.56)/ EA1_CONTRACT §7 / OWNER_DECISION_PACKAGE / 10 §3 / 14 §1 §3 §4 / 12 R-202 / MR-0009。

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft | 2026-07-16 | 初稿:四级内容寻址信任链 + staged-rename 原子物化 + shim 切换拟案 + curl.exe 载体拟案 + 发布侧对称自动化 + 两段式冷启动协议;§9 Q-A~Q-D owner-pending 呈 OWNER_DECISION_PACKAGE,Q5~Q10 agent 拟裁;失败模式表 11 条全 fail-closed;诚实边界三句(自签证书零信任贡献/bootstrap 空窗/不防 repo 级失陷)入 §4.5 | Full RFC(Draft) |
