<!-- Assisted-by: Cursor Agent（G27.1 治理波） -->
# RFC-0044 — G27 几何 device 化——HZB device kernel 兑现 + M61 重判程序 + cluster P4 差距重判程序 + M98-l4 重判窗程序

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0044（立项时实测 `registry/number_ledger.json` namespaces.RFC next_free=44 顺位领取） |
| 状态 | **Agent Approved**（2026-08-25；D-409 对抗性评审 11 findings〔0 blocker/5 major/6 minor〕全部 disposition，v0.2 修法批落档——评审全文 `milestones/g27/design/rfc0044_adversarial_review.md`） |
| 判档 | Full RFC（HZB device kernel 兑现为几何剔除 device 登记面；渲染器库面零新语言语义条款，G5 先例） |
| 承接 | G27.2 M-a/M-b + G27.3 M-c/M-d + G27.4 M-e（RD-039「HZB 两阶段」device 化长线锚 + M61/M98-l4 重判锚兑现；RFC-0037 §1.6 device kernel 车道 out-of-scope 承接命中） |
| 上游 | `milestones/g25/g25_campaign_handover_registry.json`（M61 / M98-l4 两行 = G27 法定输入）、`registry/deferred.json` RD-039、RFC-0037（host 参考臂 + §1.6 device 车道锚）、RFC-0034 只追加重判表（G20.3 M-c 重判在案） |

## 1. HZB device kernel 兑现语义（M-a）

1. **kernel 面（双 kernel）**：
   - `kernels/g27_hzb_reduce.rx` 单级 2×2 farther-of 归约：输入上级单通道深度 mip（SSBO）+ params（上级/本级 w、h + 约定选择位）；输出纹素 (x,y) = footprint {(2x,2y),(2x+1,2y),(2x,2y+1),(2x+1,2y+1)} 内**最远**深度——reverse-Z 取 min / standard-Z 取 max（host `DepthConvention::farther` 同律，保守遮挡语义唯一合法归约方向），越界 clamp 复采边纹素（坐标 `min(·, 上级边长−1)`，host `HzbPyramid::build` 逐字），非 2 幂 ceil 减半（`nw = ceil(pw/2) max 1`）。host 逐级 dispatch 直至 1×1 建全金字塔（mip0 = 全分辨率拷贝，不经 kernel）。**零容差协议（F1 disposition：浮点语义钉死）**：纯 min/max 比较归约、零算术舍入、零乘加（FMA 收缩面不适用）⇒ device mips 与 host `HzbPyramid::build` 逐级**位级相等**（不设容差带；G26 标定容差协议不适用本面）。**域前提字面**：深度场由 host 单源生成一次并以原字节上传（device 不重生成——`sin()` 等超越函数只在 host 出现一次，双端消费同一 f32 位型）；生成器值域 = [0,1] 有限正值闭集（NaN/±Inf/−0 构造上不可达），min/max 对有限 f32 为纯选择运算（只搬运既有位型、不产生新值）⇒ 位级相等性由「同选择决策 + 同输入位型」蕴含。
   - `kernels/g27_hzb_test.rx` 逐 rect 单 invocation 保守遮挡测试：输入金字塔全 mips（单 SSBO 平铺传入 + 逐 mip offset/尺寸表，host 打包）+ rect 流（uv_min/uv_max/nearest_depth）+ 约定位；逐 rect 复算 host `test_rect` 字面——像素化 `x0 = floor(clamp(u_min,0,1)·w0)` clamp [0,w0−1]、`x1 = ceil(clamp(u_max,0,1)·w0)` clamp [1,w0] − 1（y 同律）；`span = max(x1−x0+1, y1−y0+1)`；mip 选择 **`while (span >> mip) > 2` 逐字**（定界迭代 + 算术门锁存形，mip 上界 clamp 至金字塔级数−1）；≤2×2 纹素窗（`x0>>mip` 起、`min(x1>>mip, mip_w−1)` 止）farther-of 取最远；`is_farther(nearest_depth, farthest)`（reverse-Z `<` / standard-Z `>`）⇒ Occluded，否则 Visible（保守两态，host `Occlusion` 同义——Visible = 不能证明被遮，非必可见）。
