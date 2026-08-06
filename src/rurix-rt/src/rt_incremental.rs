//! G8.2 M50 RT pipeline 增量运行时面(RXS-0326/0327;RFC-0019 §4.1)。
//!
//! - [`plan_sbt_v2`]:四 region(raygen/miss/hit/callable)SBT 布局(既有 [`crate::vk::plan_sbt`] 0-byte)
//! - [`pack_shader_record`]:唯一合法 record 编码入口(禁 repr(C) memcpy 契约)
//! - [`compute_rt_stack_size`]:stack 保守公式(版本进 evidence)
//! - [`run_rt_pipeline_offscreen`]:增量 pipeline device 入口(既有 `run_ray_tracing_offscreen` 0-byte)
//!
//! unsafe 全部归 **U30 扩注**(0 新 U)。

use crate::vk::{self, align_up};

/// stack 保守公式版本(进 evidence;`configured >= required` 复核)。
pub const RT_STACK_FORMULA_VERSION: &str = "rurix.rt-stack.v1";

/// SBT v2 四 region 布局(RXS-0326)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SbtRegionsV2 {
    pub handle_size: u64,
    pub raygen_offset: u64,
    pub raygen_stride: u64,
    pub miss_offset: u64,
    pub miss_stride: u64,
    pub miss_size: u64,
    pub hit_offset: u64,
    pub hit_stride: u64,
    pub hit_size: u64,
    pub callable_offset: u64,
    pub callable_stride: u64,
    pub callable_size: u64,
    pub total_size: u64,
}

/// 四 region SBT 布局纯 host 计算(RXS-0326)。
///
/// `miss_count` / `hit_count` ≥ 1;`callable_count` 可为 0(此时 callable region size=0)。
/// region stride = `align_up(handle_size + max_record_bytes, handle_alignment)`。
//@ spec: RXS-0326
pub fn plan_sbt_v2(
    handle_size: u64,
    handle_alignment: u64,
    base_alignment: u64,
    miss_count: u32,
    hit_count: u32,
    callable_count: u32,
    raygen_record_bytes: u64,
    miss_record_bytes: u64,
    hit_record_bytes: u64,
    callable_record_bytes: u64,
) -> Result<SbtRegionsV2, String> {
    if miss_count == 0 || hit_count == 0 {
        return Err("plan_sbt_v2: miss_count/hit_count must be ≥ 1".into());
    }
    let region_stride = |rec: u64| align_up(handle_size + rec, handle_alignment.max(1));
    let rg_stride = align_up(region_stride(raygen_record_bytes), base_alignment.max(1));
    let miss_stride = region_stride(miss_record_bytes);
    let hit_stride = region_stride(hit_record_bytes);
    let call_stride = if callable_count == 0 {
        0
    } else {
        region_stride(callable_record_bytes)
    };

    let raygen_offset = 0u64;
    let raygen_stride = rg_stride; // size == stride
    let miss_offset = raygen_offset + raygen_stride;
    let miss_size = align_up(miss_stride * miss_count as u64, base_alignment.max(1));
    let hit_offset = miss_offset + miss_size;
    let hit_size = align_up(hit_stride * hit_count as u64, base_alignment.max(1));
    let callable_offset = hit_offset + hit_size;
    let callable_size = if callable_count == 0 {
        0
    } else {
        align_up(call_stride * callable_count as u64, base_alignment.max(1))
    };
    let total_size = callable_offset + callable_size;
    Ok(SbtRegionsV2 {
        handle_size,
        raygen_offset,
        raygen_stride,
        miss_offset,
        miss_stride,
        miss_size,
        hit_offset,
        hit_stride,
        hit_size,
        callable_offset,
        callable_stride: call_stride,
        callable_size,
        total_size,
    })
}

