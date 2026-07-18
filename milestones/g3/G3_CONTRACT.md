---
contract: G3
title: G3 工业渲染期：RD-027 毒径归因闸门 + 五特性面全量落地（采样超集 / bindless / render graph 自动 barrier / UC-04 窗口 present / mesh-task-RT 双后端）
status: active            # active → closed（close-out 只追加 §8,上方条款 0-byte;基准切换与 g3-closed tag 归 G3.7,agent 自主签署;若 EA1 先收口则链为 mb1→ea1→g3,否则 mb1→g3,以 close-out 时点默认基准为准）
version: v1.0
date: 2026-07-18
timebox: "约 8–10 周（主线 G3.0→G3.7 严格串行合入 + RFC 流水线重叠见 G3_PLAN.md;周为相对刻度,非日历承诺）"
rfc_required: RFC-0013, RFC-0014, RFC-0015, RFC-0016, RFC-0017    # 五特性面全部 Full-RFC-gated（P2-5 present 判档争议向上取严 = Full,硬规则 8;06 §4.2 禁区增补面(采样/UAV memory-order/pass happens-before)必须 Full）。G3.1 spike 为纯取证不占 RFC（G2.2 DXIL spike 先例）;RD-027 处置若为工具行为 workaround 判 Mini（MR-0011 留号)或 Direct。脚手架本身为结构件不实现语义
upstream_docs:
  - "06 §4.2（禁区面:quad 导数/隐式 LOD/跨线程可见性 memory-order——本期条款本体增补对象,Full RFC 全文批准前置）"
  - "registry/deferred.json RD-027（毒径挂起=闸门,backfill_condition 为 G-G3-1 DoD 骨架）/ RD-022/023/024（采样超集）/ RD-018（bindless）/ RD-020（自动资源状态跟踪,明记 barrier 语义本体另归 Full RFC）/ RD-019（UC-04 窗口 present）/ RD-012+RD-029（mesh/task/RT DXIL+Vulkan 两侧）"
  - "rfcs/0002（着色阶段类型面,九阶段 AST 已建缺 intersection/callable）/ 0005 §9 Q-Bindless / 0006 §9 Q-Present·Q-Barrier / 0007 §8（采样三缺口）/ 0009（std::gpu 宿主编排,render graph 语言面地基）/ 0011（Vulkan 后端,RXS-0200~0213）"
  - "13 D-406 v2.0（agent 完全自主）/ D-130（窗/泵/输入不进语言——present 面红线）/ D-131（DXIL 混合 compute=A/图形=B——新特性 DXIL 腿一律走 B 链）/ D-207（PTX 收集根排除着色阶段——采样/图形面 PTX 腿结构性不适用）/ D-409（Full RFC 对抗性评审,评审 provenance ≠ 起草 provenance）"
  - "12 R-109（RD-027 毒径风险载体）/ R-603（范围蔓延——五面各 RFC §8 锁边界,超界登 RD-034+）/ R-606（裸 subprocess 禁令——spike 全程 proc_guard）"
  - "14 §1 §3 §4 §5（契约/预算零占位/deferred/证据分级）/ 10 §3（三档门）§9.5（编号永不复用）/ agents/AGENTS.md（硬规则十条）"
  - ".tmp/work_items_20260717.md §P2（scratch 候选清单,非治理物——本契约为其 P2-1~P2-6 的治理化落地;P2-7~P2-12 不带入）"