2. **判据面**（程序产禁手写）：
   - ① **mips 逐级位级相等**：device 金字塔 vs host `HzbPyramid::build` 全级零容差全等（§1.1 零容差协议）；
   - ② **800 rect × 双约定判定序列全等**：`g20_hzb_probe` 夹具逐字同源（193×117 非 2 幂确定性深度场 + `det_rects(800)` + reverse-Z/standard-Z 双臂；**域前提（F2）**：夹具保证 0 ≤ u_min < u_max ≤ 1 ⇒ 像素化恒有 x1 ≥ x0、y1 ≥ y0，u32 下溢构造上不可达），device 判定序列与 host `test_rect` **逐 rect 逐字节**全等（非仅组合 digest）；**序列化字面（F11）**：判定位序列 = 每 rect 1 字节（0=Visible/1=Occluded）按 rect 索引序拼接，金字塔字节 = 逐级 f32 LE 拼接，digest = sha256(判定位序列 ‖ 金字塔字节)（`g20_hzb_probe` 同字面）；
   - ③ **零假阳性硬不变量（独立纵深防御面，F3 声明）**：device 判 Occluded ⇒ `exact_rect_occluded`（host 裁判函数，逐像素精确真值金标准）必同判遮挡——机核逐 rect 复验，任一假阳性即 FAIL（漏剔合法：保守性只损效率不损正确）。与 ② 的关系显式声明：② 全等 + host 已证零假阳性 ⇒ ③ 逻辑蕴含成立，但 ③ 独立直接对裁判函数复核、不依赖 ② 判定链路的实现正确性——两判据并列为纵深防御，冗余是设计而非疏漏；
   - ④ **device 双跑位级一致**：同设备同驱动窗口内固定输入两跑输出 digest 位级相等；
   - ⑤ **篡改 RED 臂（F4 disposition：构造性注入协议）**：注入面 = host 预算定位的单一金字塔纹素——臂 A（漂移检出）：由 host 扫描选取**被 ≥1 个 rect 的 ≤2×2 采样窗覆盖**的 mip 纹素（构造性保证消费路径命中），写入「更近」极值（reverse-Z 写 1.0 / standard-Z 写 0.0）→ 逐 rect 字节序列必异（比较面 = 逐 rect 字节，非仅组合 digest）；臂 B（假阳性哨兵）：同注入使 ≥1 个 host-Visible rect 翻为 device-Occluded → ③ 裁判函数必检出 ≥1 假阳性；任一臂漏检即 RED；
   - ⑥ **spirv-val**：双 kernel 编译产物 SPIR-V 验证通过。
3. **三态协议**：无 Vulkan loader/设备 → device 腿 `SKIP DEV_ENV_DEGRADE`（退 0，非 fake pass，如实登记不冒充；evidence schema `skipped_dev_env` 合法态；`RURIX_REQUIRE_REAL=1` 下 SKIP→硬红由 smoke 脚本层裁决）；host 腿恒跑；判据不符 / RED 臂失效 ⇒ FAIL 退 1。
4. **确定性协议**（`kernels/g18_light_transport_depth.rx` 头注 RXS-0357 L2 同律继承）：禁 atomic、逐纹素（reduce）/逐 rect（test）独立顺序求值、输出直写无跨 invocation 交互、全 f32、分支判定 min/max 算术门白名单形、固定输入双跑位级一致。
5. **host 参考臂冻结**：`src/rurix-render/src/geometry/hzb.rs`（对拍承重面）与 `geometry/cull.rs`、`geometry/visbuffer.rs`（生产剔除链面）三文件 vs `g26-closed` **0-byte**（git-diff 机核，RFC-0043 F2 冻结先例同模）；device 臂为 bin-local adapter 加性面（`g26_framegen_device` 集成路径同模，经 `rurix_rt::vk::run_compute` 派发），不回写 host 模块、不接线任何生产车道（剔除 pass 接线显式 out-of-scope §5.7）。

## 2. M61 重判程序（M-b）

