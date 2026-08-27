//! VSM host 系统:页标记/分配/失效三 pass + 多视图深度光栅 + 投影采样
//! (报告3 §2.1 四机制咬合、§5 pass/缓冲清单;RFC-0016 §4.D2–D3;StratusGFX
//! SVSM 骨架参照)。
//!
//! 帧流程(P0/P1 保守失效起步,报告3 §2.1 实现代价):
//! 1. [`Vsm::begin_frame`][]:帧号推进、驻留页帧龄 +1(饱和)、按相机重排
//!    clipmap 窗口(原点页粒度 snap,平移触发环形更新带标脏,toroidal
//!    语义见 [`crate::shadow::clipmap`]);
//! 2. [`Vsm::page_mark`][]:主相机深度缓冲逐像素反投影 → 选级 → 标记所需页;
//! 3. [`Vsm::page_alloc`][]:紧凑请求(近级优先)→ 共享池分配,不足按帧龄
//!    LRU 驱逐(本帧标记页不可驱逐);
//! 4. [`Vsm::invalidate_aabb`] / [`Vsm::invalidate_light_direction`][]:
//!    失效三源(图元移动/灯变/级联原点平移——第三源在 begin_frame 内自动处理);
//! 5. [`Vsm::shadow_depth_raster`][]:多视图(每 clipmap 级一视图,接口首日
//!    按视图数组设计——报告3 §2.5 VSM 是剔除管线的多视图客户)CPU 边函数
//!    光栅,仅处理「脏且驻留」页并清脏;
//! 6. [`Vsm::sample_shadow`][]:投影采样硬阴影(0/1)。
//!
//! 跨帧纪律(RFC-0016 §4.0-3):页表与物理页池为**跨帧外部资源**(render graph
//! imported 语义,不入 transient 池;device 接线属 W3,本模块为对拍金标准)。

use crate::geometry::gpu_scene::{InstanceRecord, transform_point};
use crate::geometry::skinning::SkinCache;
use crate::geometry::visible_cluster_set::VisibleClusterSet;
use crate::graph::types::{AccessKind, ClusterRecord, TextureFormat};
use crate::shadow::clipmap::{
    ClipmapConfig, LightBasis, PAGE_TABLE_DIM, slot_of, world_page_coord, world_page_of_slot,
};
use crate::shadow::page_table::{AGE_MAX, PageId, PageTable, PageTableEntry};
use crate::shadow::pool::{PAGE_FLOATS, PhysicalPagePool};
use crate::temporal::common::Mat4;
use crate::temporal::image::ImageF32;

const DIM: usize = PAGE_TABLE_DIM as usize;
const SLOTS: usize = DIM * DIM;

/// VSM 配置(报告3 §4 阶段不变量 + §5.3 策略旋钮)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VsmConfig {
    /// clipmap 栈(级数/基准半径/深度范围)。
    pub clip: ClipmapConfig,
    /// 共享物理页池预算(页数,跨全部级;UE 实战 4096–8192 参照,测试用小值)。
    pub pool_pages: u16,
    /// 投影采样深度比较 bias(深度 [0,1] 单位;防自遮挡条纹)。
    pub depth_bias: f32,
}

impl Default for VsmConfig {
    fn default() -> Self {
        Self {
            clip: ClipmapConfig::default(),
            pool_pages: 512,
            depth_bias: 1e-3,
        }
    }
}

/// 世界空间三角形(阴影深度光栅输入;与章 C 剔除管线对接的最小面)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowTri {
    pub v: [[f32; 3]; 3],
}

impl ShadowTri {
    pub fn new(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Self {
        Self { v: [a, b, c] }
    }
}

/// 单 clipmap 级光栅视图描述(多视图接口首日:报告3 §2.5——VSM 是章 C 剔除/
/// 光栅管线的多视图客户;device 接线时视图数组直接映射多视图 pass)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowView {
    pub level: u8,
    /// 窗口起点(世界页坐标,toroidal)。
    pub window_min_pages: [i32; 2],
    /// 单页世界尺寸。
    pub page_world: f32,
    /// 正交深度区间 [zmin, zmax](灯空间 z_l)。
    pub z_range: [f32; 2],
}

/// 页标记统计(度量埋点:每帧标记页数 = 页需求画像)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarkStats {
    /// 参与标记的有效像素数(剔除远平面/天空)。
    pub pixels: u32,
    /// 本帧新标记的去重页数。
    pub pages: u32,
}

/// 页分配统计(度量埋点:池水位/驱逐率,报告3 §6 页池颠簸画像)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AllocStats {
    /// 新分配物理页的页数。
    pub allocated: u32,
    /// 被驱逐页数。
    pub evicted: u32,
    /// 池满且无驱逐候选(全部被本帧标记保护)而拒绝的请求数。
    pub denied: u32,
    /// 驱逐序列(确定性:远级→近级、槽位升序、龄大优先、同龄取先遇)。
    pub evicted_pages: Vec<PageId>,
}

/// 深度光栅统计(增量语义验收:本帧实际重光栅页数)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RasterStats {
    pub pages: u32,
}

/// 脏且驻留页引用(M19 multi-view batch / device 装配)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirtyPageRef {
    pub view_id: u32,
    pub level: u8,
    pub slot: (u8, u8),
    pub phys: u16,
    pub origin: [f32; 2],
    pub page_world: f32,
    pub z_range: [f32; 2],
}

/// VSM host 系统(单方向光;页表/物理池跨帧持久)。
#[derive(Debug, Clone)]
pub struct Vsm {
    cfg: VsmConfig,
    basis: LightBasis,
    light_dir: [f32; 3],
    camera: [f32; 3],
    frame: u32,
    /// 每级页表(128×128,跨帧)。
    tables: Vec<PageTable>,
    /// 每级槽位 → 当前世界页坐标(toroidal 有效性依据)。
    slot_wp: Vec<[[i32; 2]; SLOTS]>,
    /// 每级窗口起点(世界页坐标)。
    window_min: Vec<[i32; 2]>,
    /// 每级正交深度区间。
    z_range: Vec<[f32; 2]>,
    /// 每级帧标记戳(= 当前帧号表示本帧已标记;初值 u32::MAX 避开第 0 帧)。
    mark_stamp: Vec<[u32; SLOTS]>,
    /// 共享物理页池(跨帧)。
    pool: PhysicalPagePool,
    windows_valid: bool,
}

impl Vsm {
    /// 新建系统(第 0 帧;窗口按初始相机建立,不触发环形标脏)。
    pub fn new(cfg: VsmConfig, light_dir: [f32; 3], camera: [f32; 3]) -> Self {
        cfg.clip.validate();
        assert!(cfg.depth_bias >= 0.0, "depth bias 不得为负");
        let levels = usize::from(cfg.clip.levels);
        let mut vsm = Self {
            cfg,
            basis: LightBasis::from_direction(light_dir),
            light_dir,
            camera,
            frame: 0,
            tables: vec![PageTable::new(); levels],
            slot_wp: vec![[[0; 2]; SLOTS]; levels],
            window_min: vec![[0; 2]; levels],
            z_range: vec![[0.0; 2]; levels],
            mark_stamp: vec![[u32::MAX; SLOTS]; levels],
            pool: PhysicalPagePool::new(cfg.pool_pages),
            windows_valid: false,
        };
        let _ = vsm.update_windows();
        vsm
    }

    pub fn frame(&self) -> u32 {
        self.frame
    }

    pub fn light_dir(&self) -> [f32; 3] {
        self.light_dir
    }

    /// 单级页表(页表可视化/对拍用)。
    pub fn table(&self, level: u8) -> &PageTable {
        &self.tables[usize::from(level)]
    }

    /// 共享物理页池(只读)。
    pub fn pool(&self) -> &PhysicalPagePool {
        &self.pool
    }

    /// 本帧是否已标记该槽位(屏幕反馈可视化/单测)。
    pub fn is_marked(&self, level: u8, x: u8, y: u8) -> bool {
        self.mark_stamp[usize::from(level)][usize::from(y) * DIM + usize::from(x)] == self.frame
    }

    /// 槽位当前对应的世界页坐标(toroidal)。
    pub fn slot_world_page(&self, level: u8, x: u8, y: u8) -> [i32; 2] {
        self.slot_wp[usize::from(level)][usize::from(y) * DIM + usize::from(x)]
    }