in_scope:
  - rd027_gate_spike           # G3.1 闸门:RD-027 毒径判别 spike——四层嫌疑（rurixc IR/LLVM NVPTX/ptxas/驱动 JIT）对照实验矩阵 + 最小化复现 + 归因;owner 已裁「归因落地即开闸」（§7 ②)→ 归因证据合入 main 即解锁五面 RFC 合入;处置尾项（修复或上游备包+护栏）不阻塞开闸但属 close-out 前置。纯取证不占 RFC
  - present_windowed           # G3.2 P2-5:UC-04 D3D12 可见窗口 flip-model swapchain present + resize 重建 + backbuffer readback 校验;Vulkan 侧 OUT_OF_DATE 重建收尾 → Full RFC(RFC-0013);D-130 红线:窗/泵/输入维持 C++ shim/运行时层,语言面零新语法（复用 RXS-0197 typestate）;offscreen 硬门(步骤 48)不动
  - sampling_superset          # G3.3 P2-2:采样超集全量（RD-022a 隐式 LOD/导数/显式 grad/bias + RD-022b 可配置 sampler(宿主 SamplerDesc+静态 sampler 属性) + RD-023 texel fetch + RD-024 shadow/gather/多分量/UAV 写+memory-order 条款）→ Full RFC(RFC-0014,06 §4.2 禁区增补);DXIL 腿走 B 链(spirv-cross→dxc 成熟主干),Vulkan 腿原生,PTX 腿 D-207 结构性不适用如实标注;含 vk.rs graphics descriptor 运行时建面（后续面共用底座）
  - bindless_descriptor_indexing  # G3.4 P2-3:无界资源数组签名形参 + 动态索引 + nonuniform 标注(strict-only) + RuntimeDescriptorArray/SPV descriptor indexing + RTS0 unbounded range 独占 space + update-after-bind 运行时 + std::gpu TextureTable → Full RFC(RFC-0015);SM6.6 heap 直索引语法糖无 SPIR-V 对应 = 显式收窄登 RD-034+（RD-018 close 留痕写明,非静默砍面）
  - render_graph_auto_barrier  # G3.5 P2-4:Graph/Pass 声明式宿主库面(lang-item,无新语法) + 自动资源状态推导(rurix-rt 纯 host safe 模块,双后端映射同源) + pass 边界 happens-before 语义本体条款(🔒,RD-020 明记归 Full RFC) + uc04 手动 plan_barriers 保留为推导产物独立复核门(双实现互证) → Full RFC(RFC-0016);重排调度/多 queue/split barrier 显式不进首期登 RD-034+
  - mesh_task_rt_stages        # G3.6 P2-6:mesh/task/RT 六阶段全量（AST 补 intersection/callable;payload/attribute 契约升全量;AccelStruct/trace_ray;SPIR-V 1.4 分叉）;Vulkan 主腿全量（mesh pipeline + BLAS/TLAS/SBT/TraceRays,新 U 号）;DXIL 腿分层:mesh/task probe-first（spirv-cross mesh 能力实测先行）,RT 腿预判上游 blocked（spirv-cross HLSL 无 SPV_KHR_ray_tracing + RD-015 未解）→ 以 probe 证据落 RD-034+ 尾门或翻活,不伪造;RX6008 预留码正式改接（RD-012） → Full RFC(RFC-0017)
  - ei1_contract_scaffold      # G3.0 顺手件:milestones/ei1/ 契约四件套（owner 2026-07-18 双轨立项之另一轨,「脚手架 G3 先合」）——仅契约,§0 gated,零实现零共享编号消费
out_of_scope:
  - window_input_system        # 窗口输入/事件循环 API:D-130 红线维持,demo 只泵消息不暴露输入面;SG-010 软保留方向不触
  - heap_direct_index_sugar    # SM6.6 ResourceDescriptorHeap[] 语法糖:无 SPIR-V 标准对应无法过 B 链;unbounded array 语义等价覆盖,语法糖登 RD-034+ 不进本期
  - graph_reorder_multiqueue   # render graph 重排调度/async compute 多 queue/split barrier/D3D12 enhanced barriers:首期 pass 声明序即提交序、单 queue 全序,超界面登 RD-034+
  - msaa_blend_stencil_indirect # P2-8（MSAA/blending/stencil/indirect draw):当前零 deferred 登记,不静默带入;需要时先补登记再评估（scratch 原案）
  - amd_witness                # AMD 侧 device 见证:G-MB1-6 硬件尾门存续,G3 全部 device 门锚定本机 RTX 4070 Ti,AMD 面标 pending-hardware 不伪造
  - upstream_filing            # 上游提报动作本体（NVIDIA 驱动/ptxas/LLVM/spirv-cross):agent 只备包全部 DRAFT — do NOT file,提报 owner 亲自（EA1 §8 纪律沿用）
  - ptx_texture_path           # kernel 内纹理 PTX 路（tex.2d):采样面仅存在于图形着色阶段（D-207 收集根排除),PTX 腿不承诺不伪造;OptiX 方向不登记不讨论
  - ei1_execution              # EI1 执行面（RD-009 #[export(c)]/UC-05 RHI):仅契约脚手架,激活 gated on G3 close-out + 立项确认,共享编号零消费
  - production_adoption_claim  # 外部采纳/用户数维度:显式 carve-out 沿 MS1/EA1 先例,验收全锚定自方可控工程物
deferred_refs: [RD-012, RD-018, RD-019, RD-020, RD-022, RD-023, RD-024, RD-027, RD-029]   # 九条全 open,本期兑现/处置对象;执行期新 RD 自 RD-034 起（RD-033=EA1 claim,RD-016/028 跳号永不复用;以合入时 deferred.json 实际为准双侧标注）
deliverables:
  - id: D-G3-1
    name: G3.0 治理包四件（本契约 + G3_PLAN + CI_GATES + g3_budget.json 空壳）+ number_ledger reserved_in_flight G3 行
  - id: D-G3-2
    name: G3.0 EI1 契约脚手架四件套（milestones/ei1/,gated,零实现零共享编号消费）
  - id: D-G3-3
    name: G3.1 RD-027 spike 全案——spike/rd027-pt-poison/ 探针 + evidence JSON/schema + 取证报告 + 归因结论 + deferred RD-027 history 追加;处置尾项（rurixc 修复+切片升回 256spp/4 回填,或上游 DRAFT 备包+护栏）
  - id: D-G3-4
    name: G3.2 present——RFC-0013 + 条款 RXS-0220~0222（预期）+ uc04 present 段/vk.rs 重建 + CI 步骤 61 红绿 + counter
  - id: D-G3-5
    name: G3.3 采样超集——RFC-0014 + 条款 RXS-0223~0230（预期）+ 三层实现（typeck/双腿 codegen/双后端运行时含 vk graphics descriptor 建面）+ CI 步骤 62/63 红绿 + counter
  - id: D-G3-6
    name: G3.4 bindless——RFC-0015 + 条款 RXS-0231~0235（预期）+ 三层实现 + CI 步骤 64 红绿 + counter
  - id: D-G3-7
    name: G3.5 render graph——RFC-0016 + 条款 RXS-0236~0241（预期）+ graph.rs 推导核心/双后端执行器/uc04 迁移互证 + CI 步骤 65 红绿 + counter
  - id: D-G3-8
    name: G3.6 mesh/task/RT——RFC-0017 + 条款 RXS-0242~0249（预期）+ Vulkan 主腿全量/DXIL 腿 probe 分层 + CI 步骤 66/67（±68/69）红绿 + counter + RX6008 改接
  - id: D-G3-9
    name: G3.7 close-out 终审（全量回归冻结 + 基准切换 + g3-closed tag + RD/SG 处置 + ledger 校准）