1. **输入面（F6 disposition：三项闭集合法性逐字引证）**：handover registry M61 行重判条件字面（`milestones/g20/G20_P2_DECISIONS.md` §1 M61 行逐字）= 「**重判条件 = cluster P4 差距闭集清零 + HZB device 化落地后只追加再判；兜底 = VS 光栅唯一 fallback 维持（字面 0-byte）**」两半 + RFC-0034 只追加重判表行 2 逐字第三件「**mesh shader HW 管线性能差 measured 证据仍缺（HW 路径零实现 ⇒ 无 A/B 可测面）……重判条件顺延 = cluster P4 差距闭集清零 + HZB device 化落地后按只追加程序再判**」——锚两半 + 原判据证据件合并为**三项机器盘点闭集**（三项合取 = G20.3 M-c 重判裁决的三个未齐面逐项翻绿才构成重判启动，重判表行 2 同源）：
   - ① **HZB device 化半边**：M-a 绿件只读盘点（evidence 存在性 + 判据面全绿；`skipped_dev_env` 态如实登记为「device 腿未真跑」不构成命中）——本期预期命中；
   - ② **cluster P4 清零半边**：`milestones/g20/g20_cluster_streaming_p4_gap.json` 四行 status 实测（M-c §3 同批产物一致性互核）——预期四行全 open 未清零；
   - ③ **mesh shader HW 性能差 measured 证据**：树内闭集搜索（禁开放式外采），evidence 必须逐条登记实际检索的路径/模式清单（**searched-paths manifest 必填**，清单为空或缺失即门 FAIL——「零命中」结论只能建立在非空搜索清单之上，RFC-0043 F6 同律）——预期零命中。
2. **决策树**：三项**全齐** → 重判程序启动（本期只登记启动事实与证据路径，mesh shader HW 管线实现与 A/B 终审归 §5.1 承接锚窗）；任一未齐 → **maintain-no-go 只追加**（RFC-0034 重判表尾追加 G27.2 行 + evidence 路径；VS 光栅唯一 fallback 兜底字面 0-byte）。
3. **防冒充硬线**：**①半边命中不得单独构成重判启动**（字面：三项闭集全齐才启动）——HZB device 化落地是必要非充分件，盘点脚本对「启动」判定的输入面 = 三项布尔合取，任何单项/两项命中均落 maintain-no-go 分支。

## 3. cluster P4 差距重判程序（M-c）

1. **输入面**：`milestones/g20/g20_cluster_streaming_p4_gap.json` 四行字面（0-byte 只读）——P4-1 cluster 页磁盘布局与驻留池 / P4-2 GPU 请求反馈链（剔除 pass 产 cluster 缺页请求 → host 驻留调度，anchor = 依赖 HZB device 化）/ P4-3 LOD cut 与驻留联动 / P4-4 异步 IO 优先级链；表级 reeval_anchor = 「HZB device 化落地 + 剔除 pass 反馈链出现」。
2. **依赖解除事实登记（F10 措辞收口）**：M-a 绿件 ⇒ reeval_anchor **半边命中**（HZB device 化落地）——P4-2 依赖面本期解除的**事实登记**（登记≠该行兑现；剔除 pass 反馈链另半边本期不出现，生产接线 out-of-scope §5.7）。**过述防线**：「HZB device 化落地」= 金字塔构建与保守测试 device 化的单件事实，**不构成** RD-039「HZB 两阶段」分项整体兑现（第二阶段两 pass 管线见 §5.8），本 RFC 任何叙述不得引为两阶段锚兑现。**程序性依据（F8）**：本重判 run 的合法性 = G20_P2 M61 行「HZB device 化落地后只追加再判」字面 + 表级 reeval_anchor 半命中即触发只追加再判登记（原表 0-byte）。
3. **逐行 reeval 决策树**：逐行树内实测现面实现痕迹（检索面 = `streaming/` 四模块〔pool/feedback/engine/resource〕+ geometry cluster 载荷面；逐行检索路径清单入 evidence）——任一行现面兑现 → 该行 closed-go；零实现 → 维持 open（预期四行全维持 open，如实登记不冒充）。
4. **产物**：`milestones/g27/g27_cluster_p4_rejudgment.json`（四行逐行 disposition + 依赖解除事实 + 检索清单）；g20 原表 **0-byte 不回写**（本表为只追加重判镜像，原始锚字面不动）。
5. **RD-039 history 只追加（F9 机核补强）**：deferred.json RD-039 history 止于 G14.1（2026-08-19 行），G15~G26 承接留痕在各期 P2 决策表未回写 deferred.json——本期按只追加纪律尾追加 G27.3 行（登记 M-a 兑现 + P4 重判 disposition + **断档口径注明**「G15~G26 承接留痕在各期 P2 表」）；id/title/reason/backfill_condition 四字段 0-byte。**append-only 机核** = 互锁同律 `check_deferred_append_only`（vs G27.0 不可变 ref：条目四字段 0-byte + history 前缀相等），M-c 门 evidence 引用该机核结论。

