//! M70 载具确定性仿真态(RFC-0021 §4.D1:drivetrain/悬挂全状态纯 Rust,
//! 天然全量可捕获/可回滚)。fixture 规模:解析式地面 + 轻物体推挤接触,
//! 固定操作序 f32,同 build/同画像逐位确定。

use rurix_pkg::sha256::{digest, hex};

use crate::capture::canonical::CaptureError;

use super::VehicleAsset;

/// 固定步长(有理数 1/60 的 f32 值;画像冻结)。
pub const FIXED_DT: f32 = 1.0 / 60.0;
/// subject 场景总 tick 数。
pub const TICKS: u64 = 240;
/// rollback 腿快照点。
pub const ROLLBACK_TICK: u64 = 120;
/// 轮相对底盘质心的纵向偏移(demo 双轮)。
pub const WHEEL_OFFSETS: [f32; 2] = [-0.9, 0.9];
/// 轻物体初始位置(世界 x)。
pub const LIGHT_OBJ_START_X: f32 = 14.0;
/// 轻物体半宽。
pub const LIGHT_OBJ_HALF: f32 = 0.3;
/// 底盘前缘相对质心偏移。
pub const CHASSIS_FRONT: f32 = 1.2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleInput {
    pub throttle: f32,
    pub brake: f32,
}

/// 固定输入脚本(确定性函数,journal 等价物)。
pub fn scripted_input(tick: u64) -> VehicleInput {
    let throttle = if tick < 20 {
        0.0
    } else if tick < 140 {
        (tick - 20) as f32 / 120.0
    } else {
        1.0
    };
    let brake = if tick >= 200 {
        ((tick - 200) as f32 / 40.0).min(1.0)
    } else {
        0.0
    };
    VehicleInput { throttle, brake }
}

/// 输入日志序列化(journal 形态:每 tick 一行,浮点 = 8 位 hex bit pattern)。
pub fn input_log_line(tick: u64, input: &VehicleInput) -> String {
    format!(
        "{}:{:08x}:{:08x}",
        tick,
        input.throttle.to_bits(),
        input.brake.to_bits()
    )
}

/// 解析输入日志;非法行/乱序/NaN 全 fail-closed。
pub fn parse_input_log(log: &str) -> Result<Vec<VehicleInput>, CaptureError> {
    let mut out = Vec::new();
    for (expect_tick, line) in log.lines().enumerate() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() != 3 {
            return Err(CaptureError::Parse(format!(
                "input log line {expect_tick}: bad arity"
            )));
        }
        let tick: u64 = parts[0]
            .parse()
            .map_err(|e| CaptureError::Parse(format!("input log tick: {e}")))?;
        if tick != expect_tick as u64 {
            return Err(CaptureError::Parse(format!(
                "input log out of order: got {tick} expect {expect_tick}"
            )));
        }
        let tbits = u32::from_str_radix(parts[1], 16)
            .map_err(|e| CaptureError::Parse(format!("throttle bits: {e}")))?;
        let bbits = u32::from_str_radix(parts[2], 16)
            .map_err(|e| CaptureError::Parse(format!("brake bits: {e}")))?;
        let throttle = f32::from_bits(tbits);
        let brake = f32::from_bits(bbits);
        if !throttle.is_finite() || !brake.is_finite() {
            return Err(CaptureError::NanFloat {
                path: format!("input_log[{expect_tick}]"),
            });
        }
        out.push(VehicleInput { throttle, brake });
    }
    Ok(out)
}

