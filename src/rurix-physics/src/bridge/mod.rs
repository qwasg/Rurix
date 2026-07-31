//! 渲染合流桥(RFC-0017 §4.B 渲染同步契约章,G6.3 交付;验收门 G-G6-4,CI 步骤 89)。
//!
//! 结构纪律(§4.B 冻结条款字面落地):
//! - §4.B1-1 单向事实源:bridge 只持 `&mut GpuScene` 的变换写口
//!   ([`PhysicsBridge::sync_frame`]),物理 API 不接受渲染侧输入;依赖方向 =
//!   rurix-physics → rurix-render 单向(render 永不反向依赖)。
//! - §4.B2 变换桥:[`compose_transform_3x4`] 为 `PhysicsTransform` → 行主 3×4
//!   的唯一合成口(P-11 单源);写 `GpuScene::update_transform` + 帧末
//!   `flush_dirty` 一次结算;静态/睡眠体零脏写(`active_transforms` 快照
//!   天然排除,bridge 无特判)。
//! - §4.B3 MV 供给:每体记上次写入基线,与当前拍差分产出 [`MotionHint`];
//!   静态/睡眠体零 MV(天然缺席);MV 缓冲格式不冻结(R-4 评审修订),
//!   禁效果 pass 私写重投影维持(RFC-0016 §4.H 0-byte 延续)。
//! - §4.B5 AS 脏信号:[`FrameSyncReport::dirty_instances`] = 本帧实际写入的
//!   实例 id(升序),与 `dirty_ranges` 同帧同源;物理不直接触碰 AS API、
//!   不新建加速结构所有者,信号交 G5 既有 refit 决策树消费。
//! - §4.B4 流送:[`StreamingBridge`] 页驻留批插/页卸载批移除 +
//!   [`RemovalReceipt`] 先卸后放类型纪律(子模块 streaming)。
//!
//! 后端无关:本模块纯 host 类型面,不引用 sys crate,两构建档同编译;
//! 真后端行为测试见 `tests/bridge.rs`(default = jolt 档)。

use std::collections::HashMap;

use rurix_render::geometry::gpu_scene::{DirtyRange, GpuScene};

use crate::budget::SyncBudget;
use crate::id::BodyId;
use crate::types::{BodyKind, PhysicsTransform};
use crate::world::PhysicsWorld;

mod streaming;

pub use streaming::{PageKey, RemovalReceipt, StreamingBridge};

/// `PhysicsTransform` → 行主 3×4 仿射(§4.B2 唯一合成口,P-11 单源;输出直喂
/// `GpuScene::update_transform`,语义 `p'ᵢ = rowᵢ·[p,1]`,平移在第 4 列)。
///
/// 旋转 = xyzw 四元数 → 3×3 标准展开。**不做单位化**——契约:调用方负责传入
/// 单位四元数(`PhysicsTransform::rotation` 文档同责;后端输出恒为单位四元数)。
pub fn compose_transform_3x4(t: &PhysicsTransform) -> [[f32; 4]; 3] {
    let [x, y, z, w] = t.rotation;
    let [tx, ty, tz] = t.translation;
    let (xx, yy, zz) = (x * x, y * y, z * z);
    let (xy, xz, yz) = (x * y, x * z, y * z);
    let (wx, wy, wz) = (w * x, w * y, w * z);
    [
        [1.0 - 2.0 * (yy + zz), 2.0 * (xy - wz), 2.0 * (xz + wy), tx],
        [2.0 * (xy + wz), 1.0 - 2.0 * (xx + zz), 2.0 * (yz - wx), ty],
        [2.0 * (xz - wy), 2.0 * (yz + wx), 1.0 - 2.0 * (xx + yy), tz],
    ]
}

