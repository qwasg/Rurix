// Assisted-by: Claude（G37 W3 frame-cut）
// G37 W3：逐帧 device cut → AS 更新 判档臂（TODO #77 生产接线 × #89 FIF 合流窗
// 的最小判档面;G36 五留窗「出帧几何冻结于装配期选层」的解冻证据臂）。
//
// ## 机制（选定候选 = **BLAS 顶点 refit 竞技场**,侦察裁决见
// artifacts/day_0830_delivery/w3_deep/frame_cut_as/REPORT.md §3）
//
// - 竞技场 = 全簇固定槽位三角形拓扑（canonical 块序×簇序,槽长 =
//   `ClusterRecord::triangle_count`;passthrough 源三角恒活尾段）。单 BLAS
//   单实例,创建期 `updatable_blas=[0]`（ALLOW_UPDATE,B5 蒙皮通路同面）;
//   创建期内容 = **全簇真几何超集**（根 AABB 覆盖一切后续 refit 内容——TLAS
//   实例 AABB 不逐帧更新〔B5 蒙皮同律,tlas_update 恒 None〕下假漏命中
//   结构性排除,只留遍历保守性 perf 面）,帧 0 单条全量上传收到 cut0。
// - 逐帧：host 金标准 cut（`select_lod_cut_grouped` 组共享判定球 +
//   `verify_cut_coverage` fail-closed,#58/W2 visbuffer 同源直调）→ 槽位增量
//   （进 cut 写真几何/出 cut 写零面积折叠 = 三顶点同点,active 恒 active——
//   Vulkan UPDATE build 合法域,拓扑/图元数恒定）→
//   `FrameUpdate::blas_refit`（pass0 后桥接 copy + UPDATE build + consume
//   barrier,后续 ray query pass 读新 BLAS——render_exec B5 冻结通路直调）→
//   ray query 渲染（命中流 = 出帧几何证据）→ sha256 digest。
// - 判据（fail-closed）：① 双跑逐帧 digest 位级（重建会话重放全轨迹）;
//   ② cut_tris 随帧单调变化（方向自检,允许平台;恒常量即 FAIL）;
//   ③ 命中槽位 ∈ 当帧已施加 cut ∪ passthrough（陈旧几何零容忍——
//   「出帧几何随相机更新」的字面机核）;④ 哨兵清写 canary（RQ 覆盖缺陷检出）;
//   ⑤ 逐帧命中数 > 0（空接线防伪）。measured 如实登记（不设通过线）：
//   cut/增量/exec 分项 ms、逐 pass GPU ns、refit 增量字节、fence 等待。
// - 确定性协议：固定轨迹（k×step 前向 dolly）+ 固定重建节拍
//   （--cut-every N,默认 1 = 逐帧）+ canonical 竞技场布局 ⇒ 帧 k 的
//   （相机,cut,竞技场内容,AS 状态）均为帧号纯函数 ⇒ digest 序列同设备可
//   复现（跨设备不作 golden——RT 遍历并列命中 tie-break 依设备,visbuffer
//   digest 同口径登记）。
// - 边界（诚实登记）：缺省 cut = host 金标准;G40 T2(#77 P2)起
//   `--cut-source device` = 决策码为源生产 dispatch(表驻留 cull 会话 +
//   决策码回读,host 影子核 verify/提升/施加链照旧 host——等价谱系 =
//   G39 B5 P1 判定码逐项全等门;P3 直写竞技场不预支);单槽 inflight
//   （FIF 流水面拒 tlas_update/blas_refit——A2/B5 既有约束,FIF×每槽 AS
//   归 #90 RFC）; presented 面 0-byte（窗口合入 = 循环后证据臂,出帧翻转
//   归 #77 全量）。

/// --cluster-per-frame-cut 臂选项（off = 既有面 0-byte）。
#[allow(dead_code)]
struct FrameCutArmOpt {
    enabled: bool,
    /// 光线画布（证据分辨率;LOD 判据分辨率 = 内部分辨率另供,visbuffer 同口径）。
    res_w: u32,
    res_h: u32,
    /// 轨迹帧数。
    frames: u32,
    /// 前向 dolly 步长（米/帧;XZ 平面归一前向）。
    step_m: f32,
    /// 重建节拍：每 N 帧施加一次 cut→refit（1 = 逐帧;>1 = 惰性节拍臂 =
    /// 候选 B 的同一实现降档,hitch 由 refit/非 refit 帧 exec_ms 对照登记）。
    cut_every: u32,
    /// 簇块子集上限（0 = 全部;显存/建面时间逃生阀,子集面如实登记）。
    blocks_limit: usize,
    /// cut_tris 单调门（probe 固定单向 dolly = true 严门;窗口真轨迹可折返
    /// = false 降为非常量门 + 方向 measured 登记——不误红不冒充）。
    monotone_gate: bool,
    /// sidecar JSON 路径（空 = 只打印）。
    out_path: String,
}

#[allow(dead_code)]
impl FrameCutArmOpt {
    fn off() -> Self {
        Self {
            enabled: false,
            res_w: 96,
            res_h: 54,
            frames: 16,
            step_m: 0.15,
            cut_every: 1,
            blocks_limit: 0,
            monotone_gate: false,
            out_path: String::new(),
        }
    }
}

/// G38 T3 扩展选项(**新类型**承载——`FrameCutArmOpt` 字段集冻结,窗口 bin
/// 以 struct 字面量构造;窗口臂经既有 `run_frame_cut_arm` 消费本默认值)。
/// (G39 T5:`cull_spv` 为 String ⇒ derive 降 `Clone`——两处消费均引用传参,
/// 复核零拷贝语义依赖。)
#[derive(Clone)]
#[allow(dead_code)]
struct FrameCutArmExtOpt {
    /// 桥接 copy 模式:false = incr(默认;差集脏槽多 region copy,帧 0 全量
    /// 单 region)/ true = full(既有恒全量单 region,对照臂)。两态 vbuf 终态
    /// 位级同 ⇒ digest 序列位级等价(GPU 批次判据)。
    copy_full: bool,
    /// 簇粒度降档:竞技场只装 level≥N 的簇(+ 链兜底根),cut 经「level<N →
    /// 首个 level≥N 祖先」提升映射(生产 `verify_cut_coverage` 提升后复核
    /// fail-closed)。0 = 现状(既有面 0-byte)。
    min_level: u32,
    // ── G39 T5(#77 P1)→ G40 T2(#77 P2)语义升级:device 决策码为源 ──
    /// false = 既有 host 路径字面 0-byte(缺省);true = **P2 生产 dispatch**:
    /// 表驻留 cull 会话(常驻,每帧仅 params 256B 上传)→ 决策码回读(n×4B)
    /// → host 由 d==4 构造 cut 集 → `verify_cut_coverage` host 影子核直跑
    /// 回读集(fail-closed 逐字保持)→ min-level 提升照旧 host → 既有差集/
    /// 上传/refit 施加链 0 改。开窗条件 = G39 B5 P1 等价门 C1-C5 全绿(在案);
    /// P1 逐帧期望码对拍随决策权移交退役(`frame_cut_device_cut_compare`
    /// 保留为谱系参考)。P3(直写竞技场)不预支。
    cut_source_device: bool,
    /// rurixc 现编 `g31_cluster_cull.spv` 路径(`cut_source_device` 时必填;
    /// bin 侧装载后 NoContraction 注入,不落盘——SPV 文件保持 rurixc 原产字节)。
    cull_spv: String,
    /// red-arm:device 消费的 lod 表构造性篡改 ⇒ 决策码翻转 ⇒ host 影子核
    /// 覆盖性必破必红(P2 形态 = 施加链真实消费 device 决策的构造性证明;
    /// 受害裁决仍凭帧 0 host 参考码,诊断臂)。
    red_arm_tamper: bool,
}

#[allow(dead_code)]
impl FrameCutArmExtOpt {
    /// 既有入口默认:incr copy(窗口臂自动受益,digest 与 full 位级等价)+
    /// 无降档 + host cut 源(G39 T5 对拍臂关断)⇒ 窗口臂/既有 probe 0 行为变。
    fn default_ext() -> Self {
        Self {
            copy_full: false,
            min_level: 0,
            cut_source_device: false,
            cull_spv: String::new(),
            red_arm_tamper: false,
        }
    }
}

/// 竞技场布局（canonical:块序×簇序全簇槽 + passthrough 尾段;帧无关纯函数）。
#[allow(dead_code)]
struct FrameCutArena {
    /// [块][簇] → 槽三角基（竞技场全局三角号）。
    slot_base: Vec<Vec<u32>>,
    /// 槽主索引（升序三角基,二分定位命中图元 → (块,簇);passthrough 不入表）。
    owner_base: Vec<u32>,
    owner_cluster: Vec<(u32, u32)>,
    /// passthrough 尾段三角基。
    passthrough_base: u32,
    /// 竞技场总三角（全簇槽 + passthrough）。
    total_tris: usize,
}

/// 相机样本（窗口臂消费同形;独立 bin 自给 dolly）。
#[derive(Clone, Copy)]
#[allow(dead_code)]
struct FrameCutCamSample {
    frame: u32,
    spec: CameraSpec,
    in_w: u32,
    in_h: u32,
}

/// 逐帧统计（measured 如实登记;判据面见文件头）。
#[allow(dead_code)]
struct FrameCutFrameStat {
    frame: u32,
    cut_clusters: u32,
    cut_tris: u64,
    /// 本帧是否施加 cut→refit（--cut-every 节拍）。
    refit: bool,
    /// 槽位增量（进/出 cut 簇数;refit 帧才非零）。
    changed_slots: u32,
    /// 增量上传字节（host→arena SSBO;桥接 copy 恒全量另计）。
    upload_bytes: u64,
    /// 命中光线数。
    hits: u64,
    cut_ms: f64,
    delta_ms: f64,
    exec_ms: f64,
    gpu_clear_ms: f64,
    gpu_rq_ms: f64,
    fence_ms: f64,
    // ── G38 T3 加性字段(既有字段口径不动)──
    /// 提升后(竞技场施加口径)cut 三角数;min_level=0 时 == cut_tris。
    /// 单调门仍用 `cut_tris`(提升前 LOD 判据面——提升是表示层映射)。
    cut_tris_promoted: u64,
    /// 桥接 copy 区段数(incr = 相邻合并后段数;full/帧 0 全量 = 1;非 refit
    /// 帧 = 0;incr 零脏槽帧 = 0〔跳 copy〕)。
    copy_regions: u32,
    /// 桥接 copy 字节(incr = 脏区段和;full/帧 0 = 竞技场全量)。
    copy_bytes: u64,
    /// 桥接 copy 段 GPU 毫秒(query 追加区;fail-soft None)。
    bridge_copy_gpu_ms: Option<f64>,
    /// 桥接 UPDATE build 段 GPU 毫秒(含 consume barrier;fail-soft None)。
    bridge_build_gpu_ms: Option<f64>,
    // ── G40 T2(#77 P2)加性字段:cut_ms 分项(host/device 双臂恒出;
    //    DESIGN §4-2「select/verify/提升三段无分项计时 evidence,P1 应加分项
    //    登记供 P2 精算」兑现)──
    /// 决策段:host 臂 = `select_lod_cut_grouped` 逐块累计;device 臂 =
    /// params 上传 + dispatch + 决策码回读 + 布尔集构造全程墙钟。
    select_ms: f64,
    /// `verify_cut_coverage` 累计(提升前 + 提升后两处;host 影子核口径)。
    verify_ms: f64,
    /// min-level 提升映射段(ml=0 恒 0——提升整段跳过)。
    promote_ms: f64,
    // ── G39 T5 P1 → G40 T2 P2 字段迁移:P1 的 device_cut_probe_ms
    //    (run_compute 全程,mean 82.7ms 上界参考)随 run_compute 逐帧路
    //    退役;P2 以 select_ms(device 臂)+ 下方 dispatch GPU 分项承载,
    //    命名区分不混口径。──
    /// device 臂 cull dispatch 纯 GPU 毫秒(cull 会话 telemetry pass 0;
    /// 证据税→生产税转正后的分项 measured;None = cut_source host)。
    device_cut_dispatch_gpu_ms: Option<f64>,
    /// device 判定码字节 sha256(跨跑/跨窗审计面;None = cut_source host)。
    device_cut_decisions_sha256: Option<String>,
    digest: String,
}

/// G40 T2:cut 决策段分项计时槽(select/verify/promote;`frame_cut_select_ext`
/// 与 device 决策消费链共用;字段语义见 [`FrameCutFrameStat`] 同名字段)。
#[derive(Default, Clone, Copy)]
#[allow(dead_code)]
struct FrameCutSelectTiming {
    select_ms: f64,
    verify_ms: f64,
    promote_ms: f64,
}

// ---------------------------------------------------------------------------
// 竞技场布局/写槽（host 纯函数;canonical 序 = 确定性协议的一半）
// ---------------------------------------------------------------------------

/// 无槽哨兵(min-level 降档下 level<N 且非链根的簇不占槽;写入面遇哨兵 =
/// 逻辑破坏,fail-closed 断言)。
#[allow(dead_code)]
const FC_NO_SLOT: u32 = u32::MAX;

/// 布局：逐块逐簇（记录序）分配固定槽,尾接 passthrough。
#[allow(dead_code)]
fn frame_cut_arena_layout(blocks: &[ClusterPackBlock], passthrough_len: usize) -> FrameCutArena {
    frame_cut_arena_layout_ext(blocks, passthrough_len, 0, &[])
}

/// G38 T3:min-level 降档布局(min_level=0 = 既有全簇布局逐字等价)。
/// 占槽判据 = `level ≥ min_level` ∨ 链根(`min_parents[ci].is_none()`——
/// 提升映射的根兜底输出必须有槽);其余簇 slot_base = [`FC_NO_SLOT`] 哨兵,
/// owner 表不登记(命中图元二分域 = 实际占槽簇)。`min_parents_all` 在
/// min_level>0 时须与 blocks 等长([`frame_cut_min_parents`] 逐块产物)。
#[allow(dead_code)]
fn frame_cut_arena_layout_ext(
    blocks: &[ClusterPackBlock],
    passthrough_len: usize,
    min_level: u32,
    min_parents_all: &[Vec<Option<u32>>],
) -> FrameCutArena {
    assert!(
        min_level == 0 || min_parents_all.len() == blocks.len(),
        "min-level 布局需逐块 min_parents(调用面破坏)"
    );
    let mut slot_base: Vec<Vec<u32>> = Vec::with_capacity(blocks.len());
    let mut owner_base: Vec<u32> = Vec::new();
    let mut owner_cluster: Vec<(u32, u32)> = Vec::new();
    let mut next: u64 = 0;
    for (bi, b) in blocks.iter().enumerate() {
        let mut bases = Vec::with_capacity(b.records.len());
        for (ci, r) in b.records.iter().enumerate() {
            let eligible = min_level == 0
                || b.nodes[ci].level >= min_level
                || min_parents_all[bi][ci].is_none();
            if !eligible {
                bases.push(FC_NO_SLOT);
                continue;
            }
            bases.push(next as u32);
            owner_base.push(next as u32);
            owner_cluster.push((bi as u32, ci as u32));
            next += u64::from(r.triangle_count);
        }
        slot_base.push(bases);
    }
    let passthrough_base = next as u32;
    next += passthrough_len as u64;
    assert!(next <= u32::MAX as u64, "竞技场三角数超 u32 域");
    FrameCutArena {
        slot_base,
        owner_base,
        owner_cluster,
        passthrough_base,
        total_tris: next as usize,
    }
}

/// 逐块父映射(局部号;同组多父取**最小 id**——生产 `apply_page_fallback` 的
/// `min_parents` 同律,确定性;根 = `None`)。帧无关纯函数,调用方逐块预计算
/// 一次(提升映射与降档布局共用事实源)。
#[allow(dead_code)]
fn frame_cut_min_parents(b: &ClusterPackBlock) -> Vec<Option<u32>> {
    let mut parent: Vec<Option<u32>> = vec![None; b.records.len()];
    for (p, n) in b.nodes.iter().enumerate() {
        let end = n.first_child as usize + n.child_count as usize;
        for &c in &b.children[n.first_child as usize..end] {
            let slot = &mut parent[c as usize];
            *slot = Some(match *slot {
                None => p as u32,
                Some(q) => q.min(p as u32),
            });
        }
    }
    parent
}

/// G38 T3:脏区段追加(升序追加流;与上一段字节相邻则合并——差集循环槽序
/// 升序天然满足前置)。host 纯函数,selftest 直测。
#[allow(dead_code)]
fn frame_cut_merge_region(regions: &mut Vec<(u64, u64)>, off: u64, len: u64) {
    match regions.last_mut() {
        Some(last) if last.0 + last.1 == off => last.1 += len,
        _ => regions.push((off, len)),
    }
}

