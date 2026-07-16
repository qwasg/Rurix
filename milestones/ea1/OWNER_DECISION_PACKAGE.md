# EA1 Owner Decision Package — 分发信任面四项裁决(轻量版)

> **性质**:本文件把 EA1(「十分钟上手」分发与门面期)保留给 owner 的裁决点摊清,并给出 agent 拟裁与备选后果。与 MB1 版的三点差异:① EA1 **不触任何红线**,本包不 gate 里程碑激活——只 gate EA1.1b 网络面(裁决 A,[RD-025](../../registry/deferred.json) backfill_condition 明记「网络拉取若引入须先裁 D-312 相关面」= 契约性前置)+ 三项轻确认(B/C/D);治理包/RFC 起草/文档支线/上游备包**不被 gate**;EA1.1a 待 RFC-0012 Approved(其翻 Approved 与裁决落地同 PR,见 §3),其活跃切换子面按裁决 B——「零空转」仅指支线与起草面,如实声明。② 仓库默认治理(D-406 v2.0)为 agent 完全自主——本包是 RD-025 契约性兑现 + outward-facing 惯例的落实,**非** owner 覆盖默认。③ 既有治理文件(`13_DECISION_LOG.md` / `registry/spike_gating.json`)在治理包分支保持 pristine,且**裁决落地也不改写它们**——D-312 维持「待决」原状,留痕只进 [EA1_CONTRACT.md](EA1_CONTRACT.md) §7 + RFC-0012 §9 + RD-025 history。
>
> **诚实前置(裁决须知)**:① 当前 Authenticode 为**自签测试证书**,对外部用户信任贡献为零(生产签名 = Azure,secret+人工门控,spec/release.md §4 禁区,本期不动)——裁决 A 的信任根设计不依赖它,它只是纵深。② 用户首次获取 rurixup.exe 本身只有 TLS + 手动核对 SHA256SUMS 两道保护(bootstrap 空窗,与 rustup-init.exe 同构),文档如实写明,不假装闭环。③ channel 清单今天**不被签名**,信任靠内容寻址交叉核对——裁决 A 就是为它补信任根。

---

## 0. 裁决摘要(owner 勾选;A 为 EA1.1b 硬前置,B/C/D 为轻确认)

