//! 材质 classify / resolve host 参考(报告1 §3.4 P2;RFC-0016 §4.C4)——
//! `mat_classify`(tile × 材质分桶)与 `mat_resolve`(16 位材质槽 ID 窄缓冲)的
//! host 金标准,语义对应 GPU 线程模型:
//!
//! | host 函数 | GPU pass | 线程映射 |
//! |---|---|---|
//! | [`classify`] | `mat_classify` | 1 线程/像素块,tile 内原子计数 + 前缀和 |
//! | [`resolve`] | `mat_resolve` | 1 线程/像素,按桶间接分派 |
//!
//! 裁决(报告1 §3.4,UE5 `r.Nanite.ClassifyWithResolve` 路径参照):
//! - **材质解析是独立的 16 位窄缓冲 pass**,非从 64 位 VisBuffer 反查宽缓冲——
//!   带宽节省 3/4、RTX 2070S 该 pass 提速 40% 的 UE 实测路径;窄缓冲即下游
//!   GBuffer 求值的分类键(章 G `MaterialTable` 以 material_id 为索引)。
//! - 像素 → 材质的桥 = VisBuffer cluster27(本帧可见簇列表下标)→ 可见簇 →
//!   实例 → `material_id`(u32 收窄 u16,fail-closed 断言;[`visible_cluster_materials`]
//!   为调用方编组辅助)。同 mesh 多实例多材质因此可区分(可见簇携带实例)。
//! - 无效像素(VisBuffer clear)约定 = [`MATERIAL_INVALID`](u16::MAX),不进任何桶。
//! - 确定性序:tile 线性 row-major,桶内按**像素扫描首见序**(host 金标准;
//!   device 原子计数 + 前缀和的散射序不锚定,对拍比「逐 tile 桶集合 + 计数」)。

use crate::graph::types::visbuffer_unpack;

use super::cull::VisibleCluster;
use super::gpu_scene::InstanceRecord;
use super::visbuffer::{CLUSTER_INVALID, VisBufferCpu};

/// 无效像素的材质窄缓冲约定值(报告1 §3.4 无效路径;不进 classify 分桶)。
pub const MATERIAL_INVALID: u16 = u16::MAX;

/// 可见簇 → 材质槽 ID(u16 窄域;`InstanceRecord::material_id` u32 收窄,
/// 超域 fail-closed panic——窄缓冲契约被违反是装配 bug,暴露优于静默)。
///
/// 返回 Vec 下标 = 可见簇列表位置(与 VisBuffer cluster27 一一对应),
/// 即 [`classify`]/[`resolve`] 的 `cluster_to_material` 参数。
pub fn visible_cluster_materials(
    instances: &[InstanceRecord],
    visible: &[VisibleCluster],
) -> Vec<u16> {
    visible
        .iter()
        .map(|vc| {
            u16::try_from(instances[vc.instance as usize].material_id)
                .expect("material_id 超 16 位窄缓冲域(RFC-0016 §4.C4)")
        })
        .collect()
}

/// 单 tile 单材质桶(计数;GPU 对应 = tile 内逐材质原子计数)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileBucket {
    pub material_slot: u16,
    pub pixel_count: u32,
}

/// classify 产物(tile × 材质分桶的紧凑列表 + 前缀和偏移;确定性序见模块文档)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassifyOut {
    pub tile_size: u32,
    pub tiles_x: u32,
    pub tiles_y: u32,
    /// tile 前缀和(len = tiles_x·tiles_y + 1;tile 线性 row-major)。
    pub tile_offsets: Vec<u32>,
    /// 紧凑桶数组(tile 内像素扫描首见序)。
    pub buckets: Vec<TileBucket>,
}

impl ClassifyOut {
    /// 指定 tile 的桶切片(下标 row-major)。
    pub fn tile_buckets(&self, tile: u32) -> &[TileBucket] {
        let (s, e) = (
            self.tile_offsets[tile as usize] as usize,
            self.tile_offsets[tile as usize + 1] as usize,
        );
        &self.buckets[s..e]
    }
}