/// Record 字段描述(与编译器 layout 律同构:顺序布局;i64/u64 8 对齐、余 4)。
#[derive(Debug, Clone)]
pub struct RecordFieldDesc {
    pub name: String,
    pub ty: RecordFieldTy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordFieldTy {
    U32,
    I32,
    F32,
    U64,
    I64,
}

impl RecordFieldTy {
    fn size(self) -> u64 {
        match self {
            Self::U32 | Self::I32 | Self::F32 => 4,
            Self::U64 | Self::I64 => 8,
        }
    }
    fn align(self) -> u64 {
        self.size()
    }
}

/// Record schema(含 schema_hash;packer 唯一入口核验精确匹配)。
#[derive(Debug, Clone)]
pub struct RecordSchema {
    pub schema_hash: [u8; 32],
    pub fields: Vec<RecordFieldDesc>,
}

/// 字段值(与 [`RecordFieldTy`] 对应)。
#[derive(Debug, Clone, Copy)]
pub enum RecordValue {
    U32(u32),
    I32(i32),
    F32(f32),
    U64(u64),
    I64(i64),
}

/// 唯一合法 shader-record 编码入口(RXS-0326)。
/// `expected_hash` 必须与 schema.schema_hash 逐字节相等,否则 fail-closed。
//@ spec: RXS-0326
pub fn pack_shader_record(
    schema: &RecordSchema,
    expected_hash: &[u8; 32],
    values: &[RecordValue],
) -> Result<Vec<u8>, String> {
    if schema.schema_hash != *expected_hash {
        return Err(format!(
            "pack_shader_record: schema_hash mismatch (fail-closed, RXS-0326)"
        ));
    }
    if values.len() != schema.fields.len() {
        return Err(format!(
            "pack_shader_record: value count {} != field count {}",
            values.len(),
            schema.fields.len()
        ));
    }
    let mut out = Vec::new();
    for (f, v) in schema.fields.iter().zip(values.iter()) {
        let align = f.ty.align();
        let pad = (align - (out.len() as u64 % align)) % align;
        out.extend(std::iter::repeat_n(0u8, pad as usize));
        match (f.ty, v) {
            (RecordFieldTy::U32, RecordValue::U32(x)) => out.extend_from_slice(&x.to_le_bytes()),
            (RecordFieldTy::I32, RecordValue::I32(x)) => out.extend_from_slice(&x.to_le_bytes()),
            (RecordFieldTy::F32, RecordValue::F32(x)) => {
                out.extend_from_slice(&x.to_bits().to_le_bytes())
            }
            (RecordFieldTy::U64, RecordValue::U64(x)) => out.extend_from_slice(&x.to_le_bytes()),
            (RecordFieldTy::I64, RecordValue::I64(x)) => out.extend_from_slice(&x.to_le_bytes()),
            _ => {
                return Err(format!(
                    "pack_shader_record: type/value mismatch on field `{}`",
                    f.name
                ));
            }
        }
    }
    Ok(out)
}

/// stack 查询结果(逐组逐类;evidence 记录)。
#[derive(Debug, Clone, Default)]
pub struct RtStackQuery {
    pub raygen: u32,
    pub chit_max: u32,
    pub miss_max: u32,
    pub intersection_max: u32,
    pub anyhit_max: u32,
    pub callable_max: u32,
}

/// 保守公式:`raygen + max(chit, miss, intersection+anyhit) × 1 + callable`
/// (递归 1、callable 深度 1、禁嵌套;RXS-0327)。
//@ spec: RXS-0327
pub fn compute_rt_stack_size(q: &RtStackQuery) -> u32 {
    let hit_or_miss = q
        .chit_max
        .max(q.miss_max)
        .max(q.intersection_max.saturating_add(q.anyhit_max));
    q.raygen
        .saturating_add(hit_or_miss)
        .saturating_add(q.callable_max)
}

/// Hit group SPIR-V 描述。
#[derive(Debug, Clone)]
pub struct RtHitGroupSpv<'a> {
    pub kind: RtHitGroupKind,
    pub closest_hit: &'a [u32],
    pub any_hit: Option<&'a [u32]>,
    pub intersection: Option<&'a [u32]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtHitGroupKind {
    Triangles,
    Procedural,
}

/// 增量 RT 场景几何。
#[derive(Debug, Clone)]
pub struct RtIncrementalScene<'a> {
    /// 三角形 BLAS 顶点(每 BLAS = 9*N f32)。
    pub triangle_blases: &'a [&'a [f32]],
    /// AABB BLAS(每 AABB = 6 f32 = minxyz/maxxyz);可空。
    pub aabb_blases: &'a [&'a [f32]],
    /// 实例:(blas_kind, blas_index, sbt_record_offset, transform?)
    /// blas_kind: 0=triangle, 1=aabb。
    pub instances: &'a [RtIncrementalInstance],
}

#[derive(Debug, Clone, Copy)]
pub struct RtIncrementalInstance {
    pub is_aabb: bool,
    pub blas_index: u32,
    pub sbt_record_offset: u32,
    pub transform: [f32; 12],
}

/// SBT record bytes(按 region;紧随 handle 之后由 runtime 铺设)。
#[derive(Debug, Clone)]
pub struct RtSbtRecords<'a> {
    pub raygen: &'a [u8],
    pub miss: &'a [&'a [u8]],
    pub hit: &'a [&'a [u8]],
    pub callable: &'a [&'a [u8]],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtPipelineMode {
    Monolithic,
    LibraryLink,
}

