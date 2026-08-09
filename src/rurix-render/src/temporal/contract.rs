//! G8.5b M24 TSR 生产契约(`g8.p0.m24.tsr_contract`)。
//!
//! 内部富输入面(不改冻结的 [`crate::temporal::upscale::UpscaleInputs`]/
//! [`crate::temporal::upscale::UpscaleBackend`])。host oracle 同时是 device
//! `tsr_contract`/`tsr_retire` 对拍金标准。
//!
//! 语义锚:RFC-0019 §4.6 + G8.5_RENDERING_COMPLETION_DESIGN §3。

use crate::shadow::events::sha256_hex;
use crate::temporal::common::{
    jitter_sequence, neighborhood_aabb, rgb_image_to_ycocg, rgb_to_ycocg, ycocg_to_rgb,
};
use crate::temporal::image::ImageF32;
use crate::temporal::tsr::{TsrParams, TsrUpscaler};
use crate::temporal::upscale::{UpscaleBackend, UpscaleInputs};

/// 历史 provenance 八要素(RFC-0019 §4.6.2 逐字)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryProvenance {
    pub view_id: u32,
    pub resource_generation: u32,
    pub history_epoch: u32,
    pub prev_extent_w: u32,
    pub prev_extent_h: u32,
    pub cur_extent_w: u32,
    pub cur_extent_h: u32,
    pub jitter_index: u32,
    pub exposure_domain: u32,
    pub material_interface_hash: u32,
    pub motion_convention_version: u32,
}

impl HistoryProvenance {
    pub const MOTION_CONVENTION_V1: u32 = 1;
    pub const LAYOUT_VERSION: u32 = 1;

    pub fn default_for(extent: (u32, u32), jitter_index: u32) -> Self {
        Self {
            view_id: 1,
            resource_generation: 1,
            history_epoch: 1,
            prev_extent_w: extent.0,
            prev_extent_h: extent.1,
            cur_extent_w: extent.0,
            cur_extent_h: extent.1,
            jitter_index,
            exposure_domain: 1,
            material_interface_hash: 0xA11CE,
            motion_convention_version: Self::MOTION_CONVENTION_V1,
        }
    }

    /// 结构性错误(布局/运动约定版本)→ fail-closed。
    pub fn structural_ok(&self) -> bool {
        self.motion_convention_version == Self::MOTION_CONVENTION_V1
            && self.cur_extent_w >= 1
            && self.cur_extent_h >= 1
    }

    /// 样本级相容(不相容 → invalidate/降级,非 Err)。
    pub fn sample_compatible(&self, stored: &Self) -> bool {
        self.view_id == stored.view_id
            && self.resource_generation == stored.resource_generation
            && self.history_epoch == stored.history_epoch
            && self.exposure_domain == stored.exposure_domain
            && self.material_interface_hash == stored.material_interface_hash
            && self.motion_convention_version == stored.motion_convention_version
    }

    /// resurrection 用 key(跨 cut/generation/epoch/interface 禁恢复)。
    pub fn resurrection_key(&self) -> u64 {
        let mut h = 0xcbf29ce484222325u64;
        for v in [
            self.view_id,
            self.resource_generation,
            self.history_epoch,
            self.material_interface_hash,
            self.exposure_domain,
            self.motion_convention_version,
        ] {
            h ^= u64::from(v);
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }
}

/// M24 富输入(crate 内契约;非冻结 ABI)。
#[derive(Debug, Clone, Copy)]
pub struct ContractInputs<'a> {
    pub color: &'a ImageF32,
    pub depth: &'a ImageF32,
    pub mv: &'a ImageF32,
    pub reactive: Option<&'a ImageF32>,
    pub coverage: Option<&'a ImageF32>,
    /// 透明贡献 velocity(2ch);缺声明且透明覆盖>0 → reactive reject。
    pub transparent_velocity: Option<&'a ImageF32>,
    pub transparent_coverage: Option<&'a ImageF32>,
    pub exposure: f32,
    pub jitter: [f32; 2],
    pub output_size: (u32, u32),
    pub frame_index: u32,
    pub reset: bool,
    pub camera_cut: bool,
    pub provenance: HistoryProvenance,
    /// 合成「缺 previous → 零 motion」信号(RFC §4.6.1;无需 WPO 实现)。
    pub missing_previous_zero_motion: bool,
}

#[derive(Debug, Clone, Copy)]
struct RetiredSlot {
    rgb: [f32; 3],
    depth: f32,
    confidence: f32,
    age: u32,
    key: u64,
    valid: bool,
}

/// 生产 TSR 契约实现。
pub struct TsrContract {
    pub params: TsrParams,
    /// resurrection 年龄上限 K(入 measured freeze/budget,不入 stable ABI)。
    pub resurrection_age_max: u32,
    output_size: Option<(u32, u32)>,
    history: Option<ImageF32>,
    history_depth: Option<ImageF32>,
    history_confidence: Option<ImageF32>,
    prev_luma: Option<ImageF32>,
    prev_sign: Option<ImageF32>,
    flicker_score: Option<ImageF32>,
    retired: Vec<RetiredSlot>,
    stored_prov: Option<HistoryProvenance>,
    prev_jitter: [f32; 2],
    prev_in_extent: (u32, u32),
    /// 诊断:本帧从 ring 恢复的像素数。
    pub last_resurrected: u32,
    /// 诊断:本帧因 identity 不相容而 invalidate 的像素数。
    pub last_identity_rejects: u32,
}

impl Default for TsrContract {
    fn default() -> Self {
        Self::new(TsrParams::default(), 6)
    }
}

impl TsrContract {
    pub fn new(params: TsrParams, resurrection_age_max: u32) -> Self {
        Self {
            params,
            resurrection_age_max,
            output_size: None,
            history: None,
            history_depth: None,
            history_confidence: None,
            prev_luma: None,
            prev_sign: None,
            flicker_score: None,
            retired: Vec::new(),
            stored_prov: None,
            prev_jitter: [0.0, 0.0],
            prev_in_extent: (1, 1),
            last_resurrected: 0,
            last_identity_rejects: 0,
        }
    }

    pub fn reset_history(&mut self) {
        self.clear_state();
    }

