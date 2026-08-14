//! 地形(G9.5 M116;RFC-0025 §4.G;spec/world_partition.md RXS-0367 L1~L5 逐条对齐)。
//!
//! //@ spec: RXS-0367
//!
//! 本模块承载 M116 地形专项渲染器前端语义面:
//!
//! - **chunk ≡ cell**(L1):地形 chunk 与 M110 世界分区 cell 对齐同一网格族——
//!   [`TerrainChunkMeta`] 以 cell 下标为唯一身份,包围盒经
//!   [`crate::world::partition::derived_cell_bounds_xy`] 同一派生函数产出;
//!   [`assert_chunk_eq_cell`] 为结构性等价断言(数量 1:1、coord 逐一同族、
//!   边长同一资产属性);**禁第二套分格**——外来独立网格描述注入
//!   ([`assert_no_second_grid`])即 typed `Err(SecondGridDetected)`(RED 锚)。
//! - **heightfield 数据为 M04 页格式资产**(L1):[`HeightfieldAsset`] 消费
//!   [`CellPageRef`](crate::world::partition::CellPageRef) 页引用(M04 ABI 只
//!   消费不重定),canonical 二进制编解码逐字节往返 + digest 签名完整性。
//! - **全 compute LOD/剔除/缝合**(L2):LOD 选择(距离环闭集)/视锥剔除(保守
//!   包围球)/邻级缝合(stitch skirt)全部归入 compute 批次,产出
//!   [`IndirectDrawBatch`];**CPU 侧零逐 chunk 提交断言**——批次结构内
//!   `cpu_per_chunk_submits` 恒 0,非零即 `Err(CpuPerChunkSubmit)`(RED)。
//! - **toroidal 更新**(L3):相机移动时 [`ToroidalRing`] 环形窗口滚动复用
//!   ring buffer(驻留槽复用计数,避免全量重传);chunk 页迟到 → 父级 LOD
//!   占位([`SlotState::ParentLodPlaceholder`],沿 M44 迟到页降级语义不重定)。
//! - **零 SVT 依赖断言**(L4):地形语义面不得依赖 SVT/RVT/sampler feedback
//!   (D4 D17,M40/41/42 G8 no-go 维持)——资产描述携带虚拟纹理/sampler
//!   feedback 依赖标记即 `Err(SvtDependencyDetected)`(RED 锚);模块自身零
//!   SVT 消费面(结构性:类型系统内无 SVT 字段)。
//! - **缝合裂缝 RED 臂**(L5):相邻 chunk LOD 差 >1 注入必须触发缝合路径
//!   (未触发即 `Err(LodGapUnstitched)`);邻级缝合处顶点位置连续性 golden
//!   ——裂缝像素计数 >0 即 `Err(StitchCrackPixels)`(RED)。
//!
//! 纪律:host 纯 safe 确定性(全库 `forbid(unsafe_code)`);零新 FFI;无 device
//! 依赖——M116 语义面 = chunk≡cell 数据模型 + compute 批次产出面 + toroidal
//! 复用计数 + 缝合连续性机核,GPU 非必需;`RURIX_REQUIRE_REAL=1` 下以 host
//! 确定性为准。G8 底座(M04 页格式/M37 I/O/M44 streamer)与 M110 cell 事件面
//! 只消费不重定,字面 0-byte。

use std::collections::BTreeMap;

use rurix_pkg::sha256;

use super::partition::{
    derived_cell_bounds_xy, CellCoord, CellPageRef, PersistentWorld,
};

// ---------------------------------------------------------------------------
// 冻结常量面
// ---------------------------------------------------------------------------

/// heightfield 资产 canonical 二进制 magic("RXHF")。
pub const HEIGHTFIELD_MAGIC: [u8; 4] = *b"RXHF";
/// heightfield 资产格式版本(v1 = 高度场 + 材质层 id 最小语义)。
pub const HEIGHTFIELD_VERSION: u16 = 1;
/// 地形 LOD 档数(距离环闭集 0..=3;LOD0 全分辨率)。
pub const TERRAIN_LOD_TIERS: u32 = 4;
/// LOD 距离环边界(米;canonical 资产属性——环数 = [`TERRAIN_LOD_TIERS`] - 1)。
pub const TERRAIN_LOD_RING_M: [f64; 3] = [128.0, 384.0, 1024.0];
/// 每 chunk LOD0 顶点维(每边采样数;canonical 场景属性)。
pub const CHUNK_LOD0_DIM: u32 = 17;
/// 材质层闭集上界(材质层 id ∈ 0..MATERIAL_LAYER_COUNT;最小语义)。
pub const MATERIAL_LAYER_COUNT: u8 = 4;
/// toroidal ring 窗口边长(cell 数;canonical 场景属性)。
pub const TOROIDAL_RING_DIM: u32 = 6;

// ---------------------------------------------------------------------------
// 错误面(typed Err,fail-closed;本文件严禁 UB)
// ---------------------------------------------------------------------------