acceptance_gates:
  - id: G-G3-1
    check: "RD-027 归因闸门（owner 已裁「归因落地即开闸」,§7 ②）:① 当前工具链下毒径基线判定留档（工具链 13.2→13.3 漂移须先重立基线;每 GPU 运行经 bench/proc_guard 硬超时,超时=诚实红 124,零裸 launch,R-606）;② 四层判别矩阵（rurixc IR / LLVM NVPTX / ptxas / 驱动 JIT）至少给出一个排除或定罪结论,证据链含:同一 artifact 双装载路（cubin AOT vs PTX JIT）对照 + ptxas/JIT 优化档扫描 + 最小化触发构造（或诚实记录未收敛）+ compute-sanitizer 前置排除（OOB→应用缺陷改道亦为合法归因）;③ 开闸判定 = ② 归因证据（evidence JSON 过 check_schemas + 取证报告）合入 main 即成立,五面 RFC PR 自此可合;处置尾项——(a) rurixc/LLVM 侧修复 PR + golden 重 bless 留痕 + params.rx 切片升回 256spp/4 + ci/uc07_bench.py 补丁摘除 + ms1.bench.uc07_offline_frame_s measured 重测回填（RD-027 backfill_condition 全量兑现,RD-027 close）,或 (b) NVIDIA 侧 evidence/upstream-reports/ DRAFT 包（do-NOT-file + owner 复核门）+ 护栏决定留痕（如 ptxas -O pin）,RD-027 诚实存续——不阻塞开闸但属 G3 close-out 前置;④ 不伪造归因、不以超时无果充『驱动缺陷』:无法归因时本门维持未达、五面不开,G3 收敛路由由 §7 追加裁决;零 nightly/CI 新增毒径步骤、挂起后金丝雀门（已知绿基准秒级复绿）记录在案方可采信后续实验"
  - id: G-G3-2
    check: "present 面真实红绿（步骤 61）:RFC-0013 Approved 合入先于任何实现 PR（失败测试先行:步骤 61 脚本在 RFC 合入时点 main 不存在 = RED）;条款体 + 每条 ≥1 `//@ spec:` 锚定同 PR、commit 序条款在前;D3D12 可见窗口 flip-model swapchain present N 帧逐帧成功 + backbuffer readback 像素断言（与步骤 48 offscreen 同判据,承 MB1 W6 反『present 无 headless 校验』纪律）+ ResizeBuffers 重建后再 readback 绿;Vulkan OUT_OF_DATE/SUBOPTIMAL 重建路径真跑;RED:篡改 present 前 barrier（漏 PRESENT 态迁移）→ debug layer 报错翻红;无显示环境 SKIP（dev-env degrade 非 fake pass）+ RURIX_REQUIRE_REAL=1 翻硬红;offscreen 步骤 48 硬门 0-byte 不动（RD-019 backfill 明记 present 不得替代 offscreen）;本机 RTX 4070 Ti device 真跑 measured + run URL 归 §8"
  - id: G-G3-3
    check: "采样超集面真实红绿（步骤 62 codegen/host + 63 device）:RFC-0014 Approved 前置（06 §4.2 禁区增补条款全文批准,隐式 LOD 非均匀控制流语义对齐 D3D/Vulkan quad 语义不发明自有语义,UAV memory-order 对齐既有 Atomic scope 取最保守子集,严禁 UB 节）;全量模式集 device 见证:隐式 LOD/显式 lod/grad/texel fetch/可配置 sampler(wrap vs clamp 像素对照)/shadow 比较/gather/多分量/UAV 写回读 ≥6 模式数值判据（mip 金字塔逐层异色证真 mip;篡改→像素变红绿）;DXIL 腿 B 链 dxv+签名门不旁路,Vulkan 腿同一 SPIR-V 语义源双后端数值一致性对照;PTX 腿 D-207 不适用如实标注;RX3014/RX6023 违例 reject 通道扩类别经 UI golden;既有采样路（RXS-0174~0176 显式 LOD 0）零回归;device measured + run URL"
  - id: G-G3-4
    check: "bindless 面真实红绿（步骤 64）:RFC-0015 Approved 前置;无界数组推导从 Unmappable 拒改合法路径且既有有界推导零回归(binding_layout 单测回归网全绿);device 索引红绿:≥4 纹理注册表按屏幕象限动态索引采样、四象限像素==四色,篡改注册序→像素换位 RED;nonuniform 缺失→新 RX 码 strict 拒（UI golden）;unbounded range 独占新 space 分配律条款化;Vulkan descriptor indexing feature 探测缺失→确定性 Err 非 fake;SM6.6 heap 直索引收窄留痕（RD-018 close 记显式砍面 + RD-034+ 登记）;device measured + run URL"
  - id: G-G3-5
    check: "render graph 面真实红绿（步骤 65）:RFC-0016 Approved 前置（pass 边界 happens-before 语义本体 🔒 条款全文批准,仅承诺 pass 粒度全序单 queue,严禁 UB 节）;host 互证金标准:uc04 deferred 三 pass 图自动推导 barrier 集 == RXS-0169 手动锚点集（双实现互证,纯 host 单测恒跑）;环/写写冲突/未声明访问 reject 通道;device:uc04 迁 Graph API 重跑步骤 48 同判据（同一像素产出,手动换自动推导即最强回归证）+ 故意漏声明 read→strict 拒 RED + Vulkan 侧同图同判据;范围爬行零发生（重排/多 queue 面 RFC §8 锁死,出现即登 RD-034+ 不实现）;device measured + run URL"
  - id: G-G3-6
    check: "mesh/task/RT 面真实红绿（步骤 66/67,DXIL 腿视 probe ±68/69）:RFC-0017 Approved 前置;AST/parser/typeck 六 RT 阶段+mesh/task 全量类型面（intersection/callable 补齐,payload/attribute 契约错配 reject）;SPIR-V 1.4 分叉独立 PR 且 dxil 既有路（SPIR-V 1.0）零回归;Vulkan mesh:程序化网格 offscreen 像素判据+篡改 SetMeshOutputs 顶点数 RED;Vulkan RT:单三角形 BLAS/TLAS raygen/miss/closesthit 三件套命中/miss 双色断言+移动顶点→命中区域移动 RED（数据流红绿）;spirv-val vulkan1.2/spv1.4 三态 gate（Skipped 非 fake,REQUIRE_REAL 翻硬红）;DXIL mesh/task:probe-first（最小 SPIR-V→spirv-cross→dxc -T ms_6_5→dxv 证据先行）,绿则全量落+RX6008 改接,红则与 DXIL RT 同以 probe 证据落 RD-034+ 尾门（blocked 探针入 CI 防静默腐烂,对齐 RD-011/RD-015 跟踪纪律）;新 unsafe 全部 U30+ 登记;device measured + run URL"
  - id: G-G3-7
    check: "条款锚定延续门:本期全部新增 RXS 条款每条 ≥1 `//@ spec:` 锚定,trace_matrix --check 全程全锚定（215→N/N）;stable 快照因条款增长同 PR 重 bless + bless_log 同 diff（步骤 49 硬红不可分 PR）;新 RX 码 en/zh 成对（bilingual 96→N）;修订表表头「版本」数据行「版号」纪律全程"
  - id: G-G3-8
    check: "EI1 脚手架门:milestones/ei1/ 契约四件套合入,§0 gated on G3 close-out + 立项确认措辞就位（MB1 §0 先例）,零实现代码、零共享编号消费（RFC/RXS/RD/CI 步骤/U 全不 claim,届时以 ledger 实际为准);G3∥EI1 合并序约定（G3 先合、共享面后合者 rebase）治理化落 EI1 契约"
  - id: G-G3-9
    check: "close-out 收口门:全量回归冻结真实输出追加 §8（cargo fmt/clippy/test + trace N/N + stable --check + bilingual N/N + schemas/structure + budget_eval --strict 全局零 estimated + guardrails 当前默认基准 PASS + number_ledger + contribution + redistribution + 步骤 61~67 全冒烟 REQUIRE_REAL 真跑 + saxpy smoke）;验收门终审表逐门结论（blocked 面照 G-MB1-6 措辞『OPEN 尾门越过 close-out 存续,不签不伪造,状态翻转不依赖新契约』）;status active→closed;check_guardrails resolve_base 默认基准切 g3-closed（若 EA1 已先收口则自 ea1-closed 切,链单线性）+ 双基准 advisory 复核;合入后 annotated g3-closed tag（不匹配 release.yml 触发器）;RD-012/018/019/020/022/023/024/027/029 逐条 close/收窄/存续留痕（不 force-close）;SG-001~009 复评 + SG-010 留续号;被驱逐 main run 先 rerun 补绿再归档 run URL;number_ledger 校准 revision（G3 行收口）"