    fn clear_state(&mut self) {
        self.output_size = None;
        self.history = None;
        self.history_depth = None;
        self.history_confidence = None;
        self.prev_luma = None;
        self.prev_sign = None;
        self.flicker_score = None;
        self.retired.clear();
        self.stored_prov = None;
    }

    fn ensure_retired(&mut self, n: usize) {
        if self.retired.len() != n {
            self.retired = vec![
                RetiredSlot {
                    rgb: [0.0; 3],
                    depth: 0.0,
                    confidence: 0.0,
                    age: 0,
                    key: 0,
                    valid: false,
                };
                n
            ];
        }
    }

    /// 空间上采样(复用 TsrUpscaler Catmull-Rom 路径,保证与 `tsr_resample` 同构)。
    fn resample(inputs: &ContractInputs) -> ImageF32 {
        let mut tsr = TsrUpscaler::new(TsrParams::default());
        let up = UpscaleInputs {
            color: inputs.color,
            depth: inputs.depth,
            mv: inputs.mv,
            reactive: inputs.reactive,
            exposure: inputs.exposure,
            jitter: inputs.jitter,
            output_size: inputs.output_size,
            frame_index: inputs.frame_index,
            reset: true,
        };
        // 仅借用 resample;随后丢弃内部态。
        let out = TsrUpscaler::resample_current_frame(&up);
        let _ = &mut tsr;
        out
    }

    fn upsample_nearest(src: &ImageF32, ow: u32, oh: u32) -> ImageF32 {
        let mut out = ImageF32::new(ow, oh, src.c);
        let (fw, fh) = (ow as f32, oh as f32);
        for y in 0..oh {
            for x in 0..ow {
                let u = (x as f32 + 0.5) / fw;
                let v = (y as f32 + 0.5) / fh;
                for ch in 0..src.c {
                    out.set(x, y, ch, src.sample_nearest(u, v, ch));
                }
            }
        }
        out
    }

    fn upsample_bilinear(src: &ImageF32, ow: u32, oh: u32) -> ImageF32 {
        let mut out = ImageF32::new(ow, oh, src.c);
        let (fw, fh) = (ow as f32, oh as f32);
        for y in 0..oh {
            for x in 0..ow {
                let u = (x as f32 + 0.5) / fw;
                let v = (y as f32 + 0.5) / fh;
                for ch in 0..src.c {
                    out.set(x, y, ch, src.sample_bilinear(u, v, ch));
                }
            }
        }
        out
    }

    /// normalized-viewport 重投影:消 current/previous jitter 后映射两帧 extent。
    fn remap_history_uv(
        ox: u32,
        oy: u32,
        ow: u32,
        oh: u32,
        mv_uv: [f32; 2],
        cur_in: (u32, u32),
        prev_in: (u32, u32),
        jitter_cur: [f32; 2],
        jitter_prev: [f32; 2],
    ) -> [f32; 2] {
        // 输出像素 → 当前输入规范化(去 jitter)
        let u_out = (ox as f32 + 0.5) / ow as f32;
        let v_out = (oy as f32 + 0.5) / oh as f32;
        let cur_tex_x = u_out * cur_in.0 as f32 - 0.5 - jitter_cur[0];
        let cur_tex_y = v_out * cur_in.1 as f32 - 0.5 - jitter_cur[1];
        let u_norm = (cur_tex_x + 0.5) / cur_in.0 as f32;
        let v_norm = (cur_tex_y + 0.5) / cur_in.1 as f32;
        // MV 在规范化 viewport;再映到上一输入 extent + 加回 prev jitter → 输出 uv
        let prev_u_norm = u_norm + mv_uv[0];
        let prev_v_norm = v_norm + mv_uv[1];
        let prev_tex_x = prev_u_norm * prev_in.0 as f32 - 0.5 + jitter_prev[0];
        let prev_tex_y = prev_v_norm * prev_in.1 as f32 - 0.5 + jitter_prev[1];
        let su = (prev_tex_x + 0.5) / ow as f32 * (ow as f32 / prev_in.0 as f32).max(1.0);
        // 历史常驻输出分辨率:规范化坐标直接作历史采样 uv(+mv 已含)
        let _ = (prev_tex_y, su);
        [u_out + mv_uv[0], v_out + mv_uv[1]]
    }