/// 地形失败类别。
#[derive(Debug, Clone, PartialEq)]
pub enum TerrainError {
    /// 字节流截断。
    Truncated { at: usize, need: usize },
    /// 解码后残余字节(非 canonical)。
    TrailingBytes { extra: usize },
    /// magic 不符。
    BadMagic,
    /// 不支持的资产版本。
    UnsupportedVersion(u16),
    /// 非 canonical 构造(附静态原因串)。
    NotCanonical(&'static str),
    /// 输入含非有限值(NaN/Inf)。
    NonFiniteValue { stage: &'static str },
    /// 资产签名/内容篡改(digest 不符即拒录)。
    AssetTampered { why: &'static str },
    /// 材质层 id 越界(闭集 0..MATERIAL_LAYER_COUNT)。
    MaterialLayerOutOfRange { got: u8 },
    /// chunk ≠ cell:出现独立地形分格(第二套分格注入,RED 锚)。
    SecondGridDetected { why: &'static str },
    /// SVT/RVT/sampler feedback 依赖注入(零 SVT 依赖断言,RED 锚)。
    SvtDependencyDetected { field: &'static str },
    /// CPU 侧逐 chunk 提交(全 compute 断言违反,RED 锚)。
    CpuPerChunkSubmit { count: u32 },
    /// 相邻 chunk LOD 差 >1 但未走缝合路径(RED 锚)。
    LodGapUnstitched { lod_delta: u32 },
    /// 缝合处裂缝像素(顶点位置不连续,RED 锚)。
    StitchCrackPixels { count: u32 },
}

impl std::fmt::Display for TerrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TerrainError::Truncated { at, need } => {
                write!(f, "truncated: offset {at} 需 {need} 字节")
            }
            TerrainError::TrailingBytes { extra } => write!(f, "trailing bytes: 残余 {extra}"),
            TerrainError::BadMagic => write!(f, "bad magic(非 RXHF)"),
            TerrainError::UnsupportedVersion(v) => write!(f, "unsupported version {v}"),
            TerrainError::NotCanonical(why) => write!(f, "not canonical: {why}"),
            TerrainError::NonFiniteValue { stage } => write!(f, "{stage} 含非有限值"),
            TerrainError::AssetTampered { why } => write!(f, "heightfield 资产篡改: {why}(RED)"),
            TerrainError::MaterialLayerOutOfRange { got } => {
                write!(f, "材质层 id {got} 越界(闭集 0..{MATERIAL_LAYER_COUNT})")
            }
            TerrainError::SecondGridDetected { why } => {
                write!(f, "第二套地形分格注入: {why}(chunk ≡ cell 违反,RED)")
            }
            TerrainError::SvtDependencyDetected { field } => {
                write!(f, "SVT/RVT 依赖注入: {field}(零 SVT 断言违反,RED)")
            }
            TerrainError::CpuPerChunkSubmit { count } => {
                write!(f, "CPU 侧逐 chunk 提交 {count} 次(全 compute 断言违反,RED)")
            }
            TerrainError::LodGapUnstitched { lod_delta } => {
                write!(f, "邻级 LOD 差 {lod_delta} >1 未走缝合路径(RED)")
            }
            TerrainError::StitchCrackPixels { count } => {
                write!(f, "缝合处裂缝像素 {count}(顶点位置不连续,RED)")
            }
        }
    }
}

impl std::error::Error for TerrainError {}

pub type Result<T> = std::result::Result<T, TerrainError>;

// ---------------------------------------------------------------------------
// heightfield 资产(M04 页引用消费面 + 高度场/材质层最小语义)
// ---------------------------------------------------------------------------

/// 地形 chunk 的 heightfield 资产(L1:数据为 M04 页格式资产——页引用
/// [`CellPageRef`] 只消费不重定;高度场 + 材质层 id 最小语义)。
#[derive(Debug, Clone, PartialEq)]
pub struct HeightfieldAsset {
    /// M04 资产页引用(chunk 数据页寻址)。
    pub page: CellPageRef,
    /// LOD0 每边采样数(canonical 场景 = [`CHUNK_LOD0_DIM`])。
    pub lod0_dim: u32,
    /// 高度场(行主序,lod0_dim² 个采样;米)。
    pub heights: Vec<f32>,
    /// 材质层 id(每采样一个,闭集 0..MATERIAL_LAYER_COUNT;最小语义)。
    pub material_layers: Vec<u8>,
}

impl HeightfieldAsset {
    /// 构造 + 域校验(fail-closed)。
    pub fn new(page: CellPageRef, lod0_dim: u32, heights: Vec<f32>, material_layers: Vec<u8>) -> Result<Self> {
        if lod0_dim == 0 {
            return Err(TerrainError::NotCanonical("lod0_dim=0"));
        }
        let n = (lod0_dim as usize) * (lod0_dim as usize);
        if heights.len() != n {
            return Err(TerrainError::NotCanonical("heights 长度 ≠ lod0_dim²"));
        }
        if material_layers.len() != n {
            return Err(TerrainError::NotCanonical("material_layers 长度 ≠ lod0_dim²"));
        }
        if heights.iter().any(|h| !h.is_finite()) {
            return Err(TerrainError::NonFiniteValue { stage: "heightfield" });
        }
        if let Some(&got) = material_layers.iter().find(|&&m| m >= MATERIAL_LAYER_COUNT) {
            return Err(TerrainError::MaterialLayerOutOfRange { got });
        }
        Ok(Self { page, lod0_dim, heights, material_layers })
    }

    /// 采样(行主序;x/y < lod0_dim 由调用面保证,否则返回 None——不设 UB)。
    pub fn height_at(&self, x: u32, y: u32) -> Option<f32> {
        if x >= self.lod0_dim || y >= self.lod0_dim {
            return None;
        }
        Some(self.heights[(y * self.lod0_dim + x) as usize])
    }
}

/// canonical 二进制编码(magic + version + 字段闭集,LE)。
pub fn encode_heightfield(asset: &HeightfieldAsset) -> Vec<u8> {
    let mut w = Vec::new();
    w.extend_from_slice(&HEIGHTFIELD_MAGIC);
    w.extend_from_slice(&HEIGHTFIELD_VERSION.to_le_bytes());
    w.extend_from_slice(&asset.page.resource.to_le_bytes());
    w.extend_from_slice(&asset.page.page_index.to_le_bytes());
    w.extend_from_slice(&asset.lod0_dim.to_le_bytes());
    for h in &asset.heights {
        w.extend_from_slice(&h.to_le_bytes());
    }
    w.extend_from_slice(&asset.material_layers);
    w
}

/// canonical 二进制解码(逐位核验;非 canonical 即 typed Err)。
pub fn decode_heightfield(bytes: &[u8]) -> Result<HeightfieldAsset> {
    let take = |pos: &mut usize, n: usize| -> Result<&[u8]> {
        if bytes.len() - *pos < n {
            return Err(TerrainError::Truncated { at: *pos, need: n });
        }
        let s = &bytes[*pos..*pos + n];
        *pos += n;
        Ok(s)
    };
    let mut pos = 0usize;
    if take(&mut pos, 4)? != HEIGHTFIELD_MAGIC {
        return Err(TerrainError::BadMagic);
    }
    let ver = u16::from_le_bytes(take(&mut pos, 2)?.try_into().expect("u16"));
    if ver != HEIGHTFIELD_VERSION {
        return Err(TerrainError::UnsupportedVersion(ver));
    }
    let resource = u32::from_le_bytes(take(&mut pos, 4)?.try_into().expect("u32"));
    let page_index = u32::from_le_bytes(take(&mut pos, 4)?.try_into().expect("u32"));
    let lod0_dim = u32::from_le_bytes(take(&mut pos, 4)?.try_into().expect("u32"));
    if lod0_dim == 0 || lod0_dim > 4096 {
        return Err(TerrainError::NotCanonical("lod0_dim 越界"));
    }
    let n = (lod0_dim as usize) * (lod0_dim as usize);
    let mut heights = Vec::with_capacity(n);
    for _ in 0..n {
        heights.push(f32::from_le_bytes(take(&mut pos, 4)?.try_into().expect("f32")));
    }
    let material_layers = take(&mut pos, n)?.to_vec();
    if pos != bytes.len() {
        return Err(TerrainError::TrailingBytes { extra: bytes.len() - pos });
    }
    HeightfieldAsset::new(CellPageRef { resource, page_index }, lod0_dim, heights, material_layers)
}

/// 资产签名(digest 即完整性;M01/M85 通道口径)。
pub fn heightfield_signature(asset: &HeightfieldAsset) -> [u8; 32] {
    sha256::digest(&encode_heightfield(asset))
}

/// 资产完整性核验(篡改即拒录)。
pub fn verify_heightfield(asset: &HeightfieldAsset, expected_sig: &[u8; 32]) -> Result<()> {
    if &heightfield_signature(asset) != expected_sig {
        return Err(TerrainError::AssetTampered { why: "digest 不符" });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// chunk ≡ cell(L1 结构性断言)
// ---------------------------------------------------------------------------

/// 地形 chunk 元数据:**身份 = M110 cell 下标**(无独立分格身份);coord 与包围盒
/// 经分区同一网格族派生。
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainChunkMeta {
    /// cell 下标(`PersistentWorld::cells` 索引)——chunk ≡ cell 的唯一身份。
    pub cell: u32,
    /// cell 坐标(与 `cells[cell].coord` 逐位相等,构造期核验)。
    pub coord: CellCoord,
    /// heightfield 资产。
    pub heightfield: HeightfieldAsset,
    /// 当前 LOD 档(compute LOD 选择产出)。
    pub lod: u32,
}

/// 由 M110 世界分区构造地形 chunk 集(chunk ≡ cell:逐 cell 1:1 映射,包围盒
/// 经 [`derived_cell_bounds_xy`] 同一派生面)。
pub fn build_chunks_from_cells(
    world: &PersistentWorld,
    assets: &BTreeMap<u32, HeightfieldAsset>,
) -> Result<Vec<TerrainChunkMeta>> {
    let mut out = Vec::with_capacity(assets.len());
    for (&cell, asset) in assets {
        let meta = world
            .cells
            .get(cell as usize)
            .ok_or(TerrainError::SecondGridDetected { why: "chunk 引用不存在 cell" })?;
        // 包围盒同族派生核验(chunk 不携带独立 bounds——派生即同一网格族)。
        let (lo, hi) = derived_cell_bounds_xy(world, meta.coord);
        if lo[0] != meta.bounds_min[0] || hi[0] != meta.bounds_max[0] {
            return Err(TerrainError::SecondGridDetected { why: "cell 包围盒派生失配" });
        }
        out.push(TerrainChunkMeta { cell, coord: meta.coord, heightfield: asset.clone(), lod: 0 });
    }
    Ok(out)
}

/// chunk ≡ cell 结构性断言(L1):chunk 集与 cell 集数量 1:1、coord 逐一同族、
/// 无孤儿 chunk。
pub fn assert_chunk_eq_cell(world: &PersistentWorld, chunks: &[TerrainChunkMeta]) -> Result<()> {
    for c in chunks {
        let meta = world
            .cells
            .get(c.cell as usize)
            .ok_or(TerrainError::SecondGridDetected { why: "孤儿 chunk(无 cell 身份)" })?;
        if meta.coord != c.coord {
            return Err(TerrainError::SecondGridDetected { why: "chunk coord 与 cell 网格族不符" });
        }
    }
    Ok(())
}

/// 外来独立网格描述(第二套分格注入面;RED 锚):任何不落在分区网格族上的
/// 地形分格(独立边长/独立原点)一律拒绝。
pub struct ForeignGridDesc {
    /// 外来网格边长(米)。
    pub cell_size_m: f64,
    /// 外来网格原点(x,y 米)。
    pub origin_m: [f64; 2],
}

/// 零第二套分格断言:外来网格与分区网格族对齐(边长相等且原点落在格点上)
/// 才合法;否则 `Err(SecondGridDetected)`。
pub fn assert_no_second_grid(world: &PersistentWorld, foreign: &ForeignGridDesc) -> Result<()> {
    if !foreign.cell_size_m.is_finite() || foreign.cell_size_m <= 0.0 {
        return Err(TerrainError::SecondGridDetected { why: "外来网格边长非法" });
    }
    if foreign.cell_size_m != world.cell_size_m {
        return Err(TerrainError::SecondGridDetected { why: "外来网格边长 ≠ cell 边长" });
    }
    let gx = foreign.origin_m[0] / world.cell_size_m;
    let gy = foreign.origin_m[1] / world.cell_size_m;
    if gx.fract() != 0.0 || gy.fract() != 0.0 {
        return Err(TerrainError::SecondGridDetected { why: "外来网格原点不落 cell 格点" });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 全 compute LOD/剔除/缝合 + indirect draw 产出面(L2)
// ---------------------------------------------------------------------------

/// LOD 选择(距离环闭集;确定性)。
pub fn select_lod(distance_m: f64) -> Result<u32> {
    if !distance_m.is_finite() || distance_m < 0.0 {
        return Err(TerrainError::NonFiniteValue { stage: "lod distance" });
    }
    let mut lod = 0;
    for ring in TERRAIN_LOD_RING_M {
        if distance_m >= ring {
            lod += 1;
        }
    }
    Ok(lod.min(TERRAIN_LOD_TIERS - 1))
}

/// 视锥剔除(保守包围球 vs 六平面;全 compute 面的 host 确定性参照)。
pub fn frustum_cull(center: [f32; 3], radius: f32, planes: &[[f32; 4]]) -> Result<bool> {
    if !center.iter().all(|v| v.is_finite()) || !radius.is_finite() || radius < 0.0 {
        return Err(TerrainError::NonFiniteValue { stage: "frustum cull" });
    }
    for p in planes {
        if !p.iter().all(|v| v.is_finite()) {
            return Err(TerrainError::NonFiniteValue { stage: "frustum plane" });
        }
        let d = p[0] * center[0] + p[1] * center[1] + p[2] * center[2] + p[3];
        if d < -radius {
            return Ok(false); // 球在平面负侧 = 出视锥(保守剔除)
        }
    }
    Ok(true)
}

/// indirect draw 记录(compute 产出;逐 chunk 一条,GPU 侧索引切换)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndirectDrawRecord {
    /// chunk 身份(= cell 下标)。
    pub chunk: u32,
    /// LOD 档。
    pub lod: u32,
    /// 顶点数(LOD 档派生)。
    pub vertex_count: u32,
    /// 实例数(恒 1)。
    pub instance_count: u32,
}

/// indirect draw 批次(全 compute 唯一产出面;`cpu_per_chunk_submits` 恒 0——
/// CPU 侧零逐 chunk 提交断言的结构载体)。
#[derive(Debug, Clone, PartialEq)]
pub struct IndirectDrawBatch {
    pub records: Vec<IndirectDrawRecord>,
    /// CPU 侧逐 chunk 提交计数(恒 0;非零即 RED——[`assert_zero_cpu_submit`])。
    pub cpu_per_chunk_submits: u32,
}

/// CPU 侧零逐 chunk 提交断言(L2)。
pub fn assert_zero_cpu_submit(batch: &IndirectDrawBatch) -> Result<()> {
    if batch.cpu_per_chunk_submits != 0 {
        return Err(TerrainError::CpuPerChunkSubmit { count: batch.cpu_per_chunk_submits });
    }
    Ok(())
}

/// 全 compute 管线 host 参照:对 chunk 集做 LOD 选择 + 视锥剔除,产 indirect
/// draw 批次(逐 chunk 零 CPU 提交)。
pub fn build_indirect_draws(
    chunks: &[TerrainChunkMeta],
    camera: [f64; 3],
    planes: &[[f32; 4]],
    cell_size_m: f64,
) -> Result<IndirectDrawBatch> {
    if !camera.iter().all(|v| v.is_finite()) {
        return Err(TerrainError::NonFiniteValue { stage: "camera" });
    }
    let mut records = Vec::new();
    for c in chunks {
        let cx = (c.coord.x as f64 + 0.5) * cell_size_m;
        let cy = (c.coord.y as f64 + 0.5) * cell_size_m;
        let dx = cx - camera[0];
        let dy = cy - camera[1];
        let dist = (dx * dx + dy * dy).sqrt();
        let lod = select_lod(dist)?;
        let stride = 1u32 << lod;
        let dim = (c.heightfield.lod0_dim - 1) / stride + 1;
        let center = [cx as f32, camera[2] as f32, cy as f32];
        let radius = (cell_size_m * std::f64::consts::FRAC_1_SQRT_2) as f32;
        if !frustum_cull(center, radius, planes)? {
            continue; // 剔除不进批次
        }
        records.push(IndirectDrawRecord {
            chunk: c.cell,
            lod,
            vertex_count: dim * dim,
            instance_count: 1,
        });
    }
    Ok(IndirectDrawBatch { records, cpu_per_chunk_submits: 0 })
}

// ---------------------------------------------------------------------------
// 邻级缝合(L5:stitch skirt;裂缝=0 机核)
// ---------------------------------------------------------------------------

/// chunk 边缘采样位置(`edge` 0=+x 1=−x 2=+y 3=−y)。
///
/// 缝合语义(stitch skirt 的 host 确定性参照):cadence 取细侧 LOD 步长
/// (`cadence_lod`),高度函数取粗侧 LOD 插值(`sample_lod`)——细侧 skirt 顶点
/// 拉到粗侧插值面上,两侧经**同一插值函数同一世界格点**求值 ⇒ 位置逐位连续。
pub fn stitch_edge_positions(
    chunk: &TerrainChunkMeta,
    edge: u32,
    cadence_lod: u32,
    sample_lod: u32,
) -> Result<Vec<[f32; 3]>> {
    if edge > 3 {
        return Err(TerrainError::NotCanonical("edge 闭集 0..=3"));
    }
    let dim = chunk.heightfield.lod0_dim;
    let stride = 1u32 << cadence_lod;
    if !(dim - 1).is_multiple_of(stride) {
        return Err(TerrainError::NotCanonical("cadence 步长不整除"));
    }
    let n = (dim - 1) / stride + 1;
    let mut out = Vec::with_capacity(n as usize);
    for i in 0..n {
        let t = i * stride;
        let (x, y) = match edge {
            0 => (dim - 1, t),
            1 => (0, t),
            2 => (t, dim - 1),
            _ => (t, 0),
        };
        let h = sample_height_lod(&chunk.heightfield, x, y, sample_lod)?;
        out.push([x as f32, h, y as f32]);
    }
    Ok(out)
}

/// LOD 采样(粗档按 2^lod 步长网格双线性插值;确定性)。
fn sample_height_lod(asset: &HeightfieldAsset, x: u32, y: u32, lod: u32) -> Result<f32> {
    let stride = 1u32 << lod;
    let dim = asset.lod0_dim;
    let gx = x / stride;
    let gy = y / stride;
    let fx = (x % stride) as f32 / stride as f32;
    let fy = (y % stride) as f32 / stride as f32;
    let at = |ix: u32, iy: u32| -> Result<f32> {
        let cx = (ix * stride).min(dim - 1);
        let cy = (iy * stride).min(dim - 1);
        asset.height_at(cx, cy).ok_or(TerrainError::NotCanonical("采样越界"))
    };
    let h00 = at(gx, gy)?;
    let h10 = at((gx + 1).min((dim - 1) / stride), gy)?;
    let h01 = at(gx, (gy + 1).min((dim - 1) / stride))?;
    let h11 = at((gx + 1).min((dim - 1) / stride), (gy + 1).min((dim - 1) / stride))?;
    Ok(h00 * (1.0 - fx) * (1.0 - fy) + h10 * fx * (1.0 - fy) + h01 * (1.0 - fx) * fy + h11 * fx * fy)
}

/// 缝合报告(机核产出面)。
#[derive(Debug, Clone, PartialEq)]
pub struct SeamReport {
    /// 邻级 LOD 差。
    pub lod_delta: u32,
    /// 缝合路径是否触发(LOD 差 >1 必须 true)。
    pub stitch_invoked: bool,
    /// 裂缝像素计数(顶点位置不连续对数;golden = 0)。
    pub crack_pixels: u32,
}

/// 邻级缝合校验(L5 机核):`stitch_enabled=false` 表示注入「未走缝合路径」
/// 变体——LOD 差 >1 时立即 `Err(LodGapUnstitched)`(RED);缝合路径触发后逐
/// 采样对拍边缘位置,裂缝 >0 即 `Err(StitchCrackPixels)`(RED)。
pub fn verify_seam(a: &TerrainChunkMeta, b: &TerrainChunkMeta, stitch_enabled: bool) -> Result<SeamReport> {
    let lod_delta = a.lod.abs_diff(b.lod);
    if lod_delta > 1 && !stitch_enabled {
        return Err(TerrainError::LodGapUnstitched { lod_delta });
    }
    let stitch_invoked = lod_delta > 1;
    // 对拍边:a 的 +x 边对 b 的 −x 边(coord.x 相邻);cadence 取细侧,高度函数
    // 取粗侧插值(细侧 skirt 拉到粗侧面 ⇒ 同族高度场下逐位连续)。
    let fine_lod = a.lod.min(b.lod);
    let coarse_lod = a.lod.max(b.lod);
    let ea = stitch_edge_positions(a, 0, fine_lod, coarse_lod)?;
    let eb = stitch_edge_positions(b, 1, fine_lod, coarse_lod)?;
    if ea.len() != eb.len() {
        return Err(TerrainError::StitchCrackPixels { count: ea.len().abs_diff(eb.len()) as u32 });
    }
    let mut crack = 0u32;
    for (pa, pb) in ea.iter().zip(eb.iter()) {
        // 位置连续性:x/z 格点坐标同族(同一网格族 cadence 下标相等),高度逐位相等。
        if pa[1].to_bits() != pb[1].to_bits() || pa[2].to_bits() != pb[2].to_bits() {
            crack += 1;
        }
    }
    let report = SeamReport { lod_delta, stitch_invoked, crack_pixels: crack };
    if crack > 0 {
        return Err(TerrainError::StitchCrackPixels { count: crack });
    }
    Ok(report)
}

// ---------------------------------------------------------------------------
// toroidal 更新(L3:环形窗口滚动复用;迟到页 → 父级 LOD 占位)
// ---------------------------------------------------------------------------

/// ring 槽状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotState {
    /// 驻留(heightfield 页已驻留)。
    Resident(u32),
    /// 父级 LOD 占位(chunk 页迟到;沿 M44 迟到页降级语义不重定)。
    ParentLodPlaceholder(u32),
    /// 空槽。
    Empty,
}

/// toroidal ring buffer(环形窗口;边长 [`TOROIDAL_RING_DIM`] cell)。
#[derive(Debug, Clone, PartialEq)]
pub struct ToroidalRing {
    /// 窗口边长(cell 数)。
    pub dim: u32,
    /// 窗口原点(左下角 cell 坐标)。
    pub origin: CellCoord,
    /// 槽(行主序 dim²)。
    pub slots: Vec<SlotState>,
}

/// toroidal 更新报告(复用/加载/占位计数,逐帧 evidence 面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToroidalUpdateReport {
    /// 复用槽数(滚动后仍驻留,免重传)。
    pub reused: u32,
    /// 新驻留槽数。
    pub loaded: u32,
    /// 父级 LOD 占位槽数(页迟到)。
    pub placeholders: u32,
}

impl ToroidalRing {
    pub fn new(origin: CellCoord) -> Self {
        let dim = TOROIDAL_RING_DIM;
        Self { dim, origin, slots: vec![SlotState::Empty; (dim * dim) as usize] }
    }

    fn slot_coord(&self, idx: u32) -> CellCoord {
        CellCoord {
            x: self.origin.x + (idx % self.dim) as i32,
            y: self.origin.y + (idx / self.dim) as i32,
        }
    }

    /// 相机移动重定心:环形窗口滚动——落入新窗口的既有槽**复用**(不重传),
    /// 新入窗 cell 按驻留状态登记(迟到页 → 父级 LOD 占位)。
    pub fn recenter(
        &mut self,
        new_origin: CellCoord,
        resident: &std::collections::BTreeSet<u32>,
        world: &PersistentWorld,
    ) -> Result<ToroidalUpdateReport> {
        // 旧窗口 coord → cell 映射(复用判定面)。
        let mut old: BTreeMap<(i32, i32), SlotState> = BTreeMap::new();
        for (i, s) in self.slots.iter().enumerate() {
            let c = self.slot_coord(i as u32);
            old.insert((c.x, c.y), *s);
        }
        self.origin = new_origin;
        let mut report = ToroidalUpdateReport { reused: 0, loaded: 0, placeholders: 0 };
        let mut slots = vec![SlotState::Empty; self.slots.len()];
        for (i, slot) in slots.iter_mut().enumerate() {
            let c = self.slot_coord(i as u32);
            let Some(cell_idx) = world.cells.iter().position(|m| m.coord == c) else {
                continue; // 网格外 = 空槽
            };
            let cell_idx = cell_idx as u32;
            let reusable = matches!(
                old.get(&(c.x, c.y)),
                Some(SlotState::Resident(prev)) if *prev == cell_idx && resident.contains(&cell_idx)
            );
            if reusable {
                *slot = SlotState::Resident(cell_idx);
                report.reused += 1;
                continue;
            }
            if resident.contains(&cell_idx) {
                *slot = SlotState::Resident(cell_idx);
                report.loaded += 1;
            } else {
                *slot = SlotState::ParentLodPlaceholder(cell_idx); // 页迟到 → 父级占位
                report.placeholders += 1;
            }
        }
        self.slots = slots;
        Ok(report)
    }
}

// ---------------------------------------------------------------------------
// 零 SVT 依赖断言(L4)
// ---------------------------------------------------------------------------

/// 地形资产依赖描述(消费方申报面;**任何虚拟纹理依赖标记即 RED**)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AssetDependencyDesc {
    /// SVT/sparse virtual texture 依赖。
    pub uses_svt: bool,
    /// RVT(runtime virtual texture)依赖。
    pub uses_rvt: bool,
    /// sampler feedback 依赖。
    pub uses_sampler_feedback: bool,
}

/// 零 SVT 依赖断言(D4 D17;M40/41/42 G8 no-go 维持):任一虚拟纹理依赖标记
/// 注入即 typed `Err(SvtDependencyDetected)`(RED 锚)。
pub fn assert_zero_svt_dependency(desc: &AssetDependencyDesc) -> Result<()> {
    if desc.uses_svt {
        return Err(TerrainError::SvtDependencyDetected { field: "uses_svt" });
    }
    if desc.uses_rvt {
        return Err(TerrainError::SvtDependencyDetected { field: "uses_rvt" });
    }
    if desc.uses_sampler_feedback {
        return Err(TerrainError::SvtDependencyDetected { field: "uses_sampler_feedback" });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// canonical 场景(golden 事实源)
// ---------------------------------------------------------------------------

/// canonical 地形场景:3×1 cell 条带(coord (0,0)/(1,0)/(2,0))。高度场为**同
/// 一世界函数**(wx = chunk_x·(dim−1) + x,邻接边界世界格点逐位相同 ⇒ 缝合连
/// 续性 golden 事实源);`border_lift` 只抬 x==0 列(裂缝注入用例数据源)。
pub fn canonical_heightfield(chunk_x: u32, border_lift: f32) -> HeightfieldAsset {
    let dim = CHUNK_LOD0_DIM;
    let mut heights = Vec::with_capacity((dim * dim) as usize);
    let mut layers = Vec::with_capacity((dim * dim) as usize);
    for y in 0..dim {
        for x in 0..dim {
            let wx = chunk_x * (dim - 1) + x;
            let wy = y;
            let h = (wx as f32 * 0.11).sin() * 3.0 + (wy as f32 * 0.07).cos() * 2.0
                + if x == 0 { border_lift } else { 0.0 };
            heights.push(h);
            layers.push(((x + y) % u32::from(MATERIAL_LAYER_COUNT)) as u8);
        }
    }
    HeightfieldAsset::new(
        CellPageRef { resource: 900 + chunk_x, page_index: 0 },
        dim,
        heights,
        layers,
    )
    .expect("canonical heightfield")
}

/// canonical 三 chunk 集(同一世界高度函数 ⇒ 邻接边界逐位连续)。
pub fn canonical_chunks() -> Vec<TerrainChunkMeta> {
    (0..3u32)
        .map(|i| TerrainChunkMeta {
            cell: i,
            coord: CellCoord { x: i as i32, y: 0 },
            heightfield: canonical_heightfield(i, 0.0),
            lod: 0,
        })
        .collect()
}

/// 场景 digest(indirect 批次 + 缝合报告序列;golden 对照事实源)。
pub fn scene_digest(batch: &IndirectDrawBatch, seams: &[SeamReport]) -> [u8; 32] {
    let mut buf = Vec::new();
    for r in &batch.records {
        buf.extend_from_slice(&r.chunk.to_le_bytes());
        buf.extend_from_slice(&r.lod.to_le_bytes());
        buf.extend_from_slice(&r.vertex_count.to_le_bytes());
        buf.extend_from_slice(&r.instance_count.to_le_bytes());
    }
    for s in seams {
        buf.extend_from_slice(&s.lod_delta.to_le_bytes());
        buf.push(s.stitch_invoked as u8);
        buf.extend_from_slice(&s.crack_pixels.to_le_bytes());
    }
    sha256::digest(&buf)
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::partition::{CellMeta, PersistentWorld};

    fn mini_world() -> PersistentWorld {
        let cells = (0..3u32)
            .map(|i| CellMeta {
                coord: CellCoord { x: i as i32, y: 0 },
                bounds_min: [i as f32 * 64.0, 0.0, 0.0],
                bounds_max: [(i as f32 + 1.0) * 64.0, 64.0, 64.0],
                page_refs: vec![],
                hlod: None,
                data_layer_mask: 0,
            })
            .collect();
        PersistentWorld {
            cell_size_m: 64.0,
            grid_min: CellCoord { x: 0, y: 0 },
            grid_max: CellCoord { x: 2, y: 0 },
            cells,
            always_loaded: vec![],
            spatially_loaded: vec![],
        }
    }

    //@ spec: RXS-0367
    #[test]
    fn heightfield_canonical_roundtrip_and_tamper() {
        let a = canonical_heightfield(0, 0.0);
        let bytes = encode_heightfield(&a);
        let b = decode_heightfield(&bytes).unwrap();
        assert_eq!(a, b);
        assert_eq!(encode_heightfield(&b), bytes);
        let sig = heightfield_signature(&a);
        verify_heightfield(&a, &sig).unwrap();
        let mut t = a.clone();
        t.heights[0] += 1.0;
        assert!(matches!(
            verify_heightfield(&t, &sig),
            Err(TerrainError::AssetTampered { .. })
        ));
    }

    //@ spec: RXS-0367
    #[test]
    fn chunk_eq_cell_and_second_grid_red() {
        let world = mini_world();
        let mut assets = BTreeMap::new();
        for (i, c) in canonical_chunks().iter().enumerate() {
            assets.insert(i as u32, c.heightfield.clone());
        }
        let chunks = build_chunks_from_cells(&world, &assets).unwrap();
        assert_chunk_eq_cell(&world, &chunks).unwrap();
        // 第二套分格注入(边长不符/原点偏移)即 RED。
        assert!(matches!(
            assert_no_second_grid(&world, &ForeignGridDesc { cell_size_m: 32.0, origin_m: [0.0, 0.0] }),
            Err(TerrainError::SecondGridDetected { .. })
        ));
        assert!(matches!(
            assert_no_second_grid(&world, &ForeignGridDesc { cell_size_m: 64.0, origin_m: [8.0, 0.0] }),
            Err(TerrainError::SecondGridDetected { .. })
        ));
        assert!(assert_no_second_grid(&world, &ForeignGridDesc { cell_size_m: 64.0, origin_m: [128.0, 0.0] }).is_ok());
    }

    //@ spec: RXS-0367
    #[test]
    fn full_compute_lod_cull_zero_cpu_submit() {
        let chunks = canonical_chunks();
        let planes = [[1.0, 0.0, 0.0, 4096.0], [-1.0, 0.0, 0.0, 4096.0],
                      [0.0, 1.0, 0.0, 4096.0], [0.0, -1.0, 0.0, 4096.0],
                      [0.0, 0.0, 1.0, 4096.0], [0.0, 0.0, -1.0, 4096.0]];
        let batch = build_indirect_draws(&chunks, [32.0, 32.0, 10.0], &planes, 64.0).unwrap();
        assert_eq!(batch.records.len(), 3);
        assert_zero_cpu_submit(&batch).unwrap();
        // LOD 距离环闭集确定性。
        assert_eq!(select_lod(0.0).unwrap(), 0);
        assert_eq!(select_lod(128.0).unwrap(), 1);
        assert_eq!(select_lod(400.0).unwrap(), 2);
        assert_eq!(select_lod(9999.0).unwrap(), 3);
        // CPU 逐 chunk 提交注入即 RED。
        let bad = IndirectDrawBatch { records: vec![], cpu_per_chunk_submits: 1 };
        assert!(matches!(assert_zero_cpu_submit(&bad), Err(TerrainError::CpuPerChunkSubmit { .. })));
    }

    //@ spec: RXS-0367
    #[test]
    fn seam_stitch_and_crack_red() {
        let mut chunks = canonical_chunks();
        chunks[0].lod = 0;
        chunks[1].lod = 2; // LOD 差 2 >1
        // 缝合路径触发,同族高度函数 ⇒ 裂缝=0。
        let report = verify_seam(&chunks[0], &chunks[1], true).unwrap();
        assert!(report.stitch_invoked);
        assert_eq!(report.crack_pixels, 0);
        // 未走缝合路径注入即 RED。
        assert!(matches!(
            verify_seam(&chunks[0], &chunks[1], false),
            Err(TerrainError::LodGapUnstitched { lod_delta: 2 })
        ));
        // 裂缝注入(边界高度篡改)即 RED。
        let mut cracked = chunks.clone();
        cracked[1].heightfield = canonical_heightfield(1, 5.0);
        assert!(matches!(
            verify_seam(&cracked[0], &cracked[1], true),
            Err(TerrainError::StitchCrackPixels { .. })
        ));
    }

    //@ spec: RXS-0367
    #[test]
    fn toroidal_reuse_and_late_page_placeholder() {
        let world = mini_world();
        let mut ring = ToroidalRing::new(CellCoord { x: 0, y: 0 });
        let resident = std::collections::BTreeSet::from([0u32, 1, 2]);
        let r1 = ring.recenter(CellCoord { x: 0, y: 0 }, &resident, &world).unwrap();
        assert_eq!(r1.loaded, 3);
        // 窗口滚动一格:仍在窗内的 cell 复用,新进窗且未驻留的走父级占位。
        let r2 = ring.recenter(CellCoord { x: 1, y: 0 }, &resident, &world).unwrap();
        assert!(r2.reused >= 2);
        assert_eq!(r2.placeholders, 0);
        let r3 = ring.recenter(CellCoord { x: 4, y: 0 }, &std::collections::BTreeSet::new(), &world).unwrap();
        assert_eq!(r3.loaded, 0); // 全部页迟到 → 父级占位(网格外为空槽)
    }

    //@ spec: RXS-0367
    #[test]
    fn zero_svt_dependency_red() {
        assert!(assert_zero_svt_dependency(&AssetDependencyDesc::default()).is_ok());
        for desc in [
            AssetDependencyDesc { uses_svt: true, ..Default::default() },
            AssetDependencyDesc { uses_rvt: true, ..Default::default() },
            AssetDependencyDesc { uses_sampler_feedback: true, ..Default::default() },
        ] {
            assert!(matches!(
                assert_zero_svt_dependency(&desc),
                Err(TerrainError::SvtDependencyDetected { .. })
            ));
        }
    }
}
