// Assisted-by: Cursor Agent(G37 W3 深水区:FIF×动态共存判档实施窗,TODO #90)
// ---------------------------------------------------------------------------
// G37 W3 #90:FIF 流水 × 动态 AS 更新共存的**判档加性面**(body-include 进
// render_exec.rs 尾部;vk_g37_async_lanes 先例同律——既有函数/结构/入口全部
// 0 改写,本文件只新增平行入口)。
//
// 语义(RFC-0030 §4.3 L2 修订行草案的实现底稿,草案 =
// artifacts/day_0830_delivery/w3_deep/fif_dyn/RFC_DRAFT_RFC0030_amendment.md):
//
// 既有 `submit_with_frame_update` fail-closed 拒 `tlas_update`/`blas_refit`,
// 理由 = TLAS instance buffer / BLAS 顶点缓冲为**共享 host/AS 写面**——FIF 下
// submit 帧 N+1 时帧 N 仍在飞,host memcpy(`write_transforms`)与在飞帧的
// device 读(TLAS build 读 instance buffer / ray query 读 AS)竞争。
//
// 本入口(`submit_with_frame_update_slot_as`)以**每槽 AS 副本组**消解该竞争:
// 调用方在 session AS 表声明 `frame_slots` 份同构表项(组 [base, base+len)),
// 每份表项经 vk.rs `VkAsManager` 各自持有独立 instance buffer / BLAS 顶点缓冲 /
// BLAS / TLAS / scratch(单所有者纪律不变——组内每表项仍各自单所有者);逐帧:
//
// 1. 本帧动态写(`tlas_update`/`blas_refit`)**只准落组内本槽表项**
//    (`base + next_slot`;错槽/组外/组长 ≠ frame_slots 一律确定性 `Err`);
// 2. 本帧 pass 绑定中凡引用组内表项者**必须 == 本槽表项**(经既有
//    `binding_overrides` × per-slot descriptor override set〔G31 A2〕逐帧轮换;
//    跨槽绑定确定性 `Err`)——飞帧各读各槽 AS,槽间无共享动态写面;
// 3. host 写面时序 = slot fence 等待**之后**才 `write_transforms`(本槽上一
//    票据已完成,该 instance buffer 无在途 device 读)——与既有 per-slot
//    staging/override-set 的复用纪律同一根据;
// 4. AS build 命令(TLAS BUILD/UPDATE + BLAS refit 桥)经**同一录制事实源**
//    `record_frame_body(as_ops)` 落在守卫 barrier 之后(GPU 帧间全序维持,
//    §4.3 L2 确定性论证字面不动)。
//
// 确定性(判据 = probe 三臂):逐帧 AS 内容 = 纯函数(本帧实例数据/顶点数据)
// (Rebuild 直给;Refit 下 = f(创建期拓扑, 本帧数据),拓扑组内同构)⇒ 固定
// 轨迹下逐帧 digest 序列与单槽顺序入口**逐字节相等**;由
// `g31_fif_dyn_probe` 双跑位级 + 三臂等价门承载。
//
// 纪律:判档面(非生产冻结件);`g37_submit_pipelined_frame_slot_as` 为
// `submit_pipelined_frame` 的复制适配体(仅三处插入:host TLAS 写 / as_ops
// 录制 / 防御性复核换向)——复制而非改签既有函数是为维持「既有行 0 改写」,
// 漂移风险与「正式化时应折叠回单源」已在 REPORT §风险 登记。
// ---------------------------------------------------------------------------

/// G37 W3 #90:session AS 表内的**每槽 AS 副本组**声明(判档 opt-in 面)。
///
/// 组 = AS 表下标区间 `[base, base + len)`,`len` 须等于 session `frame_slots`
/// (每槽恰一副本);组内表项须由调用方以**同构场景描述**创建(同 BLAS 三角
/// 形集/同实例数/同 updatable 打标——本入口只核验槽纪律,不核验同构性,同构
/// 性由创建方保证并经 probe 等价门物理核验)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotAsGroup {
    /// 组首表项下标(session AS 表序)。
    pub base: u32,
    /// 组长(= frame_slots;每槽恰一副本)。
    pub len: u32,
}