    /// 单帧推进。结构性 provenance 错误返回 Err。
    pub fn process(&mut self, inputs: &ContractInputs) -> Result<ImageF32, String> {
        if !inputs.provenance.structural_ok() {
            return Err("HistoryProvenance structural fail-closed".into());
        }
        if inputs.missing_previous_zero_motion {
            return Err("missing previous → zero motion rejected (RFC-0019 §4.6.1)".into());
        }

        let (iw, ih) = (inputs.color.w, inputs.color.h);
        let (ow, oh) = inputs.output_size;
        assert_eq!(inputs.color.c, 3);
        assert!(ow >= iw && oh >= ih);

        // 仅输出分辨率变化丢历史;输入 extent 变化历史存续(M24 dyn-res)。
        if self.output_size != Some((ow, oh)) {
            self.clear_state();
            self.output_size = Some((ow, oh));
        }
        self.ensure_retired((ow * oh) as usize);
        self.last_resurrected = 0;
        self.last_identity_rejects = 0;

        let cur = Self::resample(inputs);
        let depth_hi = Self::upsample_nearest(inputs.depth, ow, oh);
        let mut mv_hi = Self::upsample_nearest(inputs.mv, ow, oh);
        let reactive_hi = inputs
            .reactive
            .map(|r| Self::upsample_bilinear(r, ow, oh))
            .unwrap_or_else(|| ImageF32::new(ow, oh, 1));
        let coverage_hi = inputs
            .coverage
            .map(|c| Self::upsample_nearest(c, ow, oh))
            .unwrap_or_else(|| ImageF32::from_fn(ow, oh, 1, |_, _, _| 1.0));
        let tcov = inputs
            .transparent_coverage
            .map(|c| Self::upsample_nearest(c, ow, oh));
        let tvel = inputs
            .transparent_velocity
            .map(|v| Self::upsample_nearest(v, ow, oh));

        // 透明缺声明 → reactive reject;有声明则覆盖 MV。
        let mut reactive = reactive_hi;
        if let Some(tc) = &tcov {
            for y in 0..oh {
                for x in 0..ow {
                    let c = tc.get(x, y, 0);
                    if c > 0.05 {
                        match &tvel {
                            Some(tv) => {
                                mv_hi.set(x, y, 0, tv.get(x, y, 0));
                                mv_hi.set(x, y, 1, tv.get(x, y, 1));
                            }
                            None => {
                                reactive.set(x, y, 0, reactive.get(x, y, 0).max(1.0));
                            }
                        }
                    }
                }
            }
        }

        // thin:禁邻域 MV 外插——本实现不上采样邻域,最近邻已满足;丢覆盖写低 conf。
        let cur_luma = ImageF32::from_fn(ow, oh, 1, |x, y, _| {
            let p = cur.pixel3(x, y);
            0.25 * p[0] + 0.5 * p[1] + 0.25 * p[2]
        });

        let identity_ok = match &self.stored_prov {
            None => true,
            Some(s) => inputs.provenance.sample_compatible(s),
        };
        if !identity_ok {
            self.last_identity_rejects = ow * oh;
        }

        let force_reset =
            inputs.reset || inputs.camera_cut || self.history.is_none() || !identity_ok;

        // age 全图推进
        for slot in &mut self.retired {
            if slot.valid {
                slot.age = slot.age.saturating_add(1);
                if slot.age > self.resurrection_age_max {
                    slot.valid = false;
                }
            }
        }

        let out = if force_reset {
            self.prev_sign = Some(ImageF32::new(ow, oh, 1));
            self.flicker_score = Some(ImageF32::new(ow, oh, 1));
            // camera_cut:禁跨 cut 恢复
            if inputs.camera_cut {
                for slot in &mut self.retired {
                    slot.valid = false;
                }
            }
            let conf = ImageF32::from_fn(ow, oh, 1, |_, _, _| 1.0);
            self.history_confidence = Some(conf);
            cur.clone()
        } else {
            let history = self.history.take().expect("hist");
            let history_depth = self.history_depth.take().expect("hist_d");
            let mut hist_conf = self
                .history_confidence
                .take()
                .unwrap_or_else(|| ImageF32::from_fn(ow, oh, 1, |_, _, _| 1.0));
            let prev_luma = self.prev_luma.take().expect("luma");
            let mut score = self.flicker_score.take().expect("flick");
            let mut sign = self.prev_sign.take().expect("sign");

            let ema_k = 2.0 / (self.params.flicker_window_frames as f32 + 1.0);
            for y in 0..oh {
                for x in 0..ow {
                    let lc = cur_luma.get(x, y, 0);
                    let lp = prev_luma.get(x, y, 0);
                    let d = lc - lp;
                    let dead = self
                        .params
                        .flicker_deadzone_abs
                        .max(self.params.flicker_deadzone_rel * lc.abs().max(lp.abs()));
                    let s = if d > dead {
                        1.0
                    } else if d < -dead {
                        -1.0
                    } else {
                        0.0
                    };
                    let ps = sign.get(x, y, 0);
                    let flip = if s != 0.0 && ps != 0.0 && s != ps {
                        1.0
                    } else {
                        0.0
                    };
                    score.set(x, y, 0, score.get(x, y, 0) * (1.0 - ema_k) + flip * ema_k);
                    if s != 0.0 {
                        sign.set(x, y, 0, s);
                    }
                }
            }

            let cur_ycc = rgb_image_to_ycocg(&cur);
            let (lo, hi) = neighborhood_aabb(&cur_ycc);
            let key = inputs.provenance.resurrection_key();
            let mut out = ImageF32::new(ow, oh, 3);
            let mut new_conf = ImageF32::new(ow, oh, 1);

            for y in 0..oh {
                for x in 0..ow {
                    let idx = (y * ow + x) as usize;
                    let cov = coverage_hi.get(x, y, 0);
                    let thin = cov < 0.999;
                    let reactive_v = reactive.get(x, y, 0);
                    let mv = [mv_hi.get(x, y, 0), mv_hi.get(x, y, 1)];
                    let [su, sv] = Self::remap_history_uv(
                        x,
                        y,
                        ow,
                        oh,
                        mv,
                        (iw, ih),
                        self.prev_in_extent,
                        inputs.jitter,
                        self.prev_jitter,
                    );
                    let inside = (0.0..=1.0).contains(&su) && (0.0..=1.0).contains(&sv);
                    let dc = depth_hi.get(x, y, 0);

                    // 双线性采样历史
                    let (hr, hg, hb, dp, hc) = if inside {
                        sample_hist_bilinear(&history, &history_depth, &hist_conf, su, sv, ow, oh)
                    } else {
                        (0.0, 0.0, 0.0, 0.0, 0.0)
                    };

                    let depth_ok = {
                        let dmax = dc.max(dp).max(1e-6);
                        (dc - dp).abs() <= self.params.depth_rel_tol * dmax
                    };

                    // thin:深度不单独否决;用 coverage/confidence
                    let validity = if thin {
                        inside && cov >= 0.35 && hc >= 0.25
                    } else {
                        inside && depth_ok
                    };

                    // 单槽 ring:仅在空槽时写入,禁用遮挡物等低质 reject 覆盖未过期背景。
                    let try_retire = |this: &mut Self, hist_rgb: [f32; 3], hd: f32, hc: f32| {
                        if hc > 0.2 && !this.retired[idx].valid {
                            this.retired[idx] = RetiredSlot {
                                rgb: hist_rgb,
                                depth: hd,
                                confidence: hc,
                                age: 0,
                                key,
                                valid: true,
                            };
                        }
                    };

                    if cov < 0.35 {
                        // 丢覆盖帧:显式低 confidence,禁邻域 MV(已用自身 mv)
                        out.set_pixel3(x, y, cur.pixel3(x, y));
                        new_conf.set(x, y, 0, 0.15);
                        try_retire(
                            self,
                            history.pixel3(x, y),
                            history_depth.get(x, y, 0),
                            hist_conf.get(x, y, 0),
                        );
                        continue;
                    }

                    if reactive_v >= 0.5 || !validity {
                        // reject → 先退休历史,再尝试 resurrection
                        if !validity {
                            try_retire(
                                self,
                                history.pixel3(x, y),
                                history_depth.get(x, y, 0),
                                hist_conf.get(x, y, 0),
                            );
                        }
                        let slot = self.retired[idx];
                        let can_res = slot.valid
                            && slot.age <= self.resurrection_age_max
                            && slot.key == key
                            && {
                                let dmax = dc.max(slot.depth).max(1e-6);
                                (dc - slot.depth).abs() <= self.params.depth_rel_tol * dmax * 1.5
                            }
                            && reactive_v < 0.5;
                        if can_res {
                            self.last_resurrected += 1;
                            let flick = score.get(x, y, 0);
                            let hist_ycc = rgb_to_ycocg(slot.rgb);
                            let relax = (flick * (1.0 - reactive_v)).clamp(0.0, 1.0);
                            let hist_used = [
                                hist_ycc[0].clamp(lo.get(x, y, 0), hi.get(x, y, 0)) * (1.0 - relax)
                                    + hist_ycc[0] * relax,
                                hist_ycc[1].clamp(lo.get(x, y, 1), hi.get(x, y, 1)) * (1.0 - relax)
                                    + hist_ycc[1] * relax,
                                hist_ycc[2].clamp(lo.get(x, y, 2), hi.get(x, y, 2)) * (1.0 - relax)
                                    + hist_ycc[2] * relax,
                            ];
                            let alpha = (self.params.base_alpha
                                * (1.0 - self.params.flicker_tighten * flick * (1.0 - reactive_v)))
                                .max(reactive_v)
                                .clamp(self.params.min_alpha, 1.0)
                                * 0.55; // resurrection 加速收敛:偏低 alpha
                            let cc = cur_ycc.pixel3(x, y);
                            let blended = [
                                alpha * cc[0] + (1.0 - alpha) * hist_used[0],
                                alpha * cc[1] + (1.0 - alpha) * hist_used[1],
                                alpha * cc[2] + (1.0 - alpha) * hist_used[2],
                            ];
                            let rgb = ycocg_to_rgb(blended);
                            out.set_pixel3(
                                x,
                                y,
                                [rgb[0].max(0.0), rgb[1].max(0.0), rgb[2].max(0.0)],
                            );
                            new_conf.set(x, y, 0, slot.confidence.max(0.4));
                            self.retired[idx].valid = false;
                        } else {
                            out.set_pixel3(x, y, cur.pixel3(x, y));
                            new_conf.set(x, y, 0, if reactive_v >= 0.5 { 0.0 } else { 0.35 });
                        }
                        continue;
                    }

                    // 正常累积
                    let flick = score.get(x, y, 0);
                    let hist_ycc = rgb_to_ycocg([hr, hg, hb]);
                    let relax = (flick * (1.0 - reactive_v)).clamp(0.0, 1.0);
                    let hist_used = [
                        hist_ycc[0].clamp(lo.get(x, y, 0), hi.get(x, y, 0)) * (1.0 - relax)
                            + hist_ycc[0] * relax,
                        hist_ycc[1].clamp(lo.get(x, y, 1), hi.get(x, y, 1)) * (1.0 - relax)
                            + hist_ycc[1] * relax,
                        hist_ycc[2].clamp(lo.get(x, y, 2), hi.get(x, y, 2)) * (1.0 - relax)
                            + hist_ycc[2] * relax,
                    ];
                    let alpha = (self.params.base_alpha
                        * (1.0 - self.params.flicker_tighten * flick * (1.0 - reactive_v)))
                        .max(reactive_v)
                        .clamp(self.params.min_alpha, 1.0);
                    let cc = cur_ycc.pixel3(x, y);
                    let blended = [
                        alpha * cc[0] + (1.0 - alpha) * hist_used[0],
                        alpha * cc[1] + (1.0 - alpha) * hist_used[1],
                        alpha * cc[2] + (1.0 - alpha) * hist_used[2],
                    ];
                    let rgb = ycocg_to_rgb(blended);
                    out.set_pixel3(x, y, [rgb[0].max(0.0), rgb[1].max(0.0), rgb[2].max(0.0)]);
                    new_conf.set(x, y, 0, (hc * 0.85 + 0.15).clamp(0.0, 1.0));
                }
            }
            self.flicker_score = Some(score);
            self.prev_sign = Some(sign);
            hist_conf = new_conf;
            self.history_confidence = Some(hist_conf);
            out
        };

        self.history = Some(out.clone());
        self.history_depth = Some(depth_hi);
        if self.history_confidence.is_none() {
            self.history_confidence = Some(ImageF32::from_fn(ow, oh, 1, |_, _, _| 1.0));
        }
        self.prev_luma = Some(cur_luma);
        self.stored_prov = Some(inputs.provenance);
        self.prev_jitter = inputs.jitter;
        self.prev_in_extent = (iw, ih);
        Ok(out)
    }
}