guardrails:
  - "milestones/m0~ea1 的 measured_local 既有预算条目 git diff 0-byte（ms1.bench.uc07_offline_frame_s 的 RD-027 回填属 backfill_condition 明记动作,按其协议重测回填并留痕,非本条违例;新增 g3 条目允许）;g3_budget.json 经 *_budget.json glob 自动纳入 + 命名空间强制前缀 g3.;counter/entries 不预造——登记与 ci/budget_eval.py evaluator 分支同实现 PR 落（未知 id 强制 FAIL）;全程零 estimated（EA1×G3 双活跃互斥保险);永不立外部采纳类条目"
  - "milestones/m0~mb1 的 *_CONTRACT.md（均 closed）只追加不修改（check_closed_contracts glob 已泛化);EA1_CONTRACT（active）本期 0-byte 不代动（其 close-out 归 EA1 自身轨道;RD-027 回填触其 CI_GATES/ms1_budget 按各自修订纪律留痕）;本契约 close-out 翻 closed 后自动纳入字节守卫"
  - "registry/deferred.json 与 registry/spike_gating.json 只追加;RD 处置仅由 agent 自主签署留痕追加;SG-001~009 维持现状、SG-010 留续号（窗口/UI 扩张诱惑出现登记 gating 而非提案);13_DECISION_LOG 执行 PR 字节冻结,开工裁决记本契约 §7,勘误走 00 §6.3 独立 errata PR"
  - "registry/error_codes.json 错误码语义可加不可改;codegen 新码自 RX6027 续号,RX6008 = RD-012 预留码本期正式改接（唯一预留已存在码）,RX6009 burned 不用;工具类确需自 RX7023;en+zh messages 成对(bilingual 96→N)"
  - "evidence/ 只增不删不改;上游备包全部文件 DRAFT — do NOT file 标头强制,agent 不对外提报;spike 探针标 // SPIKE(RD-027) 不入 src/ 生产路径不随产品编译"
  - "00–14 共 15 份规划文档不被执行 PR 改写（check_planning_docs);MS1 §7 旧文『g 系无 G3』语境由本契约 §7 ① 命名裁决覆盖,如需 11 号勘误走独立 errata PR"
  - "GPU 实验纪律:全部经 bench/proc_guard.guarded_run（禁裸 subprocess,R-606);挂起判定后强制金丝雀门;实验窗与 CI run/nightly 错峰;ptxas 输入恒 ASCII 路径;僵尸 exe 隔离 build/quarantine/;TDR/系统态零改动如实记录"
  - "src/ 新 unsafe 全部 // SAFETY: + unsafe-audit U30 起续号登记（U29=EA1 预留显式跳让,无论其释放与否不回收);单块单操作;vk.rs 手写 FFI 扩展沿 U26/U27 审计模式"
  - "既有零回归不变量:dxil 套件（404+ 恒定）/ vulkan 套件 grow-only / 步骤 41/48/54~58 既有冒烟判据 0-byte 只增;B 链 dxv validator + 签名门（RX6011/6012）不可裁剪不旁路;SPIR-V 1.4 分叉不动 1.0 路径"
  - "release.yml 触发器维持收窄;g3-closed tag 不匹配触发器零误触发;生产签名门控 0-byte"
  - "仓库 LF byte-exact（* -text）:新文件 LF + 尾换行,禁 Python 文本模式写文件;提交前逐文件字节核 CR + 尾字节（git numstat + 二进制读,禁 grep $'\\r'）"
  - "spec 修订表表头维持「版本」列名,数据行避「版本」子串（用「版号」）、忌「日期」子串入 bless 数据行;本契约既有条款 0-byte,close-out 只追加 §8;status 翻转/基准切换/g3-closed tag/RD·SG 处置由 agent 自主签署"
  - "guardrail 回退基准默认 = mb1-closed（PR 路径以 GITHUB_BASE_REF 为准;EA1 若先收口则届时默认自动为 ea1-closed,本契约措辞兼容两序);G3.7 close-out 切至 g3-closed 并双基准 advisory 复核"