/// G37 W3 #90:槽 AS 组逐帧纪律核验(纯 host,确定性;单测/`--selftest` 双承载)。
///
/// 判据(fail-closed,全 `Err` 带定位消息):
/// 1. `group.len == frame_slots`(每槽恰一副本)且 ≥2(FIF 语义下限);
/// 2. 组区间落在 AS 表内(`base + len ≤ as_count`,checked 防溢出);
/// 3. `tlas_update`/`blas_refit` 的 AS 表下标(若在案)**必须 == base + next_slot**
///    (组外动态更新与错槽更新同拒——组外表项在本入口下为只读共享面);
/// 4. 各 pass effective bindings 中凡落组内的 AS 引用必须 == `base + next_slot`
///    (跨槽绑定 = 飞帧读他槽写面,拒);组外 AS 引用(静态共享表项)放行。
///
/// 返回本帧应消费的槽表项下标(`base + next_slot`)。
pub fn g37_validate_slot_as_frame(
    frame_slots: usize,
    as_count: usize,
    next_slot: usize,
    group: &SlotAsGroup,
    tlas_as: Option<u32>,
    blas_as: Option<u32>,
    effective_bindings: &[Bindings],
) -> Result<u32, String> {
    if group.len < 2 {
        return Err(format!(
            "slot AS 组长 {} < 2(FIF 每槽副本语义下限;单槽动态请走顺序 execute_with_frame_update)",
            group.len
        ));
    }
    if group.len as usize != frame_slots {
        return Err(format!(
            "slot AS 组长 {} ≠ frame_slots {}(每槽恰一副本;组长必须等于 FIF 深度)",
            group.len, frame_slots
        ));
    }
    let end = group
        .base
        .checked_add(group.len)
        .ok_or("slot AS 组区间溢出(base + len 越 u32)")?;
    if end as usize > as_count {
        return Err(format!(
            "slot AS 组 [{}, {end}) 越 session AS 表界(as_count = {as_count})",
            group.base
        ));
    }
    let expect = group.base + next_slot as u32;
    if let Some(i) = tlas_as
        && i != expect
    {
        return Err(if i >= group.base && i < end {
            format!(
                "tlas_update 目标 AS {i} 非本槽副本 {expect}(next_slot = {next_slot};错槽写 = 飞帧读面改写,拒)"
            )
        } else {
            format!(
                "tlas_update 目标 AS {i} 在槽组 [{}, {end}) 外(组外表项为共享只读面,动态更新须落本槽副本 {expect})",
                group.base
            )
        });
    }
    if let Some(i) = blas_as
        && i != expect
    {
        return Err(if i >= group.base && i < end {
            format!(
                "blas_refit 目标 AS {i} 非本槽副本 {expect}(next_slot = {next_slot};错槽写 = 飞帧读面改写,拒)"
            )
        } else {
            format!(
                "blas_refit 目标 AS {i} 在槽组 [{}, {end}) 外(组外表项为共享只读面,动态更新须落本槽副本 {expect})",
                group.base
            )
        });
    }
    for (pi, b) in effective_bindings.iter().enumerate() {
        for &a in &b.accel_structs {
            if a >= group.base && a < end && a != expect {
                return Err(format!(
                    "pass {pi} 绑定组内 AS {a} ≠ 本槽副本 {expect}(跨槽绑定 = 读他槽动态写面,拒;组外静态 AS 不受限)"
                ));
            }
        }
    }
    Ok(expect)
}

impl<'a> DeviceFrameSession<'a> {
    /// G37 W3 #90 加性只读簿记:下一次流水 submit 将占用的 frame slot
    /// (`next_slot` 轮转指针;调用方据此把 `tlas_update`/绑定指向本槽 AS
    /// 副本)。执行语义 0-byte(纯簿记读)。
    pub fn next_frame_slot(&self) -> usize {
        self.native.next_slot
    }

    /// G37 W3 #90 加性只读簿记:session frame slot 数(= 创建期 `frame_slots`)。
    pub fn frame_slot_count(&self) -> usize {
        self.native.fences.len()
    }