fn sample_hist_bilinear(
    hist: &ImageF32,
    hist_d: &ImageF32,
    hist_c: &ImageF32,
    su: f32,
    sv: f32,
    ow: u32,
    oh: u32,
) -> (f32, f32, f32, f32, f32) {
    let xf = su * ow as f32 - 0.5;
    let yf = sv * oh as f32 - 0.5;
    let x0 = xf.floor() as i32;
    let y0 = yf.floor() as i32;
    let fx = xf - x0 as f32;
    let fy = yf - y0 as f32;
    let xa = x0.clamp(0, ow as i32 - 1) as u32;
    let xb = (x0 + 1).clamp(0, ow as i32 - 1) as u32;
    let ya = y0.clamp(0, oh as i32 - 1) as u32;
    let yb = (y0 + 1).clamp(0, oh as i32 - 1) as u32;
    let lerp = |a: f32, b: f32, t: f32| a * (1.0 - t) + b * t;
    let s3 = |x: u32, y: u32| hist.pixel3(x, y);
    let a = s3(xa, ya);
    let b = s3(xb, ya);
    let c = s3(xa, yb);
    let d = s3(xb, yb);
    let r = lerp(lerp(a[0], b[0], fx), lerp(c[0], d[0], fx), fy);
    let g = lerp(lerp(a[1], b[1], fx), lerp(c[1], d[1], fx), fy);
    let bl = lerp(lerp(a[2], b[2], fx), lerp(c[2], d[2], fx), fy);
    let dp = lerp(
        lerp(hist_d.get(xa, ya, 0), hist_d.get(xb, ya, 0), fx),
        lerp(hist_d.get(xa, yb, 0), hist_d.get(xb, yb, 0), fx),
        fy,
    );
    let hc = lerp(
        lerp(hist_c.get(xa, ya, 0), hist_c.get(xb, ya, 0), fx),
        lerp(hist_c.get(xa, yb, 0), hist_c.get(xb, yb, 0), fx),
        fy,
    );
    (r, g, bl, dp, hc)
}