| # | 裁决点 | agent 拟裁 | owner 裁决 |
|---|---|---|---|
| A | **网络端点 + 信任根 + 载体**(gate EA1.1b) | 唯一端点 = 本仓 GitHub Releases(github.com + objects.githubusercontent.com 跳转);信任根 = **repo 内 `channels/stable.json` digest 锚**(四级内容寻址:锚→channel 清单→bundle 清单→组件字节,任一级失配 fail-closed 拒装)+ TLS https-only(`--proto =https --proto-redir =https`)+ 自签 Authenticode 诚实标注为纵深非信任根;载体 = **系统 curl.exe 固定参数集**(Windows 自带,TLS 信任委托 OS;否决 FFI——破 rurixup `unsafe_code=deny`;否决外部 crate——破零第三方依赖纪律);信任根登记流 = 发布 workflow 自动开 PR、**owner 合并 = 人工门**;**拟定性 = 非 D-312 registry 激活**(单端点第一方工具链分发,无包名空间/无第三方上传面/无新服务端信任设施;SG-007 现状维持 not_triggered,rurix-pkg 零网络代码不变);锚获取通道 = ①`--channel-file` 本地路径 ②clone 仓内相对路径 ③`https://raw.githubusercontent.com/<repo>/main/channels/stable.json`(TLS);**诚实边界:digest 链防「登记后资产篡改/传输篡改」,不防 repo 级失陷(锚与 Release 资产同一仓库权限域,非独立信任域)——对冲 = 锚 PR owner 合并人工门 + git 历史可审计;离线签名留 D-312 真触发重议** | ☐ 认可全案　☐ 认可但载体改 FFI(U29)　☐ 认定触 D-312 须先裁 registry(→EA1.1b 冻结,§3 路由)　☐ 其他 |
| B | **活跃版本切换机制**(偏离 RD-025 记载的「PATH/junction」措辞,故 surface) | **shim 目录**一次入 PATH(rustup 模型:`%USERPROFILE%\.rurix\bin\rx.exe` = rurixup 拷贝,按 argv0 干名转发 default 版同名 exe,退出码透传,spawn 目标恒在 toolchains/ 下防自递归)——切换 = 确定性注册表 JSON 单写,**原子、免特殊权限、已开 shell 即时生效**;junction(rmdir+mklink 两步有窗口)降为 RFC-0012 §7 备选;PATH 接入默认**只打印指令**,`rurixup setup --add-path` 显式 opt-in(不默认改用户环境) | ☐ shim(拟裁)　☐ junction　☐ 其他 |
| C | **冷启动 <10min 验收口径**(操作化 01 §4 图景 3「从 rurixup install 到第一个 kernel…少于十分钟」;硬约束 = 消费级 VM 无 NVIDIA GPU 直通) | **两段各 ≤10min measured**:A 段 = 干净 Win11 VM(零预置依赖),T0 = 文档首条命令(下载 rurixup.exe),T1 = `rx check` 退出 0(含全部下载/校验/物化/PATH);B 段 = 开发机**干净用户账户**(用户环境无 Rust 工具链;系统级 LLVM 22.1.x + VS Build Tools + NVIDIA 驱动为**文档化前置,不计时**——rustup 同类口径,README/RFC 显著位诚实标注),T1 = 首 kernel device 真跑退出 0;**不跨机加总**(物理上不可比);Nsight 时间线段标注为后续,不充数;evidence 含带宽/环境画像,不进 CI 硬门;**计时重测规则 pin 死:允许重测 ≤3 次、全部尝试入 evidence、取 median;超限如实记录走 RFC-0012 §9 修订留痕,不静默放宽(反 measurement-shopping)** | ☐ 两段各 ≤10min(拟裁)　☐ GPU 段单门+VM 段辅证不设时限　☐ 其他 |
| D | **bundle 随 semver tag 自动发布为公开 Release 资产**(一次性确认,扩大公开发布面) | **认可**——semver tag 本身已是 owner 侧动作,确认后回归 tag 门控惯例,上传半程非逐次审批(**锚登记仍逐次 owner 合并,见 A——每次发布用户可安装前你仍要合一个 PR**);上传步骤仅在 release.yml 全部 hard-block 门(RXS-0139 七子门 + 第 8 门 channel-manifest)之后 + 上传后回读自校验;**首次演练走 workflow_dispatch**(注意:「防误发」只防 tag 误触发——**演练产物即首个可安装公开资产**,演练版号/pre-release 形态细则钉入 RFC-0012 §9);资产 = 裸文件 + SHA256SUMS 共 11 件(现 10 件 + rurix_rt_cabi.lib——v1.0.0 缺此件,无 Rust 环境时含 GPU 面的 `rx build` 必死,必修) | ☐ 认可(拟裁)　☐ 上传改逐次人工(回读自校验以脚本 evidence 兑现)　☐ 其他 |

> **已排除、不呈选项的方案及理由**(owner 可经各案「其他」召回):外部 HTTP crate(reqwest 等——破零第三方依赖纪律,手写 SHA-256 是为此付过的成本);纯 PATH 改写切换(setx 1024 字符截断坑 + 已开 shell 不生效);冷启动「合计单门」(跨机加总物理上不可比,§1-C)。

---

## 1. 逐案摊清

### A. 网络端点 + 信任根 + 载体