## 4. M98-l4 重判窗程序（M-d）

1. **输入面**：handover registry M98-l4 行（G20.5 终态 maintain-no-go：接口面就绪命中 + L4 计数可测未命中）重判条件字面 = 「HLOD proxy 追踪 device 腿落地 + L4 计数器接入选档 evidence」两半。
2. **两半条件树内实测**：
   - ① **HLOD proxy 追踪 device 腿**：src 树内检索零实现实测（检索面 = `src/rurix-render/src/{gi,world}/` + `kernels/`，HLOD proxy 远场追踪 device kernel/装载腿痕迹；检索清单入 evidence）——预期零命中；
   - ② **L4 计数器接入**：三处 fail-closed 入口实测——`gi/fallback_chain.rs` `ChainFrame.counters[3]`（L4 槽位）恒零（装配后 L4 行 = `LevelCounters::default()`）+ `check_l4_trigger()` 恒 `NotTriggered` + `l4_serve()` 恒 `Err(L4InterfaceNotReady)`；并复核 `world/hlod.rs` 接口面就绪在案（HlodRuntime 选择/事件总线/digest 核验臂，g9.p1.m111 门绿）——**接口面就绪 ≠ 计数器接入**（G20 M-d 终判字面：接口半命中不构成本半边命中）。
3. **决策树（F7 disposition：判定形状与锚字面对齐声明）**：锚字面（`milestones/g20/G20_P2_DECISIONS.md` §1 M98-l4 行逐字）= 「**重判条件 = HLOD proxy 追踪 device 腿落地 + L4 计数器接入选档 evidence；兜底 = L1/L2/L3 三级链维持**」——「+」为合取：**改判（三级链 → 四级链）须两半全齐**，与 M61 三项合取判定形状一致。「任一半命中 → 重判程序启动」中的「启动」= 启动登记与证据征集程序（进展事实只追加入档），**非改判**——登记阈值低于改判阈值的依据 = G20 M-d 终判先例（「接口面就绪命中」即已作为半命中事实登记而不改判）。均未命中 → **维持 L1/L2/L3 三级链**（maintain 只追加，G20 M-d 兜底字面 0-byte；L4 槽位恒零 + 三处 fail-closed 入口不动）。
4. **规范边界（不混同声明）**：**RXS-0396 世界级辐射缓存 ≠ RXS-0359 L4 Far Field**——前者为 GI 世界空间辐射缓存（屏幕探针远场能量兜底，gi 侧已落地面），后者为 HLOD proxy 追踪远场档（几何代理追踪，接口未就绪）；RXS-0396 落地事实**不得**作为 ①/② 任一半命中证据，盘点脚本检索面显式排除世界缓存实现路径。

## 5. out-of-scope（各附承接锚）