/// 动态体 motion 提示(§4.B3):本帧写入实例的上一拍/当前拍行主 3×4 差分对。
/// MV 缓冲格式不冻结(R-4),本结构即 bridge 对外的差分供给形态;静态/睡眠体
/// 零 MV(天然缺席每帧 hints 列表)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionHint {
    /// `GpuScene` 实例 id。
    pub instance: u32,
    /// 上次成功写入的行主 3×4(首写帧 = `cur_transform`,零位移基线,不产生假 MV)。
    pub prev_transform: [[f32; 4]; 3],
    /// 本帧写入的行主 3×4。
    pub cur_transform: [[f32; 4]; 3],
}

/// 一帧同步结算报告(§4.B2/B3/B5 同帧同源出口;计数进 evidence 不进硬门,P-09)。
#[derive(Debug, Clone, PartialEq)]
pub struct FrameSyncReport {
    /// 本帧 `active_transforms` 中已在 bridge 注册的体数(截断与否都计)。
    pub bodies_seen: u32,
    /// 实际写入 `GpuScene` 的体数(`update_transform` 返回 true 才计)。
    pub bodies_written: u32,
    /// 被 `SyncBudget::max_body_writes` 确定性截断的体数(§4.A6;不 panic,P-01)。
    pub writes_truncated: u32,
    /// 本帧实际写入的实例 id(升序去重)——AS「变换脏实例」信号(§4.B5),
    /// 与 `dirty_ranges` 同帧同源,交 G5 既有 refit 决策树消费。
    pub dirty_instances: Vec<u32>,
    /// 帧末 `GpuScene::flush_dirty` 返回值(合并后半开区间,升序)。
    pub dirty_ranges: Vec<DirtyRange>,
}

/// 已注册 body 槽位:body→instance 映射 + 类型 + 上次写入基线(§4.B3 差分源)。
#[derive(Debug)]
struct TrackedBody {
    instance: u32,
    kind: BodyKind,
    /// 上次成功写入的行主 3×4;`None` = 注册后尚未写入(首写帧 hint 取
    /// prev = cur 零位移基线)。
    last_written: Option<[[f32; 4]; 3]>,
}

/// 物理 → 渲染变换同步桥(§4.B2/B3/B5;单向事实源 §4.B1-1)。
///
/// 使用形态:宿主每帧在所有变换写者之后调 [`sync_frame`](Self::sync_frame)
/// (帧内变换写结算点);`world`/`scene`/`budget` 均由宿主持有,bridge 只记
/// 映射与写入基线,不持任何一侧所有权。渲染只读消费 `GpuScene`,不回写物理。
#[derive(Debug, Default)]
pub struct PhysicsBridge {
    tracked: HashMap<BodyId, TrackedBody>,
    hints: Vec<MotionHint>,
    writes_saturated: u64,
}