    /// 帧推进:驻留页帧龄 +1(饱和)、相机重排窗口(环形更新带标脏)。
    ///
    /// 返回值 = 本帧因 clipmap scroll(环形更新带)新标脏的槽位数
    /// (G8.5a M19 加性;既有忽略返回值的调用方 0-byte)。
    pub fn begin_frame(&mut self, camera: [f32; 3]) -> u32 {
        self.frame += 1;
        self.camera = camera;
        for t in &mut self.tables {
            for v in &mut t.entries {
                let mut e = PageTableEntry::unpack(*v);
                if e.resident && e.age < AGE_MAX {
                    e.age += 1;
                    *v = e.pack();
                }
            }
        }
        self.update_windows()
    }

    /// 白盒帧标记(M19 fixture / 单测夹具;与 `page_mark` 写入同一 `mark_stamp`)。
    pub fn mark_slot(&mut self, level: u8, x: u8, y: u8) {
        let idx = usize::from(y) * DIM + usize::from(x);
        self.mark_stamp[usize::from(level)][idx] = self.frame;
        let mut e = self.tables[usize::from(level)].get(x, y);
        if e.resident && e.age != 0 {
            e.age = 0;
            self.tables[usize::from(level)].set(x, y, e);
        }
    }

    /// 清页表槽(跨灯共享池驱逐时由 page_cache 回调)。
    pub fn clear_slot(&mut self, level: u8, x: u8, y: u8) {
        self.tables[usize::from(level)].set(x, y, PageTableEntry::EMPTY);
    }

    /// 强制标脏(M19 NonVirtual caster / fixture;不改 phys/resident/age)。
    pub fn dirty_slot(&mut self, level: u8, x: u8, y: u8) -> bool {
        let mut e = self.tables[usize::from(level)].get(x, y);
        if !e.resident || e.dirty {
            return false;
        }
        e.dirty = true;
        self.tables[usize::from(level)].set(x, y, e);
        true
    }

    /// 共享物理页池(可写;M19 local light 跨灯竞争)。
    pub fn pool_mut(&mut self) -> &mut PhysicalPagePool {
        &mut self.pool
    }

    /// LRU 驱逐候选(公开给跨灯分配器;语义同私有 `find_victim`)。
    pub fn find_lru_victim(&self) -> Option<(PageId, u16)> {
        self.find_victim()
    }

    /// 脏且驻留页枚举(级×槽行主序;device multi-view batch 装配契约)。
    pub fn dirty_resident_pages(&self) -> Vec<DirtyPageRef> {
        let mut out = Vec::new();
        for l in 0..self.cfg.clip.levels {
            let li = usize::from(l);
            let pw = self.cfg.clip.page_world(l);
            let zr = self.z_range[li];
            for idx in 0..SLOTS {
                let e = PageTableEntry::unpack(self.tables[li].entries[idx]);
                if !(e.resident && e.dirty) {
                    continue;
                }
                let sx = (idx % DIM) as u8;
                let sy = (idx / DIM) as u8;
                let wp = self.slot_wp[li][idx];
                out.push(DirtyPageRef {
                    view_id: u32::from(l),
                    level: l,
                    slot: (sx, sy),
                    phys: e.phys,
                    origin: [wp[0] as f32 * pw, wp[1] as f32 * pw],
                    page_world: pw,
                    z_range: zr,
                });
            }
        }
        out
    }

    /// 按当前相机/灯基重排各级窗口:原点页粒度 snap;窗口平移时仅世界页
    /// 坐标发生变化的槽位(环形更新带)标脏,留在窗口内的页槽位与内容不变。
    /// 返回本帧新标脏槽位数。
    fn update_windows(&mut self) -> u32 {
        let mut newly_dirty = 0u32;
        let cl = self.basis.to_light(self.camera);
        for l in 0..self.cfg.clip.levels {
            let li = usize::from(l);
            let pw = self.cfg.clip.page_world(l);
            let wmin = [
                world_page_coord(cl[0], pw) - (DIM / 2) as i32,
                world_page_coord(cl[1], pw) - (DIM / 2) as i32,
            ];
            if !self.windows_valid {
                self.window_min[li] = wmin;
                for sy in 0..DIM {
                    for sx in 0..DIM {
                        self.slot_wp[li][sy * DIM + sx] = [
                            world_page_of_slot(sx as u8, wmin[0]),
                            world_page_of_slot(sy as u8, wmin[1]),
                        ];
                    }
                }
            } else if wmin != self.window_min[li] {
                for sy in 0..DIM {
                    for sx in 0..DIM {
                        let idx = sy * DIM + sx;
                        let nwp = [
                            world_page_of_slot(sx as u8, wmin[0]),
                            world_page_of_slot(sy as u8, wmin[1]),
                        ];
                        if self.slot_wp[li][idx] != nwp {
                            // 环形更新带(报告3 §5.3 失效源三:级联原点切换):
                            // 槽位改指新世界页,旧内容作废 → 标脏重光栅。
                            let mut e = PageTableEntry::unpack(self.tables[li].entries[idx]);
                            if !e.dirty {
                                e.dirty = true;
                                self.tables[li].entries[idx] = e.pack();
                                newly_dirty += 1;
                            }
                            self.slot_wp[li][idx] = nwp;
                        }
                    }
                }
                self.window_min[li] = wmin;
            }
            let ext = self.cfg.clip.depth_extent;
            self.z_range[li] = [cl[2] - ext, cl[2] + ext];
        }
        self.windows_valid = true;
        newly_dirty
    }

    /// 灯平面坐标 → (槽位, 世界页坐标);出当前窗口返回 None。
    fn page_at(&self, level: u8, x_l: f32, y_l: f32) -> Option<(u8, u8, [i32; 2])> {
        let li = usize::from(level);
        let pw = self.cfg.clip.page_world(level);
        let wp = [world_page_coord(x_l, pw), world_page_coord(y_l, pw)];
        let wmin = self.window_min[li];
        if wp[0] < wmin[0]
            || wp[0] >= wmin[0] + DIM as i32
            || wp[1] < wmin[1]
            || wp[1] >= wmin[1] + DIM as i32
        {
            return None;
        }
        let (sx, sy) = (slot_of(wp[0]), slot_of(wp[1]));
        debug_assert_eq!(
            self.slot_wp[li][usize::from(sy) * DIM + usize::from(sx)],
            wp
        );
        Some((sx, sy, wp))
    }

    /// mark 位图每级字数(128×128 bit = 512 u32)。bit 序 `l*16384 + y*128 + x`
    /// 与 device 核 `vsm_page_mark_project.rx` 的 `page_bits` 逐位对齐。
    pub const MARK_WORDS_PER_LEVEL: usize = SLOTS / 32;

    /// pass a 的**纯函数内核**(device `vsm_page_mark_project` 的 host 镜像;
    /// 报告3 §5.1;RFC-0016 §4.D2;G8.5 设计 §2.3 第一核):主相机深度逐像素
    /// 反投影到世界 → 选级(按到相机距离)→ 投影出窗逐级向粗级回退 → 置位图 bit。
    ///
    /// 不改自身状态,返回 `(位图, 有效像素数)`。位图是 host/device 的**唯一对拍面**
    /// (A2.1:device readback 位图与本位图逐位相等,才允许据此记 MarkHit/MarkMiss)。
    pub fn page_mark_bits(&self, depth: &ImageF32, inv_view_proj: &Mat4) -> (Vec<u32>, u32) {
        assert_eq!(depth.c, 1, "深度图必须单通道");
        let levels = usize::from(self.cfg.clip.levels);
        let mut bits = vec![0u32; levels * Self::MARK_WORDS_PER_LEVEL];
        let (w, h) = (depth.w as f32, depth.h as f32);
        let mut pixels = 0u32;
        for y in 0..depth.h {
            for x in 0..depth.w {
                let d = depth.get(x, y, 0);
                // 远平面(深度 1.0)= 天空/无表面,不标记
                if d >= 1.0 {
                    continue;
                }
                let ndc = [
                    2.0 * ((x as f32 + 0.5) / w) - 1.0,
                    1.0 - 2.0 * ((y as f32 + 0.5) / h),
                    d,
                    1.0,
                ];
                let w4 = inv_view_proj.transform_vec4(ndc);
                if w4[3].abs() < 1e-8 {
                    continue;
                }
                let world = [w4[0] / w4[3], w4[1] / w4[3], w4[2] / w4[3]];
                pixels += 1;
                let dc = ((world[0] - self.camera[0]).powi(2)
                    + (world[1] - self.camera[1]).powi(2)
                    + (world[2] - self.camera[2]).powi(2))
                .sqrt();
                let lp = self.basis.to_light(world);
                for l in self.cfg.clip.select_level(dc)..self.cfg.clip.levels {
                    if let Some((sx, sy, _)) = self.page_at(l, lp[0], lp[1]) {
                        let bidx = usize::from(l) * SLOTS + usize::from(sy) * DIM + usize::from(sx);
                        bits[bidx / 32] |= 1u32 << (bidx % 32);
                        break;
                    }
                }
            }
        }
        (bits, pixels)
    }

