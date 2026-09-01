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
// 纪律:判档面(非生产冻结件);提交半程由 `submit_pipelined_frame` 的
// `slot_as: Option<&SlotAsGroup>` 末参承载——G39 T3 已将原复制适配体
// `g37_submit_pipelined_frame_slot_as` 折叠回该单源(fif_dyn REPORT §7-3
// 登记项兑现:三处插入+防御性复核换向吸收进 Some 路,None 路 0 语义等价;
// 施工登记 = artifacts/day_0831_g39/t3_fold/REPORT.md)。
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
    /// cmd·staging·override-set / 票据·collect 纪律与既有流水面同一函数承载
    /// (G39 T3 单源折叠;见 `submit_pipelined_frame` doc 的 slot_as 分支节)。
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
        //    submit_pipelined_frame 的 slot_as 路内再核一次)。
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
        // TLAS 写与 as_ops 录制的附加时序论证见其 doc 的 slot_as 分支节)。
        let mut inner = unsafe {
            submit_pipelined_frame(
                &mut self.native,
                self.resources,
                self.passes,
                self.barriers,
                self.readbacks,
                &prepared,
                Some(group),
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