---

# G3 契约 — 工业渲染期

> 所属:[../../06_MEMORY_AND_EXECUTION_MODEL.md](../../06_MEMORY_AND_EXECUTION_MODEL.md) §4.2 禁区面增补 + [../../registry/deferred.json](../../registry/deferred.json) 九条 open deferred 兑现 / 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1。
> 规范先行延续(AGENTS.md 硬规则第 7 条):语义面 PR 必须引用 RXS-#### 条款号;缺条款先补 spec,条款 commit 先于实现 commit。
> 基准 ref:**默认 `mb1-closed`**(EA1 尚 active 未切;PR 路径以 `GITHUB_BASE_REF` 为准;EA1 若先收口则默认自动为 `ea1-closed`,本契约全部 guardrail 措辞兼容两序,基准链保持单线性)。
> 粒度:**单 G3 阶段契约**:一份契约覆盖 G3 期,G3.0~G3.7 主线分解见 [G3_PLAN.md](G3_PLAN.md)。
> **定位口径:G3 把「工业渲染特性面」从 deferred 登记变成 measured 工程事实。**现状:图形面停在 RFC-0007 首期收敛子集(显式 LOD 0 单纹理静态 sampler)+ 全静态绑定 + 手动 barrier + offscreen-only + vertex/fragment 两阶段;RD-027 毒径挂起把 UC-07 生产档锁在 32spp/2 弹射切片,且被 scratch 依赖链判为「一切图形特性扩张」的可靠性前置。G3 以 RD-027 归因 spike 开局(owner 已裁归因落地即开闸),随后五特性面全量落地(owner 已裁全量推到底非最小切片)。「全量」的诚实边界:每条腿真实做到证据边界——DXIL RT 腿预判上游 blocked(spirv-cross HLSL 无 SPV_KHR_ray_tracing + RD-015 未解),以 probe 证据落尾门不伪造;measured-first/blocked-honest 硬规则高于「全量」表述。
> **治理口径:agent 完全自主(D-406 v2.0)**——G3 不触死亡路线红线(五面均为既登记 deferred 的兑现非扩张方向;红线 3 多后端已由 D-008 errata 解除);无 owner 裁决闸口,开工三裁已由 owner 本会话 AskUserQuestion 落定(§7 ②③④)。Full RFC 对抗性评审(D-409)全程:评审 provenance ≠ 起草 provenance,跨模型评审镜头,check_contribution 规则 4 机核。
> **脚手架口径:本契约为 G3 开工结构件,不实现任何语义面、不落条款、不打 tag;§8 close-out 开工时为空。**