/// G38 T3:cut 的 min-level 提升映射(生产语义先例 = `apply_page_fallback`
/// 的「祖先替换 + 支配成员撤出」,resident 判定换成 `level ≥ min_level`):
/// - cut 内 level<N 成员沿 `min_parents` 上行至**首个** level≥N 祖先
///   (链上全 <N 时以链根兜底——根如实保留,可能 level<N);
/// - 替换祖先 children 可达域内的全部 cut 成员同步撤出(叶域 ⊆ 祖先域,
///   保「无重叠」;含 replacement 间的后代消除);
/// - 输出升序去重。**覆盖性由调用方以生产 `verify_cut_coverage`(原 DAG 视图)
///   提升后复核 fail-closed**——本函数不自证。
/// 确定性:min_parents 最小 id 父 + 标记法撤出(与遍历序无关)+ 尾部排序。
#[allow(dead_code)]
fn frame_cut_promote_min_level(
    b: &ClusterPackBlock,
    min_parents: &[Option<u32>],
    cut: &[u32],
    min_level: u32,
) -> Vec<u32> {
    if min_level == 0 {
        return cut.to_vec();
    }
    let children_of = |id: u32| -> &[u32] {
        let n = &b.nodes[id as usize];
        &b.children[n.first_child as usize..(n.first_child + n.child_count) as usize]
    };
    let mut keep: Vec<u32> = Vec::new();
    let mut reps: Vec<u32> = Vec::new();
    let mut rep_mark = vec![false; b.records.len()];
    for &c in cut {
        if b.nodes[c as usize].level >= min_level {
            keep.push(c);
            continue;
        }
        // 上行至首个 level≥N 祖先;链尽(根)兜底。
        let mut cur = c;
        loop {
            match min_parents[cur as usize] {
                Some(p) => {
                    cur = p;
                    if b.nodes[cur as usize].level >= min_level {
                        break;
                    }
                }
                None => break,
            }
        }
        if !rep_mark[cur as usize] {
            rep_mark[cur as usize] = true;
            reps.push(cur);
        }
    }
    if reps.is_empty() {
        // 全部成员 level≥N:提升为恒等(keep 即原 cut,已升序)。
        return keep;
    }
    // 支配标记:每个 replacement 的 children 可达域(含 replacement 间后代
    // ——被标者撤出,保留更粗祖先;已标剪枝防组共享链接重复下行)。
    let mut dominated = vec![false; b.records.len()];
    for &r in &reps {
        let mut stack: Vec<u32> = children_of(r).to_vec();
        while let Some(d) = stack.pop() {
            if !dominated[d as usize] {
                dominated[d as usize] = true;
                stack.extend_from_slice(children_of(d));
            }
        }
    }
    let mut out: Vec<u32> = Vec::with_capacity(keep.len() + reps.len());
    out.extend(keep.into_iter().filter(|&c| !dominated[c as usize]));
    out.extend(reps.into_iter().filter(|&r| !dominated[r as usize]));
    out.sort_unstable();
    out.dedup();
    out
}

/// 命中图元 → 所属（块,簇）;passthrough 段返回 None。
#[allow(dead_code)]
fn frame_cut_owner(arena: &FrameCutArena, prim: u32) -> Option<(u32, u32)> {
    if prim >= arena.passthrough_base {
        return None;
    }
    let i = match arena.owner_base.binary_search(&prim) {
        Ok(i) => i,
        Err(0) => return Some(arena.owner_cluster[0]), // 不可达（base[0]=0）;防御
        Err(i) => i - 1,
    };
    Some(arena.owner_cluster[i])
}

/// 写一个簇槽真几何（9 f32/tri;`apply_cluster_lod` 粗簇解码同式——叶簇几何
/// 已由 `verify_cluster_pack` 钉死与源三角逐位一致,全簇统一走包几何段）。
#[allow(dead_code)]
fn frame_cut_write_cluster(dst: &mut [f32], b: &ClusterPackBlock, ci: usize) {
    let r = &b.records[ci];
    debug_assert_eq!(dst.len(), r.triangle_count as usize * 9);
    for t in 0..r.triangle_count as usize {
        let ti = r.triangle_offset as usize + 3 * t;
        for k in 0..3 {
            let li = b.triangle_indices[ti + k] as usize + r.vertex_offset as usize;
            let p = b.vertices[li];
            dst[t * 9 + k * 3] = p[0];
            dst[t * 9 + k * 3 + 1] = p[1];
            dst[t * 9 + k * 3 + 2] = p[2];
        }
    }
}

/// passthrough 源三角 → 9 f32/tri 流（canonical `pack.passthrough` 升序;
/// **须在 `apply_cluster_lod` 施加前**从源装配场景提取——cut 重建后源三角
/// 序不复存在。窗口合入的施加前锚点消费本函数,probe 直接消费）。
#[allow(dead_code)]
fn frame_cut_passthrough_stream(scene: &SceneData, passthrough: &[u32]) -> Vec<f32> {
    let mut v = Vec::with_capacity(passthrough.len() * 9);
    for &src in passthrough {
        let t = scene.indices[src as usize];
        for &vi in &t {
            let p = scene.positions[vi as usize];
            v.extend_from_slice(&p);
        }
    }
    v
}

/// 全量竞技场流：**全簇真几何** + passthrough 尾段——创建期 BLAS 以此初建
/// ⇒ 根 AABB = 全部可能内容的保守超集（后续任意 cut ⊆ 全簇 ∪ 零面积折叠,
/// 原地 UPDATE 永不越界创建期 AABB;TLAS 实例 AABB 不逐帧更新的 B5 蒙皮同律
/// 前提——假漏命中结构性排除,只留遍历保守性 perf 面）。
#[allow(dead_code)]
fn frame_cut_full_stream(
    blocks: &[ClusterPackBlock],
    passthrough_stream: &[f32],
    arena: &FrameCutArena,
) -> Vec<f32> {
    let mut v = vec![0.0f32; arena.total_tris * 9];
    for (bi, b) in blocks.iter().enumerate() {
        for (ci, r) in b.records.iter().enumerate() {
            // G38 T3:min-level 降档下无槽簇(哨兵)不入竞技场。
            if arena.slot_base[bi][ci] == FC_NO_SLOT {
                continue;
            }
            let base = arena.slot_base[bi][ci] as usize * 9;
            let len = r.triangle_count as usize * 9;
            frame_cut_write_cluster(&mut v[base..base + len], b, ci);
        }
    }
    let pt_base = arena.passthrough_base as usize * 9;
    v[pt_base..pt_base + passthrough_stream.len()].copy_from_slice(passthrough_stream);
    v
}