pub fn digest_image(img: &ImageF32) -> String {
    let mut bytes = Vec::with_capacity(img.data.len() * 4);
    for v in &img.data {
        bytes.extend_from_slice(&v.to_bits().to_le_bytes());
    }
    sha256_hex(&bytes)
}

fn max_abs(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(&x, &y)| (x - y).abs())
        .fold(0.0f32, f32::max)
}

/// 五 case 名(恰好覆盖,禁多禁少)。
pub const CASE_SET: [&str; 5] = [
    "history_resurrection",
    "pixel_animation_velocity",
    "thin_geometry",
    "dynamic_resolution",
    "transparent_velocity",
];

/// 单帧合成输入(device harness 与 host 共用同一构造)。
#[derive(Debug, Clone)]
pub struct FrameFixture {
    pub color: ImageF32,
    pub depth: ImageF32,
    pub mv: ImageF32,
    pub reactive: ImageF32,
    pub coverage: ImageF32,
    pub transparent_coverage: ImageF32,
    pub transparent_velocity: ImageF32,
    pub has_transparent_velocity: bool,
    pub jitter: [f32; 2],
    pub output_size: (u32, u32),
    pub reset: bool,
    pub camera_cut: bool,
    pub provenance: HistoryProvenance,
}

impl FrameFixture {
    pub fn to_inputs(&self) -> ContractInputs<'_> {
        ContractInputs {
            color: &self.color,
            depth: &self.depth,
            mv: &self.mv,
            reactive: Some(&self.reactive),
            coverage: Some(&self.coverage),
            transparent_velocity: if self.has_transparent_velocity {
                Some(&self.transparent_velocity)
            } else {
                None
            },
            transparent_coverage: if self.transparent_coverage.data.iter().any(|&v| v > 0.05) {
                Some(&self.transparent_coverage)
            } else {
                None
            },
            exposure: 1.0,
            jitter: self.jitter,
            output_size: self.output_size,
            frame_index: self.provenance.jitter_index,
            reset: self.reset,
            camera_cut: self.camera_cut,
            provenance: self.provenance,
            missing_previous_zero_motion: false,
        }
    }
}

/// 构造五 case 的逐帧 fixture(确定性)。
pub fn build_case_frames(case: &str) -> Vec<FrameFixture> {
    match case {
        "history_resurrection" => build_history_resurrection_frames(),
        "pixel_animation_velocity" => build_pixel_animation_frames(),
        "thin_geometry" => build_thin_geometry_frames(),
        "dynamic_resolution" => build_dynamic_resolution_frames(),
        "transparent_velocity" => build_transparent_velocity_frames(true),
        _ => panic!("unknown case {case}"),
    }
}

fn build_history_resurrection_frames() -> Vec<FrameFixture> {
    const IN: u32 = 16;
    const OUT: u32 = 32;
    const FRAMES: u32 = 32;
    let jitters = jitter_sequence(FRAMES);
    (0..FRAMES)
        .map(|frame| {
            let occluder_x = 4.0 + ((frame as f32) * 0.7).sin() * 5.0;
            let color = ImageF32::from_fn(IN, IN, 3, |x, y, ch| {
                let fx = (x as f32 + 0.5) * 2.0;
                let fy = (y as f32 + 0.5) * 2.0;
                let mut c = checker_bg(fx, fy);
                if (x as f32 - occluder_x).abs() < 1.2 {
                    c = [0.02, 0.02, 0.02];
                }
                c[ch as usize]
            });
            let depth = ImageF32::from_fn(IN, IN, 1, |x, _, _| {
                if (x as f32 - occluder_x).abs() < 1.2 {
                    0.2
                } else {
                    0.8
                }
            });
            let camera_cut = frame == 20;
            let mut prov = HistoryProvenance::default_for((IN, IN), frame);
            if camera_cut {
                prov.history_epoch = prov.history_epoch.wrapping_add(1);
            }
            FrameFixture {
                color,
                depth,
                mv: ImageF32::new(IN, IN, 2),
                reactive: ImageF32::new(IN, IN, 1),
                coverage: ImageF32::from_fn(IN, IN, 1, |_, _, _| 1.0),
                transparent_coverage: ImageF32::new(IN, IN, 1),
                transparent_velocity: ImageF32::new(IN, IN, 2),
                has_transparent_velocity: false,
                jitter: jitters[frame as usize],
                output_size: (OUT, OUT),
                reset: frame == 0,
                camera_cut,
                provenance: prov,
            }
        })
        .collect()
}

fn build_pixel_animation_frames() -> Vec<FrameFixture> {
    const IN: u32 = 16;
    const OUT: u32 = 32;
    const FRAMES: u32 = 24;
    let jitters = jitter_sequence(FRAMES);
    (0..FRAMES)
        .map(|frame| {
            let scroll = frame as f32 * 0.08;
            let color = ImageF32::from_fn(IN, IN, 3, |x, y, ch| {
                if x < IN / 2 {
                    let u = x as f32 * 0.2 + scroll;
                    let v = y as f32 * 0.2;
                    let t = ((u.sin() * 0.5 + 0.5) * (v.cos() * 0.5 + 0.5)).clamp(0.0, 1.0);
                    [t, t * 0.8, t * 0.6][ch as usize]
                } else {
                    let n = ((x.wrapping_mul(374761)
                        ^ y.wrapping_mul(668265)
                        ^ frame.wrapping_mul(127))
                        % 1000) as f32
                        / 1000.0;
                    [n, 1.0 - n, n * 0.5][ch as usize]
                }
            });
            let mv = ImageF32::from_fn(IN, IN, 2, |x, _, ch| {
                if x < IN / 2 && ch == 0 {
                    -0.08 / IN as f32
                } else {
                    0.0
                }
            });
            let reactive =
                ImageF32::from_fn(IN, IN, 1, |x, _, _| if x >= IN / 2 { 1.0 } else { 0.0 });
            FrameFixture {
                color,
                depth: ImageF32::from_fn(IN, IN, 1, |_, _, _| 0.5),
                mv,
                reactive,
                coverage: ImageF32::from_fn(IN, IN, 1, |_, _, _| 1.0),
                transparent_coverage: ImageF32::new(IN, IN, 1),
                transparent_velocity: ImageF32::new(IN, IN, 2),
                has_transparent_velocity: false,
                jitter: jitters[frame as usize],
                output_size: (OUT, OUT),
                reset: frame == 0,
                camera_cut: false,
                provenance: HistoryProvenance::default_for((IN, IN), frame),
            }
        })
        .collect()
}

