//! 64 位 VisBuffer CPU 光栅参考(报告1 §3.3/§3.4 P2;RFC-0016 §4.C3)——SW 软光栅
//! compute pass 的 host 金标准,语义逐条对应 GPU 线程模型(组内逐顶点变换入
//! group shared,再 1 线程/三角形 scanline,`atomicMax u64` 写 VisBuffer)。
//!
//! ## 钉死裁决(SW/HW 双路一致性的前提,W3 device 接线按此对拍,逐像素 diff = 0
//! ## 整数域,RFC §4.C3 验收口径)
//!
//! 1. **位格式与位序**:冻结契约 `u64 = depth:30 | cluster:27 | tri:7`,depth 在
//!    高位 ⇒ u64 无符号整数比较 = 字典序 (depth, cluster, tri),`atomicMax` 单
//!    指令同时完成深度测试与可见性记录(报告1 §3.4 Nanite 口径;
//!    `graph::types::visbuffer_pack` 单源)。同深度并列时按打包值序(簇/三角形
//!    序号大者胜)——GPU atomicMax 同语义,确定性。
//! 2. **深度量化 = reverse-Z 线性 30 位**:`q(z) = round((1 − z_ndc)·(2³⁰−1))`,
//!    近大远小(更近 ⇒ 更大 depth30 ⇒ atomicMax 近者胜)。依据:报告1 §7「深度位
//!    直接用与硬件深度缓冲相同的量化」——reverse-Z 是硬件深度缓冲精度最优的
//!    工业惯例(Nanite/UE 共识);ZO 投影下 `1 − z_ndc` 与 reverse-Z 投影矩阵的
//!    ndc **精确相等**(恒等式 z' = 1 − z),故本量化 ≡ reverse-Z 投影 + 线性
//!    30 位量化,无需改动相机矩阵。round 语义 = f32 最近舍入(半进远离零,
//!    非负域等价 `floor(x + 0.5)`)。量化结果 **clamp 到 ≥1**:clear 值恰为
//!    2³⁴−1,最小有效写入 `pack(1, 0, 0) = 2³⁴` ⇒ 任何有效覆盖 atomicMax 必胜,
//!    无效像素恒为 clear 值,由 `cluster == CLUSTER_INVALID` 判定。
//! 3. **绕向与填充规则**:输入索引按 RH 世界 CCW-向外(glTF/meshopt 惯例);
//!    屏幕坐标 x 右、y 下(视口 `sx = (nx+1)·W/2`,`sy = (1−ny)·H/2`),此相机
//!    约定下**正面 ⟺ signed_area2 < 0**;内部统一归一化为正绕向后,像素中心
//!    恰好落在边上时按 **top-left 规则**取等号侧——屏幕坐标正绕向下:
//!    边 (a→b),d = b−a,top-left ⟺ `d.y > 0 || (d.y == 0 && d.x > 0)`
//!    (与 D3D y-up 规则经 y 翻转共轭的推导,共享边两侧绕向相反 ⇒ 规则互补,
//!    恰好一侧命中:无双写无缝隙,`quad_two_triangles_exact_partition` 锚定)。
//! 4. **退化确定性跳过**:零面积(area2 == 0)、背面(area2 > 0)、任一顶点
//!    近平面穿越(clip w ≤ 0 ⇒ 整三角形保守丢弃——P0 简化,近平面裁剪器
//!    归 W3 device 接线时裁决)、完全屏外(包围盒裁剪后为空)。
//! 5. **深度插值**:ndc z 在屏幕空间线性(透视投影性质),按边函数重心权重
//!    直接插值,不做透视校正(校正项在 ndc 域恒等)。
//!
//! `cluster27` 写**本帧可见簇列表下标**(`visible` 参数位置),非全局簇表号——
//! 材质经可见簇 → 实例反查(报告1 §3.4;`material_pass` 同口径)。