/// VisBuffer → tile × 材质分桶(计数 + 紧凑列表)。
///
/// `cluster_to_material` 下标 = 可见簇列表位置(VisBuffer cluster27 值);
/// 无效像素跳过(不进桶)。GPU 对应 = 逐 tile 原子计数 + 前缀和 + 散射三趟,
/// host 单趟 = 其确定性等价物。
pub fn classify(vis: &VisBufferCpu, cluster_to_material: &[u16], tile_size: u32) -> ClassifyOut {
    assert!(tile_size > 0, "tile 边长必须 >0");
    let tiles_x = vis.w.div_ceil(tile_size);
    let tiles_y = vis.h.div_ceil(tile_size);
    let mut tile_offsets = Vec::with_capacity((tiles_x * tiles_y + 1) as usize);
    let mut buckets: Vec<TileBucket> = Vec::new();
    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            tile_offsets.push(buckets.len() as u32);
            // tile 内首见序计数(线性小表;tile 边长 × 材质数均小,蛮力即金标准)。
            let mut local: Vec<(u16, u32)> = Vec::new();
            let y_end = ((ty + 1) * tile_size).min(vis.h);
            let x_end = ((tx + 1) * tile_size).min(vis.w);
            for y in (ty * tile_size)..y_end {
                for x in (tx * tile_size)..x_end {
                    let (_, cluster, _) = visbuffer_unpack(vis.get(x, y));
                    if cluster == CLUSTER_INVALID {
                        continue;
                    }
                    let mat = cluster_to_material[cluster as usize];
                    match local.iter_mut().find(|(m, _)| *m == mat) {
                        Some(entry) => entry.1 += 1,
                        None => local.push((mat, 1)),
                    }
                }
            }
            buckets.extend(
                local
                    .into_iter()
                    .map(|(material_slot, pixel_count)| TileBucket {
                        material_slot,
                        pixel_count,
                    }),
            );
        }
    }
    tile_offsets.push(buckets.len() as u32);
    ClassifyOut {
        tile_size,
        tiles_x,
        tiles_y,
        tile_offsets,
        buckets,
    }
}