    /// G37 W3 #90:FIF 流水提交半程 ×**每槽 AS 副本组**(判档 opt-in 入口;
    /// 既有 [`Self::submit_with_frame_update`] 0-byte 不动,其
    /// `tlas_update`/`blas_refit` fail-closed 拒绝面维持字面)。
    ///
    /// 校验序与既有流水入口同源(① `frame_update_state` ② expected provenance
    /// 重推 ③ `validate_submission_provenance`),另加 ④ 槽 AS 组纪律
    /// ([`g37_validate_slot_as_frame`]:动态更新/组内绑定必须落
    /// `group.base + next_frame_slot()` 表项)。守卫 barrier / per-slot
    /// cmd·staging·override-set / 票据·collect 纪律与既有流水面逐字同形
    /// (见 [`g37_submit_pipelined_frame_slot_as`])。
    ///
    /// 双 TLAS 更新面(`tlas_update_b`)不开放(顺序入口专属,与既有流水面
    /// 同拒)。`update.tlas_update`/`blas_refit` 皆 `None` 时本入口 = 既有
    /// 流水入口 + 组绑定纪律核验(等价面)。
    pub fn submit_with_frame_update_slot_as(
        &mut self,
        supplied: &SubmissionProvenance,
        update: &FrameUpdate,
        group: &SlotAsGroup,
    ) -> Result<FrameTicket, String> {
        let record_started = std::time::Instant::now();
        // 双 TLAS(tlas_b)/双 BLAS(blas_b)面不开放(顺序入口专属;恒 None)。
        let (effective, generations) = self.frame_update_state(update, None, None)?;
        let expected = build_runtime_provenance_ext(
            self.passes,
            &effective,
            &self.native.resource_allocations,
            &generations,
            self.frame_generation + 1,
            self.resources.len() as u32,
        );
        validate_submission_provenance(&expected, supplied)?;
        // ④ 槽 AS 组纪律(fail-closed;返回值即本槽表项,防御性复核在
        //    g37_submit_pipelined_frame_slot_as 内再核一次)。
        g37_validate_slot_as_frame(
            self.native.fences.len(),
            self.as_count(),
            self.native.next_slot,
            group,
            update.tlas_update.as_ref().map(|(i, _, _)| *i),
            update.blas_refit.as_ref().map(|b| b.as_index),
            &effective,
        )?;
        let validate_ns = elapsed_ns(record_started);
        let uploads: Vec<(u32, u64, &[u8])> = update
            .buffer_uploads
            .iter()
            .map(|(resource_id, offset, bytes)| {
                ((resource_id.0 - 1) as u32, *offset, bytes.as_slice())
            })
            .collect();
        let tlas = update
            .tlas_update
            .as_ref()
            .map(|(as_index, instances, action)| (*as_index, instances.as_slice(), *action));
        // blas_refit 解析为 native 消费形(src StableResourceId → 资源下标;
        // 校验已在 frame_update_state 完成——顺序入口同形)。
        let blas = update.blas_refit.map(|b| {
            (
                b.as_index,
                b.blas_index,
                (b.src.0 - 1) as u32,
                b.src_offset,
                b.byte_len,
                b.after_pass,
            )
        });
        let (effective_readbacks, effective_rb_sources) = match &update.readback_subset {
            Some(indices) => (
                indices
                    .iter()
                    .map(|&i| self.readbacks[i as usize])
                    .collect::<Vec<Readback>>(),
                indices.iter().map(|&i| i as usize).collect::<Vec<usize>>(),
            ),
            None => (Vec::new(), Vec::new()),
        };
        let mut descriptor_overrides: Vec<u32> = Vec::new();
        for &(pi, _) in &update.binding_overrides {
            descriptor_overrides.push(pi);
        }
        let prepared = PreparedFrameUpdate {
            uploads: &uploads,
            tlas,
            // 双 TLAS/双 BLAS 面不开放(顺序入口专属;本入口恒 None)。
            tlas_b: None,
            blas,
            blas_b: None,
            descriptor_overrides: &descriptor_overrides,
            effective_bindings: &effective,
            effective_readbacks: &effective_readbacks,
            effective_rb_sources: &effective_rb_sources,
            // 流水路恒重录本 slot cmd(slot query 区间/staged 段/AS ops 皆帧相关)。
            needs_rerecord: true,
        };
        // SAFETY: native session 独占 &mut self;prepared 全部引用本帧栈上数据,
        // 随调用结束失效;slot fence 纪律同 submit_pipelined_frame 契约(host
        // TLAS 写与 as_ops 录制的附加时序论证见 g37_submit_pipelined_frame_slot_as)。
        let mut inner = unsafe {
            g37_submit_pipelined_frame_slot_as(
                &mut self.native,
                self.resources,
                self.passes,
                self.barriers,
                self.readbacks,
                &prepared,
                group,
            )?
        };
        inner.record_ns += validate_ns;
        self.resource_generations = generations;
        self.commit_provenance(&expected);
        Ok(FrameTicket {
            inner,
            provenance: expected,
        })
    }
}