use crate::graph::types::{
    ClusterRecord, VISBUFFER_CLUSTER_BITS, VISBUFFER_TRI_BITS, visbuffer_pack, visbuffer_unpack,
};
use crate::temporal::common::Mat4;

use super::cull::VisibleCluster;
use super::gpu_scene::{InstanceRecord, transform_point};

/// 无效簇标记(clear 值的 cluster 段;27 位全 1)。
pub const CLUSTER_INVALID: u32 = (1 << VISBUFFER_CLUSTER_BITS) - 1;
/// 无效三角形标记(clear 值的 tri 段;7 位全 1)。
pub const TRI_INVALID: u32 = (1 << VISBUFFER_TRI_BITS) - 1;
/// VisBuffer clear 值 = pack(0, CLUSTER_INVALID, TRI_INVALID) = 2³⁴−1。
/// 最小有效写入 pack(1, 0, 0) = 2³⁴ ⇒ atomicMax 下有效覆盖必胜(裁决 2)。
pub const VISBUFFER_CLEAR: u64 =
    ((CLUSTER_INVALID as u64) << VISBUFFER_TRI_BITS) | TRI_INVALID as u64;

/// 深度 30 位满量程。
pub const DEPTH30_MAX: u32 = (1 << 30) - 1;

/// reverse-Z 线性 30 位深度量化(裁决 2;近 ⇒ 大,远 ⇒ 小,有效写入 ≥1)。
pub fn quantize_depth30(z_ndc: f32) -> u32 {
    let z = z_ndc.clamp(0.0, 1.0);
    let q = ((1.0 - z) * DEPTH30_MAX as f32).round() as u32;
    q.clamp(1, DEPTH30_MAX)
}

/// CPU VisBuffer(u64 全屏,行主;device 对应 = u64 storage buffer,
/// `VK_KHR_shader_atomic_int64` 的 `shaderBufferInt64Atomics` 主承诺面,
/// RFC §9.1 R-5 修订——非 R64 image)。
#[derive(Debug, Clone, PartialEq)]
pub struct VisBufferCpu {
    pub w: u32,
    pub h: u32,
    pub data: Vec<u64>,
}

impl VisBufferCpu {
    /// 新建并清为 [`VISBUFFER_CLEAR`](无效像素约定)。
    pub fn new(w: u32, h: u32) -> Self {
        Self {
            w,
            h,
            data: vec![VISBUFFER_CLEAR; (w * h) as usize],
        }
    }

    /// 清屏(帧首语义;device = 图内 clear 或首写前屏障后的 fill)。
    pub fn clear(&mut self) {
        self.data.fill(VISBUFFER_CLEAR);
    }

    pub fn get(&self, x: u32, y: u32) -> u64 {
        self.data[(y * self.w + x) as usize]
    }

    /// 无效像素判定(clear 值等价:`cluster == CLUSTER_INVALID`)。
    pub fn is_invalid(&self, x: u32, y: u32) -> bool {
        visbuffer_unpack(self.get(x, y)).1 == CLUSTER_INVALID
    }

    /// 有效(有覆盖)像素计数(验收锚定用)。
    pub fn count_valid(&self) -> usize {
        self.data
            .iter()
            .filter(|&&v| visbuffer_unpack(v).1 != CLUSTER_INVALID)
            .count()
    }

    /// atomicMax 单点写(device 语义内核:仅当新值 > 旧值才写;u64 整数比较
    /// 天然实现「深度近者胜」,位序依据见模块文档裁决 1)。返回是否写入。
    fn atomic_max(&mut self, x: u32, y: u32, v: u64) -> bool {
        let px = &mut self.data[(y * self.w + x) as usize];
        if v > *px {
            *px = v;
            true
        } else {
            false
        }
    }

