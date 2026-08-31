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
// - 边界（诚实登记）：cut 仍为 host 金标准（device cut kernel 链归 #77 生产
//   接线自身;本臂判「逐帧 AS 更新」通路）;单槽 inflight（FIF 流水面拒
//   tlas_update/blas_refit——A2/B5 既有约束,FIF×每槽 AS 归 #90 RFC）;
//   presented 面 0-byte（窗口合入 = 循环后证据臂,出帧翻转归 #77 全量）。

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
#[derive(Clone, Copy)]
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
}

#[allow(dead_code)]
impl FrameCutArmExtOpt {
    /// 既有入口默认:incr copy(窗口臂自动受益,digest 与 full 位级等价)+
    /// 无降档。
    fn default_ext() -> Self {
        Self {
            copy_full: false,
            min_level: 0,
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
    digest: String,
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
    let (sets, cut_clusters, cut_tris, _) =
        frame_cut_select_ext(tag, blocks, spec, in_w, in_h, threshold_px, frame, 0, &[]);
    (sets, cut_clusters, cut_tris)
}

/// G38 T3:select + min-level 提升(min_level=0 = 既有 `frame_cut_select`
/// 逐字等价)。生产金标准链不动:`select_lod_cut_grouped`(原 DAG 原样)→
/// `verify_cut_coverage`(原 cut)→ [`frame_cut_promote_min_level`] →
/// `verify_cut_coverage`(提升后,同一生产校验,fail-closed)。返回
/// (提升后逐块布尔集, 提升后簇数, **提升前** cut 三角数〔LOD 判据面,单调门
/// 消费〕, 提升后 cut 三角数〔竞技场施加口径〕)。
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
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
) -> (Vec<Vec<bool>>, u32, u64, u64) {
    use rurix_render::geometry::gpu_scene::IDENTITY_3X4;
    use rurix_render::geometry::visible_cluster_set::{
        MeshDagView, select_lod_cut_grouped, verify_cut_coverage,
    };
    let cam = cluster_cull_camera(spec, in_w, in_h, threshold_px);
    let mut sets = Vec::with_capacity(blocks.len());
    let mut cut_clusters = 0u32;
    let mut cut_tris = 0u64;
    let mut cut_tris_promoted = 0u64;
    for (bi, b) in blocks.iter().enumerate() {
        let view = MeshDagView::new(&b.records, &b.nodes, &b.children)
            .unwrap_or_else(|e| fail(&format!("{tag}: 帧 {frame} 块 {bi} DAG 拓扑: {e}")));
        let cut = select_lod_cut_grouped(
            &view,
            &b.cluster_self_lod,
            &b.cluster_parent_lod,
            &IDENTITY_3X4,
            &cam,
        );
        verify_cut_coverage(&view, &cut)
            .unwrap_or_else(|e| fail(&format!("{tag}: 帧 {frame} 块 {bi} cut 覆盖性: {e}")));
        for &c in &cut {
            cut_tris += u64::from(b.records[c as usize].triangle_count);
        }
        // min-level 提升(0 = 恒等)+ 提升后生产校验复核(fail-closed)。
        let cut = if min_level == 0 {
            cut
        } else {
            let promoted =
                frame_cut_promote_min_level(b, &min_parents_all[bi], &cut, min_level);
            verify_cut_coverage(&view, &promoted).unwrap_or_else(|e| {
                fail(&format!(
                    "{tag}: 帧 {frame} 块 {bi} min-level 提升后覆盖性: {e}(fail-closed)"
                ))
            });
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

    // 帧 0 cut 先行（初始竞技场 = 帧 0 已施加;帧 0 refit 桥 = 内容恒等,节拍均匀）。
    let s0 = &samples[0];
    let t0 = std::time::Instant::now();
    let (cut0, cut0_clusters, cut0_tris, cut0_tris_promoted) = frame_cut_select_ext(
        tag,
        blocks,
        &s0.spec,
        s0.in_w,
        s0.in_h,
        threshold_px,
        s0.frame,
        ext.min_level,
        min_parents_all,
    );
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
        // ── ① host cut（帧 0 复用先行结果;逐帧覆盖性机核已在 select 内）──
        let t_cut = std::time::Instant::now();
        let (cut, cut_clusters, cut_tris, cut_tris_promoted) = if k == 0 {
            (cut0.clone(), cut0_clusters, cut0_tris, cut0_tris_promoted)
        } else {
            frame_cut_select_ext(
                tag,
                blocks,
                &s.spec,
                s.in_w,
                s.in_h,
                threshold_px,
                s.frame,
                ext.min_level,
                min_parents_all,
            )
        };
        let cut_ms = if k == 0 {
            cut0_ms
        } else {
            t_cut.elapsed().as_secs_f64() * 1e3
        };

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
    let mut sj = String::with_capacity(4096 + stats.len() * 320);
    sj.push_str(&format!(
        "{{\"schema\":\"rurix.g31.frame_cut_probe.v1\",\"threshold_px\":{},\"res\":\"{}x{}\",\"frames\":{},\"step_m\":{},\"cut_every\":{},\"blocks\":{},\"blocks_limit\":{},\"total_clusters\":{},\"passthrough_tris\":{},\"refit_copy_mode\":\"{}\",\"min_level\":{},\"determinism_note\":\"固定轨迹+固定重建节拍+canonical 竞技场 ⇒ digest 序列同设备双跑位级(本跑已核);跨设备不作 golden(RT 遍历 tie-break 依设备)。cut = host 金标准(device cut kernel 归 #77);单槽 inflight(FIF 拒 refit,#89/#90 分界)\",\"frames_data\":[",
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
            "{{\"frame\":{},\"cut_clusters\":{},\"cut_tris\":{},\"refit\":{},\"changed_slots\":{},\"upload_bytes\":{},\"hits\":{},\"cut_ms\":{:.3},\"delta_ms\":{:.3},\"exec_ms\":{:.3},\"gpu_clear_ms\":{:.3},\"gpu_rq_ms\":{:.3},\"fence_ms\":{:.3},\"cut_tris_promoted\":{},\"copy_regions\":{},\"copy_bytes\":{},\"bridge_copy_gpu_ms\":{},\"bridge_build_gpu_ms\":{},\"digest\":{}}}",
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
    eprintln!(
        "{tag}: selftest OK（布局/owner 二分/单调细化 {} 帧 {}→{} tri/增量写器/零面积折叠/双跑确定性/kernel 结构/min-level 提升+降档布局/脏区段合并,全 fail-closed 已过）",
        frames,
        seq.first().unwrap(),
        seq.last().unwrap(),
    );
}