- **背景**:RD-025 backfill_condition 原文——「registry sumdb(D-312/SG-007)与网络信任根维持 not_triggered,**网络拉取若引入须先裁 D-312 相关面**」。EA1.1b 是本仓第一段网络代码,信任根选择是外向安全面。
- **为什么拟「非 D-312 激活」**:D-312 的对象是**包生态** registry/sumdb(rurix-pkg 消费面:包名空间、第三方上传、透明日志)。EA1.1b 分发的是**工具链本体**——单一产品、单一发布者、无上传面;信任根是 repo 内一个静态 JSON(与源码同信任载体:用户信任 Rurix 源码即信任该 repo,clone 自建与下载安装同一信任面),不新增任何服务端信任设施。SG-007 触发阈(生态包 >50/社区强需求)与本设计无交集。rustup 的信任根(static.rust-lang.org 清单 over TLS)本质相同,A 案还多一层「digest 在版本控制历史里可审计」。
- **为什么否决手写签名(ed25519)**:签名实现错误 = 静默信任伪造(风险类别劣于 digest 对不上就拒的 fail-closed);私钥保管又引入 §4 禁区同类 secret 面。digest 链 + repo 锚防的是「登记后资产篡改/传输篡改」这两类主威胁,**不防 repo 级失陷**(锚与 Release 资产同一仓库权限域,非两个独立信任域——这一点如实呈报,不作「同等强度」声明);repo 级失陷由锚 PR owner 合并人工门 + git 历史可审计对冲,离线签名的独立信任域价值留 D-312 真触发时重议。
- **载体对比**:curl.exe(Windows 10 1803+ 自带,schannel 后端)——与本仓「重外部能力经受控外呼」既有模式同构(rurixc 外呼 clang/link.exe/ptxas,签名外呼 PowerShell);FFI(WinHTTP/schannel)破 rurixup 唯一的 crate 级纪律 `unsafe_code=deny` 且审计面大(若 owner 选此,登记 U29);外部 crate(reqwest 及其传递依赖树)破零第三方纪律(手写 SHA-256 是为此付过的成本),不呈选项。
- **失败模式全表**(11 条:篡改×3/截断/降级/重定向劫持/离线/磁盘满/断电×2/事后损坏,每条→fail-closed 行为)见 RFC-0012;全部以 hermetic fixture 红绿可证。
- **备选后果**:若裁「触 D-312 须先裁 registry」→ EA1.1b 冻结,EA1 收窄为本地分发面(`--from-dir`)+ 发布资产 + 文档期——「十分钟上手」退化为「两条命令上手」(手动下载 + 本地 install),仍可交付但门面弱一档。

### B. 活跃版本切换机制

- **surface 理由**:RD-025 title 记载「PATH/junction 活跃切换」——shim 方案偏离该措辞(结果等价:用户 PATH 一次配置、版本可切),按诚实纪律呈报而非静默改道。
- **shim 决定性优势**:切换 = 一次确定性 JSON 写(与全仓确定性纪律同构、原子);junction 需 rmdir+mklink 两步(切换窗口)且 mklink 须外呼 cmd.exe;纯 PATH 改写对已开 shell 不生效且 setx 有 1024 字符截断坑(直接排除,不呈选项)。shim 代价 = 每次调用多一跳进程(毫秒级)+ 自更新换文件问题(显式 defer RD-033+)。
- **若裁 junction(备选落地路径,非零成本翻转)**:RXS-0215 语义改写为 junction 原子性约定(`active` 链接 rmdir+mklink 经 cmd.exe 受控外呼、两步窗口的中断态检测与修复进失败模式表);EA1_PLAN §2 任务 3 与 CI 步骤 59 切换判据按此改写并修订留痕——探针判据「切换后 `rx` 干名指到目标版本」机制中立,不变。

### C. 冷启动验收口径

- **surface 理由**:「十分钟」是期名的操作化,直接定义本期成败尺度——agent 不自定成败尺(MS1 主语言判据 owner 三裁定先例)。
- **两段式的诚实性**:VM 段证「零预置依赖装得上」(最强干净性),但 VM 无 GPU,只能到 `rx check`;GPU 段证「真跑得通」,但开发机系统级前置(编译器/链接器/驱动)无法逐次卸装,以「干净用户账户 + 前置文档化不计时」口径诚实分段。「合计单门」跨机加总物理上不可比(两台机器),已从 §0 菜单撤下(经「其他」可召回并须同时指定单机口径);备选「GPU 段单门」丢失零预置证明——拟裁双门最完整。
- **不进 CI 硬门**:计时含下载受带宽波动,进硬门必 flaky;走 evidence(measured_local + 带宽画像),SKIP 不充绿。