    /// 单三角形光栅(边函数 + top-left + 重心深度插值 + atomicMax;
    /// `screen_tri` = 屏幕像素坐标 (x, y) + ndc 深度 z ∈ [0,1])。
    ///
    /// 跳过规则(裁决 4):area2 == 0 退化 / area2 > 0 背面(正面 ⟺ <0)。
    pub fn raster_triangle(&mut self, screen_tri: &[[f32; 3]; 3], cluster: u32, tri: u32) {
        debug_assert!(cluster < (1 << VISBUFFER_CLUSTER_BITS));
        debug_assert!(tri < (1 << VISBUFFER_TRI_BITS));
        let [a0, b0, c0] = *screen_tri;
        let area2 = cross2(sub2(b0, a0), sub2(c0, a0));
        if area2 >= 0.0 {
            return; // 退化(==0)或背面(>0),确定性跳过
        }
        // 归一化为正绕向(交换 v1/v2),top-left 规则在正绕向路径定义。
        let (a, b, c) = (a0, c0, b0);
        let pos_area = -area2;
        let edge_ab = sub2(b, a);
        let edge_bc = sub2(c, b);
        let edge_ca = sub2(a, c);
        let tl_ab = top_left(edge_ab);
        let tl_bc = top_left(edge_bc);
        let tl_ca = top_left(edge_ca);
        // 包围盒(像素角坐标),裁剪到屏内。
        let (w, h) = (self.w as i32, self.h as i32);
        let x0 = (a[0].min(b[0]).min(c[0]).floor() as i32).max(0);
        let x1 = (a[0].max(b[0]).max(c[0]).ceil() as i32).min(w);
        let y0 = (a[1].min(b[1]).min(c[1]).floor() as i32).max(0);
        let y1 = (a[1].max(b[1]).max(c[1]).ceil() as i32).min(h);
        for py in y0..y1 {
            for px in x0..x1 {
                let p = [px as f32 + 0.5, py as f32 + 0.5, 0.0];
                // 边函数(正绕向内部 ≥ 0;恰在边上按 top-left 取等号侧)。
                let e_bc = cross2(edge_bc, sub2(p, b));
                let e_ca = cross2(edge_ca, sub2(p, c));
                let e_ab = cross2(edge_ab, sub2(p, a));
                let inside = (e_bc > 0.0 || (e_bc == 0.0 && tl_bc))
                    && (e_ca > 0.0 || (e_ca == 0.0 && tl_ca))
                    && (e_ab > 0.0 || (e_ab == 0.0 && tl_ab));
                if !inside {
                    continue;
                }
                // 重心权重(ndc z 屏幕空间线性,直接插值;裁决 5)。
                let z = (e_bc * a[2] + e_ca * b[2] + e_ab * c[2]) / pos_area;
                let v = visbuffer_pack(quantize_depth30(z), cluster, tri);
                self.atomic_max(px as u32, py as u32, v);
            }
        }
    }
}

/// 光栅场景面(可见簇列表 + 实例/簇表 + 顶点/索引池 + view_proj)。
///
/// 池语义(与 `gpu_layout` 编组一致;冻结契约 `ClusterRecord` 字段口径):
/// `vertices` = 对象空间 f32×3 全局池,`indices` = u32 全局池、**簇内局部顶点
/// 下标**(0..vertex_count;离线 RXGB u8 局部索引上载时拓宽);全局顶点下标 =
/// `vertex_offset + indices[triangle_offset + 3t + k]`。
#[derive(Debug, Clone, Copy)]
pub struct RasterScene<'a> {
    pub instances: &'a [InstanceRecord],
    pub clusters: &'a [ClusterRecord],
    pub vertices: &'a [[f32; 3]],
    pub indices: &'a [u32],
    /// 视图 × 投影(行主、列向量约定;与剔除同一相机)。
    pub view_proj: [[f32; 4]; 4],
}

