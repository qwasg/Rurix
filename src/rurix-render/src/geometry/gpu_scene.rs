//! GPU 场景扁平化(报告6 §2.3;RFC-0016 章 G 前半)。
//!
//! 场景唯一事实来源的 host 侧装配:网格表(簇范围 + 对象空间 AABB)+ 扁平
//! 实例表([`InstanceRecord`],GPU buffer 元素)——Nanite 式"Load 阶段拍平场
//! 景图与实例列表到 GPU,应用改场景图时增量更新"。增量更新以**脏区间**表达:
//! [`GpuScene::update_transform`] 标脏,[`GpuScene::flush_dirty`] 返回合并后的
//! 半开区间列表并清零,供上传预算计量(报告6 §2.3/§2.4 预算化思想)。
//!
//! 两级实例化留口(报告6 §2.3 Nanite Assemblies 思想的最小实现):
//! [`GpuScene::register_part_group`] 注册部件组(同 mesh 多实例共享 part 表),
//! [`GpuScene::instantiate_group`] 展开为多条 [`InstanceRecord`],变换复合
//! (组变换 × 部件局部变换),父子关系记录于 `reserved` 字段(placement id)。

use std::collections::BTreeSet;

/// 实例 flags 位:本记录由部件组展开产生(两级实例化标记;剔除 descent 的
/// 二级间接扩展点,报告1 culling.rs 留口)。
pub const INSTANCE_FLAG_PART_EXPANSION: u32 = 1 << 0;

/// `reserved` 语义:父 placement id;无父(根实例)= [`NO_PARENT`]。
pub const NO_PARENT: u32 = u32::MAX;

/// 单位 3×4 仿射(行主)。
pub const IDENTITY_3X4: [[f32; 4]; 3] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
];

/// 扁平实例记录(GPU buffer 元素,`repr(C)` 定长,字段序冻结;96B = 6×16B,
/// 16 字节段对齐友好:变换 48B | 簇区间/材质/flags 16B | AABB+mesh+reserved 32B)。
///
/// 布局锁定单测见 `instance_record_layout`(size/offset 以 `core::mem::offset_of!`
/// 锚定,改动即红)。
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InstanceRecord {
    /// 行主 3×4 仿射(对象 → 世界;p'ᵢ = rowᵢ·[p,1])。
    pub transform: [[f32; 4]; 3],
    /// 全局簇表内本网格簇段起始偏移(add_mesh 注册)。
    pub cluster_offset: u32,
    /// 簇段长度。
    pub cluster_count: u32,
    /// 材质 id([`crate::material::MaterialTable`] 注册号;classify 分类键)。
    pub material_id: u32,
    /// 实例 flags(见 `INSTANCE_FLAG_*`)。
    pub flags: u32,
    /// 世界空间 AABB 下界(add/update 时由网格对象 AABB 经变换得出;剔除直接消费)。
    pub aabb_min: [f32; 3],
    /// 网格 id(网格表下标;几何解析键)。
    pub mesh_id: u32,
    /// 世界空间 AABB 上界。
    pub aabb_max: [f32; 3],
    /// 父 placement id(两级实例化留口;根实例 = [`NO_PARENT`])。
    pub reserved: u32,
}

/// 网格表条目(对象空间几何范围;簇段 + AABB)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshEntry {
    pub cluster_offset: u32,
    pub cluster_count: u32,
    pub aabb_min: [f32; 3],
    pub aabb_max: [f32; 3],
}

/// 脏区间(实例表下标半开区间 [start, end);上传预算按区间计量)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirtyRange {
    pub start: u32,
    pub end: u32,
}