    /// host **消费页标记位图**:置帧标记戳 + 驻留页帧龄归零(驱逐保护)。
    ///
    /// 位图来源可为 device `vsm_page_mark_project` readback 或 host 镜像
    /// [`Self::page_mark_bits`](两者逐位相等是 M19 门的判据)。返回本帧新标记页数。
    pub fn apply_mark_bitmap(&mut self, bits: &[u32]) -> u32 {
        let levels = usize::from(self.cfg.clip.levels);
        assert_eq!(
            bits.len(),
            levels * Self::MARK_WORDS_PER_LEVEL,
            "mark 位图字数须 = levels*512"
        );
        let mut pages = 0u32;
        for l in 0..self.cfg.clip.levels {
            let li = usize::from(l);
            for idx in 0..SLOTS {
                let bidx = li * SLOTS + idx;
                if (bits[bidx / 32] >> (bidx % 32)) & 1 == 0 {
                    continue;
                }
                if self.mark_stamp[li][idx] != self.frame {
                    self.mark_stamp[li][idx] = self.frame;
                    pages += 1;
                }
                let (sx, sy) = ((idx % DIM) as u8, (idx / DIM) as u8);
                let mut e = self.tables[li].get(sx, sy);
                if e.resident && e.age != 0 {
                    e.age = 0;
                    self.tables[li].set(sx, sy, e);
                }
            }
        }
        pages
    }

    /// 位图 → 标记槽列表 `(level, x, y)`(级升序、级内槽行主序)。
    ///
    /// = host 分类(MarkHit/MarkMiss)与紧凑请求列表的输入序;device 位图经同一
    /// 函数反解后与 host 序列逐槽对拍。
    pub fn marked_slots_from_bitmap(bits: &[u32], levels: u8) -> Vec<(u8, u8, u8)> {
        let mut out = Vec::new();
        for l in 0..levels {
            let li = usize::from(l);
            for idx in 0..SLOTS {
                let bidx = li * SLOTS + idx;
                if bidx / 32 >= bits.len() {
                    break;
                }
                if (bits[bidx / 32] >> (bidx % 32)) & 1 == 1 {
                    out.push((l, (idx % DIM) as u8, (idx / DIM) as u8));
                }
            }
        }
        out
    }

    /// pass a — `shadow_page_mark`:反投影内核 + host 消费位图(等价于 G7 起
    /// 的原语义,拆成「产位图 / 消费位图」两段以便 device 对拍)。
    pub fn page_mark(&mut self, depth: &ImageF32, inv_view_proj: &Mat4) -> MarkStats {
        let (bits, pixels) = self.page_mark_bits(depth, inv_view_proj);
        let pages = self.apply_mark_bitmap(&bits);
        MarkStats { pixels, pages }
    }

    /// pass b — `shadow_page_alloc`(报告3 §5.3 分配策略;RFC-0016 §4.D2):
    /// 帧标记 → 紧凑请求列表(近级优先、级内槽位行主序,确定性)→ 共享池
    /// 分配;池满按帧龄 LRU 驱逐(远级优先、同龄取扫描先遇;本帧标记页
    /// 龄 0 不可驱逐);无候选则拒绝(该页保持未驻留,采样保守取 lit)。
    pub fn page_alloc(&mut self) -> AllocStats {
        let mut st = AllocStats::default();
        // 紧凑请求:近级优先(level 升序)
        let mut requests = Vec::new();
        for l in 0..self.cfg.clip.levels {
            let li = usize::from(l);
            for idx in 0..SLOTS {
                if self.mark_stamp[li][idx] == self.frame {
                    let e = PageTableEntry::unpack(self.tables[li].entries[idx]);
                    if !e.resident {
                        requests.push(PageId {
                            level: l,
                            x: (idx % DIM) as u8,
                            y: (idx / DIM) as u8,
                        });
                    }
                }
            }
        }
        for req in requests {
            let phys = if let Some(p) = self.pool.alloc(req) {
                p
            } else if let Some((victim, vphys)) = self.find_victim() {
                // 驱逐:清受害者页表项,物理页转给请求者
                let vt = &mut self.tables[usize::from(victim.level)];
                vt.set(victim.x, victim.y, PageTableEntry::EMPTY);
                self.pool.free_page(vphys);
                let p = self.pool.alloc(req).expect("驱逐后必有空闲页");
                st.evicted += 1;
                st.evicted_pages.push(victim);
                p
            } else {
                st.denied += 1;
                continue;
            };
            // 新驻留页内容未建 → 脏,待 shadow_depth_raster
            self.tables[usize::from(req.level)].set(
                req.x,
                req.y,
                PageTableEntry {
                    phys,
                    resident: true,
                    dirty: true,
                    age: 0,
                },
            );
            st.allocated += 1;
        }
        st
    }

    /// LRU 驱逐候选:驻留且帧龄 ≥1 中龄最大者;扫描序 = 远级→近级、槽位
    /// 升序,同龄取先遇(确定性)。
    fn find_victim(&self) -> Option<(PageId, u16)> {
        let mut best: Option<(PageId, u16, u8)> = None;
        for l in (0..self.cfg.clip.levels).rev() {
            let t = &self.tables[usize::from(l)];
            for idx in 0..SLOTS {
                let e = PageTableEntry::unpack(t.entries[idx]);
                if !e.resident || e.age == 0 {
                    continue;
                }
                let older = match best {
                    Some((_, _, age)) => e.age > age,
                    None => true,
                };
                if older {
                    best = Some((
                        PageId {
                            level: l,
                            x: (idx % DIM) as u8,
                            y: (idx / DIM) as u8,
                        },
                        e.phys,
                        e.age,
                    ));
                }
            }
        }
        best.map(|(id, phys, _)| (id, phys))
    }

    /// 失效源一 — 图元移动(报告3 §5.3):世界 AABB 在各级灯平面的投影
    /// 覆盖页标脏;返回新标脏页数(已脏不重复计)。
    pub fn invalidate_aabb(&mut self, min: [f32; 3], max: [f32; 3]) -> u32 {
        let mut count = 0;
        for l in 0..self.cfg.clip.levels {
            let li = usize::from(l);
            let pw = self.cfg.clip.page_world(l);
            // 8 角点投影取灯平面包围盒
            let (mut xl0, mut xl1, mut yl0, mut yl1) = (
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
            );
            for i in 0..8 {
                let p = [
                    if i & 1 == 0 { min[0] } else { max[0] },
                    if i & 2 == 0 { min[1] } else { max[1] },
                    if i & 4 == 0 { min[2] } else { max[2] },
                ];
                let lp = self.basis.to_light(p);
                xl0 = xl0.min(lp[0]);
                xl1 = xl1.max(lp[0]);
                yl0 = yl0.min(lp[1]);
                yl1 = yl1.max(lp[1]);
            }
            let wmin = self.window_min[li];
            let lo = [
                world_page_coord(xl0, pw).max(wmin[0]),
                world_page_coord(yl0, pw).max(wmin[1]),
            ];
            let hi = [
                world_page_coord(xl1, pw).min(wmin[0] + DIM as i32 - 1),
                world_page_coord(yl1, pw).min(wmin[1] + DIM as i32 - 1),
            ];
            for wy in lo[1]..=hi[1] {
                for wx in lo[0]..=hi[0] {
                    let (sx, sy) = (slot_of(wx), slot_of(wy));
                    let idx = usize::from(sy) * DIM + usize::from(sx);
                    debug_assert_eq!(self.slot_wp[li][idx], [wx, wy]);
                    let mut e = PageTableEntry::unpack(self.tables[li].entries[idx]);
                    if !e.dirty {
                        e.dirty = true;
                        self.tables[li].entries[idx] = e.pack();
                        count += 1;
                    }
                }
            }
        }
        count
    }

    /// 失效源二 — 灯方向变化(报告3 §5.3):阴影空间基整体改变,全部页
    /// 标脏;窗口/槽位映射按新基立即重排。
    pub fn invalidate_light_direction(&mut self, new_dir: [f32; 3]) {
        self.light_dir = new_dir;
        self.basis = LightBasis::from_direction(new_dir);
        for t in &mut self.tables {
            for v in &mut t.entries {
                let mut e = PageTableEntry::unpack(*v);
                if !e.dirty {
                    e.dirty = true;
                    *v = e.pack();
                }
            }
        }
        let _ = self.update_windows();
    }