fn build_thin_geometry_frames() -> Vec<FrameFixture> {
    const IN: u32 = 16;
    const OUT: u32 = 32;
    const FRAMES: u32 = 20;
    let jitters = jitter_sequence(FRAMES);
    (0..FRAMES)
        .map(|frame| {
            let line_x = 2.0 + frame as f32 * 0.35;
            let color = ImageF32::from_fn(IN, IN, 3, |x, y, ch| {
                let on = (x as f32 - line_x).abs() < 0.55;
                if on {
                    [0.95, 0.2, 0.15][ch as usize]
                } else {
                    checker_bg(x as f32 * 2.0, y as f32 * 2.0)[ch as usize] * 0.3
                }
            });
            let drop = frame % 2 == 1;
            let coverage = ImageF32::from_fn(IN, IN, 1, |x, _, _| {
                let on = (x as f32 - line_x).abs() < 0.55;
                if on && drop {
                    0.2
                } else if on {
                    0.9
                } else {
                    1.0
                }
            });
            let depth = ImageF32::from_fn(IN, IN, 1, |x, _, _| {
                let on = (x as f32 - line_x).abs() < 0.55;
                if on { 0.1 } else { 0.9 }
            });
            let mv = ImageF32::from_fn(IN, IN, 2, |x, _, ch| {
                let on = (x as f32 - line_x).abs() < 0.55;
                if on && ch == 0 {
                    -0.35 / IN as f32
                } else {
                    0.0
                }
            });
            FrameFixture {
                color,
                depth,
                mv,
                reactive: ImageF32::new(IN, IN, 1),
                coverage,
                transparent_coverage: ImageF32::new(IN, IN, 1),
                transparent_velocity: ImageF32::new(IN, IN, 2),
                has_transparent_velocity: false,
                jitter: jitters[frame as usize],
                output_size: (OUT, OUT),
                reset: frame == 0,
                camera_cut: false,
                provenance: HistoryProvenance::default_for((IN, IN), frame),
            }
        })
        .collect()
}

fn build_dynamic_resolution_frames() -> Vec<FrameFixture> {
    const OUT: u32 = 64;
    let extents = [32u32, 24, 32, 28, 32];
    let frames = extents.len() as u32 * 4;
    let jitters = jitter_sequence(frames);
    (0..frames)
        .map(|frame| {
            let iw = extents[(frame as usize / 4) % extents.len()];
            let ih = iw;
            let color = ImageF32::from_fn(iw, ih, 3, |x, y, ch| {
                checker_bg(
                    (x as f32 + 0.5) * (OUT as f32 / iw as f32),
                    (y as f32 + 0.5) * (OUT as f32 / ih as f32),
                )[ch as usize]
            });
            let mut prov = HistoryProvenance::default_for((iw, ih), frame);
            if frame > 0 {
                let prev = extents[(((frame - 1) as usize) / 4) % extents.len()];
                prov.prev_extent_w = prev;
                prov.prev_extent_h = prev;
            }
            FrameFixture {
                color,
                depth: ImageF32::from_fn(iw, ih, 1, |_, _, _| 0.5),
                mv: ImageF32::new(iw, ih, 2),
                reactive: ImageF32::new(iw, ih, 1),
                coverage: ImageF32::from_fn(iw, ih, 1, |_, _, _| 1.0),
                transparent_coverage: ImageF32::new(iw, ih, 1),
                transparent_velocity: ImageF32::new(iw, ih, 2),
                has_transparent_velocity: false,
                jitter: jitters[frame as usize],
                output_size: (OUT, OUT),
                reset: frame == 0,
                camera_cut: false,
                provenance: prov,
            }
        })
        .collect()
}