/// World-Field 只读 buffer 槽(RFC-0024 v1.1 章 F2 🔒 显式修订行授权加性
/// 面;spec/physics.md RXS-0374 L4;RFC-0019 §8 `GpuScene` 冻结面加性修订)。
///
/// 物理侧按 tick(`PhysicsTickId`)经既有 Physics→GpuScene 桥把场采样参数
/// 提交为本槽;**渲染/VFX/材质侧只读消费、零回写**(纪律 1 单向事实源
/// 0-byte);时间域归属 `WorldFieldSampleSet` → `RenderFrameId` 显式映射
/// (R-4 🔒 字面不变)。类型面不依赖 physics crate(依赖方向 =
/// physics → render 单向,RFC-0017 §4.B1-1),tick/frame 以 u64 位面值承载。
#[derive(Debug, Clone, PartialEq)]
pub struct WorldFieldSlot {
    /// 场参数提交 tick(物理时间域 `PhysicsTickId` 位面值)。
    pub physics_tick: u64,
    /// 归属渲染帧(`RenderFrameId` 位面值;`FrameDomainMap` 显式映射)。
    pub render_frame: u64,
    /// 场采样参数 canonical 字节(场 digest × 参数位表示;只读载荷)。
    pub payload: Vec<u8>,
}

/// 渲染侧写 World-Field 缓冲的统一拒绝(RXS-0374 L4 旁路写注入 RED 臂的
/// fail-closed typed Err 锚;零回写纪律 0-byte)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldFieldWriteError {
    /// 渲染侧写/回写尝试一律拒绝(World-Field 唯一写口 = Physics→GpuScene
    /// 桥 `commit_world_field`;渲染侧只读消费)。
    RenderWriteRejected,
}

impl core::fmt::Display for WorldFieldWriteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::RenderWriteRejected => write!(
                f,
                "RenderWriteRejected: World-Field buffer is read-only on render side \
                 (RFC-0024 v1.1 F2; unique writer = Physics->GpuScene bridge)"
            ),
        }
    }
}

impl std::error::Error for WorldFieldWriteError {}

/// 部件定义(部件组 = 同 mesh 多实例共享的 part 表)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PartDef {
    pub mesh_id: u32,
    /// 部件局部变换(组 → 部件;实例化时 组变换 × 局部变换 复合)。
    pub local_transform: [[f32; 4]; 3],
    pub material_id: u32,
    pub flags: u32,
}

/// 一次组实例化的落位记录(父子关系查询面;placement id = 下标)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroupPlacement {
    /// 部件组 id。
    pub group: u32,
    /// 组(根)变换。
    pub transform: [[f32; 4]; 3],
    /// 展开实例在实例表内的起始下标。
    pub instance_start: u32,
    /// 展开实例数量(= 组内部件数)。
    pub instance_count: u32,
}

/// 3×4 仿射复合:a∘b(先 b 后 a;b 视为末行 [0,0,0,1] 的 4×4)。
pub fn compose_transform(a: &[[f32; 4]; 3], b: &[[f32; 4]; 3]) -> [[f32; 4]; 3] {
    let mut out = [[0.0; 4]; 3];
    for (i, row) in out.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = if j < 3 {
                a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j]
            } else {
                a[i][0] * b[0][3] + a[i][1] * b[1][3] + a[i][2] * b[2][3] + a[i][3]
            };
        }
    }
    out
}

/// 行主 3×4 仿射作用于点。
pub fn transform_point(m: &[[f32; 4]; 3], p: [f32; 3]) -> [f32; 3] {
    let mut out = [0.0; 3];
    for (i, o) in out.iter_mut().enumerate() {
        *o = m[i][0] * p[0] + m[i][1] * p[1] + m[i][2] * p[2] + m[i][3];
    }
    out
}

/// 对象 AABB 经仿射变换的世界 AABB(8 角点变换取分量 min/max;线性映射下盒体
/// 像为平行六面体,其 AABB 必在角点取得,结果精确非保守放大)。
fn transform_aabb(min: [f32; 3], max: [f32; 3], m: &[[f32; 4]; 3]) -> ([f32; 3], [f32; 3]) {
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for &x in &[min[0], max[0]] {
        for &y in &[min[1], max[1]] {
            for &z in &[min[2], max[2]] {
                let w = transform_point(m, [x, y, z]);
                for k in 0..3 {
                    lo[k] = lo[k].min(w[k]);
                    hi[k] = hi[k].max(w[k]);
                }
            }
        }
    }
    (lo, hi)
}