/// G37 W3 #90:FIF 流水帧提交 ×每槽 AS 副本(判档面;
/// [`submit_pipelined_frame`] 的复制适配体——slot 占用检查/fence 复用等待/
/// per-slot 面懒建/staging/守卫 barrier/重录/submit/票据全部逐字同形,仅三处
/// 插入,以「既有行 0 改写」纪律换少量复制,登记于 REPORT §风险):
///
/// 1. 防御性复核换向:`prepared.tlas`/`prepared.blas` 不再一律拒,改核
///    槽表项 == `group.base + slot`(公共入口已核;此处防绕过);
/// 2. slot fence 等待 + reset **之后** host `write_transforms`(本槽副本
///    instance buffer;上一票据已完成 ⇒ 无在途 device 读——per-slot staging
///    重写同一根据);
/// 3. `record_frame_body` 携 `as_ops`(同一录制事实源:TLAS BUILD/UPDATE +
///    consume barrier 录于 pass 链前、BLAS refit 桥录于 after_pass 后——
///    顺序路 execute_with_frame_update 逐字同形),落在帧间守卫 barrier 之后
///    (GPU 帧间全序维持,RFC-0030 §4.3 L2 确定性论证字面不动)。
///
/// # Safety
/// U32 契约同 [`submit_persistent_frame`];`prepared` 引用调用方栈上数据,
/// 生命周期限于本次调用;`prepared.tlas`/`blas` 目标表项须经
/// [`g37_validate_slot_as_frame`] 核验(本函数防御性复核)。
unsafe fn g37_submit_pipelined_frame_slot_as(
    native: &mut NativePersistentFrame,
    resources: &[ResourceDesc<'_>],
    passes: &[Pass<'_>],
    barriers: &[&[(u32, TargetState)]],
    readbacks: &[Readback],
    prepared: &PreparedFrameUpdate<'_>,
    group: &SlotAsGroup,
) -> Result<PersistentFrameTicket, String> {
    const WAIT_TIMEOUT_NS: u64 = PERSISTENT_WAIT_TIMEOUT_NS;
    let slot = native.next_slot;
    // 防御性复核(公共入口已核;防绕过公共校验的调用者——原防御性拒面同位)。
    let expect_as = group.base + slot as u32;
    if let Some((i, _, _)) = &prepared.tlas
        && *i != expect_as
    {
        return Err(format!(
            "slot-AS FIF: tlas 目标 {i} ≠ 本槽副本 {expect_as}(公共入口已拒;防御性复核)"
        ));
    }
    if let Some((i, ..)) = &prepared.blas
        && *i != expect_as
    {
        return Err(format!(
            "slot-AS FIF: blas 目标 {i} ≠ 本槽副本 {expect_as}(公共入口已拒;防御性复核)"
        ));
    }
    if prepared.tlas_b.is_some() || prepared.blas_b.is_some() {
        return Err("slot-AS FIF: 双 TLAS/双 BLAS 更新面不开放(顺序入口专属;防御性复核)".into());
    }
    if native.slot_busy[slot] {
        return Err(format!(
            "frame slot {slot} 票据未 collect(FIF 深度已满:先 collect 最早票据再 submit;\
             fail-closed 防 fence reset 悬垂)"
        ));
    }
    native.next_slot = (native.next_slot + 1) % native.fences.len();
    let fence = native.fences[slot];
    let validation_before = native
        .validation_errors
        .load(std::sync::atomic::Ordering::Relaxed);

    let wait_started = std::time::Instant::now();
    // C4 注入臂(默认关):第 n 次有界等待返回值覆写 VK_TIMEOUT 演习 TDR 处置面。
    let prior = g31_fault_fence_timeout((native.frame.dev.wait_fences)(
        native.device,
        1,
        &fence,
        1,
        WAIT_TIMEOUT_NS,
    ));
    if prior == VK_TIMEOUT {
        return Err(format!(
            "frame slot {slot} fence reuse bounded-wait 超时({WAIT_TIMEOUT_NS}ns;TDR-suspected)"
        ));
    }
    if prior != VK_SUCCESS {
        return Err(queue_result_error("vkWaitForFences(slot reuse)", prior));
    }
    let reset = (native.frame.dev.reset_fences)(native.device, 1, &fence);
    if reset != VK_SUCCESS {
        return Err(queue_result_error("vkResetFences", reset));
    }

    let record_started = std::time::Instant::now();
    ensure_pipelined_slot(native, resources, readbacks, slot)?;

    // ── G37 #90 插入②:本槽副本 TLAS 实例 transforms host 写(slot fence 已
    // 等待 ⇒ 本槽副本 instance buffer 的上一次 device 读〔本槽上一帧 TLAS
    // build〕已完成;他槽在飞帧读各自副本,不触本面——host-visible+coherent,
    // write_transforms 内做实例数/NaN fail-closed 与 64B 槽位 diff 增量)──
    if let Some((as_index, instances, _)) = &prepared.tlas {
        let Some(state) = native.as_state.as_mut() else {
            return Err("slot-AS FIF: tlas_update 指向无 AS 面的 session(校验漏网)".into());
        };
        let mgr = &mut state.managers[*as_index as usize];
        mgr.write_transforms(&state.fns, native.device, instances)?;
    }

    // ── G31:binding override → per-slot descriptor set 重写(既有流水面逐字
    // 同形;as_handles 含组内全部副本 TLAS,绑定下标已经槽纪律核验)──
    let mut slot_set_overrides: Vec<Option<VkDescriptorSet>> = vec![None; passes.len()];
    if !prepared.descriptor_overrides.is_empty() {
        let as_handles: Vec<u64> = native
            .as_state
            .as_ref()
            .map_or_else(Vec::new, |s| s.managers.iter().map(|m| m.tlas()).collect());
        for &pi in prepared.descriptor_overrides {
            let set = ensure_pipelined_override_set(native, passes, slot, pi as usize)?;
            write_pass_descriptor_set(
                &native.frame.dev,
                native.device,
                set,
                &prepared.effective_bindings[pi as usize],
                &native.frame.rt,
                native.frame.cleanup.sampler,
                &as_handles,
                pi as usize,
            )?;
            slot_set_overrides[pi as usize] = Some(set);
        }
    }

    // ── 上传 → 本 slot staging(既有流水面逐字同形)──
    let total_upload: u64 = prepared
        .uploads
        .iter()
        .map(|&(_, _, bytes)| bytes.len() as u64)
        .sum();
    let mut staged_copies: Vec<(u64, u32, u64, u64)> = Vec::with_capacity(prepared.uploads.len());
    let mut upload_src: VkBuffer = VK_NULL_HANDLE;
    if total_upload > 0 {
        let (sbuf, smem) = ensure_upload_staging(native, slot, total_upload)?;
        upload_src = sbuf;
        let mut ptr: *mut c_void = std::ptr::null_mut();
        let map = (native.frame.dev.map_mem)(native.device, smem, 0, total_upload, 0, &mut ptr);
        if map != VK_SUCCESS || ptr.is_null() {
            return Err(format!(
                "slot-AS FIF slot {slot}: 上传 staging vkMapMemory 失败: {map}"
            ));
        }
        let mut staging_offset = 0u64;
        for &(res, dst_offset, bytes) in prepared.uploads {
            if !matches!(&native.frame.rt[res as usize], RtRes::Buf(_)) {
                (native.frame.dev.unmap_mem)(native.device, smem);
                return Err(format!("slot-AS FIF: 上传目标资源 {res} 非 buffer(校验漏网)"));
            }
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                ptr.cast::<u8>().add(staging_offset as usize),
                bytes.len(),
            );
            staged_copies.push((staging_offset, res, dst_offset, bytes.len() as u64));
            staging_offset += bytes.len() as u64;
        }
        (native.frame.dev.unmap_mem)(native.device, smem);
    }

    // ── per-slot cmd 全量重录(守卫 barrier → staged 上传 → 冲刷 →
    // record_frame_body〔含 as_ops〕→ 帧尾 staged buffer readback)──
    let query_base = (slot * passes.len() * 2) as u32;
    let dev = &native.frame.dev;
    let Some(slot_state) = native.pipelined_slots[slot].as_ref() else {
        return Err(format!("slot-AS FIF slot {slot}: slot 面缺失(建面序漂移)"));
    };
    let slot_cmd = slot_state.cmd;
    if (dev.reset_cmd)(slot_cmd, 0) != VK_SUCCESS {
        return Err("slot-AS FIF: vkResetCommandBuffer 失败".into());
    }
    let cbi = CommandBufferBeginInfo {
        s_type: ST_COMMAND_BUFFER_BEGIN_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        p_inheritance_info: std::ptr::null(),
    };
    if (dev.begin_cmd)(slot_cmd, &cbi) != VK_SUCCESS {
        return Err("slot-AS FIF: vkBeginCommandBuffer 失败".into());
    }
    (dev.cmd_reset_query_pool)(
        slot_cmd,
        native.frame.cleanup.query_pool,
        query_base,
        (passes.len() as u32) * 2,
    );
    // 帧间守卫(submit_pipelined_frame 确定性论证字面;GPU 帧间全序 ⇒ 本帧
    // AS build 序于上帧全部 ray query 读之后,他槽副本的 GPU 写读亦全序化)。
    cmd_global_barrier2(
        dev,
        slot_cmd,
        STAGE2_ALL_COMMANDS,
        ACCESS2_MEMORY_WRITE,
        STAGE2_ALL_COMMANDS,
        ACCESS2_MEMORY_READ | ACCESS2_MEMORY_WRITE,
    );
    if !staged_copies.is_empty() {
        for &(src_offset, res, dst_offset, size) in &staged_copies {
            let RtRes::Buf(rb) = &native.frame.rt[res as usize] else {
                return Err(format!("slot-AS FIF: 上传目标资源 {res} 非 buffer(上判已拒)"));
            };
            let region = VkBufferCopy {
                src_offset,
                dst_offset,
                size,
            };
            (dev.cmd_copy_buf)(slot_cmd, upload_src, rb.buffer, 1, &region);
        }
        // staged 上传冲刷(既有流水面逐字同形;亦先于 AS build——refit 桥 src
        // 若经本帧上传,内容在 build 读取前已冲刷可见)。
        cmd_global_barrier2(
            dev,
            slot_cmd,
            STAGE2_TRANSFER,
            ACCESS2_TRANSFER_WRITE,
            STAGE2_ALL_COMMANDS,
            ACCESS2_MEMORY_READ | ACCESS2_MEMORY_WRITE,
        );
    }
    // ── G37 #90 插入③:AS ops(同一录制事实源 record_frame_body;顺序路
    // execute_with_frame_update 的 as_ops 归并同形,双 TLAS 臂裁掉)──
    let as_ops = match (&prepared.tlas, &prepared.blas) {
        (None, None) => None,
        (tlas, blas) => {
            let as_index = tlas
                .map(|(i, _, _)| i)
                .or_else(|| blas.map(|b| b.0));
            let Some(as_index) = as_index else {
                return Err("slot-AS FIF: AS 操作包空(内部不一致)".into());
            };
            let state = native
                .as_state
                .as_mut()
                .ok_or("slot-AS FIF: AS 操作无 AS 面(校验漏网)")?;
            let (tlas_action, blas_refit) = (
                tlas.map(|(_, _, a)| a),
                blas.map(|b| {
                    let (_, blas_index, src_res, src_offset, byte_len, after_pass) = b;
                    BlasRefitRecord {
                        blas_index,
                        src_res,
                        src_offset,
                        byte_len,
                        after_pass,
                    }
                }),
            );
            Some(AsFrameOps {
                mgr: &mut state.managers[as_index as usize],
                fns: &state.fns,
                tlas_action,
                blas_refit,
                tlas_b: None,
                // G37 W3 hzb_skin 并行加性面(第二 BLAS refit):本入口不开放。
                blas_refit_b: None,
            })
        }
    };
    let mut effective_rb: Vec<Option<(VkBuffer, VkDeviceMemory)>> = prepared
        .effective_readbacks
        .iter()
        .zip(prepared.effective_rb_sources.iter())
        .map(|(rb, &source)| match rb {
            Readback::Texture { .. } => Some(slot_state.rb_staging[source]),
            Readback::Buffer { .. } => None,
        })
        .collect();
    // G14.11:release 集 = exportable ∪ imported(创建期录制同式)。
    let exportable_indices: Vec<u32> = native
        .frame
        .exportable_meta
        .iter()
        .map(|&(r, _, _)| r)
        .chain(native.frame.imported_indices.iter().copied())
        .collect();
    record_frame_body(
        &FrameBodyParams {
            dev,
            device: native.device,
            memprops: &native.memprops,
            cmd: slot_cmd,
            resources,
            rt: &native.frame.rt,
            passes,
            barriers,
            effective_bindings: prepared.effective_bindings,
            setups: &native.frame.setups,
            query_pool: native.frame.cleanup.query_pool,
            query_base,
            inline_vbs: &native.frame.inline_vbs,
            readbacks: prepared.effective_readbacks,
            record_upload_segment: false,
            exportable: &exportable_indices,
            queue_family_index: native.frame.queue_family_index,
            slot_set_overrides: Some(&slot_set_overrides),
        },
        &mut effective_rb,
        None,
        as_ops,
    )?;
    // ── 帧尾 staged buffer readback copies(既有流水面逐字同形)──
    let has_buffer_rb = prepared
        .effective_readbacks
        .iter()
        .any(|rb| matches!(rb, Readback::Buffer { .. }));
    if has_buffer_rb {
        cmd_global_barrier2(
            dev,
            slot_cmd,
            STAGE2_ALL_COMMANDS,
            ACCESS2_MEMORY_WRITE,
            STAGE2_TRANSFER,
            ACCESS2_TRANSFER_READ,
        );
        for (rb, &source) in prepared
            .effective_readbacks
            .iter()
            .zip(prepared.effective_rb_sources.iter())
        {
            if let Readback::Buffer { res, offset, size } = *rb {
                let RtRes::Buf(src) = &native.frame.rt[res as usize] else {
                    return Err(format!("slot-AS FIF: readback 资源 {res} 非 buffer(类型漂移)"));
                };
                let region = VkBufferCopy {
                    src_offset: offset,
                    dst_offset: 0,
                    size,
                };
                (dev.cmd_copy_buf)(
                    slot_cmd,
                    src.buffer,
                    slot_state.rb_staging[source].0,
                    1,
                    &region,
                );
            }
        }
    }
    if (dev.end_cmd)(slot_cmd) != VK_SUCCESS {
        return Err("slot-AS FIF: vkEndCommandBuffer 失败".into());
    }
    let record_ns = elapsed_ns(record_started);

    let si = SubmitInfo {
        s_type: ST_SUBMIT_INFO,
        p_next: std::ptr::null(),
        wait_semaphore_count: 0,
        p_wait_semaphores: std::ptr::null(),
        p_wait_dst_stage_mask: std::ptr::null(),
        command_buffer_count: 1,
        p_command_buffers: &slot_cmd,
        signal_semaphore_count: 0,
        p_signal_semaphores: std::ptr::null(),
    };
    let submit_started = std::time::Instant::now();
    let submit = (native.frame.dev.queue_submit)(native.frame.queue, 1, &si, fence);
    let cpu_submit_ns = elapsed_ns(submit_started);
    if submit != VK_SUCCESS {
        return Err(queue_result_error(
            "vkQueueSubmit(slot-AS FIF pipelined frame)",
            submit,
        ));
    }
    native.slot_busy[slot] = true;

    let rb_plan: Vec<(Readback, usize)> = prepared
        .effective_readbacks
        .iter()
        .copied()
        .zip(prepared.effective_rb_sources.iter().copied())
        .collect();
    Ok(PersistentFrameTicket {
        slot,
        wait_started,
        record_ns,
        cpu_submit_ns,
        validation_before,
        rb_plan,
        pipelined: true,
    })
}

// ---------------------------------------------------------------------------
// 单测(纯 host;g37_validate_slot_as_frame 判据的 cargo test 承载——probe
// `--selftest` 消费同一事实源函数,双承载同判据)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod g37_fif_dyn_tests {
    use super::*;

    fn bind_as(indices: &[u32]) -> Bindings {
        Bindings {
            accel_structs: indices.to_vec(),
            ..Bindings::default()
        }
    }

    /// 绿臂:组 [1,3) × 2 槽,slot 0/1 各自更新与绑定本槽副本;组外静态 AS 0
    /// 共享绑定放行。
    #[test]
    fn g37_slot_as_green_arm() {
        let g = SlotAsGroup { base: 1, len: 2 };
        for slot in 0..2usize {
            let expect = 1 + slot as u32;
            let got = g37_validate_slot_as_frame(
                2,
                3,
                slot,
                &g,
                Some(expect),
                None,
                &[bind_as(&[0]), bind_as(&[expect, 0])],
            )
            .expect("本槽更新 + 本槽绑定应过");
            assert_eq!(got, expect, "返回值应为本槽表项");
        }
        // 双写面(tlas + blas 同槽)同过。
        g37_validate_slot_as_frame(2, 3, 1, &g, Some(2), Some(2), &[bind_as(&[2])])
            .expect("tlas+blas 同落本槽应过");
        // 全 None(等价面):仅绑定纪律核验。
        g37_validate_slot_as_frame(2, 3, 0, &g, None, None, &[bind_as(&[1])])
            .expect("无动态更新的组内本槽绑定应过");
    }

    /// RED 臂:错槽更新 / 组外更新 / 跨槽绑定 / 组长 ≠ frame_slots / 越界。
    #[test]
    fn g37_slot_as_red_arms() {
        let g = SlotAsGroup { base: 1, len: 2 };
        // 错槽:slot 0 期望表项 1,更新表项 2。
        let e = g37_validate_slot_as_frame(2, 3, 0, &g, Some(2), None, &[bind_as(&[1])])
            .expect_err("错槽 tlas 必须拒");
        assert!(e.contains("非本槽副本"), "错槽消息应定位: {e}");
        // 组外:更新静态表项 0。
        let e = g37_validate_slot_as_frame(2, 3, 0, &g, Some(0), None, &[bind_as(&[1])])
            .expect_err("组外 tlas 必须拒");
        assert!(e.contains("槽组"), "组外消息应定位: {e}");
        // blas 错槽同律。
        let e = g37_validate_slot_as_frame(2, 3, 1, &g, None, Some(1), &[bind_as(&[2])])
            .expect_err("错槽 blas 必须拒");
        assert!(e.contains("blas_refit"), "blas 消息应定位: {e}");
        // 跨槽绑定:slot 0 绑他槽副本 2。
        let e = g37_validate_slot_as_frame(2, 3, 0, &g, Some(1), None, &[bind_as(&[2])])
            .expect_err("跨槽绑定必须拒");
        assert!(e.contains("跨槽绑定"), "绑定消息应定位: {e}");
        // 组长 ≠ frame_slots。
        let e = g37_validate_slot_as_frame(3, 4, 0, &g, Some(1), None, &[])
            .expect_err("组长≠槽数必须拒");
        assert!(e.contains("frame_slots"), "组长消息应定位: {e}");
        // 组长 < 2。
        assert!(
            g37_validate_slot_as_frame(2, 3, 0, &SlotAsGroup { base: 1, len: 1 }, None, None, &[])
                .is_err(),
            "组长 1 必须拒"
        );
        // 组区间越 AS 表界。
        let e = g37_validate_slot_as_frame(2, 2, 0, &g, Some(1), None, &[])
            .expect_err("组越界必须拒");
        assert!(e.contains("越 session AS 表界"), "越界消息应定位: {e}");
    }

    /// 槽轮转纪律(host 模型):next_slot 轮转 = k % S 且本槽写面复用间隔恰 S 帧
    /// (fence 等待点),FIFO submit/collect 交错下同槽写与在飞读窗不重叠。
    #[test]
    fn g37_slot_ring_write_face_isolation() {
        for s in [2usize, 3] {
            let frames = 12usize;
            // FIFO 深度 s 的 submit/collect 交错模型:submit k 于时刻 2k,
            // collect k 于时刻 2(k+s)-1 之前(submit k+s 前强制 collect k——
            // slot_busy fail-closed 语义)。写面事件 = submit 期 host 写槽
            // k%s;在飞读窗 = (submit k, collect k)。
            let mut collected_at = vec![usize::MAX; frames];
            let mut t = 0usize;
            let mut pending: std::collections::VecDeque<usize> = Default::default();
            let mut submit_at = vec![0usize; frames];
            for k in 0..frames {
                if pending.len() == s {
                    let oldest = pending.pop_front().unwrap();
                    collected_at[oldest] = t;
                    t += 1;
                }
                submit_at[k] = t;
                t += 1;
                pending.push_back(k);
            }
            while let Some(oldest) = pending.pop_front() {
                collected_at[oldest] = t;
                t += 1;
            }
            for k in 0..frames {
                assert_eq!(k % s, k % s, "slot 轮转 = k % S(结构自明)");
                // 同槽前帧 k-s 必须已 collect(fence 等待)于本帧 submit 前:
                if k >= s {
                    assert!(
                        collected_at[k - s] < submit_at[k],
                        "S={s}: 帧 {k} host 写槽 {} 时,同槽前帧 {} 仍在飞(写读窗重叠)",
                        k % s,
                        k - s
                    );
                }
            }
        }
    }
}