/// 可见簇列表 → 全屏 VisBuffer(host 蛮力:逐簇逐三角形变换 + 光栅;
/// device = sw_raster compute pass,组内先逐顶点变换入 shared 再 1 线程/三角形)。
///
/// `visible` 的**位置**即写入像素的 cluster27 值(帧内可见簇列表下标裁决)。
/// 任一顶点 clip w ≤ 0 ⇒ 整三角形保守丢弃(裁决 4,近平面 P0 简化)。
pub fn raster_clusters(
    vis: &mut VisBufferCpu,
    visible: &[VisibleCluster],
    scene: &RasterScene<'_>,
) {
    let vp = Mat4 { m: scene.view_proj };
    let (w_px, h_px) = (vis.w as f32, vis.h as f32);
    for (vis_idx, vc) in visible.iter().enumerate() {
        let inst = &scene.instances[vc.instance as usize];
        let c = &scene.clusters[vc.cluster as usize];
        let cluster27 = vis_idx as u32;
        for t in 0..c.triangle_count {
            let mut screen = [[0.0f32; 3]; 3];
            let mut valid = true;
            for (k, sv) in screen.iter_mut().enumerate() {
                let local = scene.indices[(c.triangle_offset + 3 * t) as usize + k];
                let obj = scene.vertices[(c.vertex_offset + local) as usize];
                let world = transform_point(&inst.transform, obj);
                let clip = vp.transform_vec4([world[0], world[1], world[2], 1.0]);
                if clip[3] <= 0.0 {
                    valid = false; // 近平面穿越/相机背后 ⇒ 保守丢弃(裁决 4)
                    break;
                }
                let inv_w = 1.0 / clip[3];
                let nx = clip[0] * inv_w;
                let ny = clip[1] * inv_w;
                let nz = (clip[2] * inv_w).clamp(0.0, 1.0);
                *sv = [(nx + 1.0) * 0.5 * w_px, (1.0 - ny) * 0.5 * h_px, nz];
            }
            if valid {
                vis.raster_triangle(&screen, cluster27, t);
            }
        }
    }
}

