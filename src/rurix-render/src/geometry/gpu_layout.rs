//! GPU 数据编组(报告1 §6.1 数据结构/§6.3 资源布局;RFC-0016 §4.C + §4.0-3
//! 冻结清单)——W3 device 接线的**字节面**:簇记录/实例记录/顶点索引池/剔除
//! uniform 的 GPU buffer 上传源。
//!
//! 纪律:`#![forbid(unsafe_code)]` 下**不做 repr 重解释**(无 bytemuck 式
//! transmute),全部手写小端(LE)逐字段序列化,字段序 = 冻结契约 `repr(C)`
//! 声明序;`core::mem::offset_of!`/`size_of` 断言把手写字节布局锚定到 `repr(C)`
//! 真值(布局漂移即红),字节往返单测锁死。LE 与主机/GPU 两侧原生序一致,
//! 跨平台确定性。
//!
//! 偏移语义(冻结契约 `ClusterRecord` 字段口径,与离线 `rurix-geom-build`
//! RXGB 一致性校验同口径——**元素单位,非字节**):
//! - `vertex_offset`:顶点记录下标(每记录 12B,f32×3);
//! - `triangle_offset`:索引池元素下标(u32,每三角形 3 元素)。
//!
//! 顶点池格式:f32×3 标量流 12B/顶点(着色器按 scalar 索引,vertex pulling
//! 语义留口;**非** std430 `vec3[]` 数组——std430 vec3 数组 stride 为 16B,
//! 本格式按 `float[]`/位模式读取,陷阱不入字节面)。

use crate::graph::types::ClusterRecord;

use super::cull::CullCamera;
use super::gpu_scene::InstanceRecord;

/// 簇记录 → GPU buffer 字节(64B × n,LE,字段序 = 冻结契约声明序)。
pub fn flatten_clusters(clusters: &[ClusterRecord]) -> Vec<u8> {
    let mut out = Vec::with_capacity(clusters.len() * 64);
    for c in clusters {
        let start = out.len();
        for &x in &c.center {
            put_f32(&mut out, x);
        }
        put_f32(&mut out, c.radius);
        for &x in &c.cone_axis {
            put_f32(&mut out, x);
        }
        put_f32(&mut out, c.cone_cutoff);
        put_f32(&mut out, c.error);
        put_f32(&mut out, c.parent_error);
        put_u32(&mut out, c.vertex_offset);
        put_u32(&mut out, c.triangle_offset);
        put_u32(&mut out, c.vertex_count);
        put_u32(&mut out, c.triangle_count);
        put_u32(&mut out, c.page_id);
        put_u32(&mut out, c.reserved);
        debug_assert_eq!(out.len() - start, 64);
    }
    out
}

/// 实例记录 → GPU buffer 字节(96B × n,LE,字段序 = `gpu_scene::InstanceRecord`
/// 冻结声明序;96B = 6×16B 段对齐友好)。
pub fn flatten_instances(instances: &[InstanceRecord]) -> Vec<u8> {
    let mut out = Vec::with_capacity(instances.len() * 96);
    for r in instances {
        let start = out.len();
        for row in &r.transform {
            for &x in row {
                put_f32(&mut out, x);
            }
        }
        put_u32(&mut out, r.cluster_offset);
        put_u32(&mut out, r.cluster_count);
        put_u32(&mut out, r.material_id);
        put_u32(&mut out, r.flags);
        for &x in &r.aabb_min {
            put_f32(&mut out, x);
        }
        put_u32(&mut out, r.mesh_id);
        for &x in &r.aabb_max {
            put_f32(&mut out, x);
        }
        put_u32(&mut out, r.reserved);
        debug_assert_eq!(out.len() - start, 96);
    }
    out
}