impl PhysicsBridge {
    /// 构造空桥(等价 `Default::default()`)。
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册 body→instance 映射(宿主侧场景实例分配后调用;重复注册 = 重映射
    /// 到新 instance,写入基线重置为首写帧零位移)。
    pub fn register(&mut self, body: BodyId, instance: u32, kind: BodyKind) {
        self.tracked.insert(
            body,
            TrackedBody {
                instance,
                kind,
                last_written: None,
            },
        );
    }

    /// 注销映射(body 移除/页卸载后调用;返回原 instance,未知 body → `None`)。
    pub fn unregister(&mut self, body: BodyId) -> Option<u32> {
        self.tracked.remove(&body).map(|e| e.instance)
    }

    /// body → instance 反查。
    pub fn instance_of(&self, body: BodyId) -> Option<u32> {
        self.tracked.get(&body).map(|e| e.instance)
    }

    /// body → 类型反查(注册时登记的 `BodyKind`)。
    pub fn kind_of(&self, body: BodyId) -> Option<BodyKind> {
        self.tracked.get(&body).map(|e| e.kind)
    }

    /// 当前注册的映射条数。
    pub fn tracked_count(&self) -> usize {
        self.tracked.len()
    }

    /// 帧内变换写结算点(§4.B2;**应在所有变换写者之后调用**——帧末
    /// `flush_dirty` 一次结算全帧脏集)。
    ///
    /// 流程(确定性,§4.0-4):遍历 `world.active_transforms()`(只含 active
    /// 动态/运动体,已按 `BodyId` 升序;静态/睡眠体天然排除 = 零脏写零 MV,
    /// §4.A3/§4.B2)→ 未注册 body 跳过 → 每体先消耗
    /// `SyncBudget::try_consume_body_write`,耗尽 → `writes_truncated` 计数并
    /// 跳过(确定性截断,不 panic,P-01)→ 额度内合成 3×4 写 `GpuScene`;
    /// 写入成功(`update_transform` 返回 true)才更新该体写入基线与 motion
    /// hint(prev = 上次写入值,cur = 新值,§4.B3);注册 instance 越界(场景侧
    /// 已删实例)→ 确定性跳过(不计 written、不动基线/hint)。
    pub fn sync_frame(
        &mut self,
        world: &PhysicsWorld,
        scene: &mut GpuScene,
        budget: &mut SyncBudget,
    ) -> FrameSyncReport {
        self.hints.clear();
        let mut bodies_seen = 0u32;
        let mut bodies_written = 0u32;
        let mut writes_truncated = 0u32;
        let mut dirty_instances: Vec<u32> = Vec::new();
        for (body, transform) in world.active_transforms() {
            let Some(entry) = self.tracked.get_mut(&body) else {
                continue;
            };
            bodies_seen = bodies_seen.saturating_add(1);
            // 静态体零脏写(§4.B2):active_transforms 已排除静态/睡眠体,
            // 本断言为映射登记面双保险(登记类型与快照口径不一致 = 宿主 bug)。
            debug_assert!(
                entry.kind != BodyKind::Static,
                "静态体不应出现在 active_transforms"
            );
            if !budget.try_consume_body_write() {
                writes_truncated = writes_truncated.saturating_add(1);
                continue;
            }
            let cur = compose_transform_3x4(&transform);
            if scene.update_transform(entry.instance, cur) {
                let prev = entry.last_written.unwrap_or(cur);
                self.hints.push(MotionHint {
                    instance: entry.instance,
                    prev_transform: prev,
                    cur_transform: cur,
                });
                entry.last_written = Some(cur);
                bodies_written = bodies_written.saturating_add(1);
                dirty_instances.push(entry.instance);
            }
        }
        dirty_instances.sort_unstable();
        dirty_instances.dedup();
        self.writes_saturated = self
            .writes_saturated
            .saturating_add(u64::from(writes_truncated));
        FrameSyncReport {
            bodies_seen,
            bodies_written,
            writes_truncated,
            dirty_instances,
            dirty_ranges: scene.flush_dirty(),
        }
    }

    /// 本帧 motion hints(每次 `sync_frame` 重建,只含本帧实际写入的体;
    /// 睡眠体上周期的 prev 在唤醒后 ≈ cur——睡眠期未动,不产生假 MV,§4.B3)。
    pub fn motion_hints(&self) -> &[MotionHint] {
        &self.hints
    }

    /// 预算截断累计计数(单调,saturating;§4.A6 饱和计数上报出口,
    /// 计数进 evidence 不进硬门)。
    pub fn writes_saturated_total(&self) -> u64 {
        self.writes_saturated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BodyKind;

    fn approx34(a: &[[f32; 4]; 3], b: &[[f32; 4]; 3]) -> bool {
        (0..3).all(|i| (0..4).all(|j| (a[i][j] - b[i][j]).abs() <= 1e-6))
    }

    #[test]
    fn compose_identity_transform() {
        let m = compose_transform_3x4(&PhysicsTransform::IDENTITY);
        let expect: [[f32; 4]; 3] = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ];
        assert_eq!(m, expect, "恒等变换 → 单位 3×4(逐位)");
    }

    #[test]
    fn compose_pure_translation() {
        let t = PhysicsTransform {
            translation: [3.0, -2.0, 7.5],
            rotation: [0.0, 0.0, 0.0, 1.0],
        };
        let m = compose_transform_3x4(&t);
        let expect: [[f32; 4]; 3] = [
            [1.0, 0.0, 0.0, 3.0],
            [0.0, 1.0, 0.0, -2.0],
            [0.0, 0.0, 1.0, 7.5],
        ];
        assert_eq!(m, expect, "纯平移:旋转部单位,平移入第 4 列");
    }

    #[test]
    fn compose_rotation_z_90() {
        // 绕 Z +90°:quat(xyzw) = (0, 0, sin45°, cos45°);R·(1,0,0) = (0,1,0)。
        let s = std::f32::consts::FRAC_1_SQRT_2;
        let t = PhysicsTransform {
            translation: [1.0, 2.0, 3.0],
            rotation: [0.0, 0.0, s, s],
        };
        let m = compose_transform_3x4(&t);
        let expect: [[f32; 4]; 3] = [
            [0.0, -1.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 2.0],
            [0.0, 0.0, 1.0, 3.0],
        ];
        assert!(approx34(&m, &expect), "绕 Z 90° 已知值不符:{m:?}");
    }

    #[test]
    fn compose_rotation_x_90() {
        // 绕 X +90°:quat = (sin45°, 0, 0, cos45°);R·(0,1,0) = (0,0,1)。
        let s = std::f32::consts::FRAC_1_SQRT_2;
        let t = PhysicsTransform {
            translation: [0.0; 3],
            rotation: [s, 0.0, 0.0, s],
        };
        let m = compose_transform_3x4(&t);
        let expect: [[f32; 4]; 3] = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
        ];
        assert!(approx34(&m, &expect), "绕 X 90° 已知值不符:{m:?}");
    }