### D. bundle 自动发布确认

- **surface 理由**:把可执行工具链二进制做成公开 Release 资产 = 扩大公开发布面 + 资产 URL 稳定性承诺,outward-facing 一次性确认。
- **既有门控不变**:semver tag(触发器 `v[0-9]+.[0-9]+.[0-9]+*`)本身是 owner 侧动作;八子门 hard-block 全绿才上传;上传后回读自校验失配即 job 红;信任根登记另有 owner 合并人工门(A 案)——四道闸,自动化只是把「手工上传」换成「门后机器上传」。

## 2. agent 自主留痕清单(信息性——owner 无需动作,可 veto)

| # | 事项 | 依据 |
|---|---|---|
| ① | docs 合入(en-front-door + 中文 README 语言头 + 00_install 改写):agent 自主 push/PR/merge | D-406 v2.0 默认 + MS1 期自主 merge 先例;docs 面**可逆、无安全面、不承诺资产 URL 稳定性**,故不适用裁决 D 的一次性确认惯例(甄别标准显式化);纯新增/门面文案,诚实边界自查(不宣称采纳,「十分钟」表述须带两段式限定) |
| ② | 规划文档状态勘误(00/11/12/13):独立 errata PR,check_planning_docs 预期红 | 00 §6.3 + PR #140 先例;与执行 PR 严格分离 |
| ③ | 上游三连备包:只备包,全部 `DRAFT — do NOT file`,提报 owner 亲自 | 既有 GRX 先例;契约 out_of_scope upstream_filing |
| ④ | 命名 EA1 / namespace ea1. / tag ea1-closed | MS1 命名先例 = agent 自主记契约 §7;EA1 名已见于 owner 认可的评审结论 |
| ⑤ | nightly 根治 = 契约外并行轨道 | owner 2026-07-16 本会话 AskUserQuestion 已裁,此处备案 |

## 3. 应用顺序(owner 勾选后)

1. agent 代录**裁决落地小 PR**:本文件 §0 勾选留痕(☐→☑ + 日期;**本文件为裁决工作文档,非契约 0-byte 冻结对象**,勾选回填合规)+ EA1_CONTRACT §7 追加裁决行 + RFC-0012 §9 回填四问 + RFC 状态 Draft→**Approved** + registry/deferred.json RD-025 history 追加(裁决 A 定性留痕);**零 13 号文档/spike_gating 改动**。
2. 裁决 A 落地 → EA1.1b(网络面)解锁;裁决 D → EA1.2 发布面确认;EA1.1a 不等裁决,RFC Approved 后即开工。
3. 若裁决 A 选「触 D-312」→ EA1.1b 冻结:以契约 **§7 追加裁决行**留痕(YAML 头 in_scope 原文 **0-byte 不动**),G-EA1-3/D-EA1-3 走其内置 blocked 条件分支,EA1 按 §1-A 备选后果收窄推进(修订记录追加,不静默)。
4. **裁决部分落地协议**:未裁项维持 pending,不阻塞已裁项落地;RFC-0012 翻 Approved 需 **A、B 已裁**(条款语义依赖——RXS-0216/0217 依 A,RXS-0215 依 B);C 未裁则 G-EA1-6 以拟口径推进并标 pending 待回填;D 未裁则 EA1.2 上传面 pending(离线打包面照常),RFC §9 对应行留 owner-pending。

## 4. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-16 | 初版(EA1 治理包配套;四项裁决 A~D 摊清 + agent 拟裁 + 备选后果;轻量范式 = 只 gate 网络面与三项轻确认,不 gate 里程碑激活;裁决落地零 13 号文档/spike_gating 改动) |