/// 顶点池打包(f32×3 标量流,12B/顶点;全局顶点下标 =
/// `ClusterRecord::vertex_offset + 簇内局部下标`)。
pub fn pack_vertex_pool(vertices: &[[f32; 3]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(vertices.len() * 12);
    for v in vertices {
        for &x in v {
            put_f32(&mut out, x);
        }
    }
    out
}

/// 索引池打包(u32 LE;簇内局部顶点下标,每三角形 3 元素;离线 RXGB u8
/// 局部索引上载时拓宽为 u32)。
pub fn pack_index_pool(indices: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(indices.len() * 4);
    for &i in indices {
        put_u32(&mut out, i);
    }
    out
}

/// 剔除 pass uniform(std430 兼容布局,96B;`repr(C)` + 显式 reserved 使布局
/// 与 std430 规则逐字节一致——mat4 64B 后 cam_pos 落 16B 对齐槽 64,标量三联
/// 76/80/84,reserved 88 补齐 16B 倍数)。
///
/// std430 锚定:`mat4` align 16(偏移 0);`vec3` align 16(偏移 64 ✓);
/// struct align = 16、size = 96 ✓。布局锁定单测见 `cull_uniforms_std430_layout_and_bytes`。
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CullUniforms {
    /// 视图 × 投影(行主,列向量约定;与 [`CullCamera::view_proj`] 同)。
    pub view_proj: [[f32; 4]; 4],
    /// 相机世界位置(vec3,偏移 64,16B 对齐槽)。
    pub cam_pos: [f32; 3],
    /// 视口高(像素)。
    pub screen_height_px: f32,
    /// LOD 误差阈值(屏幕像素)。
    pub error_threshold_px: f32,
    /// SW/HW 分箱阈值(屏幕像素;`cull::DEFAULT_BIN_THRESHOLD_PX` 档)。
    pub bin_threshold_px: f32,
    /// 补齐 16B 倍数(std430 struct 尺寸对齐;写 0)。
    pub reserved: [f32; 2],
}

impl CullUniforms {
    /// 自剔除相机装配(分箱阈值显式给出)。
    pub fn from_camera(cam: &CullCamera, bin_threshold_px: f32) -> Self {
        Self {
            view_proj: cam.view_proj,
            cam_pos: cam.cam_pos,
            screen_height_px: cam.screen_height_px,
            error_threshold_px: cam.error_threshold_px,
            bin_threshold_px,
            reserved: [0.0; 2],
        }
    }

    /// → GPU buffer 字节(96B,LE,字段序 = 声明序)。
    pub fn to_bytes(&self) -> [u8; 96] {
        let mut out = Vec::with_capacity(96);
        for row in &self.view_proj {
            for &x in row {
                put_f32(&mut out, x);
            }
        }
        for &x in &self.cam_pos {
            put_f32(&mut out, x);
        }
        put_f32(&mut out, self.screen_height_px);
        put_f32(&mut out, self.error_threshold_px);
        put_f32(&mut out, self.bin_threshold_px);
        for &x in &self.reserved {
            put_f32(&mut out, x);
        }
        let bytes: [u8; 96] = out.try_into().expect("CullUniforms 序列化必须 96B");
        bytes
    }
}

fn put_u32(b: &mut Vec<u8>, v: u32) {
    b.extend_from_slice(&v.to_le_bytes());
}

fn put_f32(b: &mut Vec<u8>, v: f32) {
    b.extend_from_slice(&v.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::gpu_scene::IDENTITY_3X4;

    fn rd_u32(b: &[u8], off: usize) -> u32 {
        u32::from_le_bytes(b[off..off + 4].try_into().unwrap())
    }

    fn rd_f32(b: &[u8], off: usize) -> f32 {
        f32::from_le_bytes(b[off..off + 4].try_into().unwrap())
    }

    /// 位型独特的簇记录(全部字段可区分;f32 取二进精确值避免舍入噪声)。
    fn distinctive_cluster() -> ClusterRecord {
        ClusterRecord {
            center: [1.5, -2.25, 3.5],
            radius: 0.75,
            cone_axis: [0.5, -0.5, 0.25],
            cone_cutoff: 0.125,
            error: 0.0625,
            parent_error: 1024.0,
            vertex_offset: 0x1122_3344,
            triangle_offset: 0x5566_7788,
            vertex_count: 0x99AA,
            triangle_count: 0xBBCC,
            page_id: 0xDDEE,
            reserved: 0xFF00_00FF,
        }
    }

    fn assert_cluster_at(b: &[u8], off: usize, c: &ClusterRecord) {
        for k in 0..3 {
            assert_eq!(rd_f32(b, off + k * 4), c.center[k], "center[{k}]");
        }
        assert_eq!(rd_f32(b, off + 12), c.radius);
        for k in 0..3 {
            assert_eq!(
                rd_f32(b, off + 16 + k * 4),
                c.cone_axis[k],
                "cone_axis[{k}]"
            );
        }
        assert_eq!(rd_f32(b, off + 28), c.cone_cutoff);
        assert_eq!(rd_f32(b, off + 32), c.error);
        assert_eq!(rd_f32(b, off + 36), c.parent_error);
        assert_eq!(rd_u32(b, off + 40), c.vertex_offset);
        assert_eq!(rd_u32(b, off + 44), c.triangle_offset);
        assert_eq!(rd_u32(b, off + 48), c.vertex_count);
        assert_eq!(rd_u32(b, off + 52), c.triangle_count);
        assert_eq!(rd_u32(b, off + 56), c.page_id);
        assert_eq!(rd_u32(b, off + 60), c.reserved);
    }

    #[test]
    fn cluster_record_layout_and_roundtrip() {
        // 布局锚定(repr(C) 真值;手写字节序以之为据,漂移即红)。
        assert_eq!(core::mem::size_of::<ClusterRecord>(), 64);
        assert_eq!(core::mem::offset_of!(ClusterRecord, center), 0);
        assert_eq!(core::mem::offset_of!(ClusterRecord, radius), 12);
        assert_eq!(core::mem::offset_of!(ClusterRecord, cone_axis), 16);
        assert_eq!(core::mem::offset_of!(ClusterRecord, cone_cutoff), 28);
        assert_eq!(core::mem::offset_of!(ClusterRecord, error), 32);
        assert_eq!(core::mem::offset_of!(ClusterRecord, parent_error), 36);
        assert_eq!(core::mem::offset_of!(ClusterRecord, vertex_offset), 40);
        assert_eq!(core::mem::offset_of!(ClusterRecord, triangle_offset), 44);
        assert_eq!(core::mem::offset_of!(ClusterRecord, vertex_count), 48);
        assert_eq!(core::mem::offset_of!(ClusterRecord, triangle_count), 52);
        assert_eq!(core::mem::offset_of!(ClusterRecord, page_id), 56);
        assert_eq!(core::mem::offset_of!(ClusterRecord, reserved), 60);
        let c0 = distinctive_cluster();
        let mut c1 = c0;
        c1.center = [-7.5, 8.0, -9.25];
        c1.page_id = 42;
        let bytes = flatten_clusters(&[c0, c1]);
        assert_eq!(bytes.len(), 128);
        // 字节锚点:center[0] = 1.5 = 0x3FC00000 → LE [00,00,C0,3F]。
        assert_eq!(&bytes[0..4], &[0x00, 0x00, 0xC0, 0x3F]);
        // center[1] = −2.25 = 0xC0100000;center[2] = 3.5 = 0x40600000。
        assert_eq!(&bytes[4..8], &[0x00, 0x00, 0x10, 0xC0]);
        assert_eq!(&bytes[8..12], &[0x00, 0x00, 0x60, 0x40]);
        // 往返:两条记录逐字段还原。
        assert_cluster_at(&bytes, 0, &c0);
        assert_cluster_at(&bytes, 64, &c1);
    }

    #[test]
    fn instance_record_layout_and_roundtrip() {
        // gpu_scene 侧已锁 size/offset;编组边界再锚一次(字节面与 repr(C) 一致)。
        assert_eq!(core::mem::size_of::<InstanceRecord>(), 96);
        let r0 = InstanceRecord {
            transform: [
                [1.0, 0.0, 0.0, 7.5],
                [0.0, 2.0, 0.0, -3.25],
                [0.0, 0.0, 4.0, 11.0],
            ],
            cluster_offset: 0x0102_0304,
            cluster_count: 0x0506,
            material_id: 0x0708,
            flags: 0xA5A5_0000,
            aabb_min: [-1.5, -2.5, -3.5],
            mesh_id: 0x0B0C,
            aabb_max: [4.5, 5.5, 6.5],
            reserved: 0xFFFF_FFFE,
        };
        let r1 = InstanceRecord {
            transform: IDENTITY_3X4,
            cluster_offset: 0,
            cluster_count: 0,
            material_id: 0,
            flags: 0,
            aabb_min: [0.0; 3],
            mesh_id: 0,
            aabb_max: [0.0; 3],
            reserved: u32::MAX,
        };
        let bytes = flatten_instances(&[r0, r1]);
        assert_eq!(bytes.len(), 192);
        // 字节锚点:transform[0][3] = 7.5 = 0x40F00000 于偏移 12。
        assert_eq!(rd_f32(&bytes, 12), 7.5);
        assert_eq!(&bytes[12..16], &[0x00, 0x00, 0xF0, 0x40]);
        // 往返:逐字段还原两条记录。
        for (i, want) in [r0, r1].iter().enumerate() {
            let off = i * 96;
            for (r, row) in want.transform.iter().enumerate() {
                for (c, &x) in row.iter().enumerate() {
                    assert_eq!(
                        rd_f32(&bytes, off + (r * 4 + c) * 4),
                        x,
                        "transform[{r}][{c}]"
                    );
                }
            }
            assert_eq!(rd_u32(&bytes, off + 48), want.cluster_offset);
            assert_eq!(rd_u32(&bytes, off + 52), want.cluster_count);
            assert_eq!(rd_u32(&bytes, off + 56), want.material_id);
            assert_eq!(rd_u32(&bytes, off + 60), want.flags);
            for k in 0..3 {
                assert_eq!(rd_f32(&bytes, off + 64 + k * 4), want.aabb_min[k]);
            }
            assert_eq!(rd_u32(&bytes, off + 76), want.mesh_id);
            for k in 0..3 {
                assert_eq!(rd_f32(&bytes, off + 80 + k * 4), want.aabb_max[k]);
            }
            assert_eq!(rd_u32(&bytes, off + 92), want.reserved);
        }
    }

    #[test]
    fn pools_packing_and_offset_semantics() {
        let vertices = [[1.0, 2.0, 3.0], [-4.0, 5.5, -6.25]];
        let vbytes = pack_vertex_pool(&vertices);
        assert_eq!(vbytes.len(), 24, "12B/顶点");
        // 顶点 1 的 x = −4.0 = 0xC0800000 于字节 12。
        assert_eq!(&vbytes[12..16], &[0x00, 0x00, 0x80, 0xC0]);
        let indices = [0x0102_0304u32, 7, 0xFFFF_FFFF];
        let ibytes = pack_index_pool(&indices);
        assert_eq!(ibytes.len(), 12);
        assert_eq!(rd_u32(&ibytes, 0), 0x0102_0304);
        assert_eq!(rd_u32(&ibytes, 8), 0xFFFF_FFFF);
        // 偏移语义(元素单位):vertex_offset = 1 ⇒ 字节 12 处读顶点 1;
        // triangle_offset = 1 ⇒ 索引元素 1(u32 = 7)处读三角形索引。
        let rec = ClusterRecord {
            vertex_offset: 1,
            triangle_offset: 1,
            ..distinctive_cluster()
        };
        let vx = rd_f32(&vbytes, rec.vertex_offset as usize * 12);
        assert_eq!(vx, -4.0);
        let ix = rd_u32(&ibytes, rec.triangle_offset as usize * 4);
        assert_eq!(ix, 7);
    }

    #[test]
    fn cull_uniforms_std430_layout_and_bytes() {
        // std430 兼容布局锚定(96B,16B 倍数;cam_pos 落 16B 对齐槽 64)。
        assert_eq!(core::mem::size_of::<CullUniforms>(), 96);
        assert_eq!(core::mem::offset_of!(CullUniforms, view_proj), 0);
        assert_eq!(core::mem::offset_of!(CullUniforms, cam_pos), 64);
        assert_eq!(core::mem::offset_of!(CullUniforms, screen_height_px), 76);
        assert_eq!(core::mem::offset_of!(CullUniforms, error_threshold_px), 80);
        assert_eq!(core::mem::offset_of!(CullUniforms, bin_threshold_px), 84);
        assert_eq!(core::mem::offset_of!(CullUniforms, reserved), 88);
        let cam = CullCamera {
            view_proj: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 2.0, 0.0, 0.0],
                [0.0, 0.0, -1.0, -0.2],
                [0.0, 0.0, -1.0, 0.0],
            ],
            cam_pos: [3.5, -4.25, 5.75],
            screen_height_px: 1080.0,
            error_threshold_px: 1.0,
        };
        let u = CullUniforms::from_camera(&cam, 32.0);
        assert_eq!(u.reserved, [0.0; 2]);
        let bytes = u.to_bytes();
        // 字节锚点:view_proj[1][1] = 2.0 = 0x40000000 于偏移 20;
        // cam_pos[0] = 3.5 于 64;screen_height = 1080.0 = 0x44870000 于 76。
        assert_eq!(&bytes[20..24], &[0x00, 0x00, 0x00, 0x40]);
        assert_eq!(rd_f32(&bytes, 64), 3.5);
        assert_eq!(rd_f32(&bytes, 68), -4.25);
        assert_eq!(rd_f32(&bytes, 72), 5.75);
        assert_eq!(rd_f32(&bytes, 76), 1080.0);
        assert_eq!(rd_f32(&bytes, 80), 1.0);
        assert_eq!(rd_f32(&bytes, 84), 32.0);
        assert_eq!(rd_f32(&bytes, 88), 0.0);
        assert_eq!(rd_f32(&bytes, 92), 0.0);
        // 往返:view_proj 逐元素还原。
        for r in 0..4 {
            for c in 0..4 {
                assert_eq!(rd_f32(&bytes, (r * 4 + c) * 4), cam.view_proj[r][c]);
            }
        }
        // 确定性:同输入逐字节同输出。
        assert_eq!(
            u.to_bytes(),
            CullUniforms::from_camera(&cam, 32.0).to_bytes()
        );
    }
}