/// GPU 场景(扁平实例表 + 网格表 + 增量脏跟踪 + 部件组两级实例化留口)。
#[derive(Debug, Default)]
pub struct GpuScene {
    meshes: Vec<MeshEntry>,
    instances: Vec<InstanceRecord>,
    dirty: BTreeSet<u32>,
    groups: Vec<Vec<PartDef>>,
    placements: Vec<GroupPlacement>,
    /// World-Field 只读 buffer 槽序列(RFC-0024 v1.1 F2 🔒 加性面;提交序 =
    /// tick 序;**唯一写口 = `commit_world_field`(Physics→GpuScene 桥专用),
    /// 渲染侧经 `world_field_slots` 只读消费、零回写**;既有面字段 0-byte)。
    world_field_slots: Vec<WorldFieldSlot>,
}

impl GpuScene {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册网格(簇范围 + 对象空间 AABB)→ mesh_id。
    pub fn add_mesh(
        &mut self,
        cluster_offset: u32,
        cluster_count: u32,
        aabb_min: [f32; 3],
        aabb_max: [f32; 3],
    ) -> u32 {
        let id = u32::try_from(self.meshes.len()).expect("网格数超 u32");
        self.meshes.push(MeshEntry {
            cluster_offset,
            cluster_count,
            aabb_min,
            aabb_max,
        });
        id
    }

    /// 注册实例(变换 + 材质)→ instance_id(实例表下标)。簇范围/AABB 取自
    /// 网格表,AABB 立即变换为世界空间;新实例标脏(首批上传覆盖)。
    pub fn add_instance(
        &mut self,
        mesh_id: u32,
        transform: [[f32; 4]; 3],
        material_id: u32,
        flags: u32,
    ) -> u32 {
        let mesh = self.meshes[mesh_id as usize];
        let (lo, hi) = transform_aabb(mesh.aabb_min, mesh.aabb_max, &transform);
        let id = u32::try_from(self.instances.len()).expect("实例数超 u32");
        self.instances.push(InstanceRecord {
            transform,
            cluster_offset: mesh.cluster_offset,
            cluster_count: mesh.cluster_count,
            material_id,
            flags,
            aabb_min: lo,
            mesh_id,
            aabb_max: hi,
            reserved: NO_PARENT,
        });
        self.dirty.insert(id);
        id
    }

    /// 更新实例变换(标脏 + 世界 AABB 重算;增量上传预算服务的唯一改写口)。
    ///
    /// 组展开实例经此口改写后脱离组联动(视为拍平后的独立记录);组整体运动
    /// 应重新 instantiate(留口,报告6 §2.3)。
    pub fn update_transform(&mut self, instance_id: u32, transform: [[f32; 4]; 3]) -> bool {
        let Some(rec) = self.instances.get_mut(instance_id as usize) else {
            return false;
        };
        let mesh = self.meshes[rec.mesh_id as usize];
        let (lo, hi) = transform_aabb(mesh.aabb_min, mesh.aabb_max, &transform);
        rec.transform = transform;
        rec.aabb_min = lo;
        rec.aabb_max = hi;
        self.dirty.insert(instance_id);
        true
    }