---

## 1. 目标

G3 期结束时项目获得:① RD-027 毒径归因闭环——四层判别矩阵定罪或排除,修复(切片升回 256spp/4 弹射)或上游备包+护栏,UC-07 生产档可靠性结论落档;② 工业渲染五特性面全量落地——采样超集(隐式 LOD/导数/可配置 sampler/fetch/shadow/gather/UAV 写)、bindless 描述符索引、render graph 自动 barrier(含 pass 边界 happens-before 语义本体)、UC-04 窗口 swapchain present、mesh/task/RT 六阶段(Vulkan 主腿全量 + DXIL 腿 probe 分层),全部 Full RFC + 条款先行 + 本机 device 真跑 measured;③ EI1 契约脚手架就位(双轨另一半,gated);④ 九条 open deferred(RD-012/018/019/020/022/023/024/027/029)逐条 close/收窄/存续留痕。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | gating | 对应交付物 |
|---|---|---|---|
| rd027_gate_spike | RD-027 四层判别 spike + 归因 + 处置 | 纯取证不占 RFC;归因合入 = 五面开闸 | D-G3-3 |
| present_windowed | UC-04 窗口 present + swapchain 重建 | **Full RFC(RFC-0013)** + G-G3-1 开闸 | D-G3-4 |
| sampling_superset | 采样超集全量(RD-022a/b+023+024) | **Full RFC(RFC-0014,06 §4.2 禁区)** + 开闸 | D-G3-5 |
| bindless_descriptor_indexing | 无界数组+动态索引+nonuniform | **Full RFC(RFC-0015)** + 开闸 | D-G3-6 |
| render_graph_auto_barrier | Graph/Pass+自动状态推导+🔒语义本体 | **Full RFC(RFC-0016)** + 开闸 | D-G3-7 |
| mesh_task_rt_stages | 六阶段全量,Vulkan 主腿+DXIL probe 分层 | **Full RFC(RFC-0017)** + 开闸 | D-G3-8 |
| ei1_contract_scaffold | EI1 契约四件套(gated,零实现) | 结构件,G3 脚手架先合 | D-G3-2 |

### 2.2 out-of-scope(显式排除)

见 YAML 头 `out_of_scope` 字段逐项(window_input_system / heap_direct_index_sugar / graph_reorder_multiqueue / msaa_blend_stencil_indirect / amd_witness / upstream_filing / ptx_texture_path / ei1_execution / production_adoption_claim);11 §2 红线不触碰。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-G3-1 | G3 治理包四件 + ledger 登记 | milestones/g3/ + number_ledger v1.3 | 本 PR |
| D-G3-2 | EI1 契约脚手架 | milestones/ei1/ 四件套 | G-G3-8 |
| D-G3-3 | RD-027 spike 全案 + 处置 | spike/ + evidence/ + deferred history | G-G3-1 |
| D-G3-4 | present 面 | RFC-0013 + 条款 + 实现 + 步骤 61 | G-G3-2 |
| D-G3-5 | 采样超集面 | RFC-0014 + 条款 + 三层实现 + 步骤 62/63 | G-G3-3 |
| D-G3-6 | bindless 面 | RFC-0015 + 条款 + 三层实现 + 步骤 64 | G-G3-4 |
| D-G3-7 | render graph 面 | RFC-0016 + 条款 + graph.rs + 步骤 65 | G-G3-5 |
| D-G3-8 | mesh/task/RT 面 | RFC-0017 + 条款 + 双腿实现 + 步骤 66/67± | G-G3-6 |
| D-G3-9 | close-out 终审 | 契约 §8 + 基准切换 + tag + RD/SG 处置 | G-G3-9 |

## 4. 验收门(完整版,YAML 头为可提取摘要)