/// VisBuffer → 全屏 16 位材质槽 ID 窄缓冲(row-major,len = w·h;
/// 无效像素 = [`MATERIAL_INVALID`])。
pub fn resolve(vis: &VisBufferCpu, cluster_to_material: &[u16]) -> Vec<u16> {
    vis.data
        .iter()
        .map(|&v| {
            let (_, cluster, _) = visbuffer_unpack(v);
            if cluster == CLUSTER_INVALID {
                MATERIAL_INVALID
            } else {
                cluster_to_material[cluster as usize]
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::gpu_scene::IDENTITY_3X4;
    use crate::graph::types::visbuffer_pack;

    /// 合成 8×4 VisBuffer:左半(x < 4)簇 0 → 材质 3;右半 y < 2 簇 1 → 材质 5,
    /// 右半 y ≥ 2 无效。tile 4 ⇒ tiles (2,1)。
    fn synth_vis() -> VisBufferCpu {
        let mut vis = VisBufferCpu::new(8, 4);
        for y in 0..4u32 {
            for x in 0..8u32 {
                let cluster = if x < 4 {
                    0
                } else if y < 2 {
                    1
                } else {
                    continue; // 保持 clear(无效)
                };
                vis.data[(y * vis.w + x) as usize] = visbuffer_pack(quantize_z05(), cluster, 0);
            }
        }
        vis
    }

    fn quantize_z05() -> u32 {
        super::super::visbuffer::quantize_depth30(0.5)
    }

    fn inst(material_id: u32) -> InstanceRecord {
        InstanceRecord {
            transform: IDENTITY_3X4,
            cluster_offset: 0,
            cluster_count: 0,
            material_id,
            flags: 0,
            aabb_min: [0.0; 3],
            mesh_id: 0,
            aabb_max: [0.0; 3],
            reserved: u32::MAX,
        }
    }

    #[test]
    fn classify_buckets_anchor() {
        let vis = synth_vis();
        let c2m = [3u16, 5u16];
        let out = classify(&vis, &c2m, 4);
        assert_eq!((out.tiles_x, out.tiles_y, out.tile_size), (2, 1, 4));
        // 前缀和:tile 0 一桶、tile 1 一桶 ⇒ [0, 1, 2]。
        assert_eq!(out.tile_offsets, vec![0, 1, 2]);
        // tile 0:左半 4×4 = 16 像素全材质 3;tile 1:上半 4×2 = 8 像素材质 5。
        assert_eq!(
            out.buckets,
            vec![
                TileBucket {
                    material_slot: 3,
                    pixel_count: 16
                },
                TileBucket {
                    material_slot: 5,
                    pixel_count: 8
                },
            ]
        );
        assert_eq!(out.tile_buckets(0), &out.buckets[0..1]);
        assert_eq!(out.tile_buckets(1), &out.buckets[1..2]);
    }

    #[test]
    fn resolve_matches_direct_lookup_and_invalid() {
        let vis = synth_vis();
        let c2m = [3u16, 5u16];
        let mat = resolve(&vis, &c2m);
        assert_eq!(mat.len(), 32);
        for y in 0..4u32 {
            for x in 0..8u32 {
                let got = mat[(y * vis.w + x) as usize];
                let expect = if x < 4 {
                    3
                } else if y < 2 {
                    5
                } else {
                    MATERIAL_INVALID
                };
                assert_eq!(got, expect, "({x},{y})");
                // 与逐像素直查(unpack → 查表)一致:无效像素 ⇔ u16::MAX。
                let (_, cluster, _) = visbuffer_unpack(vis.get(x, y));
                let direct = if cluster == CLUSTER_INVALID {
                    MATERIAL_INVALID
                } else {
                    c2m[cluster as usize]
                };
                assert_eq!(got, direct, "({x},{y}) 直查一致性");
            }
        }
        assert_eq!(MATERIAL_INVALID, u16::MAX);
    }

    #[test]
    fn classify_resolve_consistency_per_tile() {
        // 跨 API 一致性:逐 tile 桶集合 = resolve 输出在该 tile 的材质分布(集合 + 计数)。
        let vis = synth_vis();
        let c2m = [3u16, 5u16];
        let out = classify(&vis, &c2m, 4);
        let mat = resolve(&vis, &c2m);
        for tile in 0..(out.tiles_x * out.tiles_y) {
            let (tx, ty) = (tile % out.tiles_x, tile / out.tiles_x);
            let mut counts: Vec<(u16, u32)> = Vec::new();
            for y in (ty * 4)..(ty * 4 + 4).min(vis.h) {
                for x in (tx * 4)..(tx * 4 + 4).min(vis.w) {
                    let m = mat[(y * vis.w + x) as usize];
                    if m == MATERIAL_INVALID {
                        continue;
                    }
                    match counts.iter_mut().find(|(k, _)| *k == m) {
                        Some(e) => e.1 += 1,
                        None => counts.push((m, 1)),
                    }
                }
            }
            let from_resolve: Vec<TileBucket> = counts
                .iter()
                .map(|&(material_slot, pixel_count)| TileBucket {
                    material_slot,
                    pixel_count,
                })
                .collect();
            assert_eq!(
                out.tile_buckets(tile),
                from_resolve.as_slice(),
                "tile {tile} 桶与 resolve 不一致"
            );
        }
    }

    #[test]
    fn visible_cluster_materials_mapping() {
        let instances = [inst(0), inst(65535)];
        let visible = [
            VisibleCluster {
                instance: 1,
                cluster: 9,
            },
            VisibleCluster {
                instance: 0,
                cluster: 3,
            },
        ];
        // 下标 = 可见簇列表位置(顺序保持),材质经实例反查;u16 边界值通过。
        assert_eq!(
            visible_cluster_materials(&instances, &visible),
            vec![65535, 0]
        );
    }

    #[test]
    #[should_panic(expected = "16 位窄缓冲域")]
    fn visible_cluster_materials_overflow_panics() {
        let instances = [inst(65536)];
        let visible = [VisibleCluster {
            instance: 0,
            cluster: 0,
        }];
        let _ = visible_cluster_materials(&instances, &visible);
    }
}