/// `run_rt_pipeline_offscreen` 描述符(RXS-0327)。
#[derive(Debug, Clone)]
pub struct RtPipelineDesc<'a> {
    pub raygen: &'a [u32],
    pub miss: &'a [&'a [u32]],
    pub hit_groups: &'a [RtHitGroupSpv<'a>],
    pub callables: &'a [&'a [u32]],
    pub records: RtSbtRecords<'a>,
    pub scene: RtIncrementalScene<'a>,
    pub stack_override: Option<u32>,
    pub width: u32,
    pub height: u32,
    pub mode: RtPipelineMode,
    /// 期望 hit-group 数(反代绿:`rxs0248_minimal_witness_not_sufficient`)。
    pub min_hit_groups: u32,
}

/// device 跑结果(像素 + stack evidence + readback)。
#[derive(Debug, Clone)]
pub struct RtPipelineRunResult {
    pub pixels_rgba8: Vec<u8>,
    pub hit_id_rgba8: Vec<u8>,
    pub record_readback: Vec<u8>,
    pub stack_required: u32,
    pub stack_configured: u32,
    pub stack_formula_version: String,
    pub stack_query: RtStackQuery,
    pub validation_errors: u32,
    pub hit_group_count: u32,
    pub mode: &'static str,
}

/// 增量 RT offscreen 入口(RXS-0327)。既有 [`vk::run_ray_tracing_offscreen`] 0-byte 保留。
///
/// # SAFETY(U30 扩注)
/// AS/SBT/device-address 细审计邻域与 `run_ray_tracing_offscreen` 同界;本入口加性
/// 扩多 hit group / shader-record / stack dynamic state / pipeline library。
//@ spec: RXS-0327
pub fn run_rt_pipeline_offscreen(desc: &RtPipelineDesc<'_>) -> Result<RtPipelineRunResult, String> {
    if (desc.hit_groups.len() as u32) < desc.min_hit_groups.max(2) {
        return Err(format!(
            "run_rt_pipeline_offscreen: hit_groups {} < min {} (RXS-0248 minimal witness not sufficient)",
            desc.hit_groups.len(),
            desc.min_hit_groups.max(2)
        ));
    }
    if desc.miss.is_empty() {
        return Err("run_rt_pipeline_offscreen: miss[] empty".into());
    }
    // 委托 vk 模块内实现(共享 FFI/U30 边界)。
    vk::run_rt_pipeline_offscreen_impl(desc)
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0326
    #[test]
    fn plan_sbt_v2_multi_hit_callable() {
        let s = plan_sbt_v2(32, 32, 64, 2, 2, 1, 0, 16, 16, 8).unwrap();
        assert_eq!(s.raygen_offset, 0);
        assert!(s.miss_size >= 32 * 2);
        assert!(s.hit_size >= 32 * 2);
        assert!(s.callable_size > 0);
        assert_eq!(s.total_size, s.callable_offset + s.callable_size);
        // region base 对齐
        assert_eq!(s.miss_offset % 64, 0);
        assert_eq!(s.hit_offset % 64, 0);
        assert_eq!(s.callable_offset % 64, 0);
    }

    //@ spec: RXS-0326
    #[test]
    fn packer_hash_mismatch_rejected() {
        let schema = RecordSchema {
            schema_hash: [1u8; 32],
            fields: vec![RecordFieldDesc {
                name: "id".into(),
                ty: RecordFieldTy::U32,
            }],
        };
        let bad = [2u8; 32];
        assert!(pack_shader_record(&schema, &bad, &[RecordValue::U32(7)]).is_err());
    }

    //@ spec: RXS-0326
    #[test]
    fn packer_u32_f32_layout() {
        let schema = RecordSchema {
            schema_hash: [9u8; 32],
            fields: vec![
                RecordFieldDesc {
                    name: "id".into(),
                    ty: RecordFieldTy::U32,
                },
                RecordFieldDesc {
                    name: "r".into(),
                    ty: RecordFieldTy::F32,
                },
            ],
        };
        let bytes = pack_shader_record(
            &schema,
            &[9u8; 32],
            &[RecordValue::U32(3), RecordValue::F32(0.5)],
        )
        .unwrap();
        assert_eq!(&bytes[0..4], &3u32.to_le_bytes());
        assert_eq!(&bytes[4..8], &0.5f32.to_bits().to_le_bytes());
    }

    //@ spec: RXS-0327
    #[test]
    fn stack_formula_basic() {
        let q = RtStackQuery {
            raygen: 10,
            chit_max: 20,
            miss_max: 15,
            intersection_max: 8,
            anyhit_max: 4,
            callable_max: 6,
        };
        // max(20, 15, 8+4)=20; 10+20+6=36
        assert_eq!(compute_rt_stack_size(&q), 36);
        assert_eq!(RT_STACK_FORMULA_VERSION, "rurix.rt-stack.v1");
    }
}