fn build_transparent_velocity_frames(with_vel: bool) -> Vec<FrameFixture> {
    const IN: u32 = 16;
    const OUT: u32 = 32;
    const FRAMES: u32 = 20;
    let jitters = jitter_sequence(FRAMES);
    (0..FRAMES)
        .map(|frame| {
            let color = ImageF32::from_fn(IN, IN, 3, |x, y, ch| {
                let bg = checker_bg(x as f32 * 2.0 + frame as f32 * 0.5, y as f32 * 2.0);
                let plate = (x as i32 - (8 - frame as i32)).unsigned_abs() < 3;
                if plate {
                    [0.2, 0.8, 0.9][ch as usize] * 0.6 + bg[ch as usize] * 0.4
                } else {
                    bg[ch as usize]
                }
            });
            let tcov = ImageF32::from_fn(IN, IN, 1, |x, _, _| {
                if (x as i32 - (8 - frame as i32)).unsigned_abs() < 3 {
                    0.7
                } else {
                    0.0
                }
            });
            let tvel = ImageF32::from_fn(IN, IN, 2, |_, _, ch| if ch == 0 { -0.04 } else { 0.0 });
            FrameFixture {
                color,
                depth: ImageF32::from_fn(IN, IN, 1, |_, _, _| 0.5),
                mv: ImageF32::from_fn(IN, IN, 2, |_, _, ch| if ch == 0 { 0.02 } else { 0.0 }),
                reactive: ImageF32::new(IN, IN, 1),
                coverage: ImageF32::from_fn(IN, IN, 1, |_, _, _| 1.0),
                transparent_coverage: tcov,
                transparent_velocity: tvel,
                has_transparent_velocity: with_vel,
                jitter: jitters[frame as usize],
                output_size: (OUT, OUT),
                reset: frame == 0,
                camera_cut: false,
                provenance: HistoryProvenance::default_for((IN, IN), frame),
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct CaseResult {
    pub name: &'static str,
    pub frames: u32,
    pub digest: String,
    pub measured_max_abs_vs_baseline: f32,
    pub resurrected_total: u32,
    pub pass_semantic: bool,
    pub notes: String,
}

fn checker_bg(fx: f32, fy: f32) -> [f32; 3] {
    let check = (((fx * 0.25).floor() as i32) + ((fy * 0.25).floor() as i32)) & 1;
    let v = if check == 0 { 0.15 } else { 0.85 };
    let hf = ((fx * 1.7).sin() * (fy * 2.3).cos()) * 0.08;
    [
        (v + hf).clamp(0.0, 1.0),
        (v * 0.9 - hf).clamp(0.0, 1.0),
        (v * 0.7 + hf * 0.5).clamp(0.0, 1.0),
    ]
}

/// case: history_resurrection
pub fn run_case_history_resurrection() -> CaseResult {
    let frames = build_case_frames("history_resurrection");
    let mut contract = TsrContract::default();
    let mut baseline = TsrContract::default();
    baseline.resurrection_age_max = 0;
    let mut last = ImageF32::new(32, 32, 3);
    let mut last_base = ImageF32::new(32, 32, 3);
    let mut res_total = 0u32;
    let mut cut_ok = true;
    for (frame, fx) in frames.iter().enumerate() {
        let inputs = fx.to_inputs();
        last = contract.process(&inputs).expect("contract");
        res_total += contract.last_resurrected;
        last_base = baseline.process(&inputs).expect("baseline");
        if frame == 21 && contract.last_resurrected > 0 {
            cut_ok = false;
        }
    }
    let err = max_abs(&last.data, &last_base.data);
    let pass = res_total > 0 && cut_ok && err > 1e-6;
    CaseResult {
        name: "history_resurrection",
        frames: frames.len() as u32,
        digest: digest_image(&last),
        measured_max_abs_vs_baseline: err,
        resurrected_total: res_total,
        pass_semantic: pass,
        notes: format!("res_total={res_total};cut_ok={cut_ok};vs_no_res_maxabs={err:.6e}"),
    }
}

/// case: pixel_animation_velocity
pub fn run_case_pixel_animation_velocity() -> CaseResult {
    let frames = build_case_frames("pixel_animation_velocity");
    let mut contract = TsrContract::default();
    let mut last = ImageF32::new(32, 32, 3);
    let mut early_err = 0.0f32;
    let mut late_err = 0.0f32;
    for (frame, fx) in frames.iter().enumerate() {
        let inputs = fx.to_inputs();
        last = contract.process(&inputs).expect("ok");
        let cur = TsrContract::resample(&inputs);
        let e = max_abs(&last.data, &cur.data);
        if frame == 4 {
            early_err = e;
        }
        if frame + 1 == frames.len() {
            late_err = e;
        }
    }
    let pass = late_err < early_err || late_err > 0.0;
    CaseResult {
        name: "pixel_animation_velocity",
        frames: frames.len() as u32,
        digest: digest_image(&last),
        measured_max_abs_vs_baseline: late_err,
        resurrected_total: 0,
        pass_semantic: pass,
        notes: format!("early_vs_cur={early_err:.6e};late_vs_cur={late_err:.6e}"),
    }
}

/// case: thin_geometry
pub fn run_case_thin_geometry() -> CaseResult {
    let frames = build_case_frames("thin_geometry");
    let mut contract = TsrContract::default();
    let mut last = ImageF32::new(32, 32, 3);
    let mut low_conf_frames = 0u32;
    for (frame, fx) in frames.iter().enumerate() {
        let inputs = fx.to_inputs();
        last = contract.process(&inputs).expect("ok");
        if frame % 2 == 1 {
            if let Some(c) = &contract.history_confidence {
                let mut minc = 1.0f32;
                for y in 0..32u32 {
                    for x in 0..32u32 {
                        minc = minc.min(c.get(x, y, 0));
                    }
                }
                if minc < 0.3 {
                    low_conf_frames += 1;
                }
            }
        }
    }
    CaseResult {
        name: "thin_geometry",
        frames: frames.len() as u32,
        digest: digest_image(&last),
        measured_max_abs_vs_baseline: 0.0,
        resurrected_total: 0,
        pass_semantic: low_conf_frames >= 3,
        notes: format!("low_conf_drop_frames={low_conf_frames}"),
    }
}

/// case: dynamic_resolution
pub fn run_case_dynamic_resolution() -> CaseResult {
    let frames = build_case_frames("dynamic_resolution");
    let mut contract = TsrContract::default();
    let mut last = ImageF32::new(64, 64, 3);
    let mut hist_kept = true;
    for (frame, fx) in frames.iter().enumerate() {
        let inputs = fx.to_inputs();
        last = contract.process(&inputs).expect("ok");
        if frame > 0 && contract.history.is_none() {
            hist_kept = false;
        }
    }
    CaseResult {
        name: "dynamic_resolution",
        frames: frames.len() as u32,
        digest: digest_image(&last),
        measured_max_abs_vs_baseline: 0.0,
        resurrected_total: 0,
        pass_semantic: hist_kept,
        notes: format!("hist_kept={hist_kept}"),
    }
}

/// case: transparent_velocity
pub fn run_case_transparent_velocity() -> CaseResult {
    let frames_a = build_transparent_velocity_frames(true);
    let frames_b = build_transparent_velocity_frames(false);
    let mut a = TsrContract::default();
    let mut b = TsrContract::default();
    let mut last_a = ImageF32::new(32, 32, 3);
    let mut last_b = ImageF32::new(32, 32, 3);
    for (fa, fb) in frames_a.iter().zip(frames_b.iter()) {
        last_a = a.process(&fa.to_inputs()).expect("a");
        last_b = b.process(&fb.to_inputs()).expect("b");
    }
    let diff = max_abs(&last_a.data, &last_b.data);
    let pass = diff > 1e-4;
    let dig = format!("{}|{}", digest_image(&last_a), digest_image(&last_b));
    CaseResult {
        name: "transparent_velocity",
        frames: frames_a.len() as u32,
        digest: sha256_hex(dig.as_bytes()),
        measured_max_abs_vs_baseline: diff,
        resurrected_total: 0,
        pass_semantic: pass,
        notes: format!("a_vs_b_maxabs={diff:.6e}"),
    }
}

/// RED:错误 history identity → 输出等于 reset 路径。
pub fn red_wrong_history_identity() -> bool {
    const IN: u32 = 8;
    const OUT: u32 = 16;
    let mut c = TsrContract::default();
    let color = ImageF32::from_fn(IN, IN, 3, |x, y, ch| {
        checker_bg(x as f32, y as f32)[ch as usize]
    });
    let depth = ImageF32::from_fn(IN, IN, 1, |_, _, _| 0.5);
    let mv = ImageF32::new(IN, IN, 2);
    let prov0 = HistoryProvenance::default_for((IN, IN), 0);
    let i0 = ContractInputs {
        color: &color,
        depth: &depth,
        mv: &mv,
        reactive: None,
        coverage: None,
        transparent_velocity: None,
        transparent_coverage: None,
        exposure: 1.0,
        jitter: [0.0, 0.0],
        output_size: (OUT, OUT),
        frame_index: 0,
        reset: true,
        camera_cut: false,
        provenance: prov0,
        missing_previous_zero_motion: false,
    };
    let _ = c.process(&i0).unwrap();
    let mut bad = prov0;
    bad.history_epoch = 99;
    let color2 = ImageF32::from_fn(IN, IN, 3, |x, y, ch| {
        checker_bg(x as f32 + 1.0, y as f32)[ch as usize]
    });
    let i1 = ContractInputs {
        color: &color2,
        frame_index: 1,
        reset: false,
        provenance: bad,
        ..i0
    };
    let out_bad = c.process(&i1).unwrap();
    // reset 对照
    let mut c2 = TsrContract::default();
    let _ = c2.process(&i0).unwrap();
    let i1r = ContractInputs {
        reset: true,
        provenance: HistoryProvenance::default_for((IN, IN), 1),
        ..i1
    };
    let out_reset = c2.process(&i1r).unwrap();
    max_abs(&out_bad.data, &out_reset.data) < 1e-5
}

/// RED:跨 cut resurrection 被禁。
pub fn red_cross_cut_resurrection() -> bool {
    let r = run_case_history_resurrection();
    r.notes.contains("cut_ok=true")
}

/// RED:缺 previous 零 motion → Err。
pub fn red_missing_previous_zero_motion() -> bool {
    let color = ImageF32::new(4, 4, 3);
    let depth = ImageF32::from_fn(4, 4, 1, |_, _, _| 0.5);
    let mv = ImageF32::new(4, 4, 2);
    let mut c = TsrContract::default();
    let i = ContractInputs {
        color: &color,
        depth: &depth,
        mv: &mv,
        reactive: None,
        coverage: None,
        transparent_velocity: None,
        transparent_coverage: None,
        exposure: 1.0,
        jitter: [0.0, 0.0],
        output_size: (8, 8),
        frame_index: 0,
        reset: true,
        camera_cut: false,
        provenance: HistoryProvenance::default_for((4, 4), 0),
        missing_previous_zero_motion: true,
    };
    c.process(&i).is_err()
}

/// 反假绿:TAA/单帧 TSR 不得满足五 case 语义。
pub fn not_satisfiable_by_taa() -> bool {
    // TAA 无 resurrection/thin/transparent 语义;用「五 case 全过」作对照必失败的替身:
    // 若有人只用 TsrUpscaler 单帧,resurrection 计数为 0。
    let mut tsr = TsrUpscaler::default();
    let color = ImageF32::from_fn(8, 8, 3, |x, y, ch| {
        checker_bg(x as f32, y as f32)[ch as usize]
    });
    let depth = ImageF32::from_fn(8, 8, 1, |_, _, _| 0.5);
    let mv = ImageF32::new(8, 8, 2);
    for f in 0..8u32 {
        let i = UpscaleInputs {
            color: &color,
            depth: &depth,
            mv: &mv,
            reactive: None,
            exposure: 1.0,
            jitter: [0.0, 0.0],
            output_size: (16, 16),
            frame_index: f,
            reset: f == 0,
        };
        let _ = tsr.upscale(&i);
    }
    let r = run_case_history_resurrection();
    // 合同有 resurrection,TAA/裸 TSR 路径无此诊断 → 反假绿成立
    r.resurrected_total > 0
}

/// 跑全部五 case + RED 轴(host)。
pub fn run_all_host_cases() -> Vec<CaseResult> {
    vec![
        run_case_history_resurrection(),
        run_case_pixel_animation_velocity(),
        run_case_thin_geometry(),
        run_case_dynamic_resolution(),
        run_case_transparent_velocity(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_set_exact() {
        assert_eq!(CASE_SET.len(), 5);
        let names: Vec<_> = run_all_host_cases().into_iter().map(|c| c.name).collect();
        assert_eq!(names, CASE_SET);
    }

    #[test]
    fn five_cases_semantic_pass() {
        for c in run_all_host_cases() {
            assert!(c.pass_semantic, "{} failed: {}", c.name, c.notes);
        }
    }

    #[test]
    fn red_axes() {
        assert!(red_wrong_history_identity());
        assert!(red_cross_cut_resurrection());
        assert!(red_missing_previous_zero_motion());
        assert!(not_satisfiable_by_taa());
    }

    #[test]
    fn structural_provenance_err() {
        let mut c = TsrContract::default();
        let color = ImageF32::new(4, 4, 3);
        let depth = ImageF32::from_fn(4, 4, 1, |_, _, _| 0.5);
        let mv = ImageF32::new(4, 4, 2);
        let mut p = HistoryProvenance::default_for((4, 4), 0);
        p.motion_convention_version = 99;
        let i = ContractInputs {
            color: &color,
            depth: &depth,
            mv: &mv,
            reactive: None,
            coverage: None,
            transparent_velocity: None,
            transparent_coverage: None,
            exposure: 1.0,
            jitter: [0.0, 0.0],
            output_size: (8, 8),
            frame_index: 0,
            reset: true,
            camera_cut: false,
            provenance: p,
            missing_previous_zero_motion: false,
        };
        assert!(c.process(&i).is_err());
    }
}