    #[test]
    fn compose_no_normalization_caller_duty() {
        // 契约:不调单位化——非单位四元数(2,0,0,0)按公式直通(缩放 4 倍进矩阵)。
        let t = PhysicsTransform {
            translation: [0.0; 3],
            rotation: [2.0, 0.0, 0.0, 0.0],
        };
        let m = compose_transform_3x4(&t);
        let expect: [[f32; 4]; 3] = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, -7.0, 0.0, 0.0],
            [0.0, 0.0, -7.0, 0.0],
        ];
        assert_eq!(m, expect, "非单位四元数直进矩阵,不做单位化(逐位)");
    }

    #[test]
    fn bridge_register_unregister_mapping() {
        let mut bridge = PhysicsBridge::new();
        let b0 = BodyId::new(0, 1);
        let b1 = BodyId::new(1, 1);
        assert_eq!(bridge.tracked_count(), 0);
        assert_eq!(bridge.instance_of(b0), None);
        assert_eq!(bridge.unregister(b0), None, "未知 body 注销 → None");
        bridge.register(b0, 10, BodyKind::Dynamic);
        bridge.register(b1, 20, BodyKind::Kinematic);
        assert_eq!(bridge.tracked_count(), 2);
        assert_eq!(bridge.instance_of(b0), Some(10));
        assert_eq!(bridge.instance_of(b1), Some(20));
        assert_eq!(bridge.kind_of(b0), Some(BodyKind::Dynamic));
        assert_eq!(bridge.kind_of(b1), Some(BodyKind::Kinematic));
        // 重复注册 = 重映射。
        bridge.register(b0, 11, BodyKind::Dynamic);
        assert_eq!(bridge.instance_of(b0), Some(11));
        assert_eq!(bridge.tracked_count(), 2);
        // 注销返回原 instance,映射摘除。
        assert_eq!(bridge.unregister(b0), Some(11));
        assert_eq!(bridge.instance_of(b0), None);
        assert_eq!(bridge.tracked_count(), 1);
    }

    #[test]
    fn bridge_default_equals_new_empty_state() {
        let bridge = PhysicsBridge::new();
        assert_eq!(bridge.tracked_count(), 0);
        assert!(bridge.motion_hints().is_empty());
        assert_eq!(bridge.writes_saturated_total(), 0);
        let defaulted = PhysicsBridge::default();
        assert_eq!(defaulted.tracked_count(), 0);
        assert_eq!(defaulted.writes_saturated_total(), 0);
    }
}