1. **mesh shader HW 管线实现**：承接锚 = §2 三项闭集全齐（重判启动后的执行窗按只追加程序排产）；本期 VS 光栅唯一 fallback 维持。
2. **HLOD proxy 追踪 device 腿实现**：承接锚 = 消费方需求 + 资源窗；RXS-0359 L4 消费接口冻结面维持（`l4_serve` 恒 Err 不动）。
3. **cluster P4 四行实现**：承接锚 = 各行 anchor 字面（P4-1 后续期 device 波 / P4-2 剔除 pass 反馈链出现 / P4-3 LOD cut 联动窗 / P4-4 IO 优先级窗）；本期仅重判登记。
4. **host 参考臂重写**：`geometry/hzb.rs` 冻结面；device 对拍若暴露 host 语义缺陷，按只追加程序另立重判（RFC-0043 §5.4 同律）。
5. **kernel 性能优化**（超对拍需求面的调度/占用率/共享内存优化）：承接锚 = device 车道进生产集成窗时按 measured 瓶颈立项。
6. **多 GPU / 跨设备一致性**：承接锚 = 多设备环境出现；本期双跑位级判据限同设备同驱动窗口。
7. **剔除 pass 生产接线**：`geometry/cull.rs`/`geometry/visbuffer.rs` 冻结（§1.5 机核）——HZB device 臂不接线两级剔除链/VisBuffer 生产车道；承接锚 = P4-2 反馈链立项窗（§3 行锚字面）。
8. **HZB 两阶段的第二阶段（F10 补项）**：两 pass 遮挡剔除管线语义（上帧金字塔初剔 + 本帧重建重测）——本期只兑现金字塔构建与保守测试两基元的 device 化，两阶段管线（含帧间金字塔轮换/初剔-重测调度）零实现；承接锚 = 剔除 pass 生产集成窗（RD-039「HZB 两阶段」分项 backfill_condition 字面「剔除效率成为 measured 瓶颈时优先」）。

## 6. 验收门映射

| P0 | 门 key | 波次 | 判据面 |
|---|---|---|---|
| M-a | `g27.p0.m_a.hzb_device_kernel` | G27.2 | §1（mips 位级全等 + 800 rect 双约定判定序列全等 + 零假阳性 + 双跑位级 + 篡改 RED 臂 + spirv-val + 三态） |
| M-b | `g27.p0.m_b.m61_mesh_shader_rejudgment` | G27.2 | §2 决策树（三项机器盘点 + searched-paths manifest 必填 + 防冒充硬线） |
| M-c | `g27.p0.m_c.cluster_p4_gap_rejudgment` | G27.3 | §3 决策树（四行逐行 disposition + 依赖解除事实 + RD-039 只追加） |
| M-d | `g27.p0.m_d.hlod_l4_counter_rejudgment` | G27.3 | §4 决策树（两半树内实测 + 三处 fail-closed 入口 + RXS-0396/0359 不混同） |
| M-e | `g27.p0.m_e.closed_gate_no_regression` | G27.4 | G26 受影响门 `--verify-latest` 全绿零降级；禁 `--gate` 旧脚本；`g27_` 前缀不抢 latest |

与 G27_CONTRACT §4.2 同构（M-a/M-b evidence schema 含 `skipped_dev_env` 合法态——M-b ①半边盘点消费 M-a 三态如实登记）；implemented / maintain-no-go / maintain 均为合法终态，**零冒充**。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v0.1 | 2026-08-25 | G27.1 起草（Draft）。 |
| v0.2 | 2026-08-25 | D-409 对抗评审修法批：F1 零容差浮点语义钉死（域前提 + 单源上传 + 选择运算蕴含）/ F2 rect 域前提声明 / F3 判据③独立纵深防御声明 / F4 RED 臂构造性注入协议（host 预算定位纹素 + 双臂 + 逐 rect 字节比较）/ F5 防冒充硬线维持（§2.3 既有）/ F6 三项闭集逐字引证（G20_P2 M61 行 + RFC-0034 重判表行 2）/ F7 M98-l4 判定形状对齐声明（合取改判 + 登记阈值先例）/ F8 M-c 程序性依据显式 / F9 RD-039 append-only 机核补强 / F10 out-of-scope §5.8 补项 + §3.2 过述防线 / F11 digest 序列化字面钉死；状态 Draft → **Agent Approved**（评审 `milestones/g27/design/rfc0044_adversarial_review.md`，11 findings 全 disposition）。 |
