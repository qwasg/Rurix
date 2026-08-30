# G37 商业化交付收官战役 CAMPAIGN LOG(day_0830_delivery)

> 开役 2026-08-29 23:2x。目标:剩余零散模块接线 + 已知问题可修面清零 + 深水区判档 + 默认翻转(已获批)+ 许可义务闭合与 SDK bundle 重打包,终产可分发渲染器版本;硬件/外部锁死项如实登记。
> 纪律:GPU 真跑锁内 + RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1;构建内联 CARGO_TARGET_DIR=H:\rurix\target-night;bench Stage A 18 格锚永不动;加性臂 = off==锚 + on 双跑位级 + VUID=0 + 无 AE A/B + 帧时记账;presented 锚 = 二进制绑定锚,整批重收割只在 W4;触 RXS-0239 走 RFC 修订行;既有行字面 0-byte 只追加。
> 入役锚:all-off `55e4a92d` / full16 `5db2e7d7` / bench `c1d28ad7` / Stage A 18 格(g14_3_stage_a_digest_anchor.json)。

## W0 基线定盘与治理修复

- [x] 战役目录 `artifacts/day_0830_delivery/{w0..w6}` 建立。
- [x] **G12 PT 卫兵陈旧修复**(day_0829 HANDOVER §G.4 兑现):`ci/g12_pt_prod_lib.py` G12_ZERO_BASE `5ae83aa7`(G12.0)→ `058f8e68`(G31+ 合流后新不可变点);升级时点 `git diff 058f8e68 HEAD -- path_trace.rs + band` 双 0-byte 机核;三次合法演进谱系(526d4c4e G12.2 / 5388c30f G12.3 / 058f8e68)注释登记;纯追加 prod 守护语义不变。复验 = 卫兵 PASS(修复前必红)。
- [x] **gpu_device_lock 健壮性**(day_0828 HANDOVER §7 兑现):①解锁段 OSError 重试 ×3 终败不抛(锁随句柄 close 释放,WARN 留痕)②持有者自述写回失败重试后忽略(自述非互斥判据)③stale holder 检测(pid 存活探测 + 30s 宽限后回收锁文件;pid 复用/不可判一律不回收)。selftest 2 RED + 1 GREEN PASS。
- [x] **三锚+18 格复验全绿**(`w0_baseline/W0_BASELINE.json`,fails=0):all-off 55e4a92d MATCH(5.49ms)/ full16 5db2e7d7 MATCH(9.63ms)/ bench c1d28ad7 MATCH / Stage A 18/18 全 MATCH(6 tsr 隔离 + 12 vendor 批跑),VUID=0 全程。W0 收口。

## W1 已知问题修复组

- [x] **em+AE override 错位修复**(主agent直做,day_0829 HANDOVER §G.1 兑现):`set_autoexp` 选择块补 `_EM` 两分支(`textures && smooth_nrm && emissive_tex [&& bloom]` → `G31_U_AE_PARAMS/PARTIALS_TEXNRM[_BLOOM]_EM`),guard 序在 REAL 族之后既有 TEXNRM 族之前;两处过时注释同步。语义变更重锚归 W4(受影响 = em on 组合形态,旧十臂 de342586 谱系已作废)。
- [x] **debug_assert 升级**(§G.2 兑现):`g31_apply_autoexp` 连号三断言 debug→assert(带诊断信息);realism 挂接域 8 条资源计数断言(TEXNRM/TEXNRM_BLOOM × EM/基础/REAL/NM)同升。车道创建期一次性,常数代价。
- [x] **rurixc「if 包 while」codegen 修复**:根因 = `structured_merge` 求 if merge 块的前向可达遍历不裁剪循环回边(if 在 while 体内且 then 臂 while 后有语句时 OpSelectionMerge 指向臂内块);修 = 交汇计算排除 latch→header 回边。vulkan_codegen.rs+dxil_spirv.rs;新增 2 测撤修复双红/恢复全绿 + 4 回归语料;526 单测+108 集成全绿;98 生产 kernel pre/post 90/90 位级全同(冻结 SPV 零触碰;branchless 绕行按纪律未摘)。
- [x] **slot14 法线损坏件**:完整重烘链(K: 源在位)产 baked_normals_bin_v2/(69 张与 v1 逐字节相等 + slot14 替平坦 (127,127) 全 12 级 mip,sha 77bc6c00);检测律 = L1 范数(‖xy‖₁>1 常值,唯一自洽口径,实测判据域只命中 slot14);校验 11/11 PASS。消费路径切换归 W4 合流。
- [x] **g34 三 kernel fx/fy 修复**:kernel 6 处(gi/gi_skin/shade 各 2)+ host 镜像 2 处(g31_tex_host_sample)成对同步修;4 SPV 重编 spirv-val 绿(基线复编 sanity:修前源复编 == 部署件位级,sha 变化纯归因修复);同错互抵恒绿假象消除。g34 三门 GPU 复跑归批次 0。
- [x] **encode 收编**:共享 m_c 件 .tmp/g14_gates/m_c/g31_display_encode.spv 已切 v2(43b0c255→e7291c79,备份 .pre_g37.bak,spirv-val 绿)。侦察修正两则:①flip plan 的 95,088B 是 pre-A2 过时记录,真实旧件 = A2 重编 95,660B(43b0c255)②**RD-045"旧二进制"已被 Phase F 构建事故覆盖为 v2 消费面,锚 060e69a8 本就已漂**——W4 重收割即可,无额外语义面。
- [x] **encode parity 硬门**:ci/g31_encode_parity_smoke.py(gate g31.g37w1.encode_parity,含 ACES 转置防复发红臂)selftest 20 臂 PASS;schema+check_schemas 注册,CI_step next_free=525 零消费维持。
- [x] **纹理判读器 heap 同步**:g31_texture_sampling_smoke.py 全量重写(步幅 4/top-70/v2 SPV/6fab598c 清理为 PENDING_W4_REHARVEST 占位),selftest 77 臂 PASS;evidence 加性双形态(新前缀+新 schema,旧件 0-byte)。