fn sub2(a: [f32; 3], b: [f32; 3]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

/// 2D 叉积(x、y 分量;z 不参与)。
fn cross2(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[1] - a[1] * b[0]
}

/// top-left 规则(屏幕坐标、正绕向;d = 边方向;推导见模块文档裁决 3)。
fn top_left(d: [f32; 2]) -> bool {
    d[1] > 0.0 || (d[1] == 0.0 && d[0] > 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::cull::{CullCamera, VisibleCluster, cluster_cull, instance_cull};
    use crate::geometry::gpu_scene::{GpuScene, IDENTITY_3X4};
    use crate::geometry::material_pass::{classify, resolve, visible_cluster_materials};

    #[test]
    fn clear_and_invalid_convention() {
        // clear = 2³⁴−1;与 pack(0, INVALID, INVALID) 同值;最小有效写入 2³⁴ 必胜。
        assert_eq!(VISBUFFER_CLEAR, (1u64 << 34) - 1);
        assert_eq!(
            VISBUFFER_CLEAR,
            visbuffer_pack(0, CLUSTER_INVALID, TRI_INVALID)
        );
        assert!(visbuffer_pack(1, 0, 0) > VISBUFFER_CLEAR);
        let vis = VisBufferCpu::new(4, 3);
        assert_eq!(vis.data.len(), 12);
        assert!(vis.data.iter().all(|&v| v == VISBUFFER_CLEAR));
        assert!(vis.is_invalid(0, 0));
        assert_eq!(vis.count_valid(), 0);
    }

    #[test]
    fn quantize_monotonic_reverse_z() {
        // reverse-Z:近(z_ndc 小)⇒ depth30 大;边界 z = 0 → 满量程,z = 1 → clamp 1。
        assert_eq!(quantize_depth30(0.0), DEPTH30_MAX);
        assert_eq!(quantize_depth30(1.0), 1);
        // z = 0.5 精确锚点:0.5·(2³⁰−1) = 536870911.5 半进 ⇒ 536870912 = 2²⁹。
        assert_eq!(quantize_depth30(0.5), 1 << 29);
        let samples = [0.0, 0.25, 0.5, 0.75, 0.9, 1.0];
        for w in samples.windows(2) {
            assert!(
                quantize_depth30(w[0]) > quantize_depth30(w[1]),
                "z {} 应比 {} 更大 depth30",
                w[0],
                w[1]
            );
        }
        // 越界钳制(光栅侧 ndc clamp [0,1] 的双保险)。
        assert_eq!(quantize_depth30(-0.5), DEPTH30_MAX);
        assert_eq!(quantize_depth30(1.5), 1);
    }

    #[test]
    fn single_triangle_pixel_count_anchor() {
        // 8×8,三角形 (0,0),(0,4),(4,0)(area2 = −16 <0 正面)。
        // 覆盖 ⟺ (px+0.5)+(py+0.5) ≤ 4 ⟺ px+py ≤ 3 ⇒ 手算 1+2+3+4 = 10 像素。
        let mut vis = VisBufferCpu::new(8, 8);
        vis.raster_triangle(&[[0.0, 0.0, 0.5], [0.0, 4.0, 0.5], [4.0, 0.0, 0.5]], 7, 3);
        assert_eq!(vis.count_valid(), 10);
        for y in 0..8u32 {
            for x in 0..8u32 {
                let covered = x + y <= 3;
                assert_eq!(!vis.is_invalid(x, y), covered, "({x},{y})");
                if covered {
                    // 深度恒 0.5 ⇒ depth30 = 2²⁹(quantize 锚点);簇/三角形号随写。
                    assert_eq!(
                        visbuffer_unpack(vis.get(x, y)),
                        (1 << 29, 7, 3),
                        "({x},{y})"
                    );
                }
            }
        }
    }

    #[test]
    fn depth_competition_atomic_max() {
        // 同一三角形两位写入:近(z = 0.25,簇 1)与远(z = 0.75,簇 2)。
        // 与写入顺序无关,近者胜(atomicMax = 深度测试)。
        for near_first in [true, false] {
            let mut vis = VisBufferCpu::new(8, 8);
            let near = [[0.0, 0.0, 0.25], [0.0, 4.0, 0.25], [4.0, 0.0, 0.25]];
            let far = [[0.0, 0.0, 0.75], [0.0, 4.0, 0.75], [4.0, 0.0, 0.75]];
            if near_first {
                vis.raster_triangle(&near, 1, 0);
                vis.raster_triangle(&far, 2, 0);
            } else {
                vis.raster_triangle(&far, 2, 0);
                vis.raster_triangle(&near, 1, 0);
            }
            assert_eq!(vis.count_valid(), 10);
            let (d, c, _) = visbuffer_unpack(vis.get(0, 0));
            assert_eq!(c, 1, "近者胜(顺序 near_first={near_first})");
            assert_eq!(d, quantize_depth30(0.25));
        }
        // 同深度并列:按打包值序,簇号大者胜(裁决 1 确定性 tie-break)。
        let mut vis = VisBufferCpu::new(8, 8);
        let t = [[0.0, 0.0, 0.5], [0.0, 4.0, 0.5], [4.0, 0.0, 0.5]];
        vis.raster_triangle(&t, 1, 0);
        vis.raster_triangle(&t, 2, 0);
        assert_eq!(visbuffer_unpack(vis.get(0, 0)).1, 2);
    }

    #[test]
    fn quad_two_triangles_exact_partition() {
        // 4×4 方块两三角形(对角线过像素中心 (i+0.5, i+0.5),top-left 决胜):
        // T_a (0,0),(4,4),(4,0) 区域 x ≥ y(对角线排除);T_b (0,0),(0,4),(4,4)
        // 区域 y ≥ x(对角线纳入)。手算:T_a 6 像素,T_b 10 像素(对角线 4 归 T_b)。
        let mut vis = VisBufferCpu::new(4, 4);
        vis.raster_triangle(&[[0.0, 0.0, 0.5], [4.0, 4.0, 0.5], [4.0, 0.0, 0.5]], 0, 0);
        vis.raster_triangle(&[[0.0, 0.0, 0.5], [0.0, 4.0, 0.5], [4.0, 4.0, 0.5]], 1, 0);
        assert_eq!(vis.count_valid(), 16, "整方块 4×4 恰好全覆盖");
        let (mut a, mut b) = (0u32, 0u32);
        for y in 0..4u32 {
            for x in 0..4u32 {
                let (_, cluster, _) = visbuffer_unpack(vis.get(x, y));
                if x == y {
                    assert_eq!(cluster, 1, "对角线像素 ({x},{y}) 归 T_b(top-left)");
                }
                match cluster {
                    0 => a += 1,
                    1 => b += 1,
                    _ => panic!("越界簇号"),
                }
            }
        }
        assert_eq!((a, b), (6, 10), "双写或缝隙将破坏手算分布");
    }

    #[test]
    fn degenerate_backface_offscreen_behind_skipped() {
        let mut vis = VisBufferCpu::new(8, 8);
        // 零面积(共线)。
        vis.raster_triangle(&[[0.0, 0.0, 0.5], [2.0, 2.0, 0.5], [4.0, 4.0, 0.5]], 0, 0);
        // 背面(area2 > 0)。
        vis.raster_triangle(&[[0.0, 0.0, 0.5], [4.0, 0.0, 0.5], [0.0, 4.0, 0.5]], 0, 0);
        // 完全屏外(x ∈ [10, 14])。
        vis.raster_triangle(
            &[[10.0, 0.0, 0.5], [10.0, 4.0, 0.5], [14.0, 0.0, 0.5]],
            0,
            0,
        );
        assert_eq!(vis.count_valid(), 0);
        // 近平面穿越(一个顶点 w ≤ 0)⇒ 整三角形保守丢弃(raster_clusters 面)。
        let scene = RasterScene {
            instances: &[InstanceRecord {
                transform: IDENTITY_3X4,
                cluster_offset: 0,
                cluster_count: 1,
                material_id: 0,
                flags: 0,
                aabb_min: [-1.0; 3],
                mesh_id: 0,
                aabb_max: [1.0; 3],
                reserved: u32::MAX,
            }],
            clusters: &[ClusterRecord {
                center: [0.0; 3],
                radius: 1.0,
                cone_axis: [0.0, 0.0, 1.0],
                cone_cutoff: 2.0,
                error: 0.0,
                parent_error: f32::INFINITY,
                vertex_offset: 0,
                triangle_offset: 0,
                vertex_count: 3,
                triangle_count: 1,
                page_id: 0,
                reserved: 0,
            }],
            vertices: &[[-1.0, 0.0, -2.0], [1.0, 0.0, -2.0], [0.0, 1.0, 1.0]],
            indices: &[0, 1, 2],
            view_proj: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, -1.0, -0.2],
                [0.0, 0.0, -1.0, 0.0],
            ],
        };
        let mut vis2 = VisBufferCpu::new(8, 8);
        raster_clusters(
            &mut vis2,
            &[VisibleCluster {
                instance: 0,
                cluster: 0,
            }],
            &scene,
        );
        assert_eq!(vis2.count_valid(), 0, "近平面穿越三角形必须整弃(P0 简化)");
    }

    // -----------------------------------------------------------------------
    // 端到端:手工立方体 2 实例 × 3 簇 → 两级剔除 → 光栅 → classify/resolve。
    // -----------------------------------------------------------------------

    /// 立方体 [-1,1]³ 八顶点(A..H = (∓1,∓1,∓1) 二进制下标序)。
    #[allow(clippy::type_complexity)]
    fn cube_pools() -> (Vec<[f32; 3]>, Vec<u32>, Vec<ClusterRecord>) {
        let [a, b, c, d] = [
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
        ];
        let [e, f, g, h] = [
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ];
        // 顶点池:簇 0 段 EFGH(0..4),簇 1 段 ABCD(4..8),簇 2 段 A..H(8..16)。
        let vertices = vec![e, f, g, h, a, b, c, d, a, b, c, d, e, f, g, h];
        // 索引池(簇内局部,RH CCW 向外):
        // 簇 0 = +z 面 [E,F,G],[E,G,H] → 局部 [0,1,2],[0,2,3];
        // 簇 1 = −z 面 [B,A,D],[B,D,C] → 局部 [1,0,3],[1,3,2];
        // 簇 2 = 侧四面(局部 0..8 = A..H):+x [F,B,C],[F,C,G] / −x [A,E,H],[A,H,D]
        //         / +y [H,G,C],[H,C,D] / −y [E,A,B],[E,B,F]。
        let indices = vec![
            0, 1, 2, 0, 2, 3, // 簇 0(2 三角形)
            1, 0, 3, 1, 3, 2, // 簇 1(2 三角形)
            5, 1, 2, 5, 2, 6, 0, 4, 7, 0, 7, 3, 7, 6, 2, 7, 2, 3, 4, 0, 1, 4, 1,
            5, // 簇 2(8 三角形)
        ];
        let rec = |center, radius, axis, cutoff, voff, vcnt, toff, tcnt| ClusterRecord {
            center,
            radius,
            cone_axis: axis,
            cone_cutoff: cutoff,
            error: 0.0,
            parent_error: f32::INFINITY,
            vertex_offset: voff,
            triangle_offset: toff,
            vertex_count: vcnt,
            triangle_count: tcnt,
            page_id: 0,
            reserved: 0,
        };
        let sq2 = 2.0f32.sqrt();
        let sq3 = 3.0f32.sqrt();
        let clusters = vec![
            rec([0.0, 0.0, 1.0], sq2, [0.0, 0.0, 1.0], 0.0, 0, 4, 0, 2),
            rec([0.0, 0.0, -1.0], sq2, [0.0, 0.0, -1.0], 0.0, 4, 4, 6, 2),
            rec([0.0, 0.0, 0.0], sq3, [0.0, 0.0, 1.0], 2.0, 8, 8, 12, 8),
        ];
        (vertices, indices, clusters)
    }

    #[test]
    fn end_to_end_cube_cull_raster_resolve() {
        let (vertices, indices, clusters) = cube_pools();
        // 场景:mesh 0(簇 0..3,AABB ±1);实例 0 平移 (0,0,-5) 材质 7;实例 1 平移
        // (100,0,-5) 材质 9(视锥外,实例级剔除)。
        let mut scene = GpuScene::new();
        let mesh = scene.add_mesh(0, 3, [-1.0; 3], [1.0; 3]);
        let i0 = scene.add_instance(
            mesh,
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, -5.0],
            ],
            7,
            0,
        );
        let i1 = scene.add_instance(
            mesh,
            [
                [1.0, 0.0, 0.0, 100.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, -5.0],
            ],
            9,
            0,
        );
        assert_eq!((i0, i1), (0, 1));
        // 相机:原点朝 −z,m00 = m11 = 4(精确)⇒ +z 面(世界 z = −4)ndc 恰 ±1,
        // W = H = 16 时铺满全屏;near 0.2、far +∞(剔除/光栅共用同一矩阵)。
        let vp = [
            [4.0, 0.0, 0.0, 0.0],
            [0.0, 4.0, 0.0, 0.0],
            [0.0, 0.0, -1.0, -0.2],
            [0.0, 0.0, -1.0, 0.0],
        ];
        let cam = CullCamera {
            view_proj: vp,
            cam_pos: [0.0, 0.0, 0.0],
            screen_height_px: 16.0,
            error_threshold_px: 1.0,
        };
        // 第一级:实例剔除 ⇒ 仅实例 0(实例 1 在 x = 100 窄视锥 |x| ≤ |z|/4 之外)。
        let vis_instances = instance_cull(scene.instances(), &cam);
        assert_eq!(vis_instances, vec![0]);
        // 第二级:簇剔除 ⇒ 簇 0 留(+z 面)、簇 1 锥剔(view·axis = 1 ≥ 0)、
        // 簇 2 留(锥禁用;其 8 个侧/背三角形光栅阶段逐三角形背面跳过)。
        let visible = cluster_cull(scene.instances(), &vis_instances, &clusters, &cam);
        assert_eq!(
            visible,
            vec![
                VisibleCluster {
                    instance: 0,
                    cluster: 0
                },
                VisibleCluster {
                    instance: 0,
                    cluster: 2
                },
            ]
        );
        // 光栅:+z 面铺满全屏 ⇒ 256 像素;全部来自可见簇列表位置 0(簇 0);
        // 深度恒定(平面 d = 4 ⇒ ndc z = 3.8/4 = 0.95)。
        let mut vis = VisBufferCpu::new(16, 16);
        let rs = RasterScene {
            instances: scene.instances(),
            clusters: &clusters,
            vertices: &vertices,
            indices: &indices,
            view_proj: vp,
        };
        raster_clusters(&mut vis, &visible, &rs);
        assert_eq!(vis.count_valid(), 256, "+z 面应恰铺满 16×16");
        // 共面(d = 4 ⇒ ndc z = 0.95 恒定):深度逐像素一致到 f32 舍入内——重心
        // 插值为「三次乘加 + 一次除」多步舍入,z 误差界 ≈3 ulp(0.95 ∈ [0.5,1)
        // 处 ulp = 2⁻²⁴)⇒ depth30 偏差 ≤ 3·2⁻²⁴·(2³⁰−1) ≈ 192 quanta(实测
        // 漂移 128 = 2 ulp,在界内)。绕向/插值错误将产生 ~1e8 quanta 量级漂移,
        // 非平凡锚定。
        let expect_depth = quantize_depth30(0.95);
        let (mut d_min, mut d_max) = (u32::MAX, 0u32);
        for y in 0..16u32 {
            for x in 0..16u32 {
                let (d, c, t) = visbuffer_unpack(vis.get(x, y));
                assert_eq!(c, 0, "({x},{y}) 应写可见簇列表位置 0");
                assert!(t <= 1, "({x},{y}) 簇 0 仅 2 三角形");
                d_min = d_min.min(d);
                d_max = d_max.max(d);
            }
        }
        assert!(
            d_max - d_min <= 192,
            "共面深度漂移 {d_min}..{d_max} 超 3 ulp 界"
        );
        assert!(
            expect_depth.abs_diff(d_min) <= 192 && expect_depth.abs_diff(d_max) <= 192,
            "深度 {d_min}..{d_max} 未锚定 0.95 量化值 {expect_depth}"
        );
        // resolve:两可见簇均属实例 0(材质 7)⇒ 全屏 256 像素 = 7,无无效像素。
        let c2m = visible_cluster_materials(scene.instances(), &visible);
        assert_eq!(c2m, vec![7, 7]);
        let mat = resolve(&vis, &c2m);
        assert!(mat.iter().all(|&m| m == 7));
        // classify tile 4 ⇒ 4×4 = 16 tile,每 tile 桶恰为 [(7, 16)]。
        let out = classify(&vis, &c2m, 4);
        assert_eq!((out.tiles_x, out.tiles_y), (4, 4));
        assert_eq!(out.tile_offsets.len(), 17);
        assert_eq!(out.buckets.len(), 16);
        for (i, tk) in out.buckets.iter().enumerate() {
            assert_eq!((tk.material_slot, tk.pixel_count), (7, 16), "tile {i}");
            assert_eq!(out.tile_offsets[i], i as u32, "前缀和 {i}");
        }
    }
}