    /// 多视图描述(每 clipmap 级一视图;device W3 接线契约)。
    pub fn views(&self) -> Vec<ShadowView> {
        (0..self.cfg.clip.levels)
            .map(|l| ShadowView {
                level: l,
                window_min_pages: self.window_min[usize::from(l)],
                page_world: self.cfg.clip.page_world(l),
                z_range: self.z_range[usize::from(l)],
            })
            .collect()
    }

    /// 多视图 `shadow_depth_raster`(报告3 §2.1/§5.1;RFC-0016 §4.D3):
    /// 逐视图(clipmap 级)仅对「脏且驻留」页做 CPU 边函数光栅到 128×128
    /// 物理页(先填远平面 1.0,取最小深度),完成后清脏。干净页字节不动
    /// (增量语义)。
    pub fn shadow_depth_raster(&mut self, tris: &[ShadowTri]) -> RasterStats {
        // 灯空间顶点预变换(每视图共享同一基)
        let lt: Vec<[[f32; 3]; 3]> = tris
            .iter()
            .map(|t| {
                let mut out = [[0.0; 3]; 3];
                for (o, &v) in out.iter_mut().zip(t.v.iter()) {
                    *o = self.basis.to_light(v);
                }
                out
            })
            .collect();
        let mut st = RasterStats::default();
        for l in 0..self.cfg.clip.levels {
            let li = usize::from(l);
            let pw = self.cfg.clip.page_world(l);
            let zr = self.z_range[li];
            for idx in 0..SLOTS {
                let e = PageTableEntry::unpack(self.tables[li].entries[idx]);
                if !(e.resident && e.dirty) {
                    continue;
                }
                let wp = self.slot_wp[li][idx];
                let origin = [wp[0] as f32 * pw, wp[1] as f32 * pw];
                {
                    let page = self.pool.page_mut(e.phys);
                    page.fill(1.0);
                    for t in &lt {
                        raster_tri_into_page(page, *t, origin, pw, zr);
                    }
                }
                let mut e2 = e;
                e2.dirty = false;
                self.tables[li].entries[idx] = e2.pack();
                st.pages += 1;
            }
        }
        st
    }

    /// 投影采样(`shadow_project` 并入光照着色;报告3 §5.1;RFC-0016 §4.D3):
    /// 选级 → 页表查询 → 驻留且净则物理页深度比较(最近邻,可配 bias);
    /// **未驻留/脏页与全部级出窗一律保守返回 lit = 1.0**(宁可漏影不可
    /// 误黑——缺页保守方向约定,报告3 §2.1 屏幕反馈语义)。
    pub fn sample_shadow(&self, world: [f32; 3]) -> f32 {
        let dc = ((world[0] - self.camera[0]).powi(2)
            + (world[1] - self.camera[1]).powi(2)
            + (world[2] - self.camera[2]).powi(2))
        .sqrt();
        let lp = self.basis.to_light(world);
        for l in self.cfg.clip.select_level(dc)..self.cfg.clip.levels {
            let Some((sx, sy, wp)) = self.page_at(l, lp[0], lp[1]) else {
                continue;
            };
            let li = usize::from(l);
            let e = self.tables[li].get(sx, sy);
            if !e.resident || e.dirty {
                return 1.0;
            }
            let pw = self.cfg.clip.page_world(l);
            let tx = ((lp[0] - wp[0] as f32 * pw) / pw * DIM as f32).floor() as i32;
            let ty = ((lp[1] - wp[1] as f32 * pw) / pw * DIM as f32).floor() as i32;
            let tx = tx.clamp(0, DIM as i32 - 1) as usize;
            let ty = ty.clamp(0, DIM as i32 - 1) as usize;
            let stored = self.pool.page(e.phys)[ty * DIM + tx];
            let zr = self.z_range[li];
            let dp = (lp[2] - zr[0]) / (zr[1] - zr[0]);
            return if dp <= stored + self.cfg.depth_bias {
                1.0
            } else {
                0.0
            };
        }
        1.0
    }
}