见 YAML 头 `acceptance_gates` 字段 G-G3-1 ~ G-G3-9。要点:
- **G-G3-1(归因闸门)**:四层判别矩阵 + 最小化复现;归因证据合入即开闸;处置尾项为 close-out 前置;不伪造归因。
- **G-G3-2~6(五面同构)**:RFC Approved 前置(失败测试先行) + 条款先行同 PR + device 真跑 measured + 数据流红绿 + 既有路零回归 + run URL 归 §8。
- **G-G3-7(锚定延续)**:trace 全程全锚定;快照同 PR 重 bless;bilingual 成对。
- **G-G3-8(EI1 脚手架)**:契约就位 gated,零实现零编号消费。
- **G-G3-9(收口)**:--strict 零 estimated + 终审表(open 尾门照 G-MB1-6 措辞) + 基准切换 + tag + 九条 RD 处置。

## 5. Guardrails(字节级,机器核对)

见 YAML 头 `guardrails` 字段。核对方式:`py -3 ci/check_guardrails.py`(无参默认基准 = `mb1-closed`;PR 路径以 `GITHUB_BASE_REF` 为准)。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-027 | UC-07 生产档 PT 毒径挂起(疑 PTX 重汇聚/工具链) | **闸门本体**:G3.1 归因 spike;处置按归因(close 或诚实存续+护栏) |
| RD-022/023/024 | 采样超集三缺口(RFC-0007 §8) | G3.3 兑现,close-out 关闭 |
| RD-018 | bindless/unbounded/heap 直索引 | G3.4 兑现;heap 直索引语法糖收窄登 RD-034+ |
| RD-020 | 自动资源状态跟踪(语义本体归 Full RFC) | G3.5 兑现(RFC-0016 含 🔒 本体),close-out 关闭 |
| RD-019 | UC-04 窗口 swapchain present | G3.2 兑现,close-out 关闭 |
| RD-012 | mesh/task/RT DXIL 降级(RX6008 预留) | G3.6:mesh/task 侧 probe 定,RT 侧预判 blocked→RD-034+;部分 close |
| RD-029 | mesh/task/RT Vulkan/SPIR-V 降级(MB1 首期外) | G3.6 Vulkan 主腿全量兑现,close-out 关闭 |

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源,本表仅引用。执行期按 14 §4 追加 RD-034+ 并双侧标注。

