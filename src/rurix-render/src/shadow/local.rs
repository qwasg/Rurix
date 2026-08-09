//! Local spot 页空间(G8.5a M19 加性;设计 §2.2)。
//!
//! 首期 = 1 盏 spot、单视图、单级 128×128 页表;与方向光共享物理页池
//! (跨灯淘汰竞争)。omni 6 面不做(记 open)。池占用者
//! `PageId.level = LOCAL_LEVEL_TAG`。

use crate::shadow::clipmap::PAGE_TABLE_DIM;
use crate::shadow::page_table::{PageId, PageTable, PageTableEntry};
use crate::shadow::pool::{PAGE_FLOATS, PhysicalPagePool};
use crate::shadow::vsm::ShadowTri;

const DIM: usize = PAGE_TABLE_DIM as usize;
const SLOTS: usize = DIM * DIM;

/// 池占用者 level 哨兵(与方向光 0..levels-1 不冲突)。
pub const LOCAL_LEVEL_TAG: u8 = 0xFE;

/// Spot 参数(世界空间)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalSpot {
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub range: f32,
    pub fov_y: f32,
}

/// Local light 单级页表 + 标记戳。
#[derive(Debug, Clone)]
pub struct LocalLightPages {
    pub spot: LocalSpot,
    pub table: PageTable,
    mark_stamp: [u32; SLOTS],
    frame: u32,
    pub page_world: f32,
    pub z_range: [f32; 2],
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalAllocStats {
    pub allocated: u32,
    pub evicted: u32,
    pub denied: u32,
    pub evicted_pages: Vec<PageId>,
}

impl LocalLightPages {
    pub fn new(spot: LocalSpot) -> Self {
        let page_world = (spot.range * (spot.fov_y * 0.5).tan() * 2.0 / DIM as f32).max(0.05);
        Self {
            spot,
            table: PageTable::new(),
            mark_stamp: [u32::MAX; SLOTS],
            frame: 0,
            page_world,
            z_range: [0.05, spot.range],
        }
    }

    pub fn begin_frame(&mut self) {
        self.frame = self.frame.saturating_add(1);
        for v in &mut self.table.entries {
            let mut e = PageTableEntry::unpack(*v);
            if e.resident && e.age < 255 {
                e.age = e.age.saturating_add(1);
                *v = e.pack();
            }
        }
    }

    pub fn frame(&self) -> u32 {
        self.frame
    }

    pub fn is_marked(&self, x: u8, y: u8) -> bool {
        self.mark_stamp[usize::from(y) * DIM + usize::from(x)] == self.frame
    }

    pub fn mark_slot(&mut self, x: u8, y: u8) {
        let idx = usize::from(y) * DIM + usize::from(x);
        self.mark_stamp[idx] = self.frame;
        let mut e = self.table.get(x, y);
        if e.resident && e.age != 0 {
            e.age = 0;
            self.table.set(x, y, e);
        }
    }

    pub fn clear_slot(&mut self, x: u8, y: u8) {
        self.table.set(x, y, PageTableEntry::EMPTY);
    }