/// 单三角形光栅进物理页(视口 = 页世界范围 → 128×128 纹素;重心插值
/// 深度,取最小;边界纹素含边,跨页三角形两侧均写同值,取 min 无差)。
fn raster_tri_into_page(
    page: &mut [f32],
    v: [[f32; 3]; 3],
    page_origin: [f32; 2],
    page_world: f32,
    z_range: [f32; 2],
) {
    debug_assert_eq!(page.len(), PAGE_FLOATS);
    let n = DIM as f32;
    let mut tx = [0.0f32; 3];
    let mut ty = [0.0f32; 3];
    let mut dep = [0.0f32; 3];
    for i in 0..3 {
        tx[i] = (v[i][0] - page_origin[0]) / page_world * n;
        ty[i] = (v[i][1] - page_origin[1]) / page_world * n;
        dep[i] = (v[i][2] - z_range[0]) / (z_range[1] - z_range[0]);
    }
    let edge = |ax: f32, ay: f32, bx: f32, by: f32, px: f32, py: f32| {
        (bx - ax) * (py - ay) - (by - ay) * (px - ax)
    };
    let area = edge(tx[0], ty[0], tx[1], ty[1], tx[2], ty[2]);
    if area.abs() < 1e-12 {
        return; // 退化三角形
    }
    let min_x = tx.iter().copied().fold(f32::INFINITY, f32::min);
    let max_x = tx.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let min_y = ty.iter().copied().fold(f32::INFINITY, f32::min);
    let max_y = ty.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let x0 = (min_x.floor() as i32).clamp(0, DIM as i32);
    let x1 = (max_x.ceil() as i32).clamp(0, DIM as i32);
    let y0 = (min_y.floor() as i32).clamp(0, DIM as i32);
    let y1 = (max_y.ceil() as i32).clamp(0, DIM as i32);
    for j in y0..y1 {
        for i in x0..x1 {
            let (px, py) = (i as f32 + 0.5, j as f32 + 0.5);
            let w0 = edge(tx[1], ty[1], tx[2], ty[2], px, py) / area;
            let w1 = edge(tx[2], ty[2], tx[0], ty[0], px, py) / area;
            let w2 = edge(tx[0], ty[0], tx[1], ty[1], px, py) / area;
            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                let d = w0 * dep[0] + w1 * dep[1] + w2 * dep[2];
                let cell = &mut page[j as usize * DIM + i as usize];
                if d < *cell {
                    *cell = d;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 图集成描述(W3 demo 建图引用;资源名 + 冻结契约组合——与 temporal 同例;
// 页表/物理池 = imported 跨帧外部资源,RFC-0016 §4.0-3 纪律)
// ---------------------------------------------------------------------------

/// VSM pass 单条资源声明。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShadowResourceSpec {
    pub name: &'static str,
    pub access: AccessKind,
    pub format: TextureFormat,
    /// true = 跨帧外部资源(页表/物理页池;imported 语义)。
    pub imported: bool,
}

/// VSM 帧资源组合(mark/alloc 为页表维护,raster 写物理池,sample 读两者)。
pub fn vsm_frame_desc() -> Vec<ShadowResourceSpec> {
    use AccessKind::{ShaderRead, ShaderWrite};
    vec![
        ShadowResourceSpec {
            name: "shadow.main_depth",
            access: ShaderRead,
            format: TextureFormat::R32Float,
            imported: false,
        },
        ShadowResourceSpec {
            name: "shadow.page_table",
            access: ShaderWrite,
            format: TextureFormat::R32Uint,
            imported: true,
        },
        ShadowResourceSpec {
            name: "shadow.page_pool",
            access: ShaderWrite,
            format: TextureFormat::R32Float,
            imported: true,
        },
    ]
}

// ---------------------------------------------------------------------------
// M95 VSM 消费侧(RXS-0352 L1):VisibleClusterSet → 阴影深度光栅三角形
// ---------------------------------------------------------------------------

/// `VisibleClusterSet` 可见簇 → 世界空间阴影三角形(VSM 深度光栅消费腿;
/// G9.3 M95 最小追加面)。
///
/// 簇列表来源 = 同一可见集可见元素(与 `VisibleClusterSet::feed_vsm()` 同序,
/// **禁独立再算可见性**);蒙皮簇对象空间顶点取 skin cache 槽位(蒙皮簇阴影
/// 包围体与相机路径同源,RXS-0352 L1 VSM 条)。产出直接喂
/// [`Vsm::shadow_depth_raster`];provenance 帧末校验归
/// `visible_cluster_set::verify_frame_provenance`。
pub fn shadow_tris_from_visible_set(
    set: &VisibleClusterSet,
    instances: &[InstanceRecord],
    clusters: &[ClusterRecord],
    vertices: &[[f32; 3]],
    indices: &[u32],
    skin: Option<&SkinCache>,
    skin_slot_of: &[u32],
) -> Vec<ShadowTri> {
    let mut out = Vec::new();
    for (_, e) in set.visible_entries() {
        let inst = &instances[e.instance as usize];
        let c = &clusters[e.cluster as usize];
        let slot = skin_slot_of[e.cluster as usize];
        let position_of = |local: u32| {
            if slot != u32::MAX {
                skin.expect("skin_slot_of 指派槽位时 skin cache 必须在场")
                    .slots[slot as usize]
                    .positions[local as usize]
            } else {
                vertices[(c.vertex_offset + local) as usize]
            }
        };
        for t in 0..c.triangle_count {
            let mut tri = [[0.0f32; 3]; 3];
            for (k, v) in tri.iter_mut().enumerate() {
                let local = indices[(c.triangle_offset + 3 * t) as usize + k];
                *v = transform_point(&inst.transform, position_of(local));
            }
            out.push(ShadowTri::new(tri[0], tri[1], tri[2]));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::common::{look_at_rh, perspective_rh_zo};

    fn cfg4(base_radius: f32, depth_extent: f32, pool: u16) -> VsmConfig {
        VsmConfig {
            clip: ClipmapConfig {
                levels: 4,
                base_radius,
                depth_extent,
            },
            pool_pages: pool,
            depth_bias: 1e-3,
        }
    }

    /// 顶视相机:(pos) → 原点,fov 90°,up = +y(地对空正交足迹,锚定可手算)。
    fn top_down_camera(pos: [f32; 3]) -> (Mat4, Mat4) {
        let proj = perspective_rh_zo(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
        let view = look_at_rh(pos, [pos[0], pos[1], 0.0], [0.0, 1.0, 0.0]);
        let vp = proj.mul(&view);
        let inv = vp.inverse().expect("可逆");
        (vp, inv)
    }

    /// 世界点经 vp 投影的深度 [0,1](合成深度图用)。
    fn project_depth(vp: &Mat4, world: [f32; 3]) -> f32 {
        let c = vp.transform_vec4([world[0], world[1], world[2], 1.0]);
        c[2] / c[3]
    }

    /// 白盒帧标记(直接置帧标记戳;分配/驱逐策略单测用,非 mock——
    /// 与 page_mark 写入路径同一份判定数据)。
    fn wb_mark(vsm: &mut Vsm, level: u8, x: u8, y: u8) {
        let idx = usize::from(y) * DIM + usize::from(x);
        vsm.mark_stamp[usize::from(level)][idx] = vsm.frame;
    }

    fn entry(vsm: &Vsm, level: u8, x: u8, y: u8) -> PageTableEntry {
        vsm.table(level).get(x, y)
    }

    // -----------------------------------------------------------------
    // page_mark 锚定(合成深度图,手算页集合)
    // -----------------------------------------------------------------

    #[test]
    fn mark_single_pixel_anchor() {
        // 相机 (0.37,-0.61,7) 顶视,fov90:1×1 像素 → 世界 (0.37,-0.61,0)。
        // 灯 dir=(0,0,-1):x_l = y_w = -0.61,y_l = x_w = 0.37;
        // R0=16 → p0=0.25:wp = (floor(-0.61/0.25), floor(0.37/0.25)) = (-3, 1)
        // → 槽位 (125, 1);距离 7 ≤ 16 → 0 级。
        let cam = [0.37, -0.61, 7.0];
        let (vp, inv) = top_down_camera(cam);
        let d = project_depth(&vp, [0.37, -0.61, 0.0]);
        let depth = ImageF32::from_fn(1, 1, 1, |_, _, _| d);
        let mut vsm = Vsm::new(cfg4(16.0, 16.0, 8), [0.0, 0.0, -1.0], cam);
        let st = vsm.page_mark(&depth, &inv);
        assert_eq!(st.pixels, 1);
        assert_eq!(st.pages, 1);
        assert!(vsm.is_marked(0, 125, 1));
        // 其余级不得有标记
        for l in 1..4u8 {
            for y in 0..DIM {
                for x in 0..DIM {
                    assert!(!vsm.is_marked(l, x as u8, y as u8));
                }
            }
        }
    }

    #[test]
    fn mark_quad_pixel_set_anchor() {
        // 同相机,2×2 全地面像素:ndc (±0.5, ±0.5) → 世界 (0.37∓3.5, -0.61±3.5, 0)。
        // 手算(每像素):x_l = y_w,y_l = x_w,wp = floor(l/0.25),槽位 = wp mod 128:
        // (0,0):(-3.13, 2.89) → wp(11,-13) → (11,115)
        // (1,0):( 3.87, 2.89) → wp(11, 15) → (11, 15)
        // (0,1):(-3.13,-4.11) → wp(-17,-13) → (111,115)
        // (1,1):( 3.87,-4.11) → wp(-17, 15) → (111, 15)
        let cam = [0.37, -0.61, 7.0];
        let (vp, inv) = top_down_camera(cam);
        let depth = ImageF32::from_fn(2, 2, 1, |x, y, _| {
            let ndc_x = x as f32 - 0.5;
            let ndc_y = 0.5 - y as f32;
            project_depth(&vp, [0.37 + ndc_x * 7.0, -0.61 + ndc_y * 7.0, 0.0])
        });
        let mut vsm = Vsm::new(cfg4(16.0, 16.0, 8), [0.0, 0.0, -1.0], cam);
        let st = vsm.page_mark(&depth, &inv);
        assert_eq!(st.pixels, 4);
        assert_eq!(st.pages, 4);
        for &(x, y) in &[(11u8, 115u8), (11, 15), (111, 115), (111, 15)] {
            assert!(vsm.is_marked(0, x, y), "({x},{y}) 应被标记");
        }
    }

    #[test]
    fn mark_distance_selects_different_levels() {
        // 2×1 深度图:近像素落 0 级,远像素(距离 40 > R1=32)落 2 级。
        // 像素0:世界 (-3.13,-0.61,0),距离 ≈7.83 → 0 级,wp(-3,-13) → (125,115);
        // 像素1:世界 (18.259,-0.61,-28.777),距离 40 → 2 级(R=64,p=1.0),
        //        wp(floor(-0.61), floor(18.259)) = (-1,18) → (127,18)。
        let cam = [0.37, -0.61, 7.0];
        let (vp, inv) = top_down_camera(cam);
        let near = [-3.13, -0.61, 0.0];
        let far = [0.37 + 0.5 * 35.777, -0.61, 7.0 - 35.777];
        let depth = ImageF32::from_fn(2, 1, 1, |x, _, _| {
            project_depth(&vp, if x == 0 { near } else { far })
        });
        let mut vsm = Vsm::new(cfg4(16.0, 64.0, 8), [0.0, 0.0, -1.0], cam);
        let st = vsm.page_mark(&depth, &inv);
        assert_eq!(st.pages, 2);
        assert!(vsm.is_marked(0, 125, 115), "近像素应落 0 级");
        assert!(vsm.is_marked(2, 127, 18), "远像素应落 2 级");
        assert!(!vsm.is_marked(1, 127, 18), "1 级不得有标记");
    }

    // -----------------------------------------------------------------
    // page_alloc:预算/LRU/保护/优先级
    // -----------------------------------------------------------------

    #[test]
    fn alloc_within_budget_all_resident() {
        let mut vsm = Vsm::new(cfg4(16.0, 16.0, 4), [0.0, 0.0, -1.0], [0.0, 0.0, 7.0]);
        for &(x, y) in &[(10u8, 10u8), (20, 20), (30, 30)] {
            wb_mark(&mut vsm, 0, x, y);
        }
        let st = vsm.page_alloc();
        assert_eq!((st.allocated, st.evicted, st.denied), (3, 0, 0));
        // 空闲栈弹出序 0,1,2(确定性);新页脏、龄 0
        for (i, &(x, y)) in [(10u8, 10u8), (20, 20), (30, 30)].iter().enumerate() {
            let e = entry(&vsm, 0, x, y);
            assert!(e.resident && e.dirty && e.age == 0);
            assert_eq!(e.phys, i as u16);
            assert_eq!(vsm.pool.owner(i as u16), Some(PageId { level: 0, x, y }));
        }
        assert_eq!(vsm.pool.free_count(), 1);
    }

    #[test]
    fn alloc_eviction_lru_deterministic_order() {
        let mut vsm = Vsm::new(cfg4(16.0, 16.0, 3), [0.0, 0.0, -1.0], [0.0, 0.0, 7.0]);
        // 第 0 帧:A(10,10) B(20,20) C(30,30) 全部分配(phys 0/1/2)
        for &(x, y) in &[(10u8, 10u8), (20, 20), (30, 30)] {
            wb_mark(&mut vsm, 0, x, y);
        }
        vsm.page_alloc();
        // 第 1 帧:帧龄 A=B=C=1;标记 B(龄→0)与新页 D(5,5) E(15,15)
        vsm.begin_frame([0.0, 0.0, 7.0]);
        wb_mark(&mut vsm, 0, 20, 20);
        let mut b = entry(&vsm, 0, 20, 20);
        b.age = 0;
        vsm.tables[0].set(20, 20, b);
        wb_mark(&mut vsm, 0, 5, 5);
        wb_mark(&mut vsm, 0, 15, 15);
        let st = vsm.page_alloc();
        // 请求序 D(槽 645) → E(槽 1935);驱逐序:同龄 1 取扫描先遇 → A 后 C
        assert_eq!((st.allocated, st.evicted, st.denied), (2, 2, 0));
        assert_eq!(
            st.evicted_pages,
            vec![
                PageId {
                    level: 0,
                    x: 10,
                    y: 10
                },
                PageId {
                    level: 0,
                    x: 30,
                    y: 30
                },
            ]
        );
        assert_eq!(entry(&vsm, 0, 5, 5).phys, 0, "D 接管 A 的物理页");
        assert_eq!(entry(&vsm, 0, 15, 15).phys, 2, "E 接管 C 的物理页");
        // B 帧龄保护,phys 不变
        let b = entry(&vsm, 0, 20, 20);
        assert!(b.resident && b.phys == 1);
        // 被驱逐项复位为空
        assert_eq!(entry(&vsm, 0, 10, 10), PageTableEntry::EMPTY);
    }

    #[test]
    fn alloc_marked_this_frame_eviction_protected() {
        let mut vsm = Vsm::new(cfg4(16.0, 16.0, 1), [0.0, 0.0, -1.0], [0.0, 0.0, 7.0]);
        wb_mark(&mut vsm, 0, 10, 10);
        vsm.page_alloc();
        // 次帧:A 龄 1,但本帧再次标记(龄→0)→ 不可驱逐;B 请求被拒
        vsm.begin_frame([0.0, 0.0, 7.0]);
        wb_mark(&mut vsm, 0, 10, 10);
        let mut a = entry(&vsm, 0, 10, 10);
        a.age = 0;
        vsm.tables[0].set(10, 10, a);
        wb_mark(&mut vsm, 0, 40, 40);
        let st = vsm.page_alloc();
        assert_eq!((st.allocated, st.evicted, st.denied), (0, 0, 1));
        assert!(entry(&vsm, 0, 10, 10).resident);
        assert!(!entry(&vsm, 0, 40, 40).resident);
    }

    #[test]
    fn alloc_near_level_priority() {
        let mut vsm = Vsm::new(cfg4(16.0, 16.0, 1), [0.0, 0.0, -1.0], [0.0, 0.0, 7.0]);
        // 同帧两级各一页,预算 1:近级(0)先得;远级请求面对的唯一驻留页
        // 龄 0(本帧分配)→ 保护 → 拒绝
        wb_mark(&mut vsm, 0, 10, 10);
        wb_mark(&mut vsm, 1, 10, 10);
        let st = vsm.page_alloc();
        assert_eq!((st.allocated, st.denied), (1, 1));
        assert!(entry(&vsm, 0, 10, 10).resident);
        assert!(!entry(&vsm, 1, 10, 10).resident);
    }

    // -----------------------------------------------------------------
    // invalidate 三源
    // -----------------------------------------------------------------

    #[test]
    fn invalidate_aabb_exact_page_set() {
        // 顶视灯:x_l = y_w,y_l = x_w;R0=16 → p0=0.25,窗口 wp ∈ [-64,64)。
        // AABB [0.3,0.65]²(x/y 同)按级分解(失效按级遍历,报告3 §5.3):
        // 0 级 p=0.25:wp floor(0.3/0.25)=1 .. floor(0.65/0.25)=2 → 2×2=4 页;
        // 1 级 p=0.50:wp 0..=1 → 4 页;2 级 p=1:wp 0 → 1 页;3 级 p=2:1 页。
        // 合计 10,槽位即 wp mod 128(窗口起点 -64,正 wp 槽位 = wp)。
        let mut vsm = Vsm::new(cfg4(16.0, 16.0, 8), [0.0, 0.0, -1.0], [0.0, 0.0, 7.0]);
        let n = vsm.invalidate_aabb([0.3, 0.3, -1.0], [0.65, 0.65, 1.0]);
        assert_eq!(n, 10);
        assert_eq!(vsm.table(0).dirty_count(), 4);
        for &(x, y) in &[(1u8, 1u8), (1, 2), (2, 1), (2, 2)] {
            assert!(entry(&vsm, 0, x, y).dirty, "({x},{y}) 应脏");
        }
        assert!(!entry(&vsm, 0, 0, 0).dirty);
        assert!(!entry(&vsm, 0, 3, 3).dirty);
        assert_eq!(vsm.table(1).dirty_count(), 4);
        for &(x, y) in &[(0u8, 0u8), (0, 1), (1, 0), (1, 1)] {
            assert!(entry(&vsm, 1, x, y).dirty, "1 级 ({x},{y}) 应脏");
        }
        assert_eq!(vsm.table(2).dirty_count(), 1);
        assert_eq!(vsm.table(3).dirty_count(), 1);
        assert!(entry(&vsm, 2, 0, 0).dirty && entry(&vsm, 3, 0, 0).dirty);
        // 重复调用不重复计数
        assert_eq!(vsm.invalidate_aabb([0.3, 0.3, -1.0], [0.65, 0.65, 1.0]), 0);
    }

    #[test]
    fn invalidate_light_direction_marks_all_dirty() {
        let mut vsm = Vsm::new(cfg4(16.0, 16.0, 8), [0.0, 0.0, -1.0], [0.0, 0.0, 7.0]);
        vsm.invalidate_light_direction([0.0, 0.0, -1.0]);
        for l in 0..4u8 {
            assert_eq!(vsm.table(l).dirty_count(), SLOTS as u32);
        }
        // 灯基换向:新方向生效
        vsm.invalidate_light_direction([1.0, 0.0, -1.0]);
        assert_eq!(vsm.light_dir(), [1.0, 0.0, -1.0]);
    }

    #[test]
    fn origin_shift_one_page_dirties_ring_band_only() {
        // 相机 +x 平移恰一页(0.25 世界 = 0 级页尺寸):y_l = x_w 中心页 0→1,
        // 0 级窗口 y 方向平移一页 → 环形更新带 = 一行(128 槽)标脏;
        // 1 级页 0.5 世界,平移不足一页 → 不动;更高级同理。
        let mut vsm = Vsm::new(cfg4(16.0, 16.0, 8), [0.0, 0.0, -1.0], [0.0, 0.0, 7.0]);
        vsm.begin_frame([0.25, 0.0, 7.0]);
        assert_eq!(vsm.table(0).dirty_count(), DIM as u32);
        for x in 0..DIM {
            // 带位置:离开窗口的旧行槽位 = slot_of(-64) = 64
            assert!(entry(&vsm, 0, x as u8, 64).dirty, "({x},64) 应在更新带");
            assert!(!entry(&vsm, 0, x as u8, 65).dirty);
        }
        for l in 1..4u8 {
            assert_eq!(vsm.table(l).dirty_count(), 0, "{l} 级未平移");
        }
    }

    // -----------------------------------------------------------------
    // 深度光栅
    // -----------------------------------------------------------------

    /// 白盒置驻留(标帧标记 + alloc 的同路径结果,光栅单测夹具)。
    fn wb_make_resident(vsm: &mut Vsm, level: u8, x: u8, y: u8) -> u16 {
        wb_mark(vsm, level, x, y);
        let st = vsm.page_alloc();
        assert_eq!(st.denied, 0);
        entry(vsm, level, x, y).phys
    }

    #[test]
    fn raster_single_triangle_depth_anchor() {
        // R0=16 → p0=0.25;相机 (0,0,7) → 深度区间 z_l ∈ [-71,57]。
        // 三角形 z=1(z_l=-1)覆页 wp(0,0) 下半:纹素 (i,j) 覆 ⇔ i+j ≤ 127。
        // 深度锚定 = (−1−(−71))/128 = 70/128 = 0.546875(二进制精确)。
        let mut vsm = Vsm::new(cfg4(16.0, 64.0, 4), [0.0, 0.0, -1.0], [0.0, 0.0, 7.0]);
        let phys = wb_make_resident(&mut vsm, 0, 0, 0);
        let tri = ShadowTri::new([0.0, 0.0, 1.0], [0.25, 0.0, 1.0], [0.0, 0.25, 1.0]);
        let st = vsm.shadow_depth_raster(&[tri]);
        assert_eq!(st.pages, 1);
        assert!(!entry(&vsm, 0, 0, 0).dirty, "光栅后清脏");
        let page = vsm.pool.page(phys);
        let d = 70.0 / 128.0;
        assert_eq!(page[0], d);
        assert_eq!(page[63 * DIM + 64], d, "(64,63) 在三角形内");
        assert_eq!(page[64 * DIM + 64], 1.0, "(64,64) 在三角形外");
        let covered = page.iter().filter(|&&v| v != 1.0).count();
        assert_eq!(covered, 128 * 129 / 2, "含边纹素精确计数");
    }

    #[test]
    fn raster_cross_page_quad_splits() {
        // 四页 wp (0,0)(1,0)(0,1)(1,1) 驻留;z=1 方块 [0.1,0.4]² 跨四页,
        // 每页部分覆盖(0.1/0.4 均非页界)。
        let mut vsm = Vsm::new(cfg4(16.0, 64.0, 4), [0.0, 0.0, -1.0], [0.0, 0.0, 7.0]);
        let pages: Vec<u16> = [(0u8, 0u8), (1, 0), (0, 1), (1, 1)]
            .iter()
            .map(|&(x, y)| wb_make_resident(&mut vsm, 0, x, y))
            .collect();
        let quad = [
            ShadowTri::new([0.1, 0.1, 1.0], [0.4, 0.1, 1.0], [0.4, 0.4, 1.0]),
            ShadowTri::new([0.1, 0.1, 1.0], [0.4, 0.4, 1.0], [0.1, 0.4, 1.0]),
        ];
        let st = vsm.shadow_depth_raster(&quad);
        assert_eq!(st.pages, 4);
        let d = 70.0 / 128.0;
        for (i, &phys) in pages.iter().enumerate() {
            let page = vsm.pool.page(phys);
            // 覆盖纹素深度 = 70/128(重心插值 f32 舍入容差 1e-5)
            let covered = page.iter().filter(|&&v| v < 1.0).count();
            assert!(covered > 0, "页 {i} 应有覆盖");
            assert!(covered < PAGE_FLOATS, "页 {i} 应部分覆盖(跨页分片)");
            for &v in page.iter().filter(|&&v| v < 1.0) {
                assert!((v - d).abs() < 1e-5, "页 {i} 深度 {v} ≠ {d}");
            }
        }
    }

    #[test]
    fn raster_only_dirty_pages_touched() {
        let mut vsm = Vsm::new(cfg4(16.0, 64.0, 4), [0.0, 0.0, -1.0], [0.0, 0.0, 7.0]);
        let pa = wb_make_resident(&mut vsm, 0, 0, 0);
        let pb = wb_make_resident(&mut vsm, 0, 5, 5);
        // 首帧空场景光栅清脏 → 双页全 1.0
        vsm.shadow_depth_raster(&[]);
        // 哨兵写 A 页;仅 B 页(wp(5,5) ↔ 灯面 [1.25,1.5)²)所在区域标脏。
        // AABB 按级遍历:0 级 p=0.25 → wp 5 恰 1 页;1/2/3 级各 1 页(未驻留,
        // 不影响光栅),合计 4。
        for v in vsm.pool.page_mut(pa) {
            *v = 0.123;
        }
        let n = vsm.invalidate_aabb([1.3, 1.3, -1.0], [1.4, 1.4, 1.0]);
        assert_eq!(n, 4);
        assert_eq!(vsm.table(0).dirty_count(), 1);
        let tri = ShadowTri::new([1.3, 1.3, 0.5], [1.4, 1.3, 0.5], [1.3, 1.4, 0.5]);
        let st = vsm.shadow_depth_raster(&[tri]);
        assert_eq!(st.pages, 1, "仅脏页重光栅");
        // A 页字节不变(哨兵完好)
        assert!(vsm.pool.page(pa).iter().all(|&v| v == 0.123));
        // B 页写入新深度 = (−0.5+71)/128 = 0.55078125
        let d = 70.5 / 128.0;
        assert!(vsm.pool.page(pb).iter().any(|&v| (v - d).abs() < 1e-7));
    }

    // -----------------------------------------------------------------
    // 端到端与增量语义
    // -----------------------------------------------------------------

    /// e2e 夹具:相机 (0,0,0.5) 顶视 fov90(地面足迹 ±0.5),R0=4(0 级页
    /// 0.0625,纹素 0.00048828125),40×40 深度图(像素步 0.025 < 页尺寸,
    /// 足迹内 16×16 = 256 页全标记);地面 z=0 + 悬浮板 z=0.2。
    struct E2e {
        vsm: Vsm,
        depth: ImageF32,
        inv: Mat4,
    }

    fn e2e_setup() -> E2e {
        let cam = [0.0, 0.0, 0.5];
        let (vp, inv) = top_down_camera(cam);
        let d0 = project_depth(&vp, [0.0, 0.0, 0.0]);
        let depth = ImageF32::from_fn(40, 40, 1, |_, _, _| d0);
        let vsm = Vsm::new(cfg4(4.0, 4.0, 512), [0.0, 0.0, -1.0], cam);
        E2e { vsm, depth, inv }
    }

    /// 地面 [-0.6,0.6]² z=0 + 悬浮板 [cx±0.1]² z=0.2(各 2 三角形)。
    fn e2e_scene(plate_cx: f32, plate_cy: f32) -> Vec<ShadowTri> {
        let (x0, y0, x1, y1) = (
            plate_cx - 0.1,
            plate_cy - 0.1,
            plate_cx + 0.1,
            plate_cy + 0.1,
        );
        vec![
            ShadowTri::new([-0.6, -0.6, 0.0], [0.6, -0.6, 0.0], [0.6, 0.6, 0.0]),
            ShadowTri::new([-0.6, -0.6, 0.0], [0.6, 0.6, 0.0], [-0.6, 0.6, 0.0]),
            ShadowTri::new([x0, y0, 0.2], [x1, y0, 0.2], [x1, y1, 0.2]),
            ShadowTri::new([x0, y0, 0.2], [x1, y1, 0.2], [x0, y1, 0.2]),
        ]
    }

    #[test]
    fn e2e_mark_alloc_raster_sample() {
        let mut e = e2e_setup();
        let scene = e2e_scene(0.0, 0.0);
        let m = e.vsm.page_mark(&e.depth, &e.inv);
        assert_eq!(m.pages, 256, "足迹 16×16 页全标记");
        let a = e.vsm.page_alloc();
        assert_eq!((a.allocated, a.evicted, a.denied), (256, 0, 0));
        let r = e.vsm.shadow_depth_raster(&scene);
        assert_eq!(r.pages, 256, "新页全脏,首帧全光栅");
        // 影内:悬浮板正下方(像素 (20,20) 世界 (0.0125,0.0125,0))
        assert_eq!(e.vsm.sample_shadow([0.0125, 0.0125, 0.0]), 0.0);
        // 影外:板外(像素 (32,32) 世界 (0.3125,0.3125,0))
        assert_eq!(e.vsm.sample_shadow([0.3125, 0.3125, 0.0]), 1.0);
        // 边界 ≤1 纹素(0 级纹素 = 8/16384 ≈ 4.88e-4):板缘 x_w=0.1 两侧
        // ±0.5 纹素处 影/ lit 分明 → 过渡带 ≤1 纹素
        let texel = 8.0 / 16384.0;
        assert_eq!(e.vsm.sample_shadow([0.1 - 0.5 * texel, 0.0125, 0.0]), 0.0);
        assert_eq!(e.vsm.sample_shadow([0.1 + 0.5 * texel, 0.0125, 0.0]), 1.0);
    }

    #[test]
    fn e2e_static_second_frame_zero_work() {
        let mut e = e2e_setup();
        let scene = e2e_scene(0.0, 0.0);
        e.vsm.page_mark(&e.depth, &e.inv);
        e.vsm.page_alloc();
        e.vsm.shadow_depth_raster(&scene);
        // 静态第二帧:零新分配、零驱逐、零光栅(页缓存命中,报告3 P1 验收)
        e.vsm.begin_frame([0.0, 0.0, 0.5]);
        let m = e.vsm.page_mark(&e.depth, &e.inv);
        assert_eq!(m.pages, 256);
        let a = e.vsm.page_alloc();
        assert_eq!((a.allocated, a.evicted, a.denied), (0, 0, 0));
        let r = e.vsm.shadow_depth_raster(&scene);
        assert_eq!(r.pages, 0);
        // 采样结果与首帧一致
        assert_eq!(e.vsm.sample_shadow([0.0125, 0.0125, 0.0]), 0.0);
        assert_eq!(e.vsm.sample_shadow([0.3125, 0.3125, 0.0]), 1.0);
    }

    #[test]
    fn e2e_moved_occluder_re_rasters_only_covered_pages() {
        let mut e = e2e_setup();
        let scene = e2e_scene(0.0, 0.0);
        e.vsm.page_mark(&e.depth, &e.inv);
        e.vsm.page_alloc();
        e.vsm.shadow_depth_raster(&scene);
        // 干净远页(wp (7,7),足迹内已驻留)内容快照
        let far_entry = entry(&e.vsm, 0, 7, 7);
        assert!(far_entry.resident);
        let far_snapshot = vsm_page_snapshot(&e.vsm, far_entry.phys);
        // 第二帧:悬浮板自原点移到 (0.25,0.25);新旧 AABB 标脏
        e.vsm.begin_frame([0.0, 0.0, 0.5]);
        e.vsm.page_mark(&e.depth, &e.inv);
        let a = e.vsm.page_alloc();
        assert_eq!((a.allocated, a.evicted), (0, 0));
        // 旧盒 [-0.1,0.1]²、新盒 [0.15,0.35]²(仅 0 级有驻留页,逐级分解):
        // 0 级 p=0.0625:旧 wp -2..=1 → 16 页,新 wp 2..=5 → 16 页;
        // 1 级 p=0.125:各 2×2=4 页;2/3 级各 4 页,且新盒在 2 级与旧盒
        // 重叠 1 页(已脏不重复计)→ n1 = 16+4+4+4 = 28,n2 = 16+4+4+4-1 = 23。
        let n1 = e.vsm.invalidate_aabb([-0.1, -0.1, 0.15], [0.1, 0.1, 0.25]);
        let n2 = e
            .vsm
            .invalidate_aabb([0.15, 0.15, 0.15], [0.35, 0.35, 0.25]);
        assert_eq!((n1, n2), (28, 23));
        assert_eq!(e.vsm.table(0).dirty_count(), 32, "0 级两盒不相交");
        let scene2 = e2e_scene(0.25, 0.25);
        let r = e.vsm.shadow_depth_raster(&scene2);
        assert_eq!(r.pages, 32, "仅失效覆盖页重光栅(1–3 级无驻留页)");
        // 干净远页字节不变
        assert_eq!(vsm_page_snapshot(&e.vsm, far_entry.phys), far_snapshot);
        // 旧影区 lit、新影区 shadow
        assert_eq!(e.vsm.sample_shadow([0.0125, 0.0125, 0.0]), 1.0);
        assert_eq!(e.vsm.sample_shadow([0.2625, 0.2625, 0.0]), 0.0);
    }

    fn vsm_page_snapshot(vsm: &Vsm, phys: u16) -> Vec<f32> {
        vsm.pool.page(phys).to_vec()
    }

    // -----------------------------------------------------------------
    // 图集成描述
    // -----------------------------------------------------------------

    #[test]
    fn vsm_frame_desc_resources() {
        let desc = vsm_frame_desc();
        let mut names: Vec<_> = desc.iter().map(|r| r.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), desc.len(), "资源名互异");
        // 页表/物理池 = imported 跨帧外部资源(RFC-0016 §4.0-3)
        for name in ["shadow.page_table", "shadow.page_pool"] {
            let r = desc.iter().find(|r| r.name == name).expect("存在");
            assert!(r.imported, "{name} 应为 imported");
        }
        let depth = desc
            .iter()
            .find(|r| r.name == "shadow.main_depth")
            .expect("存在");
        assert!(!depth.imported && depth.access == AccessKind::ShaderRead);
    }

    // -----------------------------------------------------------------
    // G9.3 M95(RXS-0352):VisibleClusterSet → VSM 深度光栅消费腿
    // -----------------------------------------------------------------

    //@ spec: RXS-0352
    #[test]
    fn vsm_consumes_visible_cluster_set_same_source() {
        use crate::geometry::visible_cluster_set::{
            VisibleClusterEntry, compute_provenance_digest,
        };
        // 两簇(簇 0 静态、簇 1 蒙皮)单实例场景;实例平移 z = −5。
        let rest1 = [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let skinned1: Vec<[f32; 3]> = rest1.iter().map(|v| [v[0] + 2.0, v[1], v[2]]).collect();
        let vertices = vec![[9.0f32, 9.0, 0.0], [10.0, 9.0, 0.0], [9.0, 10.0, 0.0]]
            .into_iter()
            .chain(rest1)
            .collect::<Vec<_>>();
        let indices = vec![0u32, 1, 2, 0, 1, 2];
        let rec = |voff, toff| ClusterRecord {
            center: [0.0; 3],
            radius: 2.0,
            cone_axis: [0.0, 0.0, 1.0],
            cone_cutoff: 2.0,
            error: 0.0,
            parent_error: f32::INFINITY,
            vertex_offset: voff,
            triangle_offset: toff,
            vertex_count: 3,
            triangle_count: 1,
            page_id: 0,
            reserved: 0,
        };
        let clusters = vec![rec(0, 0), rec(3, 3)];
        let inst = [InstanceRecord {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, -5.0],
            ],
            cluster_offset: 0,
            cluster_count: 2,
            material_id: 0,
            flags: 0,
            aabb_min: [-2.0; 3],
            mesh_id: 0,
            aabb_max: [2.0; 3],
            reserved: u32::MAX,
        }];
        let mut set = VisibleClusterSet {
            frame_serial: 3,
            entries: vec![
                VisibleClusterEntry {
                    cluster: 0,
                    instance: 0,
                    lod_level: 0,
                    skin_version: 0,
                    page_id: 0,
                    visible: true,
                },
                VisibleClusterEntry {
                    cluster: 1,
                    instance: 0,
                    lod_level: 0,
                    skin_version: 2,
                    page_id: 0,
                    visible: true,
                },
                // 不可见元素(视锥/锥剔标记者)不得进 VSM 深度光栅。
                VisibleClusterEntry {
                    cluster: 0,
                    instance: 0,
                    lod_level: 0,
                    skin_version: 0,
                    page_id: 0,
                    visible: false,
                },
            ],
            residency: vec![],
            fallback: vec![],
            provenance_digest: [0; 32],
        };
        set.provenance_digest = compute_provenance_digest(&set);
        let skin = SkinCache {
            slots: vec![crate::geometry::skinning::SkinCacheSlot {
                positions: skinned1.clone(),
                bound: ([0.0; 3], [0.0; 3]),
                version: 2,
                stale_frames: 0,
            }],
        };
        let tris = shadow_tris_from_visible_set(
            &set,
            &inst,
            &clusters,
            &vertices,
            &indices,
            Some(&skin),
            &[u32::MAX, 0],
        );
        // 仅可见两元素 × 各 1 三角形 = 2;不可见元素不展开。
        assert_eq!(tris.len(), 2);
        // 簇 0(静态):世界 = 静止 + (0,0,−5)。
        assert_eq!(tris[0].v[0], [9.0, 9.0, -5.0]);
        // 簇 1(蒙皮):取 skin cache(+2 x 位移),与相机路径同源。
        assert_eq!(tris[1].v[0], [2.0, 0.0, -5.0]);
        assert_eq!(tris[1].v[1], [3.0, 0.0, -5.0]);
        // provenance 链:VSM 喂 source = set digest(帧末校验面同口径)。
        assert_eq!(set.feed_vsm().source, set.provenance_digest);
        assert_eq!(set.feed_vsm().depth_clusters.len(), 2);
        // 产出的三角形可直接喂多视图深度光栅(消费冒烟,不空转)。
        let mut vsm = Vsm::new(cfg4(4.0, 20.0, 64), [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]);
        vsm.begin_frame([0.0, 0.0, 0.0]);
        let stats = vsm.shadow_depth_raster(&tris);
        let _ = stats;
    }
}