/// 解析式地面:bump 位于 x∈(6,9),幅值 0.12m;`bump_scale` 供 falsify 臂摄动。
pub fn ground_height(x: f32, bump_scale: f32) -> f32 {
    if x > 6.0 && x < 9.0 {
        0.12 * bump_scale * ((x - 6.0) / 3.0 * std::f32::consts::PI).sin()
    } else {
        0.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WheelState {
    pub suspension_m: f32,
    pub wheel_rpm: f32,
    pub contact: bool,
}

/// 载具全量状态(版本化块;capture/rollback 进 hash 的全部字段)。
#[derive(Debug, Clone, PartialEq)]
pub struct VehicleState {
    pub tick: u64,
    pub chassis_x: f32,
    pub chassis_vx: f32,
    pub chassis_y: f32,
    pub engine_rpm: f32,
    pub gear: u8,
    pub wheels: Vec<WheelState>,
    pub obj_x: f32,
    pub obj_vx: f32,
}

impl VehicleState {
    pub fn new(wheel_count: usize) -> Self {
        Self {
            tick: 0,
            chassis_x: 0.0,
            chassis_vx: 0.0,
            chassis_y: 0.9,
            engine_rpm: 900.0,
            gear: 0,
            wheels: (0..wheel_count)
                .map(|_| WheelState {
                    suspension_m: 0.4,
                    wheel_rpm: 0.0,
                    contact: false,
                })
                .collect(),
            obj_x: LIGHT_OBJ_START_X,
            obj_vx: 0.0,
        }
    }

    /// canonical 序列化:固定字段序,浮点一律 hex bit pattern,单行无空白。
    pub fn serialize(&self) -> String {
        let wheels = self
            .wheels
            .iter()
            .map(|w| {
                format!(
                    "{{\"suspension_m\":\"{:08x}\",\"wheel_rpm\":\"{:08x}\",\"contact\":{}}}",
                    w.suspension_m.to_bits(),
                    w.wheel_rpm.to_bits(),
                    u8::from(w.contact)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"tick\":{},\"chassis_x\":\"{:08x}\",\"chassis_vx\":\"{:08x}\",\"chassis_y\":\"{:08x}\",\"engine_rpm\":\"{:08x}\",\"gear\":{},\"wheels\":[{}],\"obj_x\":\"{:08x}\",\"obj_vx\":\"{:08x}\"}}",
            self.tick,
            self.chassis_x.to_bits(),
            self.chassis_vx.to_bits(),
            self.chassis_y.to_bits(),
            self.engine_rpm.to_bits(),
            self.gear,
            wheels,
            self.obj_x.to_bits(),
            self.obj_vx.to_bits()
        )
    }

    /// 严格解析 `serialize` 产物;坏字段/坏 hex/NaN/尾部垃圾全 fail-closed。
    pub fn parse(text: &str) -> Result<Self, CaptureError> {
        let mut c = Cursor::new(text);
        c.expect("{\"tick\":")?;
        let tick = c.take_u64_until(",\"chassis_x\":")?;
        let chassis_x = c.take_hex_f32("chassis_x")?;
        c.expect(",\"chassis_vx\":")?;
        let chassis_vx = c.take_hex_f32("chassis_vx")?;
        c.expect(",\"chassis_y\":")?;
        let chassis_y = c.take_hex_f32("chassis_y")?;
        c.expect(",\"engine_rpm\":")?;
        let engine_rpm = c.take_hex_f32("engine_rpm")?;
        c.expect(",\"gear\":")?;
        let gear = c.take_u64_until(",\"wheels\":[")? as u8;
        let mut wheels = Vec::new();
        if !c.rest().starts_with(']') {
            loop {
                c.expect("{\"suspension_m\":")?;
                let suspension_m = c.take_hex_f32("suspension_m")?;
                c.expect(",\"wheel_rpm\":")?;
                let wheel_rpm = c.take_hex_f32("wheel_rpm")?;
                c.expect(",\"contact\":")?;
                let contact = match c.take_until("}")? {
                    "0" => false,
                    "1" => true,
                    other => {
                        return Err(CaptureError::Parse(format!("wheel contact: {other}")));
                    }
                };
                c.expect("}")?;
                wheels.push(WheelState {
                    suspension_m,
                    wheel_rpm,
                    contact,
                });
                match c.take(1)? {
                    "," => continue,
                    "]" => break,
                    other => {
                        return Err(CaptureError::Parse(format!("wheel sep: {other}")));
                    }
                }
            }
        } else {
            c.expect("]")?;
        }
        c.expect(",\"obj_x\":")?;
        let obj_x = c.take_hex_f32("obj_x")?;
        c.expect(",\"obj_vx\":")?;
        let obj_vx = c.take_hex_f32("obj_vx")?;
        c.expect("}")?;
        c.expect_end()?;
        for (path, v) in [
            ("chassis_x", chassis_x),
            ("chassis_vx", chassis_vx),
            ("chassis_y", chassis_y),
            ("engine_rpm", engine_rpm),
            ("obj_x", obj_x),
            ("obj_vx", obj_vx),
        ] {
            if !v.is_finite() {
                return Err(CaptureError::NanFloat { path: path.into() });
            }
        }
        Ok(Self {
            tick,
            chassis_x,
            chassis_vx,
            chassis_y,
            engine_rpm,
            gear,
            wheels,
            obj_x,
            obj_vx,
        })
    }

    pub fn state_hash(&self) -> String {
        hex(&digest(self.serialize().as_bytes()))
    }
}

/// 每 tick 遥测通道(telemetry golden 的前像)。
#[derive(Debug, Clone, Copy)]
pub struct TickTelemetry {
    pub tick: u64,
    pub engine_rpm: f32,
    pub gear: u8,
    pub chassis_vx: f32,
    pub chassis_y: f32,
    pub obj_x: f32,
    pub susp0_m: f32,
    pub contact_pen_m: f32,
}

impl TickTelemetry {
    pub fn canonical_line(&self) -> String {
        format!(
            "{}:{:08x}:{}:{:08x}:{:08x}:{:08x}:{:08x}",
            self.tick,
            self.engine_rpm.to_bits(),
            self.gear,
            self.chassis_vx.to_bits(),
            self.chassis_y.to_bits(),
            self.obj_x.to_bits(),
            self.susp0_m.to_bits()
        )
    }
}

#[derive(Debug, Clone)]
pub struct VehicleSim {
    pub state: VehicleState,
    pub bump_scale: f32,
}

impl VehicleSim {
    pub fn new(asset: &VehicleAsset) -> Self {
        Self {
            state: VehicleState::new(asset.wheels.len()),
            bump_scale: 1.0,
        }
    }

    pub fn from_state(state: VehicleState) -> Self {
        Self {
            state,
            bump_scale: 1.0,
        }
    }

    /// 单步推进;返回该 tick 遥测。固定操作序,同 build 逐位确定。
    pub fn step(&mut self, asset: &VehicleAsset, input: &VehicleInput) -> TickTelemetry {
        let dt = FIXED_DT;
        let s = &mut self.state;
        // 发动机:一阶趋近油门目标转速。
        let target_rpm = 900.0 + input.throttle * 3900.0;
        s.engine_rpm += (target_rpm - s.engine_rpm) * 0.08;
        // 确定性换挡:阈值 + 齿比换算保持连续性。
        let ratios = &asset.gear_ratios;
        let gi = s.gear as usize;
        if s.engine_rpm > 4000.0 && gi + 1 < ratios.len() {
            s.engine_rpm *= ratios[gi + 1] / ratios[gi];
            s.gear += 1;
        } else if s.engine_rpm < 1400.0 && s.gear > 0 {
            s.engine_rpm *= ratios[gi - 1] / ratios[gi];
            s.gear -= 1;
        }
        // 纵向驱动/制动/阻力。
        let drive_acc = input.throttle * ratios[s.gear as usize] * 2.0;
        let brake_acc = if s.chassis_vx > 0.0 {
            input.brake * 6.0
        } else {
            0.0
        };
        let drag = s.chassis_vx * 0.02;
        s.chassis_vx += (drive_acc - brake_acc - drag) * dt;
        if s.chassis_vx < 0.0 {
            s.chassis_vx = 0.0;
        }
        s.chassis_x += s.chassis_vx * dt;
        // 悬挂:轮下地面高度 → 目标压缩量,一阶趋近(raycast 悬挂的解析 fixture 面)。
        let mut ground_sum = 0.0f32;
        for (i, w) in s.wheels.iter_mut().enumerate() {
            let wx = s.chassis_x + WHEEL_OFFSETS[i];
            let g = ground_height(wx, self.bump_scale);
            ground_sum += g;
            let rest = asset.wheels[i].suspension_rest_m;
            w.contact = g > 0.001;
            let target_susp = if w.contact {
                rest - (g * 0.5).min(0.12)
            } else {
                rest
            };
            w.suspension_m += (target_susp - w.suspension_m) * 0.25;
            w.wheel_rpm = if w.contact {
                s.chassis_vx / asset.wheels[i].radius_m
            } else {
                0.0
            };
        }
        let target_y = ground_sum / s.wheels.len() as f32 + 0.9;
        s.chassis_y += (target_y - s.chassis_y) * 0.2;
        // 轮胎推挤轻物体:前缘侵入 → 动量式交换(轻物体低速率被推开)。
        let mut contact_pen = 0.0f32;
        let front = s.chassis_x + CHASSIS_FRONT;
        if front >= s.obj_x - LIGHT_OBJ_HALF && s.chassis_vx > s.obj_vx {
            contact_pen = front - (s.obj_x - LIGHT_OBJ_HALF);
            s.obj_vx = s.chassis_vx * 0.85;
            s.chassis_vx *= 0.97;
        }
        s.obj_x += s.obj_vx * dt;
        s.obj_vx *= 0.995;
        let tele = TickTelemetry {
            tick: s.tick,
            engine_rpm: s.engine_rpm,
            gear: s.gear,
            chassis_vx: s.chassis_vx,
            chassis_y: s.chassis_y,
            obj_x: s.obj_x,
            susp0_m: s.wheels[0].suspension_m,
            contact_pen_m: contact_pen,
        };
        s.tick += 1;
        tele
    }
}

/// 供 strict parser 复用的微型游标(固定字段序,拒绝任何漂移)。
pub(crate) struct Cursor<'a> {
    text: &'a str,
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(text: &'a str) -> Self {
        Self { text, pos: 0 }
    }

    pub(crate) fn rest(&self) -> &'a str {
        &self.text[self.pos..]
    }

    pub(crate) fn take(&mut self, n: usize) -> Result<&'a str, CaptureError> {
        if self.pos + n > self.text.len() {
            return Err(CaptureError::Parse("cursor overrun".into()));
        }
        let s = &self.text[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    pub(crate) fn expect(&mut self, lit: &str) -> Result<(), CaptureError> {
        if self.text[self.pos..].starts_with(lit) {
            self.pos += lit.len();
            Ok(())
        } else {
            Err(CaptureError::Parse(format!(
                "expect {lit:?} at byte {}",
                self.pos
            )))
        }
    }

    pub(crate) fn take_until(&mut self, delim: &str) -> Result<&'a str, CaptureError> {
        match self.text[self.pos..].find(delim) {
            Some(i) => {
                let s = &self.text[self.pos..self.pos + i];
                self.pos += i;
                Ok(s)
            }
            None => Err(CaptureError::Parse(format!("delim {delim:?} not found"))),
        }
    }

    pub(crate) fn take_u64_until(&mut self, delim: &str) -> Result<u64, CaptureError> {
        let s = self.take_until(delim)?;
        self.pos += delim.len();
        s.parse()
            .map_err(|e| CaptureError::Parse(format!("u64: {e}")))
    }

    fn take_hex_f32(&mut self, path: &str) -> Result<f32, CaptureError> {
        self.expect("\"")?;
        let s = self.take_until("\"")?;
        self.pos += 1;
        let bits = u32::from_str_radix(s, 16)
            .map_err(|e| CaptureError::Parse(format!("{path} hex: {e}")))?;
        Ok(f32::from_bits(bits))
    }

    /// 十进制浮点文本(asset canonical 面);NaN/inf 由调用方 fail-closed。
    pub(crate) fn take_f32_until(&mut self, delim: &str, path: &str) -> Result<f32, CaptureError> {
        let s = self.take_until(delim)?;
        s.parse::<f32>()
            .map_err(|e| CaptureError::Parse(format!("{path} f32: {e}")))
    }

    pub(crate) fn expect_end(&self) -> Result<(), CaptureError> {
        if self.pos == self.text.len() {
            Ok(())
        } else {
            Err(CaptureError::Parse(format!(
                "trailing garbage at byte {}",
                self.pos
            )))
        }
    }
}