    /// 扁平实例表导出(GPU buffer 上传源;下标 = instance_id)。
    pub fn instances(&self) -> &[InstanceRecord] {
        &self.instances
    }

    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }

    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }

    /// 当前待上传脏下标数(预算审计用)。
    pub fn dirty_count(&self) -> usize {
        self.dirty.len()
    }

    /// 返回合并后的脏区间列表(升序、相邻合并、半开 [start, end))并清零脏
    /// 状态——一次 flush 对应一次上传预算结算。
    pub fn flush_dirty(&mut self) -> Vec<DirtyRange> {
        let mut out: Vec<DirtyRange> = Vec::new();
        for &i in &self.dirty {
            match out.last_mut() {
                Some(last) if i <= last.end => last.end = i + 1,
                _ => out.push(DirtyRange {
                    start: i,
                    end: i + 1,
                }),
            }
        }
        self.dirty.clear();
        out
    }

    /// 注册部件组(同 mesh 多实例共享 part 表)→ 组 id(报告6 §2.3:重复结构
    /// 不重复存储;单层,组不可再套组——Epic 限制照抄)。
    pub fn register_part_group(&mut self, parts: Vec<PartDef>) -> u32 {
        let id = u32::try_from(self.groups.len()).expect("部件组数超 u32");
        self.groups.push(parts);
        id
    }

    /// 组实例化:每个部件展开为一条 [`InstanceRecord`](变换复合 组 × 局部;
    /// flags 置 [`INSTANCE_FLAG_PART_EXPANSION`];`reserved` 记父 placement id)。
    /// 返回 placement id(父子关系经 [`GpuScene::placement`] 反查)。
    pub fn instantiate_group(&mut self, group: u32, transform: [[f32; 4]; 3]) -> u32 {
        let parts = self.groups[group as usize].clone();
        let start = u32::try_from(self.instances.len()).expect("实例数超 u32");
        let placement = u32::try_from(self.placements.len()).expect("placement 数超 u32");
        for part in &parts {
            let mesh = self.meshes[part.mesh_id as usize];
            let world = compose_transform(&transform, &part.local_transform);
            let (lo, hi) = transform_aabb(mesh.aabb_min, mesh.aabb_max, &world);
            let id = u32::try_from(self.instances.len()).expect("实例数超 u32");
            self.instances.push(InstanceRecord {
                transform: world,
                cluster_offset: mesh.cluster_offset,
                cluster_count: mesh.cluster_count,
                material_id: part.material_id,
                flags: part.flags | INSTANCE_FLAG_PART_EXPANSION,
                aabb_min: lo,
                mesh_id: part.mesh_id,
                aabb_max: hi,
                reserved: placement,
            });
            self.dirty.insert(id);
        }
        self.placements.push(GroupPlacement {
            group,
            transform,
            instance_start: start,
            instance_count: u32::try_from(parts.len()).expect("部件数超 u32"),
        });
        placement
    }

    /// placement 反查(父子关系记录;展开实例的 `reserved` 即其下标)。
    pub fn placement(&self, placement: u32) -> Option<&GroupPlacement> {
        self.placements.get(placement as usize)
    }

    /// World-Field 提交口(RFC-0024 v1.1 F2 🔒 修订行授权;**调用方纪律 =
    /// 既有 Physics→GpuScene 桥**按 tick 提交场采样参数,渲染侧不得调用)。
    /// 加性面:不触实例表/脏跟踪等任何既有字段(GpuScene 既有面 0-byte)。
    pub fn commit_world_field(&mut self, slot: WorldFieldSlot) {
        self.world_field_slots.push(slot);
    }

    /// World-Field 只读消费面(渲染/VFX/材质侧;**只读**——类型面不存在
    /// `&mut` 访问器,零回写纪律由类型面 + `render_write_world_field`
    /// fail-closed 守卫双重承载)。
    pub fn world_field_slots(&self) -> &[WorldFieldSlot] {
        &self.world_field_slots
    }

    /// 渲染侧写 World-Field 缓冲尝试的统一入口(RXS-0374 L4 RED 臂锚):
    /// **任何渲染侧写/回写尝试 fail-closed typed `Err`**——本守卫是渲染侧
    /// 唯一可寻址的写形态,恒拒绝且不产生任何状态变化。
    pub fn render_write_world_field(
        &mut self,
        _slot: WorldFieldSlot,
    ) -> Result<(), WorldFieldWriteError> {
        Err(WorldFieldWriteError::RenderWriteRejected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx3(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|k| (a[k] - b[k]).abs() <= 1e-6)
    }

    fn approx34(a: &[[f32; 4]; 3], b: &[[f32; 4]; 3]) -> bool {
        (0..3).all(|i| (0..4).all(|j| (a[i][j] - b[i][j]).abs() <= 1e-6))
    }

    fn translation(x: f32, y: f32, z: f32) -> [[f32; 4]; 3] {
        [[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z]]
    }

    #[test]
    fn instance_record_layout() {
        // 布局锁定(size/offset;改动即红——GPU buffer 元素冻结纪律同 graph::types)。
        assert_eq!(core::mem::size_of::<InstanceRecord>(), 96);
        assert_eq!(core::mem::align_of::<InstanceRecord>(), 4);
        assert_eq!(core::mem::offset_of!(InstanceRecord, transform), 0);
        assert_eq!(core::mem::offset_of!(InstanceRecord, cluster_offset), 48);
        assert_eq!(core::mem::offset_of!(InstanceRecord, cluster_count), 52);
        assert_eq!(core::mem::offset_of!(InstanceRecord, material_id), 56);
        assert_eq!(core::mem::offset_of!(InstanceRecord, flags), 60);
        assert_eq!(core::mem::offset_of!(InstanceRecord, aabb_min), 64);
        assert_eq!(core::mem::offset_of!(InstanceRecord, mesh_id), 76);
        assert_eq!(core::mem::offset_of!(InstanceRecord, aabb_max), 80);
        assert_eq!(core::mem::offset_of!(InstanceRecord, reserved), 92);
    }

    #[test]
    fn add_and_export_flat_table() {
        let mut s = GpuScene::new();
        let m0 = s.add_mesh(0, 64, [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let m1 = s.add_mesh(64, 128, [-2.0, -1.0, 0.0], [2.0, 1.0, 3.0]);
        assert_eq!((m0, m1), (0, 1));
        let i0 = s.add_instance(m0, translation(10.0, 0.0, 0.0), 7, 0);
        let i1 = s.add_instance(m1, IDENTITY_3X4, 8, 0);
        let i2 = s.add_instance(m0, translation(0.0, -5.0, 2.0), 7, 0);
        assert_eq!((i0, i1, i2), (0, 1, 2));
        let t = s.instances();
        assert_eq!(t.len(), 3);
        // 簇范围/材质/网格键随记录拍平。
        assert_eq!(t[0].cluster_offset, 0);
        assert_eq!(t[0].cluster_count, 64);
        assert_eq!(t[0].material_id, 7);
        assert_eq!(t[1].cluster_offset, 64);
        assert_eq!(t[1].cluster_count, 128);
        assert_eq!(t[2].mesh_id, m0);
        assert_eq!(t[0].reserved, NO_PARENT);
        // 世界 AABB:网格对象 AABB 平移 (10,0,0)。
        assert!(approx3(t[0].aabb_min, [10.0, 0.0, 0.0]));
        assert!(approx3(t[0].aabb_max, [11.0, 1.0, 1.0]));
        assert!(approx3(t[1].aabb_min, [-2.0, -1.0, 0.0]));
        assert!(approx3(t[2].aabb_min, [0.0, -5.0, 2.0]));
        assert_eq!(s.mesh_count(), 2);
        assert_eq!(s.instance_count(), 3);
    }

    #[test]
    fn dirty_tracking_incremental_merge() {
        let mut s = GpuScene::new();
        let m = s.add_mesh(0, 1, [0.0; 3], [1.0; 3]);
        for _ in 0..5 {
            s.add_instance(m, IDENTITY_3X4, 0, 0);
        }
        // 新增即脏:首批上传覆盖全表 → 单区间 [0,5)。
        assert_eq!(s.flush_dirty(), vec![DirtyRange { start: 0, end: 5 }]);
        // 清零后再 flush 为空。
        assert!(s.flush_dirty().is_empty());
        // 单点更新 → 单点区间。
        assert!(s.update_transform(2, translation(1.0, 0.0, 0.0)));
        assert_eq!(s.flush_dirty(), vec![DirtyRange { start: 2, end: 3 }]);
        // 相邻合并:{1,2,3} → [1,4);{1,3} 不合并 → 两区间。
        s.update_transform(1, IDENTITY_3X4);
        s.update_transform(2, IDENTITY_3X4);
        s.update_transform(3, IDENTITY_3X4);
        assert_eq!(s.dirty_count(), 3);
        assert_eq!(s.flush_dirty(), vec![DirtyRange { start: 1, end: 4 }]);
        s.update_transform(1, IDENTITY_3X4);
        s.update_transform(3, IDENTITY_3X4);
        assert_eq!(
            s.flush_dirty(),
            vec![
                DirtyRange { start: 1, end: 2 },
                DirtyRange { start: 3, end: 4 }
            ]
        );
        // 越界更新:不标脏、返回 false。
        assert!(!s.update_transform(99, IDENTITY_3X4));
        assert!(s.flush_dirty().is_empty());
    }

    #[test]
    fn update_transform_recomputes_aabb() {
        let mut s = GpuScene::new();
        let m = s.add_mesh(0, 1, [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let i = s.add_instance(m, IDENTITY_3X4, 0, 0);
        s.flush_dirty();
        assert!(s.update_transform(i, translation(4.0, 5.0, 6.0)));
        let r = &s.instances()[i as usize];
        assert!(approx3(r.aabb_min, [4.0, 5.0, 6.0]));
        assert!(approx3(r.aabb_max, [5.0, 6.0, 7.0]));
        assert_eq!(
            s.flush_dirty(),
            vec![DirtyRange {
                start: i,
                end: i + 1
            }]
        );
    }

    #[test]
    fn group_instantiation_expands_and_composes() {
        let mut s = GpuScene::new();
        let m = s.add_mesh(0, 32, [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        // 部件组:两个部件,局部平移 (1,0,0) 与 (0,2,0)。
        let g = s.register_part_group(vec![
            PartDef {
                mesh_id: m,
                local_transform: translation(1.0, 0.0, 0.0),
                material_id: 3,
                flags: 0,
            },
            PartDef {
                mesh_id: m,
                local_transform: translation(0.0, 2.0, 0.0),
                material_id: 4,
                flags: 0,
            },
        ]);
        assert_eq!(g, 0);
        // 组变换 = 绕 z 旋转 90° + 平移 (10,0,0):R·(x,y,z) = (-y,x,z)。
        let group_t: [[f32; 4]; 3] = [
            [0.0, -1.0, 0.0, 10.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ];
        let placement = s.instantiate_group(g, group_t);
        assert_eq!(placement, 0);
        let t = s.instances();
        assert_eq!(t.len(), 2);
        // 复合正确:R·(1,0,0)+t = (10,1,0);R·(0,2,0)+t = (8,0,0)。
        let expect0: [[f32; 4]; 3] = [
            [0.0, -1.0, 0.0, 10.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0, 0.0],
        ];
        let expect1: [[f32; 4]; 3] = [
            [0.0, -1.0, 0.0, 8.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ];
        assert!(approx34(&t[0].transform, &expect0));
        assert!(approx34(&t[1].transform, &expect1));
        assert!(approx3(
            transform_point(&t[0].transform, [0.0, 0.0, 0.0]),
            [10.0, 1.0, 0.0]
        ));
        assert!(approx3(
            transform_point(&t[1].transform, [0.0, 0.0, 0.0]),
            [8.0, 0.0, 0.0]
        ));
        // 父子关系:reserved = placement id;placement 反查展开区间。
        assert_eq!(t[0].reserved, placement);
        assert_eq!(t[1].reserved, placement);
        assert_eq!(
            t[0].flags & INSTANCE_FLAG_PART_EXPANSION,
            INSTANCE_FLAG_PART_EXPANSION
        );
        let p = s.placement(placement).expect("placement 存在");
        assert_eq!(p.group, g);
        assert_eq!(p.instance_start, 0);
        assert_eq!(p.instance_count, 2);
        assert!(approx34(&p.transform, &group_t));
        // 材质随部件拍平;世界 AABB 经复合变换(部件0:单位盒 R·box + (10,1,0))。
        assert_eq!(t[0].material_id, 3);
        assert_eq!(t[1].material_id, 4);
        assert!(approx3(t[0].aabb_min, [9.0, 1.0, 0.0]));
        assert!(approx3(t[0].aabb_max, [10.0, 2.0, 1.0]));
        // 展开实例全部标脏(上传覆盖)。
        assert_eq!(s.flush_dirty(), vec![DirtyRange { start: 0, end: 2 }]);
        // 二次落位:placement 递增,父子关系独立。
        let p2 = s.instantiate_group(g, translation(100.0, 0.0, 0.0));
        assert_eq!(p2, 1);
        assert_eq!(s.instances()[2].reserved, p2);
        assert_eq!(s.instances()[3].reserved, p2);
    }

    //@ spec: RXS-0374
    #[test]
    fn world_field_readonly_face_commit_consume_and_write_rejected() {
        // F2 修订行面:桥提交 → 渲染只读消费;渲染侧写尝试 = typed Err 且
        // 零状态变化;既有面(脏跟踪/实例表)不受提交影响。
        let mut s = GpuScene::new();
        let m = s.add_mesh(0, 1, [0.0; 3], [1.0; 3]);
        let inst = s.add_instance(m, IDENTITY_3X4, 0, 0);
        let slot = |tick: u64| WorldFieldSlot {
            physics_tick: tick,
            render_frame: tick * 2,
            payload: vec![tick as u8; 4],
        };
        s.commit_world_field(slot(0));
        s.commit_world_field(slot(1));
        // 渲染只读消费:可读、逐位一致、提交序保持。
        let slots = s.world_field_slots();
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0], slot(0));
        assert_eq!(slots[1], slot(1));
        // 渲染侧写/回写尝试 → fail-closed typed Err,槽序列零变化。
        let before = s.world_field_slots().to_vec();
        let e = s
            .render_write_world_field(slot(9))
            .expect_err("渲染侧写必须拒绝");
        assert_eq!(e, WorldFieldWriteError::RenderWriteRejected);
        assert_eq!(s.world_field_slots(), before.as_slice());
        // 既有面 0-byte:World-Field 提交不触实例表/脏跟踪。
        assert_eq!(s.instance_count(), 1);
        assert!(s.update_transform(inst, IDENTITY_3X4));
    }

    #[test]
    fn compose_transform_associativity_sanity() {
        // 复合 = 先 b 后 a:对采样点等价于逐次变换。
        let a: [[f32; 4]; 3] = [
            [2.0, 0.0, 0.0, 1.0],
            [0.0, 2.0, 0.0, 2.0],
            [0.0, 0.0, 2.0, 3.0],
        ];
        let b = translation(10.0, 20.0, 30.0);
        let ab = compose_transform(&a, &b);
        let p = [1.0, 2.0, 3.0];
        let via_steps = transform_point(&a, transform_point(&b, p));
        let via_composed = transform_point(&ab, p);
        assert!(approx3(via_steps, via_composed));
        // 单位元。
        assert!(approx34(&compose_transform(&IDENTITY_3X4, &b), &b));
        assert!(approx34(&compose_transform(&a, &IDENTITY_3X4), &a));
    }
}