/// 对流施加 cut：非 cut 簇槽写零面积折叠（[0;9] 三顶点同点 = active 零命中,
/// UPDATE 合法域——active 恒 active/图元数恒定）;cut 簇槽与 passthrough 不触。
#[allow(dead_code)]
fn frame_cut_apply_cut(
    stream: &mut [f32],
    blocks: &[ClusterPackBlock],
    arena: &FrameCutArena,
    cut: &[Vec<bool>],
) {
    for (bi, b) in blocks.iter().enumerate() {
        for (ci, r) in b.records.iter().enumerate() {
            // G38 T3:无槽簇(哨兵)不占竞技场,施加面跳过(cut 提升后
            // 恒不含无槽簇——写入侧另有 fail-closed 断言)。
            if arena.slot_base[bi][ci] == FC_NO_SLOT {
                continue;
            }
            if !cut[bi][ci] {
                let base = arena.slot_base[bi][ci] as usize * 9;
                let len = r.triangle_count as usize * 9;
                stream[base..base + len].fill(0.0);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 逐帧 host cut（生产金标准直调;visbuffer 臂/cluster_lod_frame_stat 同源口径）
// ---------------------------------------------------------------------------

/// 单帧全块 cut → 逐块布尔集 + 计数;覆盖性逐帧机核 fail-closed。
#[allow(dead_code)]
fn frame_cut_select(
    tag: &str,
    blocks: &[ClusterPackBlock],
    spec: &CameraSpec,
    in_w: u32,
    in_h: u32,
    threshold_px: f32,
    frame: u32,
) -> (Vec<Vec<bool>>, u32, u64) {
    let (sets, cut_clusters, cut_tris, _, _) = frame_cut_select_ext(
        tag,
        blocks,
        spec,
        in_w,
        in_h,
        threshold_px,
        frame,
        0,
        &[],
        &mut FrameCutSelectTiming::default(),
    );
    (sets, cut_clusters, cut_tris)
}

/// G38 T3:select + min-level 提升(min_level=0 = 既有 `frame_cut_select`
/// 逐字等价)。生产金标准链不动:`select_lod_cut_grouped`(原 DAG 原样)→
/// `verify_cut_coverage`(原 cut)→ [`frame_cut_promote_min_level`] →
/// `verify_cut_coverage`(提升后,同一生产校验,fail-closed)。返回
/// (提升后逐块布尔集, 提升后簇数, **提升前** cut 三角数〔LOD 判据面,单调门
/// 消费〕, 提升后 cut 三角数〔竞技场施加口径〕, **提升前**逐块布尔集
/// 〔G39 T5 device 对拍口径 = select 原输出——kernel 关 3 的 host 面;提升
/// 映射是 min-level 表示层后处理,对拍锚其前;min_level=0 时与首元同值〕)。
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn frame_cut_select_ext(
    tag: &str,
    blocks: &[ClusterPackBlock],
    spec: &CameraSpec,
    in_w: u32,
    in_h: u32,
    threshold_px: f32,
    frame: u32,
    min_level: u32,
    min_parents_all: &[Vec<Option<u32>>],
    // G40 T2:分项计时槽(加性尾参;select/verify/promote 累计——既有语句
    // 字面 0 改,计时为环绕追加;调用点闭集 3 处机械补)。
    timing: &mut FrameCutSelectTiming,
) -> (Vec<Vec<bool>>, u32, u64, u64, Vec<Vec<bool>>) {
    use rurix_render::geometry::gpu_scene::IDENTITY_3X4;
    use rurix_render::geometry::visible_cluster_set::{
        MeshDagView, select_lod_cut_grouped, verify_cut_coverage,
    };
    let cam = cluster_cull_camera(spec, in_w, in_h, threshold_px);
    let mut sets = Vec::with_capacity(blocks.len());
    let mut pre_sets = Vec::with_capacity(blocks.len());
    let mut cut_clusters = 0u32;
    let mut cut_tris = 0u64;
    let mut cut_tris_promoted = 0u64;
    for (bi, b) in blocks.iter().enumerate() {
        let view = MeshDagView::new(&b.records, &b.nodes, &b.children)
            .unwrap_or_else(|e| fail(&format!("{tag}: 帧 {frame} 块 {bi} DAG 拓扑: {e}")));
        let t_sel = std::time::Instant::now();
        let cut = select_lod_cut_grouped(
            &view,
            &b.cluster_self_lod,
            &b.cluster_parent_lod,
            &IDENTITY_3X4,
            &cam,
        );
        timing.select_ms += t_sel.elapsed().as_secs_f64() * 1e3;
        let t_ver = std::time::Instant::now();
        verify_cut_coverage(&view, &cut)
            .unwrap_or_else(|e| fail(&format!("{tag}: 帧 {frame} 块 {bi} cut 覆盖性: {e}")));
        timing.verify_ms += t_ver.elapsed().as_secs_f64() * 1e3;
        for &c in &cut {
            cut_tris += u64::from(b.records[c as usize].triangle_count);
        }
        // G39 T5:提升前布尔集(device 对拍口径;ml=0 时提升恒等 ⇒ 与提升后
        // set 同值)。
        let mut pre_set = vec![false; b.records.len()];
        for &c in &cut {
            pre_set[c as usize] = true;
        }
        pre_sets.push(pre_set);
        // min-level 提升(0 = 恒等)+ 提升后生产校验复核(fail-closed)。
        let cut = if min_level == 0 {
            cut
        } else {
            let t_pro = std::time::Instant::now();
            let promoted =
                frame_cut_promote_min_level(b, &min_parents_all[bi], &cut, min_level);
            timing.promote_ms += t_pro.elapsed().as_secs_f64() * 1e3;
            let t_ver2 = std::time::Instant::now();
            verify_cut_coverage(&view, &promoted).unwrap_or_else(|e| {
                fail(&format!(
                    "{tag}: 帧 {frame} 块 {bi} min-level 提升后覆盖性: {e}(fail-closed)"
                ))
            });
            timing.verify_ms += t_ver2.elapsed().as_secs_f64() * 1e3;
            promoted
        };
        let mut set = vec![false; b.records.len()];
        for &c in &cut {
            set[c as usize] = true;
            cut_tris_promoted += u64::from(b.records[c as usize].triangle_count);
        }
        cut_clusters += cut.len() as u32;
        sets.push(set);
    }
    if min_level == 0 {
        cut_tris_promoted = cut_tris;
    }
    (sets, cut_clusters, cut_tris, cut_tris_promoted, pre_sets)
}

// ---------------------------------------------------------------------------
// G39 T5(#77 P1):probe-only device 决策码回读对拍臂——host 决策权/施加链/
// 既有判据 0 字节移交;device 以冻结 `g31_cluster_cull.rx` kernel(三关超集,
// rurixc 现编 SPV 经 --cull-spv 运行时装载)0-byte 消费平行复算判定码,回读
// 逐项对拍。中和方案 A(params/数据域退化,kernel 0-byte):关 1 六平面全零
// ⇒ `0 < −radius` 恒假恒不剔;关 2 逐簇 cone_cutoff=1.0 ⇒ `cutoff < 1.0`
// 恒假关断;关 4 view 行全零 ⇒ viewz=0,near_z=−radius<znear(0.1)恒短路。
// 判定码域收缩 {2,4},与 host 期望码(in_cut ? 4 : 2)恰成对拍闭集;域外码
// = 中和面破坏,fail-closed。设计 = artifacts/day_0831_g39/t5_devicecut/
// DESIGN.md(E3;等价门 §3;红臂 §3.2)。
// ---------------------------------------------------------------------------

/// device 簇表构造(canonical 全局序 = 块序×簇序,`frame_cut_arena_layout_ext`
/// 同一遍历序):cluster_f32 10/簇 `[center 3|radius|cone_axis 零填 3|
/// cone_cutoff=1.0 中和|error|parent_error*]`;lod_f32 8/簇 `[self 球 4|
/// parent 球 4]`(`frame_cut_select_ext` 消费的同源平行表直取)。
/// `parent_error*` 上传律 = harness **字面**(g31_cluster_cull_device.rs
/// L204-209;判据① v1.1.5 全绿锚定路径):有限 → **原样透传**(含在树根簇
/// 合法编码 `f32::MAX`——dag.rs「顶层(根)parent_error = f32::MAX」+
/// 双侧饱和等价,见 C1 首红裁决 REPORT §6);非有限(+∞/NaN)→ 2.0e9
/// sentinel(NaN 必须映射:kernel `parent_e > 0` 对 NaN 假 ⇒ 0px,host
/// `dist > error` 对 NaN 假 ⇒ +∞,直传会分叉)。无域检拒项(负有限 =
/// 双侧 0px 等价,叶 error≤0 对偶);等价残余由逐项全等门 fail-closed 兜底。
/// center/radius 照填真值(关 1/4 中和下不参与判定,表意保真;error 面 =
/// 关 3 真输入)。
#[allow(dead_code)]
fn frame_cut_device_tables(blocks: &[ClusterPackBlock]) -> (Vec<f32>, Vec<f32>) {
    let n: usize = blocks.iter().map(|b| b.records.len()).sum();
    let mut cluster_f32 = Vec::with_capacity(n * 10);
    let mut lod_f32 = Vec::with_capacity(n * 8);
    for b in blocks {
        for (ci, r) in b.records.iter().enumerate() {
            let parent_e = if r.parent_error.is_finite() {
                r.parent_error
            } else {
                2.0e9
            };
            cluster_f32.extend_from_slice(&[
                r.center[0],
                r.center[1],
                r.center[2],
                r.radius,
                0.0,
                0.0,
                0.0, // cone_axis 零填(关 2 关断下不参与)
                1.0, // cone_cutoff = 1.0(关 2 中和,kernel `if cutoff < 1.0` 字面)
                r.error,
                parent_e,
            ]);
            let sl = &b.cluster_self_lod[ci];
            let pl = &b.cluster_parent_lod[ci];
            lod_f32.extend_from_slice(&[
                sl.center[0],
                sl.center[1],
                sl.center[2],
                sl.radius,
                pl.center[0],
                pl.center[1],
                pl.center[2],
                pl.radius,
            ]);
        }
    }
    (cluster_f32, lod_f32)
}

/// device params 装配(kernel 头注布局字面,64 f32):关 1 域 [0..24) 全零/
/// 关 4 域 [36..64) 全零 + znear=0.1 正值 ⇒ near_z=−radius<znear 恒短路。
/// [27] proj_factor = `cluster_cull_camera` → `projection_factor()` 字面同式
/// (vp.m[1][1]·in_h·0.5;LOD 判据分辨率 = **内部分辨率** in_w/in_h,与
/// `frame_cut_select_ext` 消费同一 `cluster_cull_camera` 口径同源)。
/// cap = n ⇒ 列表零 overflow;mode = 0(pass1)。
#[allow(dead_code)]
fn frame_cut_device_params(
    spec: &CameraSpec,
    in_w: u32,
    in_h: u32,
    threshold_px: f32,
    n: usize,
) -> [f32; 64] {
    let cam = cluster_cull_camera(spec, in_w, in_h, threshold_px);
    let mut p = [0.0f32; 64];
    p[24..27].copy_from_slice(&spec.eye);
    p[27] = cam.view_proj[1][1] * cam.screen_height_px * 0.5;
    p[28] = threshold_px;
    p[29] = n as f32;
    // p[30] = 0(mode pass1);p[31..33) uv 半径系数 0(关 4 短路下不消费)。
    p[33] = 0.1; // znear 正值(关 4 短路判据消费;harness ZNEAR 同字面)
    p[34] = 1.0; // hzb_level_count = 1(兜底绑定,短路下不读)
    p[35] = n as f32; // visible_capacity = n ⇒ 原子列表零 overflow
    p
}

/// 期望码构造(对拍口径 = **提升前** select 原输出展平,canonical 全局序):
/// in_cut ⇒ 4(可见)/非 cut ⇒ 2(LOD 非 cut)——与中和后 device 判定码域
/// {2,4} 恰成闭集。
#[allow(dead_code)]
fn frame_cut_device_expected(pre_sets: &[Vec<bool>]) -> Vec<u32> {
    let mut out = Vec::with_capacity(pre_sets.iter().map(Vec::len).sum());
    for set in pre_sets {
        for &in_cut in set {
            out.push(if in_cut { 4 } else { 2 });
        }
    }
    out
}

/// G40 T2(#77 P2):device 判定码 → 逐块提升前布尔集(canonical 全局序
/// 逆展平;闭集断言 d∈{2,4} fail-closed——0=平面非零/1=cutoff 未关/3=关 4
/// 未短路,中和面破坏打印首破全局簇号)。host 纯函数,selftest ⑧ 直测。
#[allow(dead_code)]
fn frame_cut_sets_from_decisions(
    tag: &str,
    blocks: &[ClusterPackBlock],
    decisions: &[u32],
    frame: u32,
) -> Vec<Vec<bool>> {
    let n: usize = blocks.iter().map(|b| b.records.len()).sum();
    assert_eq!(decisions.len(), n, "决策码长度与簇包错位(调用面破坏)");
    let mut sets = Vec::with_capacity(blocks.len());
    let mut g = 0usize;
    for b in blocks {
        let mut set = vec![false; b.records.len()];
        for s in set.iter_mut() {
            match decisions[g] {
                4 => *s = true,
                2 => {}
                d => fail(&format!(
                    "{tag}: 帧 {frame} device 判定码 {d} ∉ {{2,4}} 全局簇 {g}(中和面破坏:0=平面非零/1=cutoff 未关/3=关 4 未短路;fail-closed)"
                )),
            }
            g += 1;
        }
        sets.push(set);
    }
    sets
}

/// G40 T2(#77 P2):device 决策码为源的 select 后链(与
/// [`frame_cut_select_ext`] 的 select 后处理**字面同形**,仅决策源换 device):
/// 逐块 `verify_cut_coverage`(host 影子核直跑回读集,fail-closed 语义逐字
/// 保持,DESIGN §2.7 P2 行字面)→ min-level 提升(照旧 host)→ 提升后再
/// verify → (提升后布尔集, 提升后簇数, 提升前 cut_tris〔LOD 判据面,单调门
/// 消费〕, 提升后 cut_tris)。red-arm 篡改 ⇒ 决策翻转 ⇒ 覆盖性必破 ⇒
/// 本链 fail-closed 红(施加链真实消费 device 决策的构造性证明——P1 期望码
/// 对拍形态的 P2 承接,报文形态变化如实登记)。
#[allow(dead_code)]
fn frame_cut_select_from_decisions(
    tag: &str,
    blocks: &[ClusterPackBlock],
    decisions: &[u32],
    frame: u32,
    min_level: u32,
    min_parents_all: &[Vec<Option<u32>>],
    timing: &mut FrameCutSelectTiming,
) -> (Vec<Vec<bool>>, u32, u64, u64) {
    use rurix_render::geometry::visible_cluster_set::{MeshDagView, verify_cut_coverage};
    let pre_sets = frame_cut_sets_from_decisions(tag, blocks, decisions, frame);
    let mut sets = Vec::with_capacity(blocks.len());
    let mut cut_clusters = 0u32;
    let mut cut_tris = 0u64;
    let mut cut_tris_promoted = 0u64;
    for (bi, b) in blocks.iter().enumerate() {
        let view = MeshDagView::new(&b.records, &b.nodes, &b.children)
            .unwrap_or_else(|e| fail(&format!("{tag}: 帧 {frame} 块 {bi} DAG 拓扑: {e}")));
        let cut: Vec<u32> = pre_sets[bi]
            .iter()
            .enumerate()
            .filter_map(|(ci, &c)| c.then_some(ci as u32))
            .collect();
        let t_ver = std::time::Instant::now();
        verify_cut_coverage(&view, &cut).unwrap_or_else(|e| {
            fail(&format!(
                "{tag}: 帧 {frame} 块 {bi} device cut 覆盖性: {e}(host 影子核 fail-closed;red-arm/等价破坏归因面)"
            ))
        });
        timing.verify_ms += t_ver.elapsed().as_secs_f64() * 1e3;
        for &c in &cut {
            cut_tris += u64::from(b.records[c as usize].triangle_count);
        }
        let cut = if min_level == 0 {
            cut
        } else {
            let t_pro = std::time::Instant::now();
            let promoted =
                frame_cut_promote_min_level(b, &min_parents_all[bi], &cut, min_level);
            timing.promote_ms += t_pro.elapsed().as_secs_f64() * 1e3;
            let t_ver2 = std::time::Instant::now();
            verify_cut_coverage(&view, &promoted).unwrap_or_else(|e| {
                fail(&format!(
                    "{tag}: 帧 {frame} 块 {bi} device cut min-level 提升后覆盖性: {e}(fail-closed)"
                ))
            });
            timing.verify_ms += t_ver2.elapsed().as_secs_f64() * 1e3;
            promoted
        };
        let mut set = vec![false; b.records.len()];
        for &c in &cut {
            set[c as usize] = true;
            cut_tris_promoted += u64::from(b.records[c as usize].triangle_count);
        }
        cut_clusters += cut.len() as u32;
        sets.push(set);
    }
    if min_level == 0 {
        cut_tris_promoted = cut_tris;
    }
    (sets, cut_clusters, cut_tris, cut_tris_promoted)
}

/// SPIR-V NoContraction 注入(`g31_cluster_cull_device.rs`
/// `spv_inject_no_contraction` L87-119 字面同式副本——继 cluster_cull_device/
/// cluster_stream 后第三副本,如实登记,单源折叠留窗 DESIGN §5-3;挡 FMA
/// 收缩 = f32 等价门先决):OpFAdd/OpFSub/OpFMul〔op 129/131/133〕result-id
/// 收集 → 首 annotation/type 段前逐 id 注 `OpDecorate NoContraction`。
#[allow(dead_code)]
fn fc_spv_inject_no_contraction(spv: &[u32]) -> Vec<u32> {
    let mut result_ids: Vec<u32> = Vec::new();
    let mut i = 5usize;
    let mut first_decorate: Option<usize> = None;
    let mut first_type: Option<usize> = None;
    while i < spv.len() {
        let w = spv[i];
        let wc = (w >> 16) as usize;
        let op = w & 0xFFFF;
        if wc == 0 || i + wc > spv.len() {
            fail("SPIR-V 指令流越界（NoContraction 注入）");
        }
        match op {
            71 if first_decorate.is_none() => first_decorate = Some(i),
            19..=39 if first_type.is_none() => first_type = Some(i),
            129 | 131 | 133 => result_ids.push(spv[i + 2]),
            _ => {}
        }
        i += wc;
    }
    let at = first_decorate
        .or(first_type)
        .unwrap_or_else(|| fail("SPIR-V 无 annotation/type 段锚"));
    let mut out = Vec::with_capacity(spv.len() + result_ids.len() * 3);
    out.extend_from_slice(&spv[..at]);
    for id in &result_ids {
        out.push((3 << 16) | 71); // OpDecorate, wc=3
        out.push(*id);
        out.push(42); // NoContraction
    }
    out.extend_from_slice(&spv[at..]);
    out
}

/// red-arm 篡改(C3 机核 = 构造性证明对拍面真实消费,harness 判据⑥同律):
/// 在 device 消费的 lod 表上使受害簇判定码**结构性必翻**(翻转与相机/资产
/// 数值无关;host 期望码不动)。受害裁决(fail-closed,返回全局簇号;
/// 域口径:parent_error* ≥ 1e9 = **饱和域**——含在树根 f32::MAX 原样透传
/// 与非有限映射 2e9,kernel/host 双侧 parent 谓词恒真、球不参与、不可翻,
/// C1 首红裁决后与表构造律同步复核):
/// - 模式甲:首个期望 4 且 parent_error* ∈ (0,1e9)(球参与域)簇——parent
///   球半径 → −f32::MAX ⇒ dsurf 饱和 ~3.4e38 ⇒ parent_px → ~0 < thr ⇒
///   必翻 2;
/// - 模式乙(甲空):首个期望 2 且 self_error < 1e9 且 parent_error* > 0
///   簇——self 球半径 → −f32::MAX(仅 self_error>0 时;≤0 恒 0px 免篡)
///   + parent 球半径 → +f32::MAX(仅 parent_error*<1e9 时;饱和域恒
///   ≥thr 免篡)⇒ self_px<thr ∧ parent_px≥thr ⇒ 必翻 4。
/// 【偏离 DESIGN E3-4 登记】原案「全局簇 0 self 球半径 +1.0」在生产包为
/// 结构性空转:簇 0 = 叶(dag.rs 叶层在前、叶 error=0),而 select/kernel
/// 对 error≤0 恒 0px **不读球**(visible_cluster_set.rs L326-327 字面)——
/// 必红不可达;本实现保持原案形状(lod 球篡改/上传前/fail-closed),
/// 受害裁决改为期望码驱动。详 REPORT.md 偏离表。
#[allow(dead_code)]
fn frame_cut_red_arm_tamper(
    cluster_f32: &[f32],
    lod_f32: &mut [f32],
    expected0: &[u32],
    tag: &str,
) -> usize {
    for (g, &e) in expected0.iter().enumerate() {
        let pe = cluster_f32[g * 10 + 9];
        if e == 4 && pe > 0.0 && pe < 1.0e9 {
            lod_f32[g * 8 + 7] = -f32::MAX;
            eprintln!(
                "{tag}: red-arm 模式甲 受害全局簇 {g}(期望 4;parent 球半径→−MAX ⇒ device 必翻 2)"
            );
            return g;
        }
    }
    for (g, &e) in expected0.iter().enumerate() {
        let se = cluster_f32[g * 10 + 8];
        let pe = cluster_f32[g * 10 + 9];
        if e == 2 && se < 1.0e9 && pe > 0.0 {
            if se > 0.0 {
                lod_f32[g * 8 + 3] = -f32::MAX;
            }
            if pe < 1.0e9 {
                lod_f32[g * 8 + 7] = f32::MAX;
            }
            eprintln!(
                "{tag}: red-arm 模式乙 受害全局簇 {g}(期望 2;self→−MAX/parent→+MAX ⇒ device 必翻 4)"
            );
            return g;
        }
    }
    fail(&format!(
        "{tag}: red-arm 无可篡改受害簇(帧 0 期望码全不可翻——退化簇包;fail-closed)"
    ))
}

/// device 对拍会话级上下文(表/SPV 帧无关,一次性构造;red-arm 篡改已施加
/// 于 lod 表)。
#[allow(dead_code)]
struct FrameCutDeviceCtx {
    spv: Vec<u32>,
    cluster_f32: Vec<f32>,
    lod_f32: Vec<f32>,
    n: usize,
}

/// 会话建立段一次性构造:SPV 读取 + NoContraction 注入(不落盘,spirv-val
/// 由验收环在 rurixc 原产工件上覆盖)+ 表构造 + red-arm 篡改。
/// (G40 T2 P2:`expected0` 降 Option——P2 决策源 = device,host 期望码退出
/// 逐帧对拍;仅 red-arm 受害裁决仍需帧 0 host 参考码〔诊断臂,不动生产
/// 决策源〕,red_arm_tamper 而 None = 调用面破坏 fail-closed。)
#[allow(dead_code)]
fn frame_cut_device_ctx(
    tag: &str,
    ext: &FrameCutArmExtOpt,
    blocks: &[ClusterPackBlock],
    expected0: Option<&[u32]>,
    verbose: bool,
) -> FrameCutDeviceCtx {
    let bytes = std::fs::read(&ext.cull_spv)
        .unwrap_or_else(|e| fail(&format!("{tag}: cull SPV 读取 {}: {e}", ext.cull_spv)));
    if bytes.len() % 4 != 0 {
        fail(&format!("{tag}: cull SPV 字节数非 4 对齐 {}", ext.cull_spv));
    }
    let words: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    let spv = fc_spv_inject_no_contraction(&words);
    let (cluster_f32, mut lod_f32) = frame_cut_device_tables(blocks);
    let n: usize = blocks.iter().map(|b| b.records.len()).sum();
    assert_eq!(
        cluster_f32.len(),
        n * 10,
        "device 表长度与簇包错位(块切片一致性破坏)"
    );
    if let Some(e0) = expected0 {
        assert_eq!(e0.len(), n, "期望码长度与簇包错位(调用面破坏)");
    }
    if ext.red_arm_tamper {
        let e0 = expected0.unwrap_or_else(|| {
            fail(&format!(
                "{tag}: red-arm 需帧 0 host 参考码定受害簇(调用面破坏,fail-closed)"
            ))
        });
        frame_cut_red_arm_tamper(&cluster_f32, &mut lod_f32, e0, tag);
    }
    if verbose {
        eprintln!(
            "{tag}: device cut 表就绪 n={n} 表字节={}(cluster {} + lod {});P2 形态 = 决策码为源(表驻留 cull 会话,每帧仅 params 256B 上传;host 影子核 verify + 提升/施加链照旧 host)",
            n * 72,
            n * 40,
            n * 32,
        );
    }
    FrameCutDeviceCtx {
        spv,
        cluster_f32,
        lod_f32,
        n,
    }
}

/// 【G40 T2 P2 退役登记】P1 对拍臂本体(`vk::run_compute` 逐帧独立 device,
/// 82.7ms/帧上界参考)——P2 决策码为源后不再逐帧消费,保留为 P1 谱系参考
/// (G39 B5 等价门 evidence 锚在本函数口径;10 buffer 布局被 P2 cull 会话
/// 逐字继承)。
/// device 判定码对拍(等价门本体,fail-closed 闭集):10 buffer 布局 =
/// cluster_cull harness 字面(params/cluster_f32/lod_f32/input_ids 恒等
/// 0..n/hzb_data [0.0] 兜底/hzb_meta [0,1,1]/counters 12B 零/decisions n×4/
/// vis_ids/occ_ids)→ dispatch [n,1,1] → 回读 decisions:①闭集断言
/// d∈{2,4}(0=平面非零/1=cutoff 未关/3=关 4 未短路——中和面破坏,打印首破
/// 簇)②逐项全等 `d[g]==expected[g]`(mismatch 打印归因素材:全局簇号/
/// 两侧码/error/parent_error*/self/parent lod 球——§3.2 亚 ULP 边界归因
/// 输入;P2 NO-GO 判据素材)。返回 (dispatch 墙钟 ms〔run_compute 全程,
/// 证据税单列 measured〕, 判定码字节 sha256〔跨跑/跨窗审计面〕)。
#[allow(dead_code)]
fn frame_cut_device_cut_compare(
    ctx: &FrameCutDeviceCtx,
    params: &[f32; 64],
    expected: &[u32],
    tag: &str,
    frame: u32,
) -> (f64, String) {
    let n = ctx.n;
    assert_eq!(expected.len(), n, "期望码长度漂移(调用面破坏)");
    let entry = vk::entry_point_name(&ctx.spv)
        .unwrap_or_else(|| fail(&format!("{tag}: cull SPV 无 OpEntryPoint")));
    let input_ids: Vec<u8> = (0..n as u32).flat_map(|i| i.to_le_bytes()).collect();
    let mut bufs: Vec<Vec<u8>> = vec![
        bytes_f32(&params[..]),
        bytes_f32(&ctx.cluster_f32),
        bytes_f32(&ctx.lod_f32),
        input_ids,
        bytes_f32(&[0.0]),                          // hzb_data 1 texel 兜底(短路不读)
        [0u32, 1, 1].iter().flat_map(|x| x.to_le_bytes()).collect(), // hzb_meta
        vec![0u8; 12],                              // counters
        vec![0u8; n.max(1) * 4],                    // decisions
        vec![0u8; n.max(1) * 4],                    // vis_ids
        vec![0u8; n.max(1) * 4],                    // occ_ids
    ];
    let t = std::time::Instant::now();
    vk::run_compute(&ctx.spv, &entry, &mut bufs, &[], [n.max(1) as u32, 1, 1])
        .unwrap_or_else(|e| fail(&format!("{tag}: 帧 {frame} device cut dispatch: {e}")));
    let ms = t.elapsed().as_secs_f64() * 1e3;
    let dec_bytes = &bufs[7][..n * 4];
    let sha = sha256_hex(dec_bytes);
    for g in 0..n {
        let o = g * 4;
        let d = u32::from_le_bytes([
            dec_bytes[o],
            dec_bytes[o + 1],
            dec_bytes[o + 2],
            dec_bytes[o + 3],
        ]);
        if d != 2 && d != 4 {
            fail(&format!(
                "{tag}: 帧 {frame} device 判定码 {d} ∉ {{2,4}} 全局簇 {g}(中和面破坏:0=平面非零/1=cutoff 未关/3=关 4 未短路;fail-closed)"
            ));
        }
        if d != expected[g] {
            let fb = g * 10;
            let lb = g * 8;
            fail(&format!(
                "{tag}: 帧 {frame} 判定码 mismatch 全局簇 {g}: device={d} host={}(error={:e} parent_error*={:e} self_lod=[{},{},{},r={}] parent_lod=[{},{},{},r={}];fail-closed;P1 红 ⇒ P2 NO-GO 归因素材,DESIGN §3.2)",
                expected[g],
                ctx.cluster_f32[fb + 8],
                ctx.cluster_f32[fb + 9],
                ctx.lod_f32[lb],
                ctx.lod_f32[lb + 1],
                ctx.lod_f32[lb + 2],
                ctx.lod_f32[lb + 3],
                ctx.lod_f32[lb + 4],
                ctx.lod_f32[lb + 5],
                ctx.lod_f32[lb + 6],
                ctx.lod_f32[lb + 7],
            ));
        }
    }
    (ms, sha)
}

// ---------------------------------------------------------------------------
// host 光线生成（确定性 f32 针孔;raygen 上传 = 每帧 32B/线小面,免第二 kernel
// 相机数学——device 数学面收敛在 ray query 遍历本身）
// ---------------------------------------------------------------------------

/// 8 f32/光线（ox,oy,oz,dx,dy,dz,tmin,tmax;m94 harness 同布局）。
#[allow(dead_code)]
fn frame_cut_rays(spec: &CameraSpec, w: u32, h: u32) -> Vec<f32> {
    let norm = |v: [f32; 3]| -> [f32; 3] {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        [v[0] / l, v[1] / l, v[2] / l]
    };
    let cross = |a: [f32; 3], b: [f32; 3]| -> [f32; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let f = norm(spec.forward);
    let r = norm(cross(f, spec.up0));
    let u = cross(r, f);
    let th = (spec.fov_y_rad * 0.5).tan();
    let aspect = w as f32 / h as f32;
    let mut out = Vec::with_capacity((w * h) as usize * 8);
    for y in 0..h {
        for x in 0..w {
            let sx = (2.0 * (x as f32 + 0.5) / w as f32 - 1.0) * th * aspect;
            let sy = (1.0 - 2.0 * (y as f32 + 0.5) / h as f32) * th;
            let d = norm([
                f[0] + r[0] * sx + u[0] * sy,
                f[1] + r[1] * sx + u[1] * sy,
                f[2] + r[2] * sx + u[2] * sy,
            ]);
            out.extend_from_slice(&[
                spec.eye[0],
                spec.eye[1],
                spec.eye[2],
                d[0],
                d[1],
                d[2],
                1.0e-3,
                1.0e9,
            ]);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 手编 SPIR-V（bin-local;vk_clas_rt `m94_ray_query_spv` 先例形制——冻结
// kernels/*.rx 与 .tmp SPV 全 0-byte,无新 rurixc 编译面）
// ---------------------------------------------------------------------------

/// SPIR-V 汇编小助手（m94 同式）。
#[allow(dead_code)]
fn fc_spv_inst(v: &mut Vec<u32>, op: u32, ops: &[u32]) {
    v.push(op | ((ops.len() as u32 + 1) << 16));
    v.extend_from_slice(ops);
}

#[allow(dead_code)]
fn fc_spv_words(s: &str) -> Vec<u32> {
    let mut b = s.as_bytes().to_vec();
    b.push(0);
    while b.len() % 4 != 0 {
        b.push(0);
    }
    b.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

#[allow(dead_code)]
fn fc_spv_bytes(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_le_bytes()).collect()
}

/// 哨兵清写 kernel（set0 b0 = out u32 SSBO;每 invocation 清一条 4 u32 记录为
/// 0xFFFF_FFFF——RQ pass 随后必须整写覆盖,残留哨兵 = dispatch 覆盖缺陷 canary）。
#[allow(dead_code)]
fn frame_cut_clear_spv() -> Vec<u32> {
    let mut v = vec![0x0723_0203u32, 0x0001_0400, 0, 64, 0];
    fc_spv_inst(&mut v, 17, &[1]); // OpCapability Shader
    fc_spv_inst(&mut v, 14, &[0, 1]); // OpMemoryModel Logical GLSL450
    let mut ep = vec![5u32, 1];
    ep.extend(fc_spv_words("main"));
    ep.extend_from_slice(&[10, 21]);
    fc_spv_inst(&mut v, 15, &ep); // OpEntryPoint GLCompute %1 "main" %gid %out
    fc_spv_inst(&mut v, 16, &[1, 17, 1, 1, 1]); // LocalSize 1 1 1
    fc_spv_inst(&mut v, 71, &[10, 11, 28]); // %10 BuiltIn GlobalInvocationId
    fc_spv_inst(&mut v, 71, &[21, 34, 0]); // %21 DescriptorSet 0
    fc_spv_inst(&mut v, 71, &[21, 33, 0]); // %21 Binding 0
    fc_spv_inst(&mut v, 71, &[19, 2]); // %19 Block
    fc_spv_inst(&mut v, 72, &[19, 0, 35, 0]); // member0 Offset 0
    fc_spv_inst(&mut v, 71, &[18, 6, 4]); // %18 ArrayStride 4
    fc_spv_inst(&mut v, 19, &[2]); // %2 void
    fc_spv_inst(&mut v, 33, &[3, 2]); // %3 fn
    fc_spv_inst(&mut v, 21, &[4, 32, 0]); // %4 u32
    fc_spv_inst(&mut v, 23, &[8, 4, 3]); // %8 uvec3
    fc_spv_inst(&mut v, 32, &[9, 1, 8]); // %9 ptr Input uvec3
    fc_spv_inst(&mut v, 59, &[9, 10, 1]); // %10 gid
    fc_spv_inst(&mut v, 29, &[18, 4]); // %18 rtarray u32
    fc_spv_inst(&mut v, 30, &[19, 18]); // %19 struct
    fc_spv_inst(&mut v, 32, &[20, 12, 19]); // %20 ptr SB struct
    fc_spv_inst(&mut v, 59, &[20, 21, 12]); // %21 out
    fc_spv_inst(&mut v, 32, &[23, 12, 4]); // %23 ptr SB u32
    fc_spv_inst(&mut v, 43, &[4, 26, 0]); // %26 = 0
    fc_spv_inst(&mut v, 43, &[4, 27, 1]); // %27 = 1
    fc_spv_inst(&mut v, 43, &[4, 28, 4]); // %28 = 4
    fc_spv_inst(&mut v, 43, &[4, 30, 2]); // %30 = 2
    fc_spv_inst(&mut v, 43, &[4, 31, 3]); // %31 = 3
    fc_spv_inst(&mut v, 43, &[4, 32, 0xFFFF_FFFF]); // %32 = 哨兵
    fc_spv_inst(&mut v, 54, &[2, 1, 0, 3]); // %1 = OpFunction
    fc_spv_inst(&mut v, 248, &[40]); // %40 label
    fc_spv_inst(&mut v, 61, &[8, 42, 10]); // %42 = load gid
    fc_spv_inst(&mut v, 81, &[4, 43, 42, 0]); // %43 = gid.x
    fc_spv_inst(&mut v, 132, &[4, 44, 43, 28]); // %44 = i*4
    let offs = [26u32, 27, 30, 31];
    let mut next_id = 45u32;
    for (j, off) in offs.iter().enumerate() {
        let idx = if j == 0 {
            44
        } else {
            let id = next_id;
            next_id += 1;
            fc_spv_inst(&mut v, 128, &[4, id, 44, *off]);
            id
        };
        let addr = next_id;
        next_id += 1;
        fc_spv_inst(&mut v, 65, &[23, addr, 21, 26, idx]);
        fc_spv_inst(&mut v, 62, &[addr, 32]); // store 哨兵
    }
    fc_spv_inst(&mut v, 253, &[]); // OpReturn
    fc_spv_inst(&mut v, 56, &[]); // OpFunctionEnd
    v
}

/// ray query kernel（set0:b0 = TLAS / b1 = 光线 8 f32 SSBO / b2 = 输出
/// 4 u32/线 [committed, t_bits, instance_id, primitive];LocalSize 1×1×1,
/// groups.x = 光线数——vk_clas_rt `m94_ray_query_spv(false)` 逐指令同形制,
/// 命中槽位 = CommittedPrimitiveIndex 即竞技场槽三角号 = 出帧几何证据位）。
#[allow(dead_code)]
fn frame_cut_rq_spv() -> Vec<u32> {
    let mut v = vec![0x0723_0203u32, 0x0001_0400, 0, 128, 0];
    fc_spv_inst(&mut v, 17, &[1]); // OpCapability Shader
    fc_spv_inst(&mut v, 17, &[4472]); // OpCapability RayQueryKHR
    let mut ext = vec![];
    ext.extend(fc_spv_words("SPV_KHR_ray_query"));
    fc_spv_inst(&mut v, 10, &ext);
    fc_spv_inst(&mut v, 14, &[0, 1]); // OpMemoryModel Logical GLSL450
    let mut ep = vec![5u32, 1];
    ep.extend(fc_spv_words("main"));
    ep.extend_from_slice(&[10, 13, 17, 21]);
    fc_spv_inst(&mut v, 15, &ep);
    fc_spv_inst(&mut v, 16, &[1, 17, 1, 1, 1]); // LocalSize 1 1 1
    fc_spv_inst(&mut v, 71, &[10, 11, 28]); // gid BuiltIn
    fc_spv_inst(&mut v, 71, &[13, 34, 0]); // TLAS set0
    fc_spv_inst(&mut v, 71, &[13, 33, 0]); // TLAS b0
    fc_spv_inst(&mut v, 71, &[17, 34, 0]); // rays set0
    fc_spv_inst(&mut v, 71, &[17, 33, 1]); // rays b1
    fc_spv_inst(&mut v, 71, &[21, 34, 0]); // out set0
    fc_spv_inst(&mut v, 71, &[21, 33, 2]); // out b2
    fc_spv_inst(&mut v, 71, &[15, 2]); // rays Block
    fc_spv_inst(&mut v, 72, &[15, 0, 35, 0]);
    fc_spv_inst(&mut v, 71, &[19, 2]); // out Block
    fc_spv_inst(&mut v, 72, &[19, 0, 35, 0]);
    fc_spv_inst(&mut v, 71, &[14, 6, 4]); // stride 4
    fc_spv_inst(&mut v, 71, &[18, 6, 4]);
    fc_spv_inst(&mut v, 19, &[2]); // void
    fc_spv_inst(&mut v, 33, &[3, 2]); // fn
    fc_spv_inst(&mut v, 21, &[4, 32, 0]); // u32
    fc_spv_inst(&mut v, 22, &[5, 32]); // f32
    fc_spv_inst(&mut v, 20, &[6]); // bool
    fc_spv_inst(&mut v, 23, &[7, 5, 3]); // vec3f
    fc_spv_inst(&mut v, 23, &[8, 4, 3]); // uvec3
    fc_spv_inst(&mut v, 32, &[9, 1, 8]); // ptr Input uvec3
    fc_spv_inst(&mut v, 59, &[9, 10, 1]); // gid
    fc_spv_inst(&mut v, 5341, &[11]); // OpTypeAccelerationStructureKHR
    fc_spv_inst(&mut v, 32, &[12, 0, 11]); // ptr UC
    fc_spv_inst(&mut v, 59, &[12, 13, 0]); // TLAS var
    fc_spv_inst(&mut v, 29, &[14, 5]); // rtarray f32
    fc_spv_inst(&mut v, 30, &[15, 14]);
    fc_spv_inst(&mut v, 32, &[16, 12, 15]);
    fc_spv_inst(&mut v, 59, &[16, 17, 12]); // rays var
    fc_spv_inst(&mut v, 29, &[18, 4]); // rtarray u32
    fc_spv_inst(&mut v, 30, &[19, 18]);
    fc_spv_inst(&mut v, 32, &[20, 12, 19]);
    fc_spv_inst(&mut v, 59, &[20, 21, 12]); // out var
    fc_spv_inst(&mut v, 32, &[22, 12, 5]); // ptr SB f32
    fc_spv_inst(&mut v, 32, &[23, 12, 4]); // ptr SB u32
    fc_spv_inst(&mut v, 4472, &[24]); // OpTypeRayQueryKHR
    fc_spv_inst(&mut v, 32, &[25, 7, 24]); // ptr Function rq
    fc_spv_inst(&mut v, 43, &[4, 26, 0]);
    fc_spv_inst(&mut v, 43, &[4, 27, 1]); // flags Opaque / committed / 常量 1
    fc_spv_inst(&mut v, 43, &[4, 28, 4]);
    fc_spv_inst(&mut v, 43, &[4, 29, 8]);
    fc_spv_inst(&mut v, 43, &[4, 30, 0xFF]); // cull mask
    fc_spv_inst(&mut v, 43, &[4, 32, 2]);
    fc_spv_inst(&mut v, 43, &[4, 33, 3]);
    fc_spv_inst(&mut v, 43, &[4, 34, 5]);
    fc_spv_inst(&mut v, 43, &[4, 35, 6]);
    fc_spv_inst(&mut v, 43, &[4, 36, 7]);
    fc_spv_inst(&mut v, 54, &[2, 1, 0, 3]); // OpFunction
    fc_spv_inst(&mut v, 248, &[40]);
    fc_spv_inst(&mut v, 59, &[25, 41, 7]); // rq var
    fc_spv_inst(&mut v, 61, &[8, 42, 10]); // load gid
    fc_spv_inst(&mut v, 81, &[4, 43, 42, 0]); // i = gid.x
    fc_spv_inst(&mut v, 132, &[4, 44, 43, 29]); // base = i*8
    let offs = [26u32, 27, 32, 33, 28, 34, 35, 36];
    let mut next_id = 45u32;
    let mut val_ids = [0u32; 8];
    for (k, slot) in val_ids.iter_mut().enumerate() {
        let idx_id = if k == 0 {
            44
        } else {
            let id = next_id;
            next_id += 1;
            fc_spv_inst(&mut v, 128, &[4, id, 44, offs[k]]);
            id
        };
        let addr_id = next_id;
        next_id += 1;
        fc_spv_inst(&mut v, 65, &[22, addr_id, 17, 26, idx_id]);
        let val_id = next_id;
        next_id += 1;
        fc_spv_inst(&mut v, 61, &[5, val_id, addr_id]);
        *slot = val_id;
    }
    let origin = next_id;
    next_id += 1;
    fc_spv_inst(&mut v, 80, &[7, origin, val_ids[0], val_ids[1], val_ids[2]]);
    let dir = next_id;
    next_id += 1;
    fc_spv_inst(&mut v, 80, &[7, dir, val_ids[3], val_ids[4], val_ids[5]]);
    let as_id = next_id;
    next_id += 1;
    fc_spv_inst(&mut v, 61, &[11, as_id, 13]);
    fc_spv_inst(
        &mut v,
        4473,
        &[41, as_id, 27, 30, origin, val_ids[6], dir, val_ids[7]],
    );
    let loop_lbl = next_id;
    next_id += 1;
    let cont_lbl = next_id;
    next_id += 1;
    let after_lbl = next_id;
    next_id += 1;
    fc_spv_inst(&mut v, 249, &[loop_lbl]);
    fc_spv_inst(&mut v, 248, &[loop_lbl]);
    let cond = next_id;
    next_id += 1;
    fc_spv_inst(&mut v, 4477, &[6, cond, 41]); // OpRayQueryProceedKHR
    fc_spv_inst(&mut v, 246, &[after_lbl, cont_lbl, 0]);
    fc_spv_inst(&mut v, 250, &[cond, cont_lbl, after_lbl]);
    fc_spv_inst(&mut v, 248, &[cont_lbl]);
    fc_spv_inst(&mut v, 249, &[loop_lbl]);
    fc_spv_inst(&mut v, 248, &[after_lbl]);
    let ty = next_id;
    next_id += 1;
    fc_spv_inst(&mut v, 4479, &[4, ty, 41, 27]); // GetIntersectionType Committed
    let has = next_id;
    next_id += 1;
    fc_spv_inst(&mut v, 171, &[6, has, ty, 26]);
    let hit_lbl = next_id;
    next_id += 1;
    let miss_lbl = next_id;
    next_id += 1;
    let merge_lbl = next_id;
    next_id += 1;
    fc_spv_inst(&mut v, 247, &[merge_lbl, 0]);
    fc_spv_inst(&mut v, 250, &[has, hit_lbl, miss_lbl]);
    fc_spv_inst(&mut v, 248, &[hit_lbl]);
    let t_id = next_id;
    next_id += 1;
    fc_spv_inst(&mut v, 6018, &[5, t_id, 41, 27]); // committed T
    let inst_id = next_id;
    next_id += 1;
    fc_spv_inst(&mut v, 6020, &[4, inst_id, 41, 27]); // committed InstanceId
    let prim_id = next_id;
    next_id += 1;
    fc_spv_inst(&mut v, 6023, &[4, prim_id, 41, 27]); // committed PrimitiveIndex
    let tbits = next_id;
    next_id += 1;
    fc_spv_inst(&mut v, 124, &[4, tbits, t_id]);
    let o0 = next_id;
    next_id += 1;
    fc_spv_inst(&mut v, 132, &[4, o0, 43, 28]); // o0 = i*4
    let store_vals = [27, tbits, inst_id, prim_id];
    for (j, val) in store_vals.iter().enumerate() {
        let idx = if j == 0 {
            o0
        } else {
            let id = next_id;
            next_id += 1;
            fc_spv_inst(&mut v, 128, &[4, id, o0, offs[j]]);
            id
        };
        let addr = next_id;
        next_id += 1;
        fc_spv_inst(&mut v, 65, &[23, addr, 21, 26, idx]);
        fc_spv_inst(&mut v, 62, &[addr, *val]);
    }
    fc_spv_inst(&mut v, 249, &[merge_lbl]);
    fc_spv_inst(&mut v, 248, &[miss_lbl]);
    let m0 = next_id;
    next_id += 1;
    fc_spv_inst(&mut v, 132, &[4, m0, 43, 28]);
    for j in 0..4u32 {
        let idx = if j == 0 {
            m0
        } else {
            let id = next_id;
            next_id += 1;
            fc_spv_inst(&mut v, 128, &[4, id, m0, offs[j as usize]]);
            id
        };
        let addr = next_id;
        next_id += 1;
        fc_spv_inst(&mut v, 65, &[23, addr, 21, 26, idx]);
        fc_spv_inst(&mut v, 62, &[addr, 26]); // miss 全 0
    }
    fc_spv_inst(&mut v, 249, &[merge_lbl]);
    fc_spv_inst(&mut v, 248, &[merge_lbl]);
    fc_spv_inst(&mut v, 253, &[]);
    fc_spv_inst(&mut v, 56, &[]);
    v
}

// ---------------------------------------------------------------------------
// 会话与帧循环（DeviceFrameSession + FrameUpdate.blas_refit——B5 冻结通路直调;
// 单槽顺序入口,FIF 拒 refit 的 A2/B5 约束如实登记）
// ---------------------------------------------------------------------------

/// 单会话跑完整轨迹（创建 → 逐帧 cut→增量→refit→RQ→digest;返回逐帧 digest
/// 与统计）。`collect` = false 时只收 digest（双跑第二遍）。
/// G38 T3:`ext` = copy 模式/min-level 降档(双跑两遍同 ext——执行路径进
/// digest 判据域);`min_parents_all` = min_level>0 时逐块父映射预计算。
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
fn frame_cut_run_session(
    tag: &str,
    blocks: &[ClusterPackBlock],
    passthrough_stream: &[f32],
    arena: &FrameCutArena,
    opt: &FrameCutArmOpt,
    ext: &FrameCutArmExtOpt,
    min_parents_all: &[Vec<Option<u32>>],
    threshold_px: f32,
    samples: &[FrameCutCamSample],
    collect: bool,
) -> (Vec<String>, Vec<FrameCutFrameStat>) {
    let n_rays = (opt.res_w * opt.res_h) as usize;
    let arena_bytes_len = arena.total_tris as u64 * 36;

    // ── G40 T2(#77 P2):device 决策码为源——表驻留 cull 会话(常驻,
    //    每帧仅 params 256B 上传 + 决策码 n×4B 回读;DESIGN §2.7 P2 行字面)。
    //    red-arm 受害裁决需帧 0 host 参考码(诊断臂;生产决策源不回移)。──
    let device_ctx: Option<FrameCutDeviceCtx> = ext.cut_source_device.then(|| {
        let e0: Option<Vec<u32>> = ext.red_arm_tamper.then(|| {
            let s0 = &samples[0];
            let (_, _, _, _, pre) = frame_cut_select_ext(
                tag,
                blocks,
                &s0.spec,
                s0.in_w,
                s0.in_h,
                threshold_px,
                s0.frame,
                ext.min_level,
                min_parents_all,
                &mut FrameCutSelectTiming::default(),
            );
            frame_cut_device_expected(&pre)
        });
        frame_cut_device_ctx(tag, ext, blocks, e0.as_deref(), collect)
    });
    // cull 会话资源面(kernel 10 buffer 布局 = P1 run_compute 字面继承;
    // 表三件 + hzb 兜底 + counters 初值零 = device_local 驻留 staging 上传,
    // params = 逐帧上传目标 host-visible,decisions = device_local 帧尾
    // staging 回读)。counters 不逐帧清零登记:决策码逐输入项无条件写
    // (kernel 头注「固定槽位,顺序无关对拍面」),原子计数仅门 vis/occ 列表
    // 追加写(本臂零消费),溢出丢弃语义(cap=n)不回染 decisions。
    let cull_cluster_bytes: Vec<u8> = device_ctx
        .as_ref()
        .map(|c| bytes_f32(&c.cluster_f32))
        .unwrap_or_default();
    let cull_lod_bytes: Vec<u8> = device_ctx
        .as_ref()
        .map(|c| bytes_f32(&c.lod_f32))
        .unwrap_or_default();
    let cull_ids_bytes: Vec<u8> = device_ctx
        .as_ref()
        .map(|c| (0..c.n as u32).flat_map(|i| i.to_le_bytes()).collect())
        .unwrap_or_default();
    let cull_spv_bytes: Vec<u8> = device_ctx
        .as_ref()
        .map(|c| c.spv.iter().flat_map(|w| w.to_le_bytes()).collect())
        .unwrap_or_default();
    let cull_hzb_data: Vec<u8> = bytes_f32(&[0.0]);
    let cull_hzb_meta: Vec<u8> = [0u32, 1, 1].iter().flat_map(|x| x.to_le_bytes()).collect();
    let cull_zero12 = vec![0u8; 12];
    let cull_n = device_ctx.as_ref().map_or(1, |c| c.n.max(1));
    fn cull_buf<'x>(size: u64, data: Option<&'x [u8]>, device_local: bool) -> ResourceDesc<'x> {
        ResourceDesc::Buffer(BufferDesc {
            size,
            usage: BufferUsage {
                storage: true,
                ..BufferUsage::default()
            },
            data,
            device_local,
        })
    }
    let cull_resources: Vec<ResourceDesc> = if device_ctx.is_some() {
        vec![
            cull_buf(256, None, false),                                   // 0 params(逐帧上传)
            cull_buf(cull_cluster_bytes.len() as u64, Some(&cull_cluster_bytes), true), // 1 簇表驻留
            cull_buf(cull_lod_bytes.len() as u64, Some(&cull_lod_bytes), true), // 2 lod 表驻留
            cull_buf(cull_ids_bytes.len() as u64, Some(&cull_ids_bytes), true), // 3 input_ids 恒等
            cull_buf(4, Some(&cull_hzb_data), true),                      // 4 hzb_data 兜底(短路不读)
            cull_buf(12, Some(&cull_hzb_meta), true),                     // 5 hzb_meta
            cull_buf(12, Some(&cull_zero12), true),                       // 6 counters(初值零,不逐帧清)
            cull_buf(cull_n as u64 * 4, None, true),                      // 7 decisions(帧尾 staging 回读)
            cull_buf(cull_n as u64 * 4, None, true),                      // 8 vis_ids(零消费)
            cull_buf(cull_n as u64 * 4, None, true),                      // 9 occ_ids(零消费)
        ]
    } else {
        Vec::new()
    };
    let cull_passes: Vec<Pass<'_>> = if device_ctx.is_some() {
        vec![Pass::Compute(ComputePass {
            name: "fc_cull",
            spirv: &cull_spv_bytes,
            entry: None, // 自 OpEntryPoint 解析(rurixc 原产入口)
            dispatch: DispatchSpec::Direct([cull_n as u32, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
                ..Bindings::default()
            },
        })]
    } else {
        Vec::new()
    };
    let cull_plan: Vec<(u32, TargetState)> =
        (0u32..10).map(|r| (r, TargetState::StorageReadWrite)).collect();
    let cull_barriers: [&[(u32, TargetState)]; 1] = [&cull_plan];
    let cull_readbacks = [Readback::Buffer {
        res: 7,
        offset: 0,
        size: cull_n as u64 * 4,
    }];
    let mut cull_session: Option<DeviceFrameSession<'_>> = device_ctx.as_ref().map(|c| {
        let t_cs = std::time::Instant::now();
        let s = DeviceFrameSession::new(
            &cull_resources,
            &cull_passes,
            &cull_barriers,
            &cull_readbacks,
            2,
        )
        .unwrap_or_else(|e| fail(&format!("{tag}: cull 会话创建: {e}")));
        if collect {
            eprintln!(
                "{tag}: cull 会话就绪 n={} 驻留表字节={} create_ms={:.0}(P2 表驻留;每帧 params 256B 上传 + 决策码 {}B 回读)",
                c.n,
                cull_cluster_bytes.len() + cull_lod_bytes.len() + cull_ids_bytes.len(),
                t_cs.elapsed().as_secs_f64() * 1e3,
                c.n * 4,
            );
        }
        s
    });
    // 逐帧 device 决策半程:params 上传 → dispatch → 决策码回读(闭集/覆盖性
    // 消费在 select_from_decisions);返回 (决策码, 全程墙钟, dispatch GPU ms,
    // 决策码 sha256)。
    let cull_frame = |cs: &mut DeviceFrameSession<'_>,
                          spec: &CameraSpec,
                          in_w: u32,
                          in_h: u32,
                          frame: u32|
     -> (Vec<u32>, f64, Option<f64>, String) {
        let n = device_ctx.as_ref().map_or(1, |c| c.n);
        let t0 = std::time::Instant::now();
        let params = frame_cut_device_params(spec, in_w, in_h, threshold_px, n);
        let update = FrameUpdate {
            tlas_update: None,
            buffer_uploads: vec![(StableResourceId(1), 0, bytes_f32(&params[..]))],
            binding_overrides: vec![],
            push_constant_overrides: vec![],
            readback_subset: Some(vec![0]),
            blas_refit: None,
        };
        let prov = cs
            .next_provenance_with_update(&update)
            .unwrap_or_else(|e| fail(&format!("{tag}: 帧 {frame} cull provenance: {e}")));
        let out = cs
            .execute_with_frame_update(&prov, &update)
            .unwrap_or_else(|e| fail(&format!("{tag}: 帧 {frame} cull 提交: {e}")));
        if out.telemetry.validation_error_count != 0 {
            fail(&format!(
                "{tag}: 帧 {frame} cull validation ERROR {} 次(fail-closed)",
                out.telemetry.validation_error_count
            ));
        }
        let rb = &out.readbacks[0];
        let sha = sha256_hex(rb);
        let decisions: Vec<u32> = rb
            .chunks_exact(4)
            .take(n)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        let gpu_ms = out.telemetry.passes.first().map(|p| p.gpu_ns / 1e6);
        (decisions, t0.elapsed().as_secs_f64() * 1e3, gpu_ms, sha)
    };

    // 帧 0 cut 先行（初始竞技场 = 帧 0 已施加;帧 0 refit 桥 = 内容恒等,节拍均匀）。
    // G40 T2:决策源分叉——device 臂 = cull 会话决策码 + host 影子核 verify/
    // 提升链;host 臂 = 既有 select_ext 字面。
    let s0 = &samples[0];
    let mut cut0_timing = FrameCutSelectTiming::default();
    let mut cut0_dev: (Option<f64>, Option<String>) = (None, None);
    let t0 = std::time::Instant::now();
    let (cut0, cut0_clusters, cut0_tris, cut0_tris_promoted) =
        if let Some(cs) = cull_session.as_mut() {
            let (dec, wall, gpu, sha) = cull_frame(cs, &s0.spec, s0.in_w, s0.in_h, s0.frame);
            let t_sets = std::time::Instant::now();
            let r = frame_cut_select_from_decisions(
                tag,
                blocks,
                &dec,
                s0.frame,
                ext.min_level,
                min_parents_all,
                &mut cut0_timing,
            );
            cut0_timing.select_ms += wall + t_sets.elapsed().as_secs_f64() * 1e3
                - cut0_timing.verify_ms
                - cut0_timing.promote_ms;
            cut0_dev = (gpu, Some(sha));
            r
        } else {
            let (sets, c, t, tp, _) = frame_cut_select_ext(
                tag,
                blocks,
                &s0.spec,
                s0.in_w,
                s0.in_h,
                threshold_px,
                s0.frame,
                ext.min_level,
                min_parents_all,
                &mut cut0_timing,
            );
            (sets, c, t, tp)
        };
    let cut0_ms = t0.elapsed().as_secs_f64() * 1e3;
    // 创建期 BLAS = 全簇真几何超集（根 AABB 覆盖一切后续 refit 内容;TLAS 实例
    // AABB 不逐帧更新的 B5 蒙皮同律下假漏命中结构性排除）;帧 0 以单条全量
    // 上传把竞技场收到 cut0（逐槽增量自帧 1 起）。
    let arena_f32 = frame_cut_full_stream(blocks, passthrough_stream, arena);
    let arena_init_bytes: Vec<u8> = arena_f32.iter().flat_map(|x| x.to_le_bytes()).collect();
    let mut frame0_bytes: Option<Vec<u8>> = {
        let mut s = arena_f32.clone();
        frame_cut_apply_cut(&mut s, blocks, arena, &cut0);
        Some(bytes_f32(&s))
    };

    let clear_spv = fc_spv_bytes(&frame_cut_clear_spv());
    let rq_spv = fc_spv_bytes(&frame_cut_rq_spv());

    let resources = [
        // 0: 光线 SSBO（host 上传目标）。
        ResourceDesc::Buffer(BufferDesc {
            size: (n_rays * 32) as u64,
            usage: BufferUsage {
                storage: true,
                ..BufferUsage::default()
            },
            data: None,
            device_local: false,
        }),
        // 1: 命中输出 SSBO（host-visible 直回读）。
        ResourceDesc::Buffer(BufferDesc {
            size: (n_rays * 16) as u64,
            usage: BufferUsage {
                storage: true,
                ..BufferUsage::default()
            },
            data: None,
            device_local: false,
        }),
        // 2: 竞技场顶点 SSBO（refit 桥 src;host 增量上传目标）。
        ResourceDesc::Buffer(BufferDesc {
            size: arena_bytes_len,
            usage: BufferUsage {
                storage: true,
                ..BufferUsage::default()
            },
            data: Some(&arena_init_bytes),
            device_local: false,
        }),
    ];
    let passes = [
        Pass::Compute(ComputePass {
            name: "fc_clear",
            spirv: &clear_spv,
            entry: Some("main"),
            dispatch: DispatchSpec::Direct([n_rays as u32, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![1],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "fc_rq",
            spirv: &rq_spv,
            entry: Some("main"),
            dispatch: DispatchSpec::Direct([n_rays as u32, 1, 1]),
            bindings: Bindings {
                accel_structs: vec![0],
                storage_buffers: vec![0, 1],
                ..Bindings::default()
            },
        }),
    ];
    let plan0 = [(1u32, TargetState::StorageWrite)];
    let plan1 = [
        (0u32, TargetState::StorageReadWrite),
        (1u32, TargetState::StorageReadWrite),
    ];
    let barriers: [&[(u32, TargetState)]; 2] = [&plan0, &plan1];
    let readbacks = [Readback::Buffer {
        res: 1,
        offset: 0,
        size: (n_rays * 16) as u64,
    }];
    let tris_ref: [&[f32]; 1] = [&arena_f32[..]];
    let instances = [RayQueryInstanceDesc {
        blas: 0,
        custom_index: 0,
        mask: 0xFF,
        sbt_record_offset: 0,
    }];
    let accel = [AccelStructDesc {
        scene: RayQuerySceneDesc {
            blas_triangles: &tris_ref,
            instances: &instances,
        },
        transforms: None,
        // 判档核心：竞技场 BLAS 打标 ALLOW_UPDATE（B5 create_scene_ex 通路）。
        updatable_blas: &[0],
    }];
    let t_create = std::time::Instant::now();
    let mut session = DeviceFrameSession::new_with_accel_structs(
        &resources,
        &passes,
        &barriers,
        &readbacks,
        2,
        &accel,
    )
    .unwrap_or_else(|e| fail(&format!("{tag}: AS 会话创建: {e}")));
    if collect {
        eprintln!(
            "{tag}: 会话就绪 arena_tris={} arena_mb={:.1} create_ms={:.0}（单 BLAS 单实例,ALLOW_UPDATE;初始 build 含帧 0 cut）",
            arena.total_tris,
            arena_bytes_len as f64 / 1e6,
            t_create.elapsed().as_secs_f64() * 1e3,
        );
    }

    let mut applied: Vec<Vec<bool>> = cut0.clone();
    let mut digests: Vec<String> = Vec::with_capacity(samples.len());
    let mut stats: Vec<FrameCutFrameStat> = Vec::new();
    for (k, s) in samples.iter().enumerate() {
        // ── ① cut 决策(帧 0 复用先行结果;G40 T2 P2 决策源分叉:device 臂 =
        //    cull 会话决策码 → host 影子核 verify/提升〔select_from_decisions〕;
        //    host 臂 = 既有 select_ext 字面。覆盖性机核两路皆 fail-closed。
        //    双跑两遍 session 各自建 cull 会话重放 ⇒ device 决策跨跑一致性经
        //    「两跑 digest 位级」传递性成立〔B5 P1 期望码全等谱系已证同源〕)──
        let t_cut = std::time::Instant::now();
        let mut cut_timing = FrameCutSelectTiming::default();
        let mut cut_dev: (Option<f64>, Option<String>) = (None, None);
        let (cut, cut_clusters, cut_tris, cut_tris_promoted) = if k == 0 {
            cut_timing = cut0_timing;
            cut_dev = cut0_dev.clone();
            (
                cut0.clone(),
                cut0_clusters,
                cut0_tris,
                cut0_tris_promoted,
            )
        } else if let Some(cs) = cull_session.as_mut() {
            let (dec, wall, gpu, sha) = cull_frame(cs, &s.spec, s.in_w, s.in_h, s.frame);
            let t_sets = std::time::Instant::now();
            let r = frame_cut_select_from_decisions(
                tag,
                blocks,
                &dec,
                s.frame,
                ext.min_level,
                min_parents_all,
                &mut cut_timing,
            );
            cut_timing.select_ms += wall + t_sets.elapsed().as_secs_f64() * 1e3
                - cut_timing.verify_ms
                - cut_timing.promote_ms;
            cut_dev = (gpu, Some(sha));
            r
        } else {
            let (sets, c, t, tp, _) = frame_cut_select_ext(
                tag,
                blocks,
                &s.spec,
                s.in_w,
                s.in_h,
                threshold_px,
                s.frame,
                ext.min_level,
                min_parents_all,
                &mut cut_timing,
            );
            (sets, c, t, tp)
        };
        let cut_ms = if k == 0 {
            cut0_ms
        } else {
            t_cut.elapsed().as_secs_f64() * 1e3
        };
        let (device_cut_dispatch_gpu_ms, device_cut_decisions_sha256) = cut_dev;

        // ── ② 槽位增量（--cut-every 节拍;refit 帧才施加)。G38 T3:同一差集
        //    循环顺带收集桥接脏区段(槽升序天然,相邻槽合并;帧 0 全量 =
        //    None〔单 region 语义〕,--refit-copy full 对照臂恒 None)──
        let t_delta = std::time::Instant::now();
        let refit_frame = s.frame % opt.cut_every.max(1) == 0;
        let mut uploads: Vec<(StableResourceId, u64, Vec<u8>)> = Vec::new();
        let mut changed_slots = 0u32;
        let mut upload_bytes = 0u64;
        let mut copy_regions: Option<Vec<(u64, u64)>> = None;
        // 光线每帧上传（相机推进面;确定性 host f32）。
        let ray_bytes = bytes_f32(&frame_cut_rays(&s.spec, opt.res_w, opt.res_h));
        upload_bytes += ray_bytes.len() as u64;
        uploads.push((StableResourceId(1), 0, ray_bytes));
        if refit_frame {
            if let Some(bytes) = frame0_bytes.take() {
                // 帧 0:全簇超集 → cut0 的单条全量上传（逐槽增量自帧 1 起;
                // 折叠槽计数只数实际占槽簇——min-level 降档下无槽簇不计）。
                changed_slots = blocks
                    .iter()
                    .enumerate()
                    .map(|(bi, b)| {
                        (0..b.records.len())
                            .filter(|&ci| {
                                arena.slot_base[bi][ci] != FC_NO_SLOT && !cut[bi][ci]
                            })
                            .count() as u32
                    })
                    .sum();
                upload_bytes += bytes.len() as u64;
                uploads.push((StableResourceId(3), 0, bytes));
            } else {
                let mut regions: Vec<(u64, u64)> = Vec::new();
                for (bi, b) in blocks.iter().enumerate() {
                    for (ci, r) in b.records.iter().enumerate() {
                        if cut[bi][ci] == applied[bi][ci] {
                            continue;
                        }
                        // 提升映射保证 cut ⊆ 占槽簇;差集簇必有槽(fail-closed)。
                        assert!(
                            arena.slot_base[bi][ci] != FC_NO_SLOT,
                            "差集簇 (块 {bi},簇 {ci}) 无竞技场槽(min-level 提升破坏)"
                        );
                        changed_slots += 1;
                        let n9 = r.triangle_count as usize * 9;
                        let mut slot = vec![0.0f32; n9];
                        if cut[bi][ci] {
                            frame_cut_write_cluster(&mut slot, b, ci);
                        }
                        let bytes = bytes_f32(&slot);
                        upload_bytes += bytes.len() as u64;
                        let off = arena.slot_base[bi][ci] as u64 * 36;
                        let len = bytes.len() as u64;
                        uploads.push((StableResourceId(3), off, bytes));
                        // 脏区段收集(canonical 槽升序 ⇒ off 单调;相邻合并)。
                        frame_cut_merge_region(&mut regions, off, len);
                    }
                }
                if !ext.copy_full {
                    // incr 臂:差集脏区段(可空 = 本帧 cut 无变化,桥 copy 跳过,
                    // UPDATE build 照录——vbuf 与 arena SSBO 位级同步)。
                    copy_regions = Some(regions);
                }
            }
            applied = cut.clone();
        }
        let delta_ms = t_delta.elapsed().as_secs_f64() * 1e3;

        // ── ③ 提交（refit 帧:pass0 后桥接 copy〔incr = 脏区段多 region /
        //    full = 全竞技场单 region〕→ UPDATE build → consume barrier →
        //    pass1 RQ 读新 BLAS;B5 冻结通路 + G38 T3 桥扩展加性入口)──
        let update = FrameUpdate {
            tlas_update: None,
            buffer_uploads: uploads,
            binding_overrides: vec![],
            push_constant_overrides: vec![],
            // G37 W4 修正(fif_dyn 窗交叉复核检出):render_exec 字面 None =
            // 不回读,而下方 ④ 消费 readbacks[0]——须显式订阅回读槽 0。
            readback_subset: Some(vec![0]),
            blas_refit: refit_frame.then_some(BlasRefitUpdate {
                as_index: 0,
                blas_index: 0,
                src: StableResourceId(3),
                src_offset: 0,
                byte_len: arena_bytes_len,
                after_pass: 0,
            }),
        };
        // G38 T3:桥扩展(refit 帧才有桥;copy_regions None = 既有全量单
        // region 命令流逐字;计时恒开——query 追加区,不动逐 pass 口径)。
        // stat 登记口径:段数/字节按实际桥 copy 计。
        let (copy_regions_n, copy_bytes) = if refit_frame {
            match &copy_regions {
                None => (1u32, arena_bytes_len),
                Some(rs) => (rs.len() as u32, rs.iter().map(|&(_, l)| l).sum()),
            }
        } else {
            (0, 0)
        };
        let bridge = refit_frame.then(|| rurix_rt::render_exec::BlasRefitBridgeExt {
            copy_regions,
            collect_gpu_timing: true,
        });
        let t_exec = std::time::Instant::now();
        let prov = session
            .next_provenance_with_update(&update)
            .unwrap_or_else(|e| fail(&format!("{tag}: 帧 {} provenance: {e}", s.frame)));
        let out = session
            .execute_with_frame_update_bridge_ext(&prov, &update, bridge.as_ref())
            .unwrap_or_else(|e| fail(&format!("{tag}: 帧 {} 提交: {e}", s.frame)));
        let exec_ms = t_exec.elapsed().as_secs_f64() * 1e3;
        if out.telemetry.validation_error_count != 0 {
            fail(&format!(
                "{tag}: 帧 {} validation ERROR {} 次（fail-closed）",
                s.frame, out.telemetry.validation_error_count
            ));
        }

        // ── ④ 判据:哨兵 canary + 命中槽位 ∈ 已施加 cut ∪ passthrough ──
        let rb = &out.readbacks[0];
        let mut hits = 0u64;
        for i in 0..n_rays {
            let w = |j: usize| -> u32 {
                let o = (i * 4 + j) * 4;
                u32::from_le_bytes([rb[o], rb[o + 1], rb[o + 2], rb[o + 3]])
            };
            let committed = w(0);
            if committed == 0xFFFF_FFFF {
                fail(&format!(
                    "{tag}: 帧 {} 光线 {i} 残留哨兵（RQ dispatch 覆盖缺陷）",
                    s.frame
                ));
            }
            if committed == 0 {
                continue;
            }
            hits += 1;
            let prim = w(3);
            if prim as usize >= arena.total_tris {
                fail(&format!(
                    "{tag}: 帧 {} 命中图元 {prim} 越竞技场域 {}",
                    s.frame, arena.total_tris
                ));
            }
            if let Some((bi, ci)) = frame_cut_owner(arena, prim)
                && !applied[bi as usize][ci as usize]
            {
                fail(&format!(
                    "{tag}: 帧 {} 命中槽 (块 {bi},簇 {ci},图元 {prim}) 不在已施加 cut——零面积折叠失效/陈旧几何（fail-closed）",
                    s.frame
                ));
            }
        }
        if hits == 0 {
            fail(&format!("{tag}: 帧 {} 零命中（空接线防伪,fail-closed）", s.frame));
        }
        let digest = sha256_hex(rb);
        digests.push(digest.clone());
        if collect {
            let gpu_ms = |pi: usize| -> f64 {
                out.telemetry
                    .passes
                    .get(pi)
                    .map_or(0.0, |p| p.gpu_ns / 1e6)
            };
            stats.push(FrameCutFrameStat {
                frame: s.frame,
                cut_clusters,
                cut_tris,
                refit: refit_frame,
                changed_slots,
                upload_bytes,
                hits,
                cut_ms,
                delta_ms,
                exec_ms,
                gpu_clear_ms: gpu_ms(0),
                gpu_rq_ms: gpu_ms(1),
                fence_ms: out.telemetry.cpu_fence_wait_ns as f64 / 1e6,
                cut_tris_promoted,
                copy_regions: copy_regions_n,
                copy_bytes,
                bridge_copy_gpu_ms: out.telemetry.blas_bridge_copy_gpu_ms,
                bridge_build_gpu_ms: out.telemetry.blas_bridge_build_gpu_ms,
                select_ms: cut_timing.select_ms,
                verify_ms: cut_timing.verify_ms,
                promote_ms: cut_timing.promote_ms,
                device_cut_dispatch_gpu_ms,
                device_cut_decisions_sha256,
                digest,
            });
        }
    }
    (digests, stats)
}

/// 臂编排：双跑（两次独立会话重放全轨迹）→ 逐帧 digest 位级断言 →
/// cut_tris 单调变化断言。返回首跑统计。
/// (G38 T3:既有入口 = 扩展默认〔incr copy + min_level 0〕转发——窗口臂
/// 自动受益增量桥,vbuf 终态位级同 ⇒ digest 不变;窗口 bin 调用面 0 改写。)
#[allow(dead_code)]
fn run_frame_cut_arm(
    tag: &str,
    pack: &ClusterPack,
    passthrough_stream: &[f32],
    opt: &FrameCutArmOpt,
    threshold_px: f32,
    samples: &[FrameCutCamSample],
) -> Vec<FrameCutFrameStat> {
    run_frame_cut_arm_ext(
        tag,
        pack,
        passthrough_stream,
        opt,
        &FrameCutArmExtOpt::default_ext(),
        threshold_px,
        samples,
    )
}

/// G38 T3:扩展臂编排(判据面与既有入口逐字同:双跑位级 + 单调门〔仍以
/// **提升前** cut_tris 判——LOD 判据面;提升是表示层映射〕)。
#[allow(dead_code)]
fn run_frame_cut_arm_ext(
    tag: &str,
    pack: &ClusterPack,
    passthrough_stream: &[f32],
    opt: &FrameCutArmOpt,
    ext: &FrameCutArmExtOpt,
    threshold_px: f32,
    samples: &[FrameCutCamSample],
) -> Vec<FrameCutFrameStat> {
    if samples.is_empty() {
        fail("--cluster-per-frame-cut 零相机样本（采集面破坏）");
    }
    if passthrough_stream.len() != pack.passthrough.len() * 9 {
        fail(&format!(
            "passthrough 流长度 {} != 9×{}（须在 apply_cluster_lod 施加前自源场景提取）",
            passthrough_stream.len(),
            pack.passthrough.len()
        ));
    }
    let blocks: &[ClusterPackBlock] = if opt.blocks_limit == 0 {
        &pack.blocks
    } else {
        let n = opt.blocks_limit.min(pack.blocks.len());
        eprintln!(
            "{tag}: 簇块子集臂 blocks={n}/{}（--blocks-limit 逃生阀;子集面如实登记）",
            pack.blocks.len()
        );
        &pack.blocks[..n]
    };
    // G38 T3:min-level 域校验(超包内最大层 = 误配置,fail-closed)+ 逐块
    // 父映射预计算(提升映射与降档布局共用;帧无关,一次算全轨迹用)。
    let min_parents_all: Vec<Vec<Option<u32>>> = if ext.min_level == 0 {
        Vec::new()
    } else {
        let max_level = blocks
            .iter()
            .flat_map(|b| b.nodes.iter().map(|n| n.level))
            .max()
            .unwrap_or(0);
        if ext.min_level > max_level {
            fail(&format!(
                "--min-level {} 超簇包最大层 {max_level}(误配置,fail-closed)",
                ext.min_level
            ));
        }
        blocks.iter().map(frame_cut_min_parents).collect()
    };
    let arena =
        frame_cut_arena_layout_ext(blocks, pack.passthrough.len(), ext.min_level, &min_parents_all);
    if ext.min_level > 0 {
        eprintln!(
            "{tag}: min-level 降档臂 N={} arena_tris={}(全簇布局对照 = 逐块逐簇全量;cut 经 level<N→首个 level≥N 祖先提升,提升后生产 verify 复核)",
            ext.min_level, arena.total_tris,
        );
    }
    let (d1, stats) = frame_cut_run_session(
        tag,
        blocks,
        passthrough_stream,
        &arena,
        opt,
        ext,
        &min_parents_all,
        threshold_px,
        samples,
        true,
    );
    let (d2, _) = frame_cut_run_session(
        tag,
        blocks,
        passthrough_stream,
        &arena,
        opt,
        ext,
        &min_parents_all,
        threshold_px,
        samples,
        false,
    );
    // 判据①:双跑逐帧 digest 位级（重建会话 + BLAS 初建 + refit 序列重放）。
    for (k, (a, b)) in d1.iter().zip(&d2).enumerate() {
        if a != b {
            fail(&format!(
                "{tag}: 双跑帧 {k} digest 分叉 {a} != {b}（确定性协议破坏,fail-closed）"
            ));
        }
    }
    // 判据②:cut_tris 随帧变化。严门（monotone_gate,probe 固定单向 dolly）=
    // 单调不减/不增且首末不等;宽门（窗口真轨迹可折返）= 非常量 + 方向
    // measured 登记（不误红不冒充——折返轨迹单调性无定义）。
    let seq: Vec<u64> = stats.iter().map(|s| s.cut_tris).collect();
    let nondec = seq.windows(2).all(|w| w[1] >= w[0]);
    let noninc = seq.windows(2).all(|w| w[1] <= w[0]);
    let constant = seq.iter().all(|&x| x == seq[0]);
    if opt.monotone_gate {
        if seq.first() == seq.last() || (!nondec && !noninc) {
            fail(&format!(
                "{tag}: cut_tris 序列非单调变化 {:?}（前向 dolly 期望单调;轨迹/步长用 --frames/--step-m 重定,fail-closed）",
                seq
            ));
        }
    } else if constant {
        fail(&format!(
            "{tag}: cut_tris 序列恒常量 {:?}（相机未驱动 cut——采集面/轨迹破坏,fail-closed）",
            seq
        ));
    }
    eprintln!(
        "{tag}: 双跑 digest 位级 {} 帧全等;cut_tris {} → {}（{}）",
        d1.len(),
        seq.first().unwrap_or(&0),
        seq.last().unwrap_or(&0),
        if nondec {
            "单调不减"
        } else if noninc {
            "单调不增"
        } else {
            "非单调(折返轨迹 measured 登记)"
        },
    );
    stats
}

/// sidecar 落盘 + 汇总打印（独立 JSON,不动既有 evidence schema——#58/#95/W2
/// visbuffer 同律;out_path 空 = 只打印）。
/// (G38 T3:既有入口 = 扩展默认转发——窗口臂 sidecar 自动带加性新字段,
/// 既有字段口径逐字不动。)
#[allow(dead_code)]
fn frame_cut_finish(
    tag: &str,
    pack: &ClusterPack,
    opt: &FrameCutArmOpt,
    threshold_px: f32,
    stats: &[FrameCutFrameStat],
) {
    frame_cut_finish_ext(
        tag,
        pack,
        opt,
        &FrameCutArmExtOpt::default_ext(),
        threshold_px,
        stats,
    );
}

/// G38 T3:扩展收口(schema 保持 v1 + **加性**字段:顶层 refit_copy_mode/
/// min_level,逐帧 cut_tris_promoted/copy_regions/copy_bytes/
/// bridge_copy_gpu_ms/bridge_build_gpu_ms〔None → null,fail-soft 如实〕;
/// 既有消费方无 schema 断言〔w4_verify.py 判据 = 进程 rc + 臂 OK〕)。
/// (G39 T5 加性:顶层 cut_source 恒出 + device 臂 device_cut_table_bytes
/// + 逐帧 device_cut_decisions_sha256〔host 臂 null〕。)
/// (G40 T2 P2 加性:逐帧 select_ms/verify_ms/promote_ms 分项恒出〔DESIGN
/// §4-2 分项登记义务〕+ device_cut_dispatch_gpu_ms〔host 臂 null〕;P1 的
/// device_cut_probe_ms 随 run_compute 逐帧路退役,字段不再发射——G39 B5
/// 在案 evidence 不回写,谱系各自完整。)
#[allow(dead_code)]
fn frame_cut_finish_ext(
    tag: &str,
    pack: &ClusterPack,
    opt: &FrameCutArmOpt,
    ext: &FrameCutArmExtOpt,
    threshold_px: f32,
    stats: &[FrameCutFrameStat],
) {
    let refit_frames = stats.iter().filter(|s| s.refit).count();
    let exec_refit: f64 = stats.iter().filter(|s| s.refit).map(|s| s.exec_ms).sum::<f64>()
        / refit_frames.max(1) as f64;
    let n_norefit = stats.len() - refit_frames;
    let exec_norefit: f64 = stats
        .iter()
        .filter(|s| !s.refit)
        .map(|s| s.exec_ms)
        .sum::<f64>()
        / n_norefit.max(1) as f64;
    // G38 T3:桥接 GPU 分解均值(refit 帧且 query 有值才计;fail-soft 缺值
    // 如实不入均)。
    let bridge_avg = |f: fn(&FrameCutFrameStat) -> Option<f64>| -> Option<f64> {
        let vals: Vec<f64> = stats.iter().filter(|s| s.refit).filter_map(f).collect();
        (!vals.is_empty()).then(|| vals.iter().sum::<f64>() / vals.len() as f64)
    };
    let copy_avg = bridge_avg(|s| s.bridge_copy_gpu_ms);
    let build_avg = bridge_avg(|s| s.bridge_build_gpu_ms);
    eprintln!(
        "{tag}: 逐帧 cut→AS 更新臂 OK frames={} refit_frames={refit_frames} exec_ms(refit均)={exec_refit:.2}{}{}（措辞 = measured 登记不设通过线;AS 更新增量 = refit/非 refit 帧对照）",
        stats.len(),
        if n_norefit > 0 {
            format!(" exec_ms(非refit均)={exec_norefit:.2}")
        } else {
            String::new()
        },
        match (copy_avg, build_avg) {
            (Some(c), Some(b)) => format!(
                " bridge_gpu(copy均={c:.2}ms build均={b:.2}ms copy_mode={})",
                if ext.copy_full { "full" } else { "incr" }
            ),
            _ => String::new(),
        },
    );
    if opt.out_path.is_empty() {
        return;
    }
    // G39 T5:device 对拍表字节(cluster 10 f32 + lod 8 f32 = 72B/簇;按消费
    // 中块切片口径 = 会话构造同一 n;host 臂空串 = 字段不出现,加性)。
    let dev_tbl = if ext.cut_source_device {
        let dev_n: usize = if opt.blocks_limit == 0 {
            pack.blocks.iter().map(|b| b.records.len()).sum()
        } else {
            pack.blocks[..opt.blocks_limit.min(pack.blocks.len())]
                .iter()
                .map(|b| b.records.len())
                .sum()
        };
        format!("\"device_cut_table_bytes\":{},", dev_n * 72)
    } else {
        String::new()
    };
    // G40 T2 P2:determinism_note 分叉——host 臂字面逐字保持;device 臂如实
    // 陈述决策码为源形态(host 影子核 verify + 提升/施加链照旧 host)。
    let det_note = if ext.cut_source_device {
        "固定轨迹+固定重建节拍+canonical 竞技场 ⇒ digest 序列同设备双跑位级(本跑已核);跨设备不作 golden(RT 遍历 tie-break 依设备)。cut = device 决策码为源(#77 P2:表驻留 cull 会话,每帧 params 256B 上传 + 决策码回读,host 由 d==4 构造 cut 集;verify_cut_coverage host 影子核 fail-closed 逐字保持,min-level 提升/差集/上传/refit 施加链照旧 host 0 改;等价谱系 = G39 B5 P1 判定码逐项全等门);单槽 inflight(FIF 拒 refit,#89/#90 分界)"
    } else {
        "固定轨迹+固定重建节拍+canonical 竞技场 ⇒ digest 序列同设备双跑位级(本跑已核);跨设备不作 golden(RT 遍历 tie-break 依设备)。cut = host 金标准(device cut kernel 归 #77);单槽 inflight(FIF 拒 refit,#89/#90 分界)"
    };
    let mut sj = String::with_capacity(4096 + stats.len() * 320);
    sj.push_str(&format!(
        "{{\"schema\":\"rurix.g31.frame_cut_probe.v1\",\"threshold_px\":{},\"res\":\"{}x{}\",\"frames\":{},\"step_m\":{},\"cut_every\":{},\"blocks\":{},\"blocks_limit\":{},\"total_clusters\":{},\"passthrough_tris\":{},\"refit_copy_mode\":\"{}\",\"min_level\":{},\"cut_source\":\"{}\",{}\"determinism_note\":\"{det_note}\",\"frames_data\":[",
        threshold_px,
        opt.res_w,
        opt.res_h,
        stats.len(),
        opt.step_m,
        opt.cut_every,
        pack.blocks.len(),
        opt.blocks_limit,
        pack.blocks.iter().map(|b| b.records.len()).sum::<usize>(),
        pack.passthrough.len(),
        if ext.copy_full { "full" } else { "incr" },
        ext.min_level,
        if ext.cut_source_device { "device" } else { "host" },
        dev_tbl,
    ));
    // Option<f64> → JSON(null = query 不可用 fail-soft,如实不冒充)。
    let jopt = |v: Option<f64>| -> String {
        v.map_or_else(|| "null".to_owned(), |x| format!("{x:.3}"))
    };
    for (k, s) in stats.iter().enumerate() {
        if k > 0 {
            sj.push(',');
        }
        sj.push_str(&format!(
            "{{\"frame\":{},\"cut_clusters\":{},\"cut_tris\":{},\"refit\":{},\"changed_slots\":{},\"upload_bytes\":{},\"hits\":{},\"cut_ms\":{:.3},\"delta_ms\":{:.3},\"exec_ms\":{:.3},\"gpu_clear_ms\":{:.3},\"gpu_rq_ms\":{:.3},\"fence_ms\":{:.3},\"cut_tris_promoted\":{},\"copy_regions\":{},\"copy_bytes\":{},\"bridge_copy_gpu_ms\":{},\"bridge_build_gpu_ms\":{},\"select_ms\":{:.3},\"verify_ms\":{:.3},\"promote_ms\":{:.3},\"device_cut_dispatch_gpu_ms\":{},\"device_cut_decisions_sha256\":{},\"digest\":{}}}",
            s.frame,
            s.cut_clusters,
            s.cut_tris,
            s.refit,
            s.changed_slots,
            s.upload_bytes,
            s.hits,
            s.cut_ms,
            s.delta_ms,
            s.exec_ms,
            s.gpu_clear_ms,
            s.gpu_rq_ms,
            s.fence_ms,
            s.cut_tris_promoted,
            s.copy_regions,
            s.copy_bytes,
            jopt(s.bridge_copy_gpu_ms),
            jopt(s.bridge_build_gpu_ms),
            s.select_ms,
            s.verify_ms,
            s.promote_ms,
            jopt(s.device_cut_dispatch_gpu_ms),
            s.device_cut_decisions_sha256
                .as_deref()
                .map_or_else(|| "null".to_owned(), jstr),
            jstr(&s.digest),
        ));
    }
    sj.push_str("]}");
    if let Some(dir) = Path::new(&opt.out_path).parent()
        && !dir.as_os_str().is_empty()
    {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::write(&opt.out_path, &sj)
        .unwrap_or_else(|e| fail(&format!("{tag}: sidecar 写盘 {}: {e}", opt.out_path)));
    eprintln!("{tag}: 逐帧 cut sidecar → {}", opt.out_path);
}

// ---------------------------------------------------------------------------
// --selftest host 腿（零 device;合成 DAG 上核验 cut 单调细化/覆盖性/竞技场
// 增量写器/零面积折叠/双跑 host 序列确定性 + 两 kernel 结构自检）
// ---------------------------------------------------------------------------

/// 合成单块包：4 叶(层0,各 1 tri) + 2 组(层1,各 1 简化 tri) + 1 根(层2,1 tri)。
/// LodBounds 并集嵌套,error 单调（叶 0 < 组 0.05 < 根 0.2 < 父∞）。
#[allow(dead_code)]
fn frame_cut_selftest_block() -> ClusterPackBlock {
    let leaf_centers = [
        [-1.0f32, 0.0, 0.0],
        [-0.4, 0.0, 0.0],
        [0.4, 0.0, 0.0],
        [1.0, 0.0, 0.0],
    ];
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut triangle_indices: Vec<u8> = Vec::new();
    let mut records: Vec<ClusterRecord> = Vec::new();
    let mut push_tri = |c: [f32; 3], half: f32| -> (u32, u32) {
        let vo = vertices.len() as u32;
        vertices.push([c[0] - half, c[1] - half, c[2]]);
        vertices.push([c[0] + half, c[1] - half, c[2]]);
        vertices.push([c[0], c[1] + half, c[2]]);
        let to = triangle_indices.len() as u32;
        triangle_indices.extend_from_slice(&[0, 1, 2]);
        (vo, to)
    };
    let rec = |vo: u32, to: u32, err: f32, perr: f32, c: [f32; 3], r: f32| ClusterRecord {
        center: c,
        radius: r,
        cone_axis: [0.0, 0.0, 1.0],
        cone_cutoff: 1.0,
        error: err,
        parent_error: perr,
        vertex_offset: vo,
        triangle_offset: to,
        vertex_count: 3,
        triangle_count: 1,
        page_id: 0,
        reserved: 0,
    };
    // 簇 0..3 = 叶;4,5 = 组;6 = 根。
    for c in leaf_centers {
        let (vo, to) = push_tri(c, 0.3);
        records.push(rec(vo, to, 0.0, 0.05, c, 0.5));
    }
    for c in [[-0.7f32, 0.0, 0.0], [0.7, 0.0, 0.0]] {
        let (vo, to) = push_tri(c, 0.6);
        records.push(rec(vo, to, 0.05, 0.2, c, 1.1));
    }
    {
        let (vo, to) = push_tri([0.0, 0.0, 0.0], 1.2);
        records.push(rec(vo, to, 0.2, f32::INFINITY, [0.0, 0.0, 0.0], 2.0));
    }
    let nodes = vec![
        DagNodeRec { first_child: 0, child_count: 0, level: 0 },
        DagNodeRec { first_child: 0, child_count: 0, level: 0 },
        DagNodeRec { first_child: 0, child_count: 0, level: 0 },
        DagNodeRec { first_child: 0, child_count: 0, level: 0 },
        DagNodeRec { first_child: 0, child_count: 2, level: 1 },
        DagNodeRec { first_child: 2, child_count: 2, level: 1 },
        DagNodeRec { first_child: 4, child_count: 2, level: 2 },
    ];
    let children = vec![0u32, 1, 2, 3, 4, 5];
    let lb = |c: [f32; 3], r: f32| LodBounds { center: c, radius: r };
    let g0 = lb([-0.7, 0.0, 0.0], 1.1);
    let g1 = lb([0.7, 0.0, 0.0], 1.1);
    let root = lb([0.0, 0.0, 0.0], 2.0);
    let cluster_self_lod = vec![
        lb(leaf_centers[0], 0.5),
        lb(leaf_centers[1], 0.5),
        lb(leaf_centers[2], 0.5),
        lb(leaf_centers[3], 0.5),
        g0,
        g1,
        root,
    ];
    let cluster_parent_lod = vec![g0, g0, g1, g1, root, root, root];
    ClusterPackBlock {
        records,
        nodes,
        children,
        vertices,
        triangle_indices,
        leaf_source_tris: vec![0, 1, 2, 3],
        cluster_albedo: vec![[0.5; 3]; 7],
        cluster_emission: vec![[0.0; 3]; 7],
        cluster_mat: vec![0; 7],
        cluster_self_lod,
        cluster_parent_lod,
    }
}

/// host 自检腿（无 Vulkan;fail-closed 全断言,过则打印 PASS 由调用方收口）。
#[allow(dead_code)]
fn frame_cut_selftest(tag: &str) {
    let block = frame_cut_selftest_block();
    let blocks = std::slice::from_ref(&block);
    // ① 布局:槽基连续、passthrough 尾接、owner 二分闭环。
    let arena = frame_cut_arena_layout(blocks, 2);
    if arena.total_tris != 9 || arena.passthrough_base != 7 {
        fail(&format!(
            "{tag}: selftest 竞技场布局漂移 total={} pt_base={}",
            arena.total_tris, arena.passthrough_base
        ));
    }
    for prim in 0..7u32 {
        let Some((bi, ci)) = frame_cut_owner(&arena, prim) else {
            fail(&format!("{tag}: selftest owner({prim}) 意外 passthrough"));
        };
        if bi != 0 || ci != prim {
            fail(&format!("{tag}: selftest owner({prim}) = ({bi},{ci}) 漂移"));
        }
    }
    if frame_cut_owner(&arena, 7).is_some() || frame_cut_owner(&arena, 8).is_some() {
        fail(&format!("{tag}: selftest passthrough owner 判定漂移"));
    }
    // ② dolly 逼近 ⇒ cut 单调细化（根 → 组 → 叶）+ 逐帧覆盖性（select 内核验）。
    let spec = CameraSpec {
        eye: [0.0, 0.0, 600.0],
        forward: [0.0, 0.0, -1.0],
        up0: [0.0, 1.0, 0.0],
        fov_y_rad: 60f32.to_radians(),
        near: 0.1,
        far: 1000.0,
    };
    let frames = 12u32;
    let mut seq: Vec<u64> = Vec::new();
    let mut cuts: Vec<Vec<Vec<bool>>> = Vec::new();
    for k in 0..frames {
        let mut s = spec;
        // 对数逼近:600m → ~0.8m,扫过根→组(px≈1 @ d≈187)与组→叶(@ d≈47)
        // 两个翻转点（projected_error_px 标准公式手算;d ≤ ℓ 保守 +∞ 兜底近景）。
        s.eye[2] = 600.0 * (0.55f32).powi(k as i32);
        let (cut, _, tris) = frame_cut_select(tag, blocks, &s, 1920, 1080, 1.0, k);
        seq.push(tris);
        cuts.push(cut);
    }
    let nondec = seq.windows(2).all(|w| w[1] >= w[0]);
    if !nondec || seq.first() == seq.last() {
        fail(&format!(
            "{tag}: selftest cut_tris 逼近序列非单调细化 {seq:?}"
        ));
    }
    if *seq.first().unwrap() != 1 || *seq.last().unwrap() != 4 {
        fail(&format!(
            "{tag}: selftest cut 端点漂移（期望根 1 tri → 全叶 4 tri）{seq:?}"
        ));
    }
    // ③ 全量流/施加写器:全簇超集流全槽真几何;施加 cut 后非 cut 槽 =
    //    零面积折叠（全 0）,cut 槽与 passthrough 尾段不触。
    let pt_stream: Vec<f32> = (0..18).map(|i| 1.0 + i as f32 * 0.25).collect();
    let full = frame_cut_full_stream(blocks, &pt_stream, &arena);
    if full.len() != arena.total_tris * 9 {
        fail(&format!("{tag}: selftest 全量流长度漂移 {}", full.len()));
    }
    for slot in 0..7 {
        if full[slot * 9..(slot + 1) * 9].iter().all(|x| *x == 0.0) {
            fail(&format!("{tag}: selftest 全量流簇槽 {slot} 未写真几何"));
        }
    }
    if full[7 * 9..9 * 9].iter().all(|x| *x == 0.0) {
        fail(&format!("{tag}: selftest passthrough 尾段未写真几何"));
    }
    let mut applied0 = full.clone();
    frame_cut_apply_cut(&mut applied0, blocks, &arena, &cuts[0]);
    // 帧 0 = 远景根 cut:根槽(6)真几何在,叶/组槽零面积折叠,passthrough 不触。
    if applied0[6 * 9..7 * 9] != full[6 * 9..7 * 9] {
        fail(&format!("{tag}: selftest 施加后根槽漂移（cut 槽应不触）"));
    }
    if !applied0[0..6 * 9].iter().all(|x| *x == 0.0) {
        fail(&format!("{tag}: selftest 施加后非 cut 槽应零面积折叠"));
    }
    if applied0[7 * 9..9 * 9] != full[7 * 9..9 * 9] {
        fail(&format!("{tag}: selftest 施加后 passthrough 漂移（尾段应不触）"));
    }
    // ④ host 双跑确定性:全量流 + 施加流 + 光线流逐位复现。
    let full_b = frame_cut_full_stream(blocks, &pt_stream, &arena);
    let mut applied0_b = full_b.clone();
    frame_cut_apply_cut(&mut applied0_b, blocks, &arena, &cuts[0]);
    if sha256_hex(&bytes_f32(&full)) != sha256_hex(&bytes_f32(&full_b))
        || sha256_hex(&bytes_f32(&applied0)) != sha256_hex(&bytes_f32(&applied0_b))
    {
        fail(&format!("{tag}: selftest 竞技场流双跑漂移"));
    }
    let r1 = frame_cut_rays(&spec, 32, 18);
    let r2 = frame_cut_rays(&spec, 32, 18);
    if bytes_f32(&r1) != bytes_f32(&r2) || r1.len() != 32 * 18 * 8 {
        fail(&format!("{tag}: selftest 光线流双跑/长度漂移"));
    }
    // ⑤ 两 kernel 结构自检（magic + 入口名 = "main";device 腿归 GPU 验收窗）。
    for (name, words) in [
        ("clear", frame_cut_clear_spv()),
        ("rq", frame_cut_rq_spv()),
    ] {
        if words[0] != 0x0723_0203 {
            fail(&format!("{tag}: selftest {name} kernel magic 漂移"));
        }
        if vk::entry_point_name(&words).as_deref() != Some("main") {
            fail(&format!("{tag}: selftest {name} kernel 入口名漂移"));
        }
    }
    // ⑥ G38 T3:min-level 提升映射 + 降档布局 + 脏区段合并(host 纯函数面)。
    {
        use rurix_render::geometry::visible_cluster_set::{MeshDagView, verify_cut_coverage};
        let mp = frame_cut_min_parents(&block);
        // 父映射:叶 0,1→组 4;叶 2,3→组 5;组 4,5→根 6;根 = None。
        if mp != vec![Some(4), Some(4), Some(5), Some(5), Some(6), Some(6), None] {
            fail(&format!("{tag}: selftest min_parents 漂移 {mp:?}"));
        }
        let view = MeshDagView::new(&block.records, &block.nodes, &block.children)
            .unwrap_or_else(|e| fail(&format!("{tag}: selftest DAG 视图: {e}")));
        // 提升:全叶 cut → N=1 双组;混合 cut {0,1,5} → {4,5}(keep 成员 5 与
        // 提升祖先 4 共存,0/1 撤出);粗 cut 恒等;N=2 全部兜到根。
        for (cut_in, n, expect) in [
            (vec![0u32, 1, 2, 3], 1u32, vec![4u32, 5]),
            (vec![0, 1, 5], 1, vec![4, 5]),
            (vec![4, 5], 1, vec![4, 5]),
            (vec![6], 1, vec![6]),
            (vec![0, 1, 2, 3], 2, vec![6]),
        ] {
            let got = frame_cut_promote_min_level(&block, &mp, &cut_in, n);
            if got != expect {
                fail(&format!(
                    "{tag}: selftest 提升映射漂移 cut={cut_in:?} N={n} got={got:?} expect={expect:?}"
                ));
            }
            verify_cut_coverage(&view, &got).unwrap_or_else(|e| {
                fail(&format!(
                    "{tag}: selftest 提升后覆盖性 cut={cut_in:?} N={n}: {e}"
                ))
            });
        }
        // 降档布局:N=1 ⇒ 槽集 = {组 4,5, 根 6},叶槽哨兵,total = 3+2 pt。
        let mp_all = vec![mp.clone()];
        let arena1 = frame_cut_arena_layout_ext(blocks, 2, 1, &mp_all);
        if arena1.total_tris != 5 || arena1.passthrough_base != 3 {
            fail(&format!(
                "{tag}: selftest 降档布局漂移 total={} pt_base={}",
                arena1.total_tris, arena1.passthrough_base
            ));
        }
        for ci in 0..4 {
            if arena1.slot_base[0][ci] != FC_NO_SLOT {
                fail(&format!("{tag}: selftest 降档叶槽 {ci} 应为哨兵"));
            }
        }
        if arena1.slot_base[0][4] != 0 || arena1.slot_base[0][5] != 1 || arena1.slot_base[0][6] != 2
        {
            fail(&format!("{tag}: selftest 降档槽基漂移"));
        }
        if frame_cut_owner(&arena1, 0) != Some((0, 4))
            || frame_cut_owner(&arena1, 2) != Some((0, 6))
            || frame_cut_owner(&arena1, 3).is_some()
        {
            fail(&format!("{tag}: selftest 降档 owner 二分漂移"));
        }
        // 降档全量流/施加:哨兵槽不写;cut={4,5} 施加后根槽(2)折叠。
        let pt2: Vec<f32> = (0..18).map(|i| 2.0 + i as f32 * 0.5).collect();
        let full1 = frame_cut_full_stream(blocks, &pt2, &arena1);
        if full1.len() != 5 * 9 {
            fail(&format!("{tag}: selftest 降档全量流长度漂移 {}", full1.len()));
        }
        for slot in 0..3 {
            if full1[slot * 9..(slot + 1) * 9].iter().all(|x| *x == 0.0) {
                fail(&format!("{tag}: selftest 降档全量流槽 {slot} 未写真几何"));
            }
        }
        let mut cutset = vec![vec![false; 7]];
        cutset[0][4] = true;
        cutset[0][5] = true;
        let mut applied1 = full1.clone();
        frame_cut_apply_cut(&mut applied1, blocks, &arena1, &cutset);
        if applied1[0..2 * 9] != full1[0..2 * 9] {
            fail(&format!("{tag}: selftest 降档施加后 cut 槽漂移"));
        }
        if !applied1[2 * 9..3 * 9].iter().all(|x| *x == 0.0) {
            fail(&format!("{tag}: selftest 降档施加后根槽应零面积折叠"));
        }
        // 脏区段合并:相邻并段/间隙分段。
        let mut regions: Vec<(u64, u64)> = Vec::new();
        frame_cut_merge_region(&mut regions, 0, 36);
        frame_cut_merge_region(&mut regions, 36, 72);
        frame_cut_merge_region(&mut regions, 144, 36);
        if regions != vec![(0, 108), (144, 36)] {
            fail(&format!("{tag}: selftest 脏区段合并漂移 {regions:?}"));
        }
    }
    // ⑦ G39 T5:device 决策码对拍臂 host 纯函数面(表构造/sentinel 映射/
    //    params 三关中和式复算/期望码〔提升前口径〕/NoContraction 注入器/
    //    red-arm 受害裁决——GPU 对拍腿归 C1-C5 验收环)。
    {
        // 表构造:布局长度/真值照填/cone 零填+cutoff 中和/sentinel 映射。
        let (cf, lf) = frame_cut_device_tables(blocks);
        if cf.len() != 7 * 10 || lf.len() != 7 * 8 {
            fail(&format!(
                "{tag}: selftest device 表长度漂移 cluster={} lod={}",
                cf.len(),
                lf.len()
            ));
        }
        let r0 = &block.records[0];
        if cf[0..3] != r0.center[..] || cf[3] != r0.radius || cf[8] != r0.error {
            fail(&format!("{tag}: selftest device 表簇 0 真值面漂移"));
        }
        if cf[9] != r0.parent_error {
            fail(&format!("{tag}: selftest device 表簇 0 有限 parent_error 应原样透传"));
        }
        for g in 0..7 {
            let fb = g * 10;
            if cf[fb + 4] != 0.0 || cf[fb + 5] != 0.0 || cf[fb + 6] != 0.0 {
                fail(&format!("{tag}: selftest device 表簇 {g} cone_axis 未零填"));
            }
            if cf[fb + 7] != 1.0 {
                fail(&format!("{tag}: selftest device 表簇 {g} cone_cutoff 未中和(关 2)"));
            }
        }
        if cf[6 * 10 + 9] != 2.0e9 {
            fail(&format!(
                "{tag}: selftest 根 parent_error=+∞ sentinel 映射漂移(期望 2e9,得 {})",
                cf[6 * 10 + 9]
            ));
        }
        if lf[0..3] != block.cluster_self_lod[0].center[..]
            || lf[3] != block.cluster_self_lod[0].radius
            || lf[4..7] != block.cluster_parent_lod[0].center[..]
            || lf[7] != block.cluster_parent_lod[0].radius
        {
            fail(&format!("{tag}: selftest device lod 表平行直取漂移"));
        }
        // params 装配 + 三关中和式 host 复算(kernel 判式字面)。
        let p = frame_cut_device_params(&spec, 1920, 1080, 1.0, 7);
        if p[0..24].iter().any(|&x| x != 0.0) || p[36..64].iter().any(|&x| x != 0.0) {
            fail(&format!("{tag}: selftest params 关 1/4 域未零填"));
        }
        let cam_chk = cluster_cull_camera(&spec, 1920, 1080, 1.0);
        if p[24..27] != spec.eye[..]
            || p[27] != cam_chk.view_proj[1][1] * 1080.0 * 0.5
            || p[27] <= 0.0
        {
            fail(&format!("{tag}: selftest params 相机面漂移(eye/proj_factor)"));
        }
        if p[28] != 1.0
            || p[29] != 7.0
            || p[30] != 0.0
            || p[33] != 0.1
            || p[34] != 1.0
            || p[35] != 7.0
        {
            fail(&format!("{tag}: selftest params 标量面漂移"));
        }
        for g in 0..7 {
            let fb = g * 10;
            let (cx, cy, cz, r) = (cf[fb], cf[fb + 1], cf[fb + 2], cf[fb + 3]);
            for pl in 0..6 {
                // 关 1 kernel 判式字面:零平面 ⇒ 0 < −radius 恒假恒不剔。
                if p[pl * 4] * cx + p[pl * 4 + 1] * cy + p[pl * 4 + 2] * cz + p[pl * 4 + 3]
                    < 0.0 - r
                {
                    fail(&format!(
                        "{tag}: selftest 关 1 中和破坏(簇 {g} 平面 {pl} 剔除)"
                    ));
                }
            }
            // 关 4 kernel 判式字面:view 行全零 ⇒ viewz=0,near_z=−r<znear 短路。
            let viewz = -(p[60] * cx + p[61] * cy + p[62] * cz + p[63]);
            if viewz - r > p[33] {
                fail(&format!("{tag}: selftest 关 4 未短路(簇 {g} near_z>znear)"));
            }
        }
        // 期望码构造(提升前口径):远帧 cut={根 6},近帧 cut={叶 0..3}。
        let far = frame_cut_device_expected(&cuts[0]);
        if far != vec![2, 2, 2, 2, 2, 2, 4] {
            fail(&format!("{tag}: selftest 期望码(远帧)漂移 {far:?}"));
        }
        let near = frame_cut_device_expected(cuts.last().unwrap());
        if near != vec![4, 4, 4, 4, 2, 2, 2] {
            fail(&format!("{tag}: selftest 期望码(近帧)漂移 {near:?}"));
        }
        // NoContraction 注入器结构:合成流(1 decorate 锚 + 1 type +
        // FAdd/FSub/FMul 各 1)⇒ 首 annotation 前注 3 条,注入量 = 浮点乘加
        // 减指令数,原指令序逐字保持。
        let mut synth = vec![0x0723_0203u32, 0x0001_0400, 0, 64, 0];
        fc_spv_inst(&mut synth, 71, &[9, 0]); // OpDecorate(插入点锚)
        fc_spv_inst(&mut synth, 22, &[1, 32]); // OpTypeFloat %1
        fc_spv_inst(&mut synth, 129, &[1, 10, 2, 3]); // OpFAdd → %10
        fc_spv_inst(&mut synth, 131, &[1, 11, 2, 3]); // OpFSub → %11
        fc_spv_inst(&mut synth, 133, &[1, 12, 2, 3]); // OpFMul → %12
        let inj = fc_spv_inject_no_contraction(&synth);
        if inj.len() != synth.len() + 9 {
            fail(&format!(
                "{tag}: selftest NoContraction 注入量漂移 {}→{}",
                synth.len(),
                inj.len()
            ));
        }
        let expect_decos: Vec<u32> = [10u32, 11, 12]
            .iter()
            .flat_map(|&id| [(3 << 16) | 71, id, 42])
            .collect();
        if inj[5..14] != expect_decos[..] || inj[14..] != synth[5..] {
            fail(&format!("{tag}: selftest NoContraction 注入位置/形制漂移"));
        }
        // red-arm 受害裁决:近帧(全叶 cut,parent 有限)⇒ 模式甲簇 0
        // parent→−MAX;远帧(cut={根},parent=sentinel 不可翻)⇒ 甲空 →
        // 模式乙簇 0(期望 2;叶 self_e=0 免篡 + parent→+MAX)。
        let mut lf_t = lf.clone();
        let v_near = frame_cut_red_arm_tamper(&cf, &mut lf_t, &near, tag);
        if v_near != 0 || lf_t[7] != -f32::MAX || lf_t[3] != lf[3] {
            fail(&format!("{tag}: selftest red-arm 模式甲漂移 v={v_near}"));
        }
        let mut lf_t2 = lf.clone();
        let v_far = frame_cut_red_arm_tamper(&cf, &mut lf_t2, &far, tag);
        if v_far != 0 || lf_t2[7] != f32::MAX || lf_t2[3] != lf[3] {
            fail(&format!("{tag}: selftest red-arm 模式乙漂移 v={v_far}"));
        }
        // C1 首红裁决锚(REPORT §6):在树根簇合法编码 = **有限 f32::MAX**
        // (dag.rs「顶层(根)parent_error = f32::MAX」,bistro 生产包实证
        // 3.4028235e38;本夹具根 +∞ 锚的是非有限→2e9 映射路)——域检绿路
        // + 原样透传 + host 期望码/red-arm 裁决对两种根编码同判(双侧
        // parent 谓词同向饱和:kernel ≥1e9 分支 1e9px / host dsurf≤e 分支
        // +∞px,均 ≥ thr)。
        let mut block_max = frame_cut_selftest_block();
        block_max.records[6].parent_error = f32::MAX;
        let blocks_max = std::slice::from_ref(&block_max);
        let (cf2, lf2) = frame_cut_device_tables(blocks_max);
        if cf2[6 * 10 + 9] != f32::MAX {
            fail(&format!(
                "{tag}: selftest 根 f32::MAX 编码应原样透传(得 {:e})",
                cf2[6 * 10 + 9]
            ));
        }
        let (cut_max, _, _) = frame_cut_select(tag, blocks_max, &spec, 1920, 1080, 1.0, 0);
        let far2 = frame_cut_device_expected(&cut_max);
        if far2 != far {
            fail(&format!(
                "{tag}: selftest 根 f32::MAX 编码 host 期望码漂移 {far2:?}(应与 +∞ 根同判 {far:?})"
            ));
        }
        let mut lf2_t = lf2.clone();
        let v_max = frame_cut_red_arm_tamper(&cf2, &mut lf2_t, &far2, tag);
        if v_max != 0 || lf2_t[7] != f32::MAX || lf2_t[3] != lf2[3] {
            fail(&format!(
                "{tag}: selftest red-arm MAX 根裁决漂移 v={v_max}(甲应因饱和域跳根,乙应同 +∞ 根)"
            ));
        }
    }
    // ⑧ G40 T2(#77 P2):device 决策码消费链 host 纯函数面——决策码 →
    //    逐块布尔集(逆展平)→ verify/提升/计数与 host select 后链同判
    //    (决策语义等价的结构性自证:同一决策码经两条链产同一 cut 集;
    //    GPU 决策腿归 B1 验收环)。
    {
        // 远帧 cut={根 6} / 近帧 cut={叶 0..3}(⑦ 段期望码直接消费)。
        for (name, exp_codes, exp_cut) in [
            ("far", vec![2u32, 2, 2, 2, 2, 2, 4], &cuts[0]),
            ("near", vec![4, 4, 4, 4, 2, 2, 2], cuts.last().unwrap()),
        ] {
            let sets = frame_cut_sets_from_decisions(tag, blocks, &exp_codes, 0);
            if sets != *exp_cut {
                fail(&format!("{tag}: selftest ⑧ 决策码逆展平({name})漂移"));
            }
            let mut tmg = FrameCutSelectTiming::default();
            let (s2, ncl, ntri, ntri_p) =
                frame_cut_select_from_decisions(tag, blocks, &exp_codes, 0, 0, &[], &mut tmg);
            if s2 != *exp_cut || ntri != ntri_p {
                fail(&format!("{tag}: selftest ⑧ 决策消费链({name})ml0 漂移"));
            }
            let want_cl = exp_cut[0].iter().filter(|&&c| c).count() as u32;
            if ncl != want_cl {
                fail(&format!(
                    "{tag}: selftest ⑧ 簇计数({name})漂移 {ncl} ≠ {want_cl}"
                ));
            }
        }
        // ml=1 提升路:近帧决策(全叶)经消费链提升 ⇒ {组 4,5};提升前
        // tris = 叶和,提升后 = 组和(⑥ 段提升映射同锚)。
        let mp2 = frame_cut_min_parents(&block);
        let mp2_all = vec![mp2];
        let mut tmg1 = FrameCutSelectTiming::default();
        let (s_ml1, cl_ml1, tri_pre, tri_post) = frame_cut_select_from_decisions(
            tag,
            blocks,
            &[4, 4, 4, 4, 2, 2, 2],
            0,
            1,
            &mp2_all,
            &mut tmg1,
        );
        let mut want = vec![vec![false; 7]];
        want[0][4] = true;
        want[0][5] = true;
        if s_ml1 != want || cl_ml1 != 2 || tri_pre == tri_post {
            fail(&format!(
                "{tag}: selftest ⑧ ml1 决策消费链漂移 cl={cl_ml1} pre={tri_pre} post={tri_post}"
            ));
        }
    }
    eprintln!(
        "{tag}: selftest OK（布局/owner 二分/单调细化 {} 帧 {}→{} tri/增量写器/零面积折叠/双跑确定性/kernel 结构/min-level 提升+降档布局/脏区段合并/device 对拍臂 host 面〔表+sentinel 映射+f32::MAX 根编码透传+params 三关中和+期望码+NoContraction 注入器+red-arm 裁决〕/P2 决策消费链〔决策码逆展平+ml0/ml1 与 host select 后链同判〕,全 fail-closed 已过）",
        frames,
        seq.first().unwrap(),
        seq.last().unwrap(),
    );
}