    fn find_local_victim(&self) -> Option<(PageId, u16)> {
        let mut best: Option<(PageId, u16, u8)> = None;
        for idx in 0..SLOTS {
            let e = PageTableEntry::unpack(self.table.entries[idx]);
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
                        level: LOCAL_LEVEL_TAG,
                        x: (idx % DIM) as u8,
                        y: (idx / DIM) as u8,
                    },
                    e.phys,
                    e.age,
                ));
            }
        }
        best.map(|(id, phys, _)| (id, phys))
    }

    /// 分配本帧标记未驻留槽;池满先驱 local LRU,再问 `find_foreign_victim`。
    pub fn alloc_into(
        &mut self,
        pool: &mut PhysicalPagePool,
        mut find_foreign_victim: impl FnMut() -> Option<(PageId, u16)>,
        mut on_clear_foreign: impl FnMut(PageId),
    ) -> LocalAllocStats {
        let mut st = LocalAllocStats::default();
        let mut requests = Vec::new();
        for idx in 0..SLOTS {
            if self.mark_stamp[idx] != self.frame {
                continue;
            }
            let e = PageTableEntry::unpack(self.table.entries[idx]);
            if !e.resident {
                requests.push(PageId {
                    level: LOCAL_LEVEL_TAG,
                    x: (idx % DIM) as u8,
                    y: (idx / DIM) as u8,
                });
            }
        }
        for req in requests {
            let phys = if let Some(p) = pool.alloc(req) {
                p
            } else {
                let victim_opt = self.find_local_victim().or_else(&mut find_foreign_victim);
                let Some((victim, vphys)) = victim_opt else {
                    st.denied += 1;
                    continue;
                };
                if victim.level == LOCAL_LEVEL_TAG {
                    self.clear_slot(victim.x, victim.y);
                } else {
                    on_clear_foreign(victim);
                }
                pool.free_page(vphys);
                st.evicted += 1;
                st.evicted_pages.push(victim);
                pool.alloc(req).expect("驱逐后必有空闲")
            };
            self.table.set(
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

    pub fn raster_dirty(
        &mut self,
        pool: &mut PhysicalPagePool,
        tris_light: &[[[f32; 3]; 3]],
    ) -> u32 {
        let mut pages = 0u32;
        let pw = self.page_world;
        let zr = self.z_range;
        for idx in 0..SLOTS {
            let e = PageTableEntry::unpack(self.table.entries[idx]);
            if !(e.resident && e.dirty) {
                continue;
            }
            let sx = (idx % DIM) as u8;
            let sy = (idx / DIM) as u8;
            let origin = [sx as f32 * pw, sy as f32 * pw];
            {
                let page = pool.page_mut(e.phys);
                page.fill(1.0);
                for t in tris_light {
                    raster_tri_into_page(page, *t, origin, pw, zr);
                }
            }
            let mut e2 = e;
            e2.dirty = false;
            self.table.entries[idx] = e2.pack();
            pages += 1;
        }
        pages
    }

    pub fn sample(&self, pool: &PhysicalPagePool, light_xy_z: [f32; 3], bias: f32) -> f32 {
        let pw = self.page_world;
        let wp = [
            (light_xy_z[0] / pw).floor() as i32,
            (light_xy_z[1] / pw).floor() as i32,
        ];
        if wp[0] < 0 || wp[1] < 0 || wp[0] >= DIM as i32 || wp[1] >= DIM as i32 {
            return 1.0;
        }
        let (sx, sy) = (wp[0] as u8, wp[1] as u8);
        let e = self.table.get(sx, sy);
        if !e.resident || e.dirty {
            return 1.0;
        }
        let tx = ((light_xy_z[0] - sx as f32 * pw) / pw * DIM as f32)
            .floor()
            .clamp(0.0, (DIM - 1) as f32) as usize;
        let ty = ((light_xy_z[1] - sy as f32 * pw) / pw * DIM as f32)
            .floor()
            .clamp(0.0, (DIM - 1) as f32) as usize;
        let stored = pool.page(e.phys)[ty * DIM + tx];
        let zr = self.z_range;
        let dp = (light_xy_z[2] - zr[0]) / (zr[1] - zr[0]);
        if dp <= stored + bias { 1.0 } else { 0.0 }
    }

    /// 脏且驻留页参数(device multi-view batch 装配)。
    pub fn dirty_resident_pages(&self) -> Vec<(u8, u8, u16, [f32; 2])> {
        let mut out = Vec::new();
        let pw = self.page_world;
        for idx in 0..SLOTS {
            let e = PageTableEntry::unpack(self.table.entries[idx]);
            if e.resident && e.dirty {
                let sx = (idx % DIM) as u8;
                let sy = (idx / DIM) as u8;
                out.push((sx, sy, e.phys, [sx as f32 * pw, sy as f32 * pw]));
            }
        }
        out
    }
}

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
        return;
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

/// 世界三角形 → spot 局部灯空间。
pub fn world_tris_to_spot_light(spot: &LocalSpot, tris: &[ShadowTri]) -> Vec<[[f32; 3]; 3]> {
    let fwd = normalize(spot.direction);
    let right = normalize(cross([0.0, 1.0, 0.0], fwd));
    let up = cross(fwd, right);
    tris.iter()
        .map(|t| {
            let mut out = [[0.0; 3]; 3];
            for (o, &v) in out.iter_mut().zip(t.v.iter()) {
                let d = [
                    v[0] - spot.position[0],
                    v[1] - spot.position[1],
                    v[2] - spot.position[2],
                ];
                *o = [
                    d[0] * right[0] + d[1] * right[1] + d[2] * right[2],
                    d[0] * up[0] + d[1] * up[1] + d[2] * up[2],
                    d[0] * fwd[0] + d[1] * fwd[1] + d[2] * fwd[2],
                ];
            }
            out
        })
        .collect()
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-8);
    [v[0] / n, v[1] / n, v[2] / n]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_mark_alloc_raster_sample_smoke() {
        let spot = LocalSpot {
            position: [0.0, 2.0, 2.0],
            direction: [0.0, -1.0, -1.0],
            range: 8.0,
            fov_y: 1.0,
        };
        let mut local = LocalLightPages::new(spot);
        let mut pool = PhysicalPagePool::new(4);
        local.mark_slot(0, 0);
        let st = local.alloc_into(&mut pool, || None, |_| {});
        assert_eq!((st.allocated, st.denied, st.evicted), (1, 0, 0));
        let tris = [ShadowTri::new(
            [0.0, 0.0, 1.0],
            [0.2, 0.0, 1.0],
            [0.0, 0.2, 1.0],
        )];
        let lt = world_tris_to_spot_light(&spot, &tris);
        assert_eq!(local.raster_dirty(&mut pool, &lt), 1);
        let s = local.sample(&pool, [0.05, 0.05, 1.0], 1e-3);
        assert!(s == 0.0 || s == 1.0);
    }
}