## 7. 修订记录 / 开工裁决留痕

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-18 | 初版契约固化(G3 开工脚手架)。**开工裁决**(owner 2026-07-18 拍板双轨立项 + 本会话三项 AskUserQuestion 裁定 + agent 完全自主判档 D-406 v2.0,记于本节;13_DECISION_LOG 执行 PR 字节冻结,不改决策日志):① **立项 = owner 拍板**:G3 工业渲染期 ∥ EI1 引擎集成期双轨立项,「脚手架 G3 先合」;RD/U/RX 等共享编号按 main 合并序消费、共享面后合者 rebase(该合并序约定治理化落 EI1 契约)。**命名 = milestones/g3/**,namespace `g3.`,收口 tag `g3-closed`(不匹配 release.yml 收窄触发器零误触发);MS1 §7 曾记「g 系(D-002 图形三阶段 G0/G1/G2)无 G3」——本期扩展 g 系序列为工业渲染期,承 G2 图形主线语义连续,该旧文语境由本裁决覆盖,如需 11 号状态勘误走 00 §6.3 独立 errata PR。② **RD-027 闸门判据 = 归因落地即开闸**(owner 裁定,推荐案):G-G3-1 以归因证据合入为开闸点,处置尾项(修复或上游备包+护栏)不阻塞五面但属 close-out 前置;若归因驱动/上游侧,备包 DRAFT — do NOT file 留 owner 复核门,RD-027 诚实存续,五面照开。③ **五特性面深度 = 全量推到底**(owner 裁定,弃最小切片案):每面按 deferred 条目完整诉求推进含 DXIL+Vulkan 双腿与 RT 全管线;measured-first/blocked-honest 硬规则高于「全量」——真撞上游 blocked 的腿(预判:DXIL RT 大概率、DXIL mesh 视 probe)以 probe 证据落 RD-034+ 尾门,照 G-MB1-6 先例越过 close-out 存续,不算失败不伪造不阻塞收口。④ **EI1 = 顺手立契约脚手架**(owner 裁定):仅四件套,§0 gated on G3 close-out + 立项确认,零实现零共享编号消费,G3 脚手架 PR 先合、EI1 脚手架 PR 随后。⑤ **判档**:五面全部 **Full RFC**(RFC-0013 present / 0014 采样超集 / 0015 bindless / 0016 render graph / 0017 mesh-task-RT;present 判档争议向上取严 = Full,硬规则 8;采样/UAV memory-order/pass happens-before 触 06 §4.2 禁区必须 Full);G3.1 spike 纯取证不占 RFC(G2.2 先例);RD-027 处置 workaround 若改工具行为判 Mini(MR-0011 留号)、纯修复判 Direct;每 RFC 强制 §9.1 对抗性评审(D-409:评审 provenance ≠ 起草 provenance,跨模型镜头,check_contribution 规则 4 机核)。⑥ **续号 claim**(编号永不复用,10 §9.5):Full RFC = **RFC-0013~0017**;RXS 条款自 **RXS-0220** 起,预期切分 0220~0222 present / 0223~0230 采样 / 0231~0235 bindless / 0236~0241 graph / 0242~0249 mesh-RT(以实现实际为准,溢出自 RXS-0250 顺续 + ledger 校准);CI 数字步骤自 **61** 起,预期 61 present / 62~63 采样 / 64 bindless / 65 graph / 66~67 mesh-RT(DXIL 腿视 probe ±68/69,步骤 70 集成 showcase 视判档;数量随实现回填不预占);错误码 codegen 自 **RX6027** 续 + **RX6008 预留码正式改接**(RD-012,唯一预留已存在码;RX6009 burned 不用)、工具类自 RX7023;unsafe-audit 自 **U30** 起(U29=EA1 预留显式跳让不回收);新 deferred 自 **RD-034** 起(RD-033=EA1 claim,以合入时实际为准;RD-016/028 跳号维持);SG 零消费(五面均为既登记 deferred 兑现非扩张方向,SG-010 软保留维持);共享 D 段零消费(D-408=P1-2 earmark 不动;D-G3-N 仅为交付物编号)。⑦ **执行编排**:主线 G3.1→G3.6 合入序严格串行;面 k 实现期间面 k+1 RFC 可并行起草(worktree 隔离,不进合并队列;RFC(k+1) 合入不早于面 k 实现 PR 合入;MS1 §7/EA1 支线先例);spike 期间零面 RFC 合入(闸门语义);runner 合一等一(pending 槽仅 1);实现 PR 结构 = 条款 commit 先行 + 实现同 PR(EA1 #158/#159 先例,砍 G2.1 中间脚手架 PR),大面可拆栈式。⑧ **EA1 并行存续口径**:EA1 期 active 余项(冷启动 A 段 VM/AMD/真机/提报)全为 owner 环境件,与 G3 零共享面;双方零 estimated 铁律使 budget --strict 互斥不发生;基准链单线性兼容两收口序(本契约 guardrails 措辞已固化);RD-027 回填触 ms1_budget/EA1 侧文件时按各自修订纪律留痕非字节违例。⑨ **诚实边界**:G3 达成表述 =「工业渲染特性面 measured 工程闭环」;AMD 见证 pending-hardware(G-MB1-6 独立存续);上游备包不提报;PTX 纹理路不承诺;外部采纳 carve-out 维持。**G3 close-out 关闭判定 / 基准切换 / g3-closed tag / RD·SG 处置由 agent 自主签署** |
| v1.1 | 2026-07-18 | **编号更正——对齐 owner 双轨分配**(更正 PR,合并序先于 EI1 脚手架 PR;与 EI1 轨会话协调后落):owner 2026-07-18 双轨立项的编号分配为 **G3 = RFC-0013(单号)/ RXS-0220~0249 / CI 步骤 61~70,EI1 = RFC-0014 / RXS-0250~0269 / CI 步骤 71~75**;v1.0 ⑤⑥ 的「五面五 Full RFC(RFC-0013~0017)」越权 claim 了 EI1 earmark,更正为**单伞形 Full RFC-0013,五面各成章**(present 章/采样超集章/bindless 章/render graph 章/mesh-task-RT 章;MB1 RFC-0011 单期伞形先例——一份 RFC 承载 compute+graphics+present 全期)。**读法更正**:本契约全文(YAML `rfc_required`/`in_scope` 注/deliverables/G-G3-3~G-G3-6 门文/正文 §2 §3 §6 各表)中「RFC-0014 / RFC-0015 / RFC-0016 / RFC-0017」字样一律读作「RFC-0013 对应面章节」(v1.0 固化原文 0-byte 保留,以本行为准;G-EA1-3 条件分支读法先例);0014 = EI1 earmark,0015~0017 为 in-flight claim 撤回归自由池(从未 materialize 为文件,无「复用已消费号」问题;实际 next_free 以台账为准)。v1.0 ⑥「溢出自 RXS-0250 顺续」更正为「**溢出自 RXS-0270 顺续**」(RXS-0250~0269 = EI1 earmark)。**伞形 RFC 执行语义**:RFC-0013 Approved 合入 = 五面「RFC Approved 前置」一次性满足,各面失败测试先行判据不变(各面步骤脚本在 RFC-0013 合入时点 main 上不存在 = RED);对抗性评审(D-409)对伞形全文一次覆盖、逐章 findings/disposition;probe 待定面(DXIL mesh/RT)以条件分支条款写入(G-EA1-3 先例),probe 结果落 RFC 修订行不重开 RFC。G3_PLAN/CI_GATES 同步直改(各自修订行 v1.1);number_ledger v1.4 校准同 PR。MR-0011/RD-034 起/U30 起/RX6027+RX6008 等其余 claim 不变 |

---

## 8. Close-out(只追加区 — 开工时为空)

<!-- 验收记录、guardrail 核对输出、G3.1 spike 归因与处置留痕、五面 RFC/条款/步骤 61~67 run URL、EI1 脚手架留痕、RD-012/018/019/020/022/023/024/027/029 处置、SG 复评结论、验收门终审表追加于此;上方条款 0-byte 修改。G3 close-out 关闭判定 / 基准切换 / g3-closed tag / RD·SG 处置由 agent 自主签署兑现。 -->