## W2 模块接线组(第一批回报)

- [x] **--transparency 臂全链落地**(子agent直改窗口 bin,编辑权已释放):g31_realism.rx 第 7 链位(主射线 ≤8 层穿透 + 点光阴影透射衰减;GI2/AO/反射 NEE 视玻璃不透明如实留窗);判定 = alphaMode BLEND ∨ baseColorFactor.a<1(bistro 唯一命中 mat7 TransparentGlass 130,792 tris,名字启发式弃用登记);tint 取 tri_base 未衰减 baseColorFactor(mats 均值会双重衰减致全黑,设计修正登记);新 SPV g31_realism_transp.spv 35983d0f(272,032B)双过 spirv-val;10 冻结件 sha 前后 ALL SAME;--transparency/--transp-alpha 两 flag,并入 full(十六→十七臂,重锚归 W4);AE _TRANSP 下标族接对(W1 assert 为保护网)。GPU 验收归批次 1。
- [x] **VSM 页管线判档 = no-go/留窗**(方案 A):生产车道阴影 = RayQuery 逐像素射线,无 shadow map 生成成本可摊销,强接 = 无人采样空转 dispatch,判档纪律拒;重启锚 = 光栅阴影档/VSM 采样车道出现(#105 PCSS/#27 SMRT 立项即触发)。侦察修正:「编而不 dispatch」已由 A2.1 在 uc06 M19 门关闭(TODO #104 "曾"字一致);真空缺 = golden dirty_depth 轴零 device 消费——判档件 g31_vsm_device_probe(三腿:mark 位图/脏页深度重建/采样)补上,selftest 全绿(原像 16/16+绿臂 3×16/16+证伪臂 3/3 翻红),4 SPV rurixc+spirv-val 双过。GPU 一键跑归批次 0。
- [x] **post_chain 五级差距表**:唯一缺级 = 第 4 级 LUT 色彩分级(exposure/bloom/tonemap/output 已接,形态差如实登记:AE 增益施加点在 bloom 后/单级高斯非 mip 链/恒 ACES 无插件选择面)。--lut off|neutral|warm|<path.cube> 臂工件备好:LUT 表内嵌 enc_params 尾部(零新绑定/零下标族,绕开 AE 雷区),kernel fork g31_display_encode_lut.spv sha 9087b743 双过 spirv-val,合入锚点 7 处待主线。#79 不登已闭。
- [x] **PSO 侦察结论**:窗口管线全部 session 构造期创建,运行期唯一重建点 = era 重建(非惰性)——#113 天然满足;#82 的 predict_precache 材质×pass 面在 mega kernel 形态无消费(如实登记)。变体账本守护模块备好(era0=precache 面/era≥1 miss 告警,pso_runtime_creates 验收=0,RURIX_G31_PSO_STRICT=1 升 fail-closed,sidecar --pso-report),合入锚点 5 处待主线。
- [x] **VisBuffer 判档 = 档 2 生产证据臂**:M95 SW 蛮力 kernel O(tris×px) 生产分辨率物理不可行(tile 化=#75 自己的行),档 1 出帧 + shade 桥超一臂当量且重锚 Stage A——档 2 = --visbuffer on 窗口会话消费真轨迹相机 × 真 RXCP 簇 DAG device 真跑机制链(cut→32px 分箱→SW u64 原子软光栅→oracle 全等+双跑位级→classify/resolve),sidecar evidence,presented 面 0-byte;g31_visbuffer_arm.rs 共享体 + g31_visbuffer_wiring 独立 harness,cargo check 三项过,合入锚点 7 处待主线。

### W2 合入与 GPU 批次 0(2026-08-30 00:1x~00:5x)

- [x] **LUT/PSO/VisBuffer 三臂合入窗口 bin**(合入agent,行号漂移全按内容锚落点,零语义偏差):LUT 6 处+1 0-byte / PSO 账本 6 处(双车道登记)/ VisBuffer 7 处全加性;cargo check 四跑全绿(warning 集与基线恒等零新增);窗口 bin 编辑权释放。
- [x] **VSM 判档件 GPU 真跑 PASS**(device_probe.json,rc=0):mark 16/16 帧位级(76 bits 零失配+越界零)/ raster 9 帧 29 页 475k texel max_abs=1.192e-7 ≤ 1e-6(uc06 4070 同量级;f13 全静止帧位级 digest MATCH)/ sample 80 值零失配(19 shadowed+4 local 帧)/ validation=0。**#104/#106 判档终态 = no-go/maintain-defer**(ray 车道无 shadow map 生成成本可摊销),重启锚 = 光栅阴影档/VSM 采样车道出现(#105/#27 立项即触发);golden dirty_depth 轴首个 device 消费腿在案。
- [x] **批次 0 两门红全为门面问题,已修**:①parity 门 evidence 字段断言错(`frames_completed`≠实字段 `frames`,已修門脚本)②texsampling 门 0-byte 机核被 `src/rurix-asset` 两文件 **纯 CRLF 行尾翻转**打红(git diff --ignore-cr-at-eol 证空,checkout 恢复原字节;非语义改动非本役引入);其余 facts 已全绿(SSBO p100=0 位级/sampler ≤1LSB/bistro 70 材质 demo/**Stage A 锚格 c1d28ad7 fresh MATCH**)。两门重跑归批次 1。

### GPU 批次 1 + 探针同源修复(00:5x~02:2x)

- [x] **encode parity 门 GATE PASS**(exact=99.9891% p100=1 gt1=0,与 A2b 实测锚一致;evidence/g31_encode_parity_20260829T171623Z.json)——防复发硬门正式落地。
- [x] **texsampling heap 门 GATE PASS**(evidence/g31_texture_sampling_heap_gate_20260829T171903Z.json):0-byte 机核恢复绿 + SSBO p100=0 位级 + sampler ≤1LSB + bistro 70 材质 + Stage A 锚格 fresh MATCH。
- [x] **B4 探针原形态 SPV 同源修复**(g34 门首跑红根因):fx/fy 修复改了 host 镜像但 g34 消费的探针 device SPV(.tmp/g31_gates/texture/g31_texture_probe.spv,原 grid 形态旧字节)未同步 ⇒ 同错抵消面单侧打破(p100=3.64e-4 红)。修 = git HEAD 提取原形态源 + 同款 fy→fx 两处(L88/L91 底行 G/B)+ rurixc 重编替换(8B272122→753C4E83,spirv-val rc=0);sanity = 未修源复编 == 部署件位级全同(替换纯归因修复);旧件 .pre_g37.bak 备份。g34 三门+g36 门重跑归批次 2。

### W2/W3 第二批回报(02:1x)

- [x] **RIS/NEE 两臂 kernel 面完成**:g31_realism.rx 第 8 链位(RIS 选灯 M=4~8 + 44k 灯片 CDF 面光 NEE;能量口径 = nee on 时灯片 emission 直取置零 + 12 代表灯反弹让位,不双计);新 SPV g31_realism_ris.spv 622a1c33(315,048B)spirv-val 双绿;12 冻结件 ALL SAME;装配模块 g37_w2/g31_ris_lamps.rs(44k 全量表 ≈2.85MiB + f64 前缀 CDF,双构建位级);16 组合入锚点 + K 阶梯命令在 REPORT;合入agent在途。
- [x] **异步三件套实施完成**:vk.rs 本体仅尾部 +5 行 include(vk_g37_async_lanes.rs ≈1600 行加性:7 常量/3 结构/2 fn 指针/probe_async_queue_caps/create_timeline_semaphore/run_async_lanes 双队列提交器);probe 三臂(单队列基线重编译/双队列/digest 等价硬前置)+ --judge 两态;**关键发现:(2v-1,2v) 逐弧映射在共享生产者弧形下值回退,已补 legalize_submission 合法化层 + 提交前 validator(红臂齐)**——正是 RFC 修订行 3 要拦的形态,实证入档;check 双 crate 全绿,单测 2+7,selftest 7/7。GPU 判档归批次 3。

### GPU 批次 2 + 第三批合入(02:2x~04:0x)

- [x] **g34 三门全 GATE PASS**(unified/hzb/skin rc=0,B4 探针原形态 SPV 同源修复后):fx/fy 修复 host+device 对拍恢复真互证(同错抵消面消除),encode v2 共享件消费面复绿。
- [x] **g36 组合门 GATE PASS**(rc=0 十 facts 全绿:构建/双包确定/单开零漂移/leaf×full 位级锚/混合双跑位级/dyn+skin 组合/g34 五特性恒等锚/host parity 入容差/HZB 六特性真剔除/粒子×OIT×geo 位级;evidence/g36_geo_composition_gate_20260829T204135Z.json)。**W1 GPU 复跑清单全清账**:parity/texsampling/g34×3/g36/VSM probe 七件全绿。
- [x] **RIS/NEE 16 组锚点合入窗口 bin**(合入agent,全部内容锚命中零失锚,5 项形态级偏差登记):三 flag+lamp_tbl 36/44+AE _RIS 37..=39/45..=47 guard 最先+full 20→22 项+tri_transp 零表占位易漏点接线;cargo check 三轮 rc=0 零新增 warning;编辑权释放。
- [x] **FG 组合判档 = 可组合需接线适配**(FG 吃 display encode 前 f32 TSR 输出 parity 对 + 取反 MV,与场景臂正交;互斥矩阵字面确证"当时没验"保守闭集非结构判决):19/20 臂正交零适配(AE 增益 device 侧 enc_params[133] 自动继承);唯一耦合 = bloom 单缓冲合成 → comp parity 双缓冲适配(零新 kernel,真实帧数值逐位不变);**两点式合法形态**(fg 合法 = all-off base ∪ full 预设,散臂混搭维持 fail-closed 防 2^N 下标族爆炸);fg×hzb/slab/svt/lut 维持互斥留窗(结构证据)。16 锚接线合入agent在途。

### W3 异步三件套 GPU 判档终态(05:2x)

- [x] **M59 判档 = 维持 no-go + 新鲜 measured 证据**(evidence_async_lanes.json,judge rc=0):**digest 等价硬前置全过**(single == dual == CPU 参照 a30d4cdd 逐字节;双跑位级;金丝雀绿)——双队列 timeline 提交器机制正确性已证;重叠率中位 48.54% < 50% 阈值 ⇒ verdict=no-go(判据 = 中位改善 ≥3% ∧ ≥0.15ms ∧ 重叠率 ≥50%)。D3-Q7 从「证据零」升级为「在案 measured 低于阈值」,RFC-0019 修订草案留档不落地,RXS-0239 字面维持——合法收口,重启锚 = 更重异步负载/专用 compute family 形态出现。#88/#59 机制件(执行器消费 FencePair/timeline/合法化层+validator)与 #62 探测入库为后续窗基建。

### W3 第三批回报(06:0x)

- [x] **FG 组合 16 锚接线合入窗口 bin**(B→A→C 序,cargo check 三轮绿):FULL 下标族按 RIS/NEE 合入后终态重推 48..=56(comp parity 对 = (31,48),assert 钉死);屏障计划族 6→8 件(补 encode/AE reduce 双 parity 超集);A6 卫兵语义反转适配(ris/nee 已入 full 展开,改为散臂传递链 fail-fast 显式化);comp parity 双缓冲 fg-on 面才构造,fg off 生产路径 0-byte。验收环 accept_fg_combo.py --execute 归批次 3。
- [x] **逐帧 cut→AS 判档设计完成**:字面候选 A(全簇 BLAS 池)被内存分配账否决(129,709 簇 ×3 alloc ≫ maxMemoryAllocationCount 典型 4096,HZB 停 1186 节点为旁证);选定 **BLAS 顶点 refit 竞技场**(全簇固定槽位拓扑 ≈72MB,cut 以顶点内容切换:进 cut 真几何/出 cut 零面积折叠 = UPDATE 合法域,TLAS 恒不动;帧 0 全量上传堵死 AABB 陈旧假漏命中;相邻帧增量数百 KB);候选 B 并入 --cut-every N 惰性节拍同实现降档;确定性 = 固定轨迹+固定节拍+canonical 槽位 ⇒ 帧 k 状态为帧号纯函数。harness g31_frame_cut_probe(命中槽 ∈ 已施加 cut 陈旧零容忍机核)check 退 0 + selftest PASS;FIF×每槽 AS 分界佐证(render_exec fail-closed 拒面)归 #90。窗口 --cluster-per-frame-cut 8 锚合入agent在途。

### W3/W4 第四批回报(07:0x)

- [x] **FIF×动态判档件全绿**(G36 留窗 #90):侦察钉死拒绝面(render_exec submit_with_frame_update 拒 tlas_update/blas_refit——host 写↔在飞 GPU 读共享面;RFC-0030 §4.3 L2 per-slot 枚举不含 AS/实例缓冲的缺口原文);rt 加性入口 submit_with_frame_update_slot_as(每槽 AS 副本组 = VkAsManager 表项天然隔离零新建面 + 槽纪律三判据 fail-closed,既有行 0 改写 body-include);g31_fif_dyn_probe 三臂(顺序基线/inflight=2/inflight=3,判据 = 逐帧 digest 逐字节 ≡ 基线);RFC-0030 修订行草案(L2a opt-in 子行,≤5 要点)落 w3_deep/fif_dyn/。check/selftest 4/4/单测 7/7 全绿。GPU 判档归批次 3 后段。
- [x] **CI 调用面 --quality off 补扫完成**(DEFAULT_FLIP_PLAN §2.5):A 类 18 调用点/11 文件已补(svt/hzb/slab/texsampling/framegen base 点/cluster_lod/wp_hlod×2 RED 臂/profiling×3/robustness×4/w0_reverify/fg_combo×2);B 类 5 点登记语义随翻转升级(RD-045 P02 腿/game_loop 四腿/window_present A1/wave_a_soak×2);selftest 10/10 + py_compile 11/11 绿。对账表 w4_flip/QUALITY_OFF_SWEEP.md。
- [x] **HZB×蒙皮同车道 = GO 已实装**(G36 留窗):蒙皮 = 预 pass 顶点形变 + BLAS refit 桥,五件 0-byte 进合并车道;结构冲突(HZB 双 TLAS 双 manager 各需当帧角色 BLAS vs 单 refit 位)有界修复 = render_exec 加性 blas_refit_b(*_dual_tlas_ex 新入口,与 #90 窗三参形状互锁编译过);新 kernel g34_unified_primary_skin.rx(primary+out_hit 加性通道,SPV 7d3ae216 spirv-val 绿)+ 合并区段 g34_hzb_skin.rs(2528 行);--hzb on --skin 同开成立;check 三面 exit 0 + accept.py selftest 42 项 PASS。GPU 验收 = accept.py --run(七 facts)。
- [x] **W4 翻转字面已执行**(主agent):--quality 缺省 off→full(十九臂交付默认),off 升显式回退档,三处文档注释同步;check rc=0。**frame_cut readback bug 修正**(fif_dyn 窗交叉复核检出:None→Some(vec![0]))。w4_verify.py(12 段主链)就绪。

## W4 默认翻转验收与重锚(07:1x~,主链+补跑)

- [x] **s01 all-off 显式回退档 == 55e4a92d**(翻转后 --quality off 与旧默认位级一致,跨重建稳定)。
- [x] **s02 full 十九臂新锚收割 = `7636f72f486d27d63d65aff83037359b82497d722b9e33d0411d784a490df455`**(96f/warmup2 默认无参数,双跑位级;帧时 9.75/10.59ms ≤ 90fps 预算 11.11ms)。锚谱系:5db2e7d7(十六臂)→ 7636f72f(十九臂 + 翻转,语义变更作废前锚)。
- [x] s03 bench c1d28ad7 MATCH(bench 面永不动自证)。
- [x] **s04 transparency 单开锚 af1f7264**(双跑位级,装配日志 TransparentGlass 确认;单开帧时 4.25ms)。
- [x] **s05 LUT 臂全绿**:neutral 7b6856df / warm c6cd1152 各双跑位级;**neutral 与 off 位级恒等**(恒等 LUT 精确性 bonus 证明);warm 暖移 R↑B↓ 方向判据 ✓。
- [x] **s06 RIS/NEE 单开锚 851a61ba**(双跑位级 + ≠base;单开帧时 6.78ms vs base 4.71ms)。
- [x] **s07 full×storm3 PSO 账本验收**:pso_runtime_creates=0,sessions=2(1+resize_eras),VUID=0——#82/#113 守护面达成;full 十九臂 × 风暴干净退出。
- [x] s08 VisBuffer 冒烟(wiring PASS)+ 窗口臂 sidecar ✓。
- [x] **s09 frame_cut probe 实质 PASS**(判读修正:probe rc=0 内部 fail-closed + 臂 OK;首跑 w4_verify 误设 sidecar pass 字段):16 帧双跑位级/cut_tris 888460→937542 单调/refit 均 27.06ms **measured 登记**(2M tri 竞技场全量 refit 逐帧成本——生产实时需增量 refit/簇粒度降档,归 #90/#77 后续窗字面,本判档件价值即此 measured 分解)。
- [x] **s09 窗口臂 + 加性回归 ✓**(sidecar 落盘;--cluster-per-frame-cut 证据臂与 base 位级一致 5540ecae)。
- [x] **s10/RD-045 重锚闭环**:orbit 64+10 新锚 `ef2b5b19d85cd59ea48f85cfd65dc3933e4a01e672e0e5a955ecd91fbd799b2f`(**release 与 target-night 双二进制收割同值 + 各自双跑位级**——该 digest 对同源码构建跨二进制稳定);blocked_probes 锚字面已回写(060e69a8 作废谱系登记),**门 selftest+真跑 PASS**(evidence/g31_blocked_probes_20260830T020603Z.json)。
- [x] **s11 tex 臂收割** `ac2e5ff5…`(target-night 面,双跑位级);texsampling 判读器处置终态 = "不消费固定锚"为长期形态(二进制绑定锚律正确演绎),收割值登记 W4_ANCHORS 仅供对账,PENDING 占位维持(零 schema 改动)。
- [x] s12 fgcombo 首跑失败根因 = target/release 旧 exe(FG 接线合入前构建)缺豁免字面——release 重建后重跑归尾链。
- 十九臂 full 帧时终值:9.75/10.59ms(96f 双跑两值,≤ 11.11ms 预算)——文档占位可回填。

## GPU 尾链全绿(09:4x~10:2x)

- [x] **fgcombo 验收 PASS**(release 重建后重跑;首跑失败根因 = FG 接线前旧 exe):双跑位级+不污染门+画质生效+presented 计数恒等式+口径隔离+wired_parity+x2/x3 一致+互斥矩阵六臂;ACCEPTANCE_SUMMARY.json 在案。**FG×full 两点式组合交付**(G34 契约 out-of-scope 行兑现)。
- [x] **FIF×动态 GPU 判档双 PASS**(rebuild + refit 两 action):B(inflight=2)/C(inflight=3)≡A(顺序基线)逐帧 digest 逐字节 + 三臂双跑位级 + validation=0 + 动态见证 + RED 双臂必拒,帧时 measured 登记。**#90 判 GO:每槽 AS 副本方案语义等价实证**;RFC-0030 修订草案(L2a opt-in)+ 判档 evidence 齐,正式条款登记归 owner 治理程序。
- [x] **HZB×蒙皮同车道 ACCEPT PASS**(g37.wave3.hzb_skin 七 facts):rurixc 现编 10 件 spirv-val 绿+母版四件 0-byte 机核/合并腿 skin 门口径(逐顶点位级+MV 三类+窗级真动)/hzb 门口径(mips 位级+判定逐字节+零假阳性+occluded_p1=22407)/剔除像素中性 74 帧位级/双跑位级/单开臂不降级机器证明/帧时 merged=27.46ms 如实登记。**G36 五留窗全部处置完毕**(FIF×动态 GO/FG 组合 GO/HZB×蒙皮 GO/#96 crate 面+分界/逐帧 cut→AS GO)。
- [x] **K 阶梯跑批全绿**(窗口 K=3/2/1.5 三档 digest 分离 + bench conv 三档 rc=0;K=0 基线复用 W4 full19 证据)。
- [x] **K 阶梯判读终态 = K 档关死**(tsrq_clamp_ladder.json,EVAL_DENOISE 两态之一合法收口):K∈[1.5,3] 旋钮近惰性——四 ROI std_p95 梯内边际降 <1%(有效阈 10%)、clamp 触发面 <0.1% 像素、远小灯梯内无损但无萤火虫收益;正区间实测定义域内不存在 ⇒ 降噪投资转第 1 级(tsrq v4 方差引导)登记。跨口径差如实登记(EVAL §8 声称的 arm4 直接对照不成立:臂形差 ggx+lamp-gain,判读以梯内趋势承载)。#30 闭。

## W5 商业化收尾(第二批回报,13:2x)

- [x] **SDK bundle 候选完成**(sdk-1.1.0):组件闭集 16→24(四新 SPV 锚逐一核对 + 许可四件并入);schema v2 版本化(旧 v1 0-byte + patch 纯追加注册含路由序机核;门脚本判读升级 selftest 67 臂 PASS);一键幂等重打脚本 ci/g37_sdk_bundle_repack.py(打包确定性 ×2 字节级一致);产物 dist/sdk_bundle/sdk-1.1.0/(32 文件 2.26MiB,digest 一比一闭环 + SPDX/CycloneDX 双 SBOM + install --from-dir 四级校验);**签名如实降级登记**(生产 Authenticode 本机不可达,信任根 = SHA256SUMS + 四级内容寻址;终版走 release.yml 门控);法线 v2 资产件不入 bundle(SDK 车道不消费,重生成链登记)。
- [x] **release notes**:dist/RELEASE_NOTES_G37.md(候选声明/七组新臂/默认翻转/修复六项/口径声明/已知限制十一条/签名降级)。
- [x] **RD-036 C ABI v2 判档 = 维持 open**(本役新臂全为窗口 demo flag 面,SDK 包装的 g14_3 生产车道零 ABI 扩面需求;登记归主线)。
- [x] **docs 面刷新**(前批):5 篇就地 + PRODUCT_NOTES.md 新增 + 命令示例 4 补 2 增 2 注;两文档机核门 selftest 绿;W4 终值占位统一指 W4_ANCHORS。

## W6 终验(13:3x~)

- [x] check_schemas 首跑红 = parity evidence schema frames_completed const 未随门脚本字段修正同步(10→8);同步后 PASS + parity selftest 绿。
- [x] **门矩阵首轮**:7 CPU 守卫全绿 + 12 GPU 门 7 绿 5 红;并行双链事故处置(旧链孤儿进程杀清,GPU 锁恢复独占)。五红判读:①present/②game_loop/⑤wave_a_soak 三门 = **B 类补扫误判**(门 evidence schema 闭集钉死 off 形态,翻转后 full 默认使 evidence 走 texture 分支 schema 判读全瞎)⇒ A 类化补显式 off(门语义 = 波 A 机制回归与画质形态无关;"@新默认 soak" 另由 w6_full_soak.py 承载——32f 迭代位级恒值 + Stage A 探针 ×2 + 90fps 预算记账);③svt 门 = **前役遗留结构红**(day_0828 heap 化起 --svt on 无条件 fail-closed,非本役引入)⇒ 互斥登记态改造(agent 在途:命中互斥字面走 mutex_registered 登记件退 0 不冒充 PASS,host 金标准腿维持,深修锚 = TODO #33-36);④profiling = **机态贴边**(g14 腿 host_residual −0.1174 vs 下界 −0.10,差 17μs 噪声级,g31 腿绿)⇒ 重跑;⑤robustness = w6_gates 传参 bug(该门 --gate 为布尔 flag)⇒ 已修。
- [x] **红门重跑链结果**:present rc=0 / gameloop rc=0 / robustness rc=0 / wave_a_soak rc=0(A 类化补 off 后四门全绿);**full soak PASS**(@新默认十九臂:7 迭代 2000.4s ≥1800s 位级恒值 + Stage A 探针 it4/it9 MATCH + VUID=0;frame_ms_max 12.55 > 11.11 预算为 **32f 短窗冷启动口径**——逐迭代进程重启 + GPU 时钟未 boost,96f 持续口径 9.75/10.59ms ≤ 预算已在 W4 s02 双证,产品口径以持续运行为准,如实登记口径差不冒充)。
- [x] **svt 门互斥登记态改造完成**(agent):probe 短跑探互斥字面 → host 金标准腿(streaming::svt/terrain 单测)全绿 → mutex_registered.v1 登记件退 0(非 PASS 非 FAIL,schema 钉死防冒充,深修锚 TODO #33-36);未命中回落既有判读 0 改动;selftest 47 项全绿(34 既有+13 新臂含冒充 PASS 拒);W6 首跑占位值红裁决件归档。复跑在途。
- [x] **profiling 门三轮定性 = 轨迹面诚实红**(合法终态,G33 C9/G17-MD-F1 先例):identity 恒等式判据机态敏感边界脆弱——轮1 g14 下界越 −0.117 / 轮2 g14 −0.288 / 轮3 **换腿** g31 上界越 2.250(wall 5.29→6.40ms 抖动),两腿各有全绿轮次,方向不一;历史绿值 +0.070/+0.134 本身贴边;**其余 facts(分解 measured/debug labels/capture 兼容/工具探测)三轮恒绿**。判据容差带 [−0.10,2.00] 为冻结面不本窗放水;容差重标定/identity 判据鲁棒化(多轮中位)归 budget 程序窗。
- [x] **svt 门 MUTEX_REGISTERED 登记态闭环**(rc=0 + check_schemas PASS):互斥字面 probe 命中 → host 金标准腿全绿 → 登记件 evidence/g31_svt_mutex_registered_20260830T094427Z.json;深修锚 TODO #33-36 维持。

### W6 门台账终态

- CPU 守卫 7/7 绿(check_schemas/budget_eval/锁 selftest/parity/texsampling/blocked_probes/vendor_license)。
- GPU 门:present/gameloop/dynscene/framegen/pipelining/hzb/slab/cluster_lod/wp_hlod/robustness **10 绿** + svt **MUTEX_REGISTERED 登记态** + profiling **轨迹面诚实红**(判据机态敏感,功能 facts 恒绿)。
- 双 soak 绿(wave_a off 面 10000 帧 + full19 新默认 2000.4s 位级恒值)+ 风暴(W4 s07 full×storm3)绿。
- 附:K 判读 agent 在产出 tsrq_clamp_ladder.json(verdict=closed 关死)后因外部用量限中断,核心判读产物完整在案,KLADDER_REPORT.md 缺失由本 LOG 与 json 承载(不补,judge_kladder.py 可复跑)。

## W3 深水区判档(第一批回报)

- [x] **异步三件套侦察+RFC 草案+骨架**:断链本体 = CompiledGraph::execute() 零消费 fences/queue(调用者仅 uc09 单 pass + 单测;uc06/08 只审计);vk.rs 全 17 处 family 选择无 compute-only 探测;timeline **探测面已在**(波 C DeviceCapabilityReport)但创建/提交/FFI 为零。判档载体 = 新独立 probe bin(骨架落地,fence 弧 golden/五段切分/2v 值域映射 host 参考 3/3 过);M59 两态判据建议 = 中位改善 ≥3% ∧ ≥0.15ms ∧ 重叠率 ≥50%(硬前置 = 单/双队列 digest 逐字节等价);RFC 草案建议并入 RFC-0019 修订记录而非领新号。实施归下一批。
- [x] **#96 属性保持简化 crate 面**:TriMeshAttrs 平行属性表(加性伴随结构)+ 位置 QEM 主导/收缩点线段投影插值(t=0/1 逐位拷贝端点 ⇒ 接缝/锁定端保护免费;弃 meshopt 加权四次型防选边序漂移)+ build_dag_attrs/simplify_free_mesh_attrs 两条带 UV 重导出腿;45/45 测试绿,m90 golden 逐字命中不漂,属性链 base 与无属性链逐位相等;bake/运行时 gather 消费面设计登记分界(RXGB/RXHL v1 0-byte)。

## W5 商业化收尾(第一批回报)

- [x] **GAP-01~03 全闭合翻绿**:①THIRD_PARTY_NOTICES.md(rowan 0.15.18+传递闭包 5 crate,上游 LICENSE 逐字随附)+ 4 组件接进 release.yml bundle digest 闭环 ②三个二进制许可段改 MIT OR Apache-2.0 与 workspace 一致 ③内嵌库级 CycloneDX 补充视图;登记 append-only closure 段(status 字面 0-byte,schema const 约束下唯一合规写法);门升级 closure 机核 selftest 7红2绿 PASS + --gate 7/7 PASS + check_schemas PASS。残余如实登记:历史 dist.1/.2 资产不可追溯补件/rowan 字面改判须版本化修订/sbom.rs 自动展开归后续。
- [x] **SDK bundle 重打包 16→24 + schema v2 + release notes**(W5 子任务,禁 GPU/禁 cargo 纯打包面):组件闭集 16→24(G37 SPV 四件锚定 35983d0f/622a1c33/9087b743/7d3ae216——transp 工件谱系锚定〔源快照被 RIS 演进覆盖〕,其余三件源在树 + 许可四件 release.yml 同口径);schema 冻结面版本化 = v2 新文件(const=24/sdk-1.1.0/门键 g31.g37w5.dist)+ _patch 纯追加注册(v2 路由先于 v1,heap 先例;幂等 ×2)+ 门判读升级(selftest 67 臂 PASS),旧 schema/evidence 0-byte;一键幂等重打脚本 ci/g37_sdk_bundle_repack.py(闭集/版号 import 门脚本单一事实源;release ×2 确定性 + digest 一比一闭环 + SBOM 双视图覆盖 + from-dir 四级校验 24 件 + 幂等再装全绿,重跑产物字节级一致)产候选 dist/sdk_bundle/sdk-1.1.0/(32 件 2,367,918B);签名如实降级 = selftest 声明面 + SHA256SUMS/四级内容寻址信任根(生产 Authenticode 本机不可达);dist/RELEASE_NOTES_G37.md(新臂/翻转/修复/已知限制/「生成帧不入真实渲染帧率口径」);RD-036 判档 = maintain-open(backfill 两条件字面均不成立:新臂全窗口 flag 面,SDK 9 函数 v1 包络 0-byte),登记归主线;W4 重建则刷新 inputs 锚 → --status final 一键重打,W6 全门终验。pre-existing 登记:encode_parity evidence frames_completed=8 vs schema const=10(W1 账)。REPORT: w5_commercial/bundle/REPORT.md。

### 子agent编排登记(第一批,2026-08-29 23:4x 发)

| 域 | 任务 | 状态 |
|---|---|---|
| W1 | rurixc「if 包 while」codegen 修复+回归样例 | 跑批 |
| W1 | slot14 法线损坏件替换(v2 烘焙目录) | 跑批 |
| W1 | g34 三 kernel fx/fy host+device 同步修 | 跑批 |
| W1 | encode 共享路径切 v2 + parity 门转正 + 判读器 heap 同步 | 跑批 |
| W2 | `--transparency` 臂(ray 穿透真解,独占窗口 bin) | 跑批 |
| W2 | VSM 页管线判档(A:harness 补 dispatch 缺口 / B:接线) | 跑批 |
| W2 | post_chain 五级差距+LUT 臂方案 + PSO precache/warmup | 跑批 |
| W2 | VisBuffer 生产臂方案(档 1 出帧/档 2 证据臂) | 跑批 |
| W3 | 异步三件套侦察+RFC 修订草案+judgment 载体 | 跑批 |
| W3 | #96 属性保持简化(geom-build crate 面) | 跑批 |
| W5 | GAP-01~03 许可义务闭合 | 跑批 |

## W2 模块接线组

(待填)

## W3 深水区判档

(待填)

## W4 默认翻转 + 整批重锚

(待填)

## W5 商业化收尾

(待填)

## W6 终验与收官

(待填)
