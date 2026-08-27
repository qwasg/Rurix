//! 世界分区数据模型与流送预算契约(G9.5 M110;RFC-0025 §4.A;spec/world_partition.md
//! RXS-0363 逐条对齐)。
//!
//! //@ spec: RXS-0363
//!
//! 本模块承载 M110 P0 的 host 侧全量语义面:
//!
//! - **单一持久世界 schema**([`PersistentWorld`]):全世界一份持久资产;schema 层
//!   显式区分 `always_loaded`(全局/gameplay 关键对象)与 `spatially_loaded`
//!   (空间分格对象,每个对象携带 cell 归属);cell 为正方形 2D 网格,**边长为资
//!   产属性**(`cell_size_m`,非代码常量);cell 元数据 v1 字段闭集 = cell id、
//!   包围盒、资产页引用(M04 ABI 页寻址只消费不重定)、HLOD 层级引用
//!   (RXS-0364 烘焙工具语义锚);**Data Layer 掩码位只预留不接线**——字段参与
//!   序列化往返,激活语义查询 [`data_layer_active`] 一律 fail-closed typed
//!   `Err`(v2 才实现激活语义,D4 D4)。canonical 二进制 encode→decode→encode
//!   逐字节往返。
//! - **流送运行时**([`PartitionRuntime`]):streaming source(相机/玩家/自定义
//!   探针)携带距离环(loading radius / 内环常驻);每帧由距离环求 target cell
//!   集合,与 resident 集合 diff 出 load/unload 队列。
//! - **三项预算契约**([`PartitionBudget`]):`MaxStreamingCellsPerFrame` /
//!   `MaxActorsToSpawnPerFrame` / `MemoryBudgetMB` 为一等契约字段;超预算请求
//!   **排队而非抢占**(FIFO 保序滚存下帧);预算计数器逐帧落 evidence
//!   ([`FrameBudgetEvidence`],hitch 审计数据源)。**预算违约注入必排队降级**:
//!   任一契约项被压到服务不了当前请求(如 `MaxStreamingCellsPerFrame=0`)时,本
//!   帧 `budget_stall` 置位、排队深度与降级帧计数显式记账;出现静默超帧即 RED
//!   (harness 臂实测)。
//! - **cell 四事件闭集**([`CellEventKind`]):`CellLoadBegin / CellResident /
//!   CellUnloadBegin / CellEvicted` 为渲染器唯一消费面(事件总线只出不反向查
//!   询);固定相机轨迹的事件序列 digest 逐字 golden;[`validate_event_log`] 为
//!   独立序列状态机校验器,乱序注入必拒(RED 臂)。
//!
//! 纪律:host 纯 safe 确定性(全库 `forbid(unsafe_code)`);零新 FFI;无 device
//! 依赖——M110 语义面 = 数据模型 + 调度 + 预算计数 + 事件总线,GPU 非必需
//! (device 侧消费面 = 事件总线订阅,本波不接线;`RURIX_REQUIRE_REAL=1` 下以
//! host 确定性为准,无 SKIP 面)。G8 底座(M04 页格式/M37 I/O/M44 streamer)只
//! 消费不重定,字面 0-byte。

use std::collections::BTreeSet;
use std::fmt;

use rurix_pkg::sha256;

// ---------------------------------------------------------------------------
// 冻结常量面
// ---------------------------------------------------------------------------

/// 持久世界 canonical 二进制 magic("RXWP")。
pub const PARTITION_SCHEMA_MAGIC: [u8; 4] = *b"RXWP";
/// schema 版本(v1 = cell 元数据字段闭集 + Data Layer 掩码位只预留不接线)。
pub const PARTITION_SCHEMA_VERSION: u16 = 1;
/// 对象名最大字节长(canonical 编码 u16 前缀;可打印 ASCII 0x20..=0x7E)。
pub const MAX_NAME_BYTES: usize = 256;
/// soak 声明帧数阈值(G8.8a 口径继承:≥10000 帧;≥30min 墙钟析取归 G9.8a
/// close-out 聚合门,G9_ACCEPTANCE_MAP §6 体例,本 P0 门以帧数析取为准)。
pub const M110_SOAK_MIN_FRAMES: u32 = 10_000;
/// soak 默认帧数(≥ [`M110_SOAK_MIN_FRAMES`])。
pub const M110_SOAK_DEFAULT_FRAMES: u32 = 12_000;
/// soak hitch 统计 warmup 前缀(首帧含 always_loaded 注册与首帧冷路径,不进
/// p99 样本;冻结常量,golden/soak 两腿同一口径)。
pub const M110_SOAK_WARMUP_FRAMES: u32 = 64;
/// hitch p99 阈值余量纪律:阈值 = measured × 1.5(沿 g9_budget.json 既有
/// measured×1.5 冻结先例,禁手写,P-09)。
pub const M110_HITCH_THRESHOLD_MARGIN: f64 = 1.5;

// ---------------------------------------------------------------------------
// 错误面(typed Err,fail-closed;本文件严禁 UB)
// ---------------------------------------------------------------------------

/// 世界分区 schema/运行时失败类别。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartitionError {
    /// 字节流截断。
    Truncated { at: usize, need: usize },
    /// 解码后残余字节(非 canonical)。
    TrailingBytes { extra: usize },
    /// magic 不符。
    BadMagic,
    /// 不支持的 schema 版本。
    UnsupportedVersion(u16),
    /// 名称含非可打印 ASCII 或超长。
    BadName,
    /// 非 canonical 构造(排序/去重/派生字段失配等,附静态原因串)。
    NotCanonical(&'static str),
    /// cell_size 非有限或 ≤0(边长为资产属性但必须合法)。
    BadCellSize,
    /// 网格范围非法(min>max 或 cells 与稠密矩形不符)。
    BadGridExtent,
    /// spatially_loaded 对象引用不存在的 cell。
    UnknownCellRef { object: u64, cell: u32 },
    /// Data Layer 掩码位只预留不接线:激活语义查询一律本错(v2 才接线,D4 D4)。
    DataLayerNotWired,
    /// 帧序号非单调递增(运行时误用)。
    FrameNonMonotonic { prev: u32, got: u32 },
    /// streaming source 距离环非法(非有限/内环>外环/外环≤0)。
    BadSourceRing,
    /// cell 事件序列乱序(状态机期望面不符)。
    EventOutOfOrder {
        cell: u32,
        expected: &'static str,
        got: CellEventKind,
    },
    /// 静默超预算(逐帧一致性机核检出:计数器超契约上限即 RED)。
    SilentBudgetOverrun {
        field: &'static str,
        value: u64,
        cap: u64,
    },
}

impl fmt::Display for PartitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PartitionError::Truncated { at, need } => {
                write!(f, "truncated: offset {at} 需 {need} 字节")
            }
            PartitionError::TrailingBytes { extra } => {
                write!(f, "trailing bytes: 残余 {extra} 字节")
            }
            PartitionError::BadMagic => write!(f, "bad magic(非 RXWP)"),
            PartitionError::UnsupportedVersion(v) => write!(f, "unsupported version {v}"),
            PartitionError::BadName => write!(f, "bad name(非可打印 ASCII 或超长)"),
            PartitionError::NotCanonical(why) => write!(f, "not canonical: {why}"),
            PartitionError::BadCellSize => write!(f, "bad cell_size_m(非有限或 ≤0)"),
            PartitionError::BadGridExtent => write!(f, "bad grid extent"),
            PartitionError::UnknownCellRef { object, cell } => {
                write!(f, "object {object} 引用不存在 cell {cell}")
            }
            PartitionError::DataLayerNotWired => {
                write!(f, "data layer 掩码位只预留不接线(v2 才实现激活语义)")
            }
            PartitionError::FrameNonMonotonic { prev, got } => {
                write!(f, "帧序号非单调: prev={prev} got={got}")
            }
            PartitionError::BadSourceRing => write!(f, "streaming source 距离环非法"),
            PartitionError::EventOutOfOrder {
                cell,
                expected,
                got,
            } => write!(
                f,
                "cell {cell} 事件乱序: 期望 {expected},实到 {}",
                got.as_str()
            ),
            PartitionError::SilentBudgetOverrun { field, value, cap } => {
                write!(f, "静默超预算: {field} = {value} 超契约上限 {cap}")
            }
        }
    }
}

impl std::error::Error for PartitionError {}

pub type Result<T> = std::result::Result<T, PartitionError>;

// ---------------------------------------------------------------------------
// schema 数据模型(字段闭集,canonical 二进制可往返)
// ---------------------------------------------------------------------------

/// 2D cell 网格坐标(正方形网格;边长为资产属性 [`PersistentWorld::cell_size_m`])。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CellCoord {
    pub x: i32,
    pub y: i32,
}

/// 资产页引用(M04 ABI 页寻址:资源 id + 页号;页格式只消费不重定,RXS-0363 L7)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CellPageRef {
    pub resource: u32,
    pub page_index: u32,
}

/// HLOD 层级引用(烘焙工具语义锚 RXS-0364;产物即资产,digest 寻址 + 层数为烘
/// 焙属性)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellHlodRef {
    pub digest: [u8; 32],
    pub levels: u32,
}

/// cell 元数据 v1 字段闭集(RXS-0363 L1 逐字):cell id(= `cells` 下标)、包围
/// 盒、资产页引用、HLOD 层级引用、**Data Layer 掩码位只预留不接线**。
#[derive(Debug, Clone, PartialEq)]
pub struct CellMeta {
    pub coord: CellCoord,
    /// 包围盒(x/y 由 coord 与 cell_size_m 派生,解码期逐位核验;z 为资产属性)。
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub page_refs: Vec<CellPageRef>,
    pub hlod: Option<CellHlodRef>,
    /// Data Layer 掩码位:**只预留不接线**(v1 不实现激活语义;参与序列化往返,
    /// 运行时不消费;激活查询 fail-closed,见 [`data_layer_active`])。
    pub data_layer_mask: u32,
}

/// 世界对象基座(always_loaded 与 spatially_loaded 共享字段)。
#[derive(Debug, Clone, PartialEq)]
pub struct WorldObject {
    pub id: u64,
    pub name: String,
    /// spawn 成本(MaxActorsToSpawnPerFrame 预算消费单位)。
    pub actor_cost: u32,
    /// 驻留内存估计(MemoryBudgetMB 预算消费单位,字节)。
    pub mem_bytes: u64,
}

/// spatially_loaded 对象 = 基座 + cell 归属(RXS-0363 L1「每个 spatially-loaded
/// 对象携带 cell 归属」)。
#[derive(Debug, Clone, PartialEq)]
pub struct SpatialObject {
    pub object: WorldObject,
    /// cell 归属(`PersistentWorld::cells` 下标)。
    pub cell: u32,
}

/// 单一持久世界资产(RXS-0363 L1):全世界一份持久资产;`always_loaded` 与
/// `spatially_loaded` 两数组在 schema 层显式分列。
#[derive(Debug, Clone, PartialEq)]
pub struct PersistentWorld {
    /// cell 边长(米):**资产属性,非代码常量**。
    pub cell_size_m: f64,
    /// 网格闭矩形范围(含端点;cells 必须稠密覆盖该矩形)。
    pub grid_min: CellCoord,
    pub grid_max: CellCoord,
    /// cell 元数据(canonical 序 = (coord.y, coord.x) 升序)。
    pub cells: Vec<CellMeta>,
    /// always_loaded 对象(canonical 序 = id 升序;运行时装配期即激活)。
    pub always_loaded: Vec<WorldObject>,
    /// spatially_loaded 对象(canonical 序 = id 升序;按 cell 流送)。
    pub spatially_loaded: Vec<SpatialObject>,
}

// ---------------------------------------------------------------------------
// canonical 二进制编解码(LE;排序/去重/派生字段核验 = canonical 性)
// ---------------------------------------------------------------------------

struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn f64(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn name(&mut self, s: &str) -> Result<()> {
        if s.len() > MAX_NAME_BYTES || !s.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
            return Err(PartitionError::BadName);
        }
        self.u16(s.len() as u16);
        self.buf.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.buf.len() - self.pos < n {
            return Err(PartitionError::Truncated {
                at: self.pos,
                need: n,
            });
        }
        let s = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes([self.u8()?, self.u8()?]))
    }
    fn u32(&mut self) -> Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn u64(&mut self) -> Result<u64> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }
    fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }
    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }
    fn f64(&mut self) -> Result<f64> {
        Ok(f64::from_bits(self.u64()?))
    }
    fn name(&mut self) -> Result<String> {
        let n = self.u16()? as usize;
        let b = self.take(n)?;
        let s = std::str::from_utf8(b).map_err(|_| PartitionError::BadName)?;
        if !s.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
            return Err(PartitionError::BadName);
        }
        Ok(s.to_string())
    }
    fn finish(self) -> Result<()> {
        if self.pos != self.buf.len() {
            return Err(PartitionError::TrailingBytes {
                extra: self.buf.len() - self.pos,
            });
        }
        Ok(())
    }
}

/// cell 包围盒 x/y 派生值(正方形 2D 网格;`f64` 计算后单次圆整到 `f32`,
/// 确定性逐位可比)。
pub fn derived_cell_bounds_xy(world: &PersistentWorld, coord: CellCoord) -> ([f32; 2], [f32; 2]) {
    let lo = [
        (coord.x as f64 * world.cell_size_m) as f32,
        (coord.y as f64 * world.cell_size_m) as f32,
    ];
    let hi = [
        ((coord.x as f64 + 1.0) * world.cell_size_m) as f32,
        ((coord.y as f64 + 1.0) * world.cell_size_m) as f32,
    ];
    (lo, hi)
}

fn write_object(w: &mut Writer, o: &WorldObject) -> Result<()> {
    w.u64(o.id);
    w.name(&o.name)?;
    w.u32(o.actor_cost);
    w.u64(o.mem_bytes);
    Ok(())
}

fn read_object(r: &mut Reader<'_>) -> Result<WorldObject> {
    Ok(WorldObject {
        id: r.u64()?,
        name: r.name()?,
        actor_cost: r.u32()?,
        mem_bytes: r.u64()?,
    })
}

/// canonical 二进制编码(magic + version + 全字段闭集,LE)。
pub fn encode_world(world: &PersistentWorld) -> Result<Vec<u8>> {
    validate_world(world)?;
    let mut w = Writer { buf: Vec::new() };
    w.buf.extend_from_slice(&PARTITION_SCHEMA_MAGIC);
    w.u16(PARTITION_SCHEMA_VERSION);
    w.f64(world.cell_size_m);
    w.i32(world.grid_min.x);
    w.i32(world.grid_min.y);
    w.i32(world.grid_max.x);
    w.i32(world.grid_max.y);
    w.u32(world.cells.len() as u32);
    for c in &world.cells {
        w.i32(c.coord.x);
        w.i32(c.coord.y);
        for v in c.bounds_min {
            w.f32(v);
        }
        for v in c.bounds_max {
            w.f32(v);
        }
        w.u32(c.data_layer_mask);
        w.u32(c.page_refs.len() as u32);
        for p in &c.page_refs {
            w.u32(p.resource);
            w.u32(p.page_index);
        }
        match &c.hlod {
            None => w.u8(0),
            Some(h) => {
                w.u8(1);
                w.buf.extend_from_slice(&h.digest);
                w.u32(h.levels);
            }
        }
    }
    w.u32(world.always_loaded.len() as u32);
    for o in &world.always_loaded {
        write_object(&mut w, o)?;
    }
    w.u32(world.spatially_loaded.len() as u32);
    for s in &world.spatially_loaded {
        write_object(&mut w, &s.object)?;
        w.u32(s.cell);
    }
    Ok(w.buf)
}

/// canonical 二进制解码(逐字段核验 + canonical 性 fail-closed)。
pub fn decode_world(bytes: &[u8]) -> Result<PersistentWorld> {
    let mut r = Reader { buf: bytes, pos: 0 };
    if r.take(4)? != PARTITION_SCHEMA_MAGIC {
        return Err(PartitionError::BadMagic);
    }
    let version = r.u16()?;
    if version != PARTITION_SCHEMA_VERSION {
        return Err(PartitionError::UnsupportedVersion(version));
    }
    let cell_size_m = r.f64()?;
    let grid_min = CellCoord {
        x: r.i32()?,
        y: r.i32()?,
    };
    let grid_max = CellCoord {
        x: r.i32()?,
        y: r.i32()?,
    };
    let cells_len = r.u32()? as usize;
    let mut cells = Vec::with_capacity(cells_len);
    for _ in 0..cells_len {
        let coord = CellCoord {
            x: r.i32()?,
            y: r.i32()?,
        };
        let mut bounds_min = [0.0f32; 3];
        let mut bounds_max = [0.0f32; 3];
        for v in &mut bounds_min {
            *v = r.f32()?;
        }
        for v in &mut bounds_max {
            *v = r.f32()?;
        }
        let data_layer_mask = r.u32()?;
        let page_refs_len = r.u32()? as usize;
        let mut page_refs = Vec::with_capacity(page_refs_len);
        for _ in 0..page_refs_len {
            page_refs.push(CellPageRef {
                resource: r.u32()?,
                page_index: r.u32()?,
            });
        }
        let hlod = match r.u8()? {
            0 => None,
            1 => {
                let b = r.take(32)?;
                let mut digest = [0u8; 32];
                digest.copy_from_slice(b);
                Some(CellHlodRef {
                    digest,
                    levels: r.u32()?,
                })
            }
            _ => return Err(PartitionError::NotCanonical("hlod present 标志非 0/1")),
        };
        cells.push(CellMeta {
            coord,
            bounds_min,
            bounds_max,
            page_refs,
            hlod,
            data_layer_mask,
        });
    }
    let always_len = r.u32()? as usize;
    let mut always_loaded = Vec::with_capacity(always_len);
    for _ in 0..always_len {
        always_loaded.push(read_object(&mut r)?);
    }
    let spatial_len = r.u32()? as usize;
    let mut spatially_loaded = Vec::with_capacity(spatial_len);
    for _ in 0..spatial_len {
        spatially_loaded.push(SpatialObject {
            object: read_object(&mut r)?,
            cell: r.u32()?,
        });
    }
    r.finish()?;
    let world = PersistentWorld {
        cell_size_m,
        grid_min,
        grid_max,
        cells,
        always_loaded,
        spatially_loaded,
    };
    validate_world(&world)?;
    Ok(world)
}

/// canonical 性 + 合法性核验(构造期/解码期同一事实源,fail-closed)。
pub fn validate_world(world: &PersistentWorld) -> Result<()> {
    if !world.cell_size_m.is_finite() || world.cell_size_m <= 0.0 {
        return Err(PartitionError::BadCellSize);
    }
    if world.grid_min.x > world.grid_max.x || world.grid_min.y > world.grid_max.y {
        return Err(PartitionError::BadGridExtent);
    }
    let ex = (world.grid_max.x - world.grid_min.x) as u64 + 1;
    let ey = (world.grid_max.y - world.grid_min.y) as u64 + 1;
    if ex * ey != world.cells.len() as u64 {
        return Err(PartitionError::BadGridExtent);
    }
    for (i, c) in world.cells.iter().enumerate() {
        // canonical 序 = (y,x) 升序稠密矩形。
        let expect = CellCoord {
            x: world.grid_min.x + (i as u64 % ex) as i32,
            y: world.grid_min.y + (i as u64 / ex) as i32,
        };
        if c.coord != expect {
            return Err(PartitionError::NotCanonical("cells 非 (y,x) 升序稠密矩形"));
        }
        let (lo, hi) = derived_cell_bounds_xy(world, c.coord);
        if c.bounds_min[0].to_bits() != lo[0].to_bits()
            || c.bounds_min[1].to_bits() != lo[1].to_bits()
            || c.bounds_max[0].to_bits() != hi[0].to_bits()
            || c.bounds_max[1].to_bits() != hi[1].to_bits()
        {
            return Err(PartitionError::NotCanonical(
                "包围盒 x/y 与 coord×cell_size_m 派生值非逐位相等",
            ));
        }
        if !c.bounds_min[2].is_finite()
            || !c.bounds_max[2].is_finite()
            || c.bounds_min[2] > c.bounds_max[2]
        {
            return Err(PartitionError::NotCanonical("包围盒 z 非法"));
        }
        for w in c.page_refs.windows(2) {
            if w[0] >= w[1] {
                return Err(PartitionError::NotCanonical("page_refs 非严格升序"));
            }
        }
        if let Some(h) = &c.hlod
            && h.levels == 0
        {
            return Err(PartitionError::NotCanonical("hlod levels 必须 ≥1"));
        }
    }
    let mut seen_ids = BTreeSet::new();
    for w in world.always_loaded.windows(2) {
        if w[0].id >= w[1].id {
            return Err(PartitionError::NotCanonical("always_loaded 非 id 严格升序"));
        }
    }
    for o in &world.always_loaded {
        seen_ids.insert(o.id);
    }
    for w in world.spatially_loaded.windows(2) {
        if w[0].object.id >= w[1].object.id {
            return Err(PartitionError::NotCanonical(
                "spatially_loaded 非 id 严格升序",
            ));
        }
    }
    for s in &world.spatially_loaded {
        if !seen_ids.insert(s.object.id) {
            return Err(PartitionError::NotCanonical("对象 id 跨表重复"));
        }
        if s.cell as usize >= world.cells.len() {
            return Err(PartitionError::UnknownCellRef {
                object: s.object.id,
                cell: s.cell,
            });
        }
    }
    Ok(())
}

/// 世界资产 digest(canonical 编码 SHA-256;双构建/扰动对照的事实源)。
pub fn world_digest(world: &PersistentWorld) -> Result<[u8; 32]> {
    Ok(sha256::digest(&encode_world(world)?))
}

/// Data Layer 激活语义查询:**只预留不接线**——任何调用一律 fail-closed
/// typed `Err`(RXS-0363 L1 D4 D4;v2 才实现激活语义,避免 schema 二次迁移)。
pub fn data_layer_active(_world: &PersistentWorld, _cell: u32, _layer: u32) -> Result<bool> {
    Err(PartitionError::DataLayerNotWired)
}

// ---------------------------------------------------------------------------
// 三项预算契约与 streaming source
// ---------------------------------------------------------------------------

/// 三项流送预算契约(RXS-0363 L3 逐字;一等契约字段):
/// `MaxStreamingCellsPerFrame` / `MaxActorsToSpawnPerFrame` / `MemoryBudgetMB`。
/// 超预算请求**排队而非抢占**;任一项为 0 = 合法硬降级档(全部请求排队 + 报警,
/// 预算违约注入即此路径)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartitionBudget {
    pub max_streaming_cells_per_frame: u32,
    pub max_actors_to_spawn_per_frame: u32,
    pub memory_budget_mb: u32,
}

impl PartitionBudget {
    pub fn memory_budget_bytes(&self) -> u64 {
        self.memory_budget_mb as u64 * 1024 * 1024
    }
}

/// streaming source 类别(相机/玩家/自定义探针,RFC-0025 §4.A 逐字)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Camera,
    Player,
    Probe,
}

impl SourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceKind::Camera => "camera",
            SourceKind::Player => "player",
            SourceKind::Probe => "probe",
        }
    }
}

/// streaming source:携带距离环(loading radius / 内环常驻)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StreamingSource {
    pub kind: SourceKind,
    pub position_m: [f32; 2],
    pub loading_radius_m: f32,
    /// 内环常驻半径:内环 cell 加载优先级最高(距离升序同档并列 cell id 升序)。
    pub inner_radius_m: f32,
}

impl StreamingSource {
    pub fn validate(&self) -> Result<()> {
        let ok = self.position_m[0].is_finite()
            && self.position_m[1].is_finite()
            && self.loading_radius_m.is_finite()
            && self.inner_radius_m.is_finite()
            && self.loading_radius_m > 0.0
            && self.inner_radius_m >= 0.0
            && self.inner_radius_m <= self.loading_radius_m;
        if ok {
            Ok(())
        } else {
            Err(PartitionError::BadSourceRing)
        }
    }
}

// ---------------------------------------------------------------------------
// cell 四事件闭集
// ---------------------------------------------------------------------------

/// cell 生命周期四事件闭集(RXS-0363 L5 逐字):渲染器唯一消费面。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellEventKind {
    CellLoadBegin,
    CellResident,
    CellUnloadBegin,
    CellEvicted,
}

impl CellEventKind {
    pub const ALL: [CellEventKind; 4] = [
        CellEventKind::CellLoadBegin,
        CellEventKind::CellResident,
        CellEventKind::CellUnloadBegin,
        CellEventKind::CellEvicted,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            CellEventKind::CellLoadBegin => "CellLoadBegin",
            CellEventKind::CellResident => "CellResident",
            CellEventKind::CellUnloadBegin => "CellUnloadBegin",
            CellEventKind::CellEvicted => "CellEvicted",
        }
    }

    fn code(&self) -> u8 {
        match self {
            CellEventKind::CellLoadBegin => 0,
            CellEventKind::CellResident => 1,
            CellEventKind::CellUnloadBegin => 2,
            CellEventKind::CellEvicted => 3,
        }
    }
}

/// cell 事件(帧号 + cell id + 事件种类)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellEvent {
    pub frame: u32,
    pub cell: u32,
    pub kind: CellEventKind,
}

/// 事件日志 canonical 编码(frame u32 LE ‖ cell u32 LE ‖ kind u8 逐条拼接)。
pub fn encode_event_log(events: &[CellEvent]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(events.len() * 9);
    for e in events {
        buf.extend_from_slice(&e.frame.to_le_bytes());
        buf.extend_from_slice(&e.cell.to_le_bytes());
        buf.push(e.kind.code());
    }
    buf
}

/// 事件日志 digest(逐字 golden 对照事实源)。
pub fn event_log_digest(events: &[CellEvent]) -> [u8; 32] {
    sha256::digest(&encode_event_log(events))
}

/// 事件序列状态机校验器(独立核验面;乱序注入必拒):
/// 每 cell 生命周期 `LoadBegin → Resident → UnloadBegin → Evicted`(Evicted 后可
/// 再次 LoadBegin);全局帧号单调不减。
pub fn validate_event_log(events: &[CellEvent]) -> Result<()> {
    // 每 cell 阶段机:0=Absent 1=Loading 2=Active 3=Unloading。
    let mut phase: std::collections::HashMap<u32, u8> = std::collections::HashMap::new();
    let mut prev_frame = 0u32;
    let mut first = true;
    for e in events {
        if !first && e.frame < prev_frame {
            return Err(PartitionError::FrameNonMonotonic {
                prev: prev_frame,
                got: e.frame,
            });
        }
        prev_frame = e.frame;
        first = false;
        let p = phase.entry(e.cell).or_insert(0);
        let ok = match (*p, e.kind) {
            (0, CellEventKind::CellLoadBegin) => Some(1),
            (1, CellEventKind::CellResident) => Some(2),
            (2, CellEventKind::CellUnloadBegin) => Some(3),
            (3, CellEventKind::CellEvicted) => Some(0),
            _ => None,
        };
        match ok {
            Some(next) => *p = next,
            None => {
                let expected = match *p {
                    0 => "CellLoadBegin",
                    1 => "CellResident",
                    2 => "CellUnloadBegin",
                    3 => "CellEvicted",
                    _ => unreachable!(),
                };
                return Err(PartitionError::EventOutOfOrder {
                    cell: e.cell,
                    expected,
                    got: e.kind,
                });
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 流送运行时(距离环 target/resident diff + 三项预算逐帧计数 + 事件总线)
// ---------------------------------------------------------------------------

/// 逐帧预算 evidence(RXS-0363 L3:三项预算计数器逐帧落 evidence,非空;hitch
/// 审计数据源)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameBudgetEvidence {
    pub frame: u32,
    /// 本帧 target cell 数(距离环并集)。
    pub target_cells: u32,
    /// 帧末 resident cell 数。
    pub resident_cells: u32,
    /// 预算①消费:本帧流送(begin+resident)的 cell 数。
    pub streaming_cells_this_frame: u32,
    /// 预算②消费:本帧 spawn 的 actor 数。
    pub actors_spawned_this_frame: u32,
    /// 预算③消费:本帧新驻留字节数。
    pub memory_bytes_this_frame: u64,
    /// 帧末驻留内存总量(字节)。
    pub resident_memory_bytes: u64,
    /// 本帧卸载 cell 数。
    pub cells_unloaded: u32,
    /// 本帧因 target 移出而撤销的排队请求数。
    pub cancelled_pending: u32,
    /// 帧末加载队列深度(滚入下帧)。
    pub queue_depth_end: u32,
    /// 本帧存在因预算排队未服务的请求(预算违约显式报警位)。
    pub budget_stall: bool,
}

/// 流送运行时累计计数面(降级/排队审计)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StreamCounters {
    /// 发生预算排队的帧数(预算违约报警计数器)。
    pub budget_stall_frames: u64,
    pub total_cells_streamed: u64,
    pub total_actors_spawned: u64,
    pub total_bytes_streamed: u64,
    pub total_cells_unloaded: u64,
    pub total_cancelled: u64,
    pub peak_queue_depth: u32,
    pub max_resident_cells: u32,
    pub peak_resident_memory_bytes: u64,
}

/// 距离环 target 求解:cell 中心距任一 source ≤ loading_radius 即入 target;
/// 返回 (cell, 最近源距离², 是否落在某源内环),按确定性加载优先级排序(内环
/// 常驻优先,次按最近距离²,再次 cell id 升序)。求解按源距离环包围盒直取
/// (O(环) 非 O(世界);dense 矩形网格坐标 ↔ 下标闭式换算)。
pub fn target_cells(world: &PersistentWorld, sources: &[StreamingSource]) -> Vec<(u32, f64, bool)> {
    let ex = (world.grid_max.x - world.grid_min.x) as i64 + 1;
    let mut acc: Vec<(u32, f64, bool)> = Vec::new();
    let mut merged: std::collections::HashMap<u32, (f64, bool)> = std::collections::HashMap::new();
    let multi = sources.len() > 1;
    for s in sources {
        let r = s.loading_radius_m as f64;
        // 距离环包围盒 cell 坐标闭式范围(钳到网格)。
        let x0 = ((s.position_m[0] as f64 - r) / world.cell_size_m).floor() as i32;
        let x1 = ((s.position_m[0] as f64 + r) / world.cell_size_m).floor() as i32;
        let y0 = ((s.position_m[1] as f64 - r) / world.cell_size_m).floor() as i32;
        let y1 = ((s.position_m[1] as f64 + r) / world.cell_size_m).floor() as i32;
        let x0 = x0.max(world.grid_min.x);
        let y0 = y0.max(world.grid_min.y);
        let x1 = x1.min(world.grid_max.x);
        let y1 = y1.min(world.grid_max.y);
        for cy in y0..=y1 {
            for cx0 in x0..=x1 {
                let ccx = (cx0 as f64 + 0.5) * world.cell_size_m;
                let ccy = (cy as f64 + 0.5) * world.cell_size_m;
                let dx = ccx - s.position_m[0] as f64;
                let dy = ccy - s.position_m[1] as f64;
                let d2 = dx * dx + dy * dy;
                if d2 > r * r {
                    continue;
                }
                let idx =
                    ((cy - world.grid_min.y) as i64 * ex + (cx0 - world.grid_min.x) as i64) as u32;
                let ri = s.inner_radius_m as f64;
                let inner = d2 <= ri * ri;
                if multi {
                    merged
                        .entry(idx)
                        .and_modify(|e| {
                            if d2 < e.0 {
                                e.0 = d2;
                            }
                            e.1 |= inner;
                        })
                        .or_insert((d2, inner));
                } else {
                    // 单源无重复(每 cell 至多访问一次),免合并直推。
                    acc.push((idx, d2, inner));
                }
            }
        }
    }
    if multi {
        acc.extend(merged.into_iter().map(|(c, (d2, inner))| (c, d2, inner)));
    }
    // 确定性加载优先级:内环常驻优先,次按最近距离,再次 cell id 升序。
    acc.sort_by(|a, b| {
        b.2.cmp(&a.2)
            .then(a.1.partial_cmp(&b.1).expect("距离²非 NaN"))
            .then(a.0.cmp(&b.0))
    });
    acc
}

/// 世界分区流送运行时(装配期激活 always_loaded;每帧 diff 距离环 target 与
/// resident 得 load/unload 队列;三项预算排队扣账;事件总线只出不查)。
pub struct PartitionRuntime {
    world: PersistentWorld,
    budget: PartitionBudget,
    resident: BTreeSet<u32>,
    pending: Vec<u32>,
    events: Vec<CellEvent>,
    cell_actor_cost: Vec<u32>,
    cell_mem_bytes: Vec<u64>,
    resident_memory_bytes: u64,
    counters: StreamCounters,
    last_frame: Option<u32>,
}

impl PartitionRuntime {
    /// 装配:schema 合法性 fail-closed;always_loaded 对象装配期即激活(不占帧
    /// 预算);每 cell 预算消费预聚合(actor_cost/mem_bytes 按 cell 归属求和)。
    pub fn new(world: PersistentWorld, budget: PartitionBudget) -> Result<Self> {
        validate_world(&world)?;
        let n = world.cells.len();
        let mut cell_actor_cost = vec![0u32; n];
        let mut cell_mem_bytes = vec![0u64; n];
        for s in &world.spatially_loaded {
            cell_actor_cost[s.cell as usize] += s.object.actor_cost;
            cell_mem_bytes[s.cell as usize] += s.object.mem_bytes;
        }
        Ok(Self {
            world,
            budget,
            resident: BTreeSet::new(),
            pending: Vec::new(),
            events: Vec::new(),
            cell_actor_cost,
            cell_mem_bytes,
            resident_memory_bytes: 0,
            counters: StreamCounters::default(),
            last_frame: None,
        })
    }

    pub fn world(&self) -> &PersistentWorld {
        &self.world
    }
    pub fn budget(&self) -> &PartitionBudget {
        &self.budget
    }
    pub fn counters(&self) -> &StreamCounters {
        &self.counters
    }
    pub fn events(&self) -> &[CellEvent] {
        &self.events
    }
    pub fn resident(&self) -> &BTreeSet<u32> {
        &self.resident
    }
    pub fn queue_len(&self) -> usize {
        self.pending.len()
    }
    pub fn is_resident(&self, cell: u32) -> bool {
        self.resident.contains(&cell)
    }

    /// 事件总线消费面:渲染器逐帧 drain(只出不反向查询分区状态,D4 D1)。
    pub fn drain_events(&mut self) -> Vec<CellEvent> {
        std::mem::take(&mut self.events)
    }

    /// 每帧协议:距离环求 target → 与 resident diff → unload 先行(cell id 升
    /// 序),load 排队 FIFO(新请求按内环/距离/id 确定性序入队尾);三项预算逐
    /// 请求扣账,任一契约项装不下当前请求**即停**,当前及后续请求原序滚入下
    /// 帧(排队而非抢占);本帧存在未服务请求即置 `budget_stall` 报警位。
    pub fn tick(&mut self, frame: u32, sources: &[StreamingSource]) -> Result<FrameBudgetEvidence> {
        if let Some(prev) = self.last_frame
            && frame <= prev
        {
            return Err(PartitionError::FrameNonMonotonic { prev, got: frame });
        }
        self.last_frame = Some(frame);
        for s in sources {
            s.validate()?;
        }
        let ranked = target_cells(&self.world, sources);
        let target: BTreeSet<u32> = ranked.iter().map(|(c, _, _)| *c).collect();

        let mut ev = FrameBudgetEvidence {
            frame,
            ..FrameBudgetEvidence::default()
        };

        // 1) unload 队列:resident - target(cell id 升序,UnloadBegin→Evicted 紧邻)。
        let unloads: Vec<u32> = self.resident.difference(&target).copied().collect();
        for cell in unloads {
            self.events.push(CellEvent {
                frame,
                cell,
                kind: CellEventKind::CellUnloadBegin,
            });
            self.events.push(CellEvent {
                frame,
                cell,
                kind: CellEventKind::CellEvicted,
            });
            self.resident.remove(&cell);
            self.resident_memory_bytes -= self.cell_mem_bytes[cell as usize];
            ev.cells_unloaded += 1;
        }

        // 2) 排队撤销:pending 中已不在 target 的请求确定性出队(未 emit 过事件)。
        let before = self.pending.len();
        self.pending.retain(|c| target.contains(c));
        ev.cancelled_pending = (before - self.pending.len()) as u32;

        // 3) load 队列:target - resident - pending,确定性优先级序入队尾。
        let pending_set: BTreeSet<u32> = self.pending.iter().copied().collect();
        for (cell, _, _) in &ranked {
            if !self.resident.contains(cell) && !pending_set.contains(cell) {
                self.pending.push(*cell);
            }
        }

        // 4) 三项预算 FIFO 扣账:队首服务不了即停(排队而非抢占;服务即出队,
        // 后续请求顺位顶上,索引恒指队首)。
        let i = 0usize;
        while i < self.pending.len() {
            let cell = self.pending[i];
            let actors = self.cell_actor_cost[cell as usize];
            let mem = self.cell_mem_bytes[cell as usize];
            let cells_ok =
                ev.streaming_cells_this_frame < self.budget.max_streaming_cells_per_frame;
            let actors_ok = ev.actors_spawned_this_frame as u64 + actors as u64
                <= self.budget.max_actors_to_spawn_per_frame as u64;
            let mem_ok = self.resident_memory_bytes + mem <= self.budget.memory_budget_bytes();
            if !(cells_ok && actors_ok && mem_ok) {
                ev.budget_stall = true;
                break;
            }
            self.pending.remove(i);
            self.events.push(CellEvent {
                frame,
                cell,
                kind: CellEventKind::CellLoadBegin,
            });
            self.events.push(CellEvent {
                frame,
                cell,
                kind: CellEventKind::CellResident,
            });
            self.resident.insert(cell);
            self.resident_memory_bytes += mem;
            ev.streaming_cells_this_frame += 1;
            ev.actors_spawned_this_frame += actors;
            ev.memory_bytes_this_frame += mem;
        }
        // 帧末仍有排队 ⇒ 本帧必有请求未被服务(含「全部命中预算」之外的余量
        // 情形)⇒ 报警位以「队列非空且本帧有新提交/滚存未清」为准:即停已置位;
        // 此处不重复置位(只有真因预算/即停阻塞才报警)。
        ev.target_cells = target.len() as u32;
        ev.resident_cells = self.resident.len() as u32;
        ev.resident_memory_bytes = self.resident_memory_bytes;
        ev.queue_depth_end = self.pending.len() as u32;

        // 5) 累计计数面。
        if ev.budget_stall {
            self.counters.budget_stall_frames += 1;
        }
        self.counters.total_cells_streamed += ev.streaming_cells_this_frame as u64;
        self.counters.total_actors_spawned += ev.actors_spawned_this_frame as u64;
        self.counters.total_bytes_streamed += ev.memory_bytes_this_frame;
        self.counters.total_cells_unloaded += ev.cells_unloaded as u64;
        self.counters.total_cancelled += ev.cancelled_pending as u64;
        self.counters.peak_queue_depth = self.counters.peak_queue_depth.max(ev.queue_depth_end);
        self.counters.max_resident_cells = self.counters.max_resident_cells.max(ev.resident_cells);
        self.counters.peak_resident_memory_bytes = self
            .counters
            .peak_resident_memory_bytes
            .max(self.resident_memory_bytes);
        Ok(ev)
    }
}

// ---------------------------------------------------------------------------
// 确定性场景(harness 与单测同一事实源;measured 冻结,禁手写 golden)
// ---------------------------------------------------------------------------

/// 确定性 LCG(Knuth MMIX;wrapping u64,跨平台位级一致)。
pub struct Lcg(u64);

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

fn make_bounds(world_size: f64, coord: CellCoord, z_lo: f32, z_hi: f32) -> ([f32; 3], [f32; 3]) {
    (
        [
            (coord.x as f64 * world_size) as f32,
            (coord.y as f64 * world_size) as f32,
            z_lo,
        ],
        [
            ((coord.x as f64 + 1.0) * world_size) as f32,
            ((coord.y as f64 + 1.0) * world_size) as f32,
            z_hi,
        ],
    )
}

/// canonical golden 场景世界:16×16 cell,边长 64m(资产属性),4 个
/// always_loaded 对象,逐 cell 2~4 个 spatially_loaded 对象,部分 cell 携带
/// HLOD 层级引用占位(digest = sha256("hlod-canonical-{x}-{y}") 派生)。
pub fn canonical_world() -> PersistentWorld {
    let extent = 16i32;
    let cell_size = 64.0f64;
    let mut cells = Vec::with_capacity((extent * extent) as usize);
    let mut rng = Lcg::new(0x0110_5eed_5eed_0110);
    for y in 0..extent {
        for x in 0..extent {
            let coord = CellCoord { x, y };
            let (bounds_min, bounds_max) = make_bounds(cell_size, coord, -8.0, 24.0);
            let mut page_refs = Vec::new();
            let n_refs = 1 + rng.below(2) as u32;
            for k in 0..n_refs {
                page_refs.push(CellPageRef {
                    resource: (y * extent + x) as u32,
                    page_index: k,
                });
            }
            let hlod = if (x * 3 + y) % 4 == 0 {
                Some(CellHlodRef {
                    digest: sha256::digest(format!("hlod-canonical-{x}-{y}").as_bytes()),
                    levels: 2,
                })
            } else {
                None
            };
            cells.push(CellMeta {
                coord,
                bounds_min,
                bounds_max,
                page_refs,
                hlod,
                data_layer_mask: 0,
            });
        }
    }
    let always_loaded = (0..4u64)
        .map(|i| WorldObject {
            id: i + 1,
            name: format!("global_{i}"),
            actor_cost: 1,
            mem_bytes: 4096,
        })
        .collect();
    let mut spatially_loaded = Vec::new();
    let mut next_id = 100u64;
    for (i, c) in cells.iter().enumerate() {
        let n = 2 + ((c.coord.x * 7 + c.coord.y * 13) % 3) as u32;
        for k in 0..n {
            spatially_loaded.push(SpatialObject {
                object: WorldObject {
                    id: next_id,
                    name: format!("obj_{next_id}"),
                    actor_cost: 1 + (k % 3),
                    mem_bytes: 256 * 1024 * (1 + (next_id % 3)),
                },
                cell: i as u32,
            });
            next_id += 1;
        }
    }
    PersistentWorld {
        cell_size_m: cell_size,
        grid_min: CellCoord { x: 0, y: 0 },
        grid_max: CellCoord {
            x: extent - 1,
            y: extent - 1,
        },
        cells,
        always_loaded,
        spatially_loaded,
    }
}

/// canonical golden 场景预算(三项契约字段;每帧 2 cell / 48 actor / 8MiB)。
pub fn canonical_budget() -> PartitionBudget {
    PartitionBudget {
        max_streaming_cells_per_frame: 2,
        max_actors_to_spawn_per_frame: 48,
        memory_budget_mb: 8,
    }
}

/// canonical golden 场景相机轨迹(64 帧:相机沿 x 向匀速穿越世界后斜向回扫,
/// 闭式整数推进,位级确定)。
pub fn canonical_camera_path(frames: u32) -> Vec<StreamingSource> {
    (0..frames)
        .map(|f| {
            let (x, y) = if f < 32 {
                (24.0f32 + f as f32 * 28.0, 512.0f32)
            } else {
                let g = f - 31;
                (892.0f32 - g as f32 * 20.0, 512.0f32 + g as f32 * 14.0)
            };
            StreamingSource {
                kind: SourceKind::Camera,
                position_m: [x, y],
                loading_radius_m: 160.0,
                inner_radius_m: 64.0,
            }
        })
        .collect()
}

/// 代表性大世界 soak 场景:512×512 cell,边长 32m,逐 cell 0~4 个
/// spatially_loaded 对象(LCG 确定性),总对象数十万级。
pub fn soak_world() -> PersistentWorld {
    let extent = 512i32;
    let cell_size = 32.0f64;
    let mut rng = Lcg::new(0x5eed_5eed_5eed);
    let mut cells = Vec::with_capacity((extent * extent) as usize);
    for y in 0..extent {
        for x in 0..extent {
            let coord = CellCoord { x, y };
            let (bounds_min, bounds_max) = make_bounds(cell_size, coord, -16.0, 48.0);
            let n_refs = 1 + rng.below(3) as u32;
            let page_refs = (0..n_refs)
                .map(|k| CellPageRef {
                    resource: (y * extent + x) as u32,
                    page_index: k,
                })
                .collect();
            cells.push(CellMeta {
                coord,
                bounds_min,
                bounds_max,
                page_refs,
                hlod: None,
                data_layer_mask: 0,
            });
        }
    }
    let always_loaded = (0..8u64)
        .map(|i| WorldObject {
            id: i + 1,
            name: format!("soak_global_{i}"),
            actor_cost: 1,
            mem_bytes: 8192,
        })
        .collect();
    let mut spatially_loaded = Vec::new();
    let mut next_id = 1000u64;
    for (i, _) in cells.iter().enumerate() {
        let n = rng.below(5) as u32;
        for _ in 0..n {
            spatially_loaded.push(SpatialObject {
                object: WorldObject {
                    id: next_id,
                    name: format!("soak_obj_{next_id}"),
                    actor_cost: 1 + (rng.below(4) as u32),
                    mem_bytes: 128 * 1024 * (1 + rng.below(8)),
                },
                cell: i as u32,
            });
            next_id += 1;
        }
    }
    PersistentWorld {
        cell_size_m: cell_size,
        grid_min: CellCoord { x: 0, y: 0 },
        grid_max: CellCoord {
            x: extent - 1,
            y: extent - 1,
        },
        cells,
        always_loaded,
        spatially_loaded,
    }
}

/// soak 预算(三项契约字段;每帧 48 cell / 256 actor / 1GiB)。
pub fn soak_budget() -> PartitionBudget {
    PartitionBudget {
        max_streaming_cells_per_frame: 48,
        max_actors_to_spawn_per_frame: 256,
        memory_budget_mb: 1024,
    }
}

/// soak 合成相机路径(三角波双向扫场,闭式整数推进;loading radius 1536m /
/// 内环 512m;覆盖全网格并反复回访,负载含 load/unload/排队全形态)。
pub fn soak_camera_path(frames: u32) -> Vec<StreamingSource> {
    let span = 512.0f32 * 32.0; // 16384m
    let tri = |t: u64, period: u64| -> f32 {
        let m = (t % (2 * period)) as f32;
        let half = period as f32;
        (if m <= half { m } else { 2.0 * half - m }) * (span / half)
    };
    (0..frames)
        .map(|f| {
            let t = f as u64;
            StreamingSource {
                kind: SourceKind::Camera,
                position_m: [tri(t * 97, 512), tri(t * 61, 768)],
                loading_radius_m: 1536.0,
                inner_radius_m: 512.0,
            }
        })
        .collect()
}

/// soak 逐帧记录(预算三项计数 + tick 墙钟纳秒;hitch p99 计数面)。
#[derive(Debug, Clone, Copy)]
pub struct SoakFrameRecord {
    pub budget: FrameBudgetEvidence,
    pub tick_ns: u64,
    /// 本帧事件数(按四事件闭集序)。
    pub events_by_kind: [u32; 4],
}

/// 大世界 soak:单一运行时按相机路径逐帧 tick,逐帧记录预算计数与 tick 耗时;
/// 事件总线逐帧 drain(消费面语义;计数留痕,不囤积全量日志)。
pub fn run_soak(
    world: &PersistentWorld,
    budget: PartitionBudget,
    path: &[StreamingSource],
) -> Result<Vec<SoakFrameRecord>> {
    let mut rt = PartitionRuntime::new(world.clone(), budget)?;
    let mut out = Vec::with_capacity(path.len());
    for (f, s) in path.iter().enumerate() {
        let t0 = std::time::Instant::now();
        let ev = rt.tick(f as u32, std::slice::from_ref(s))?;
        let tick_ns = t0.elapsed().as_nanos() as u64;
        check_frame_budget_consistency(&ev, &budget)?;
        let mut events_by_kind = [0u32; 4];
        for e in rt.drain_events() {
            events_by_kind[e.kind.code() as usize] += 1;
        }
        out.push(SoakFrameRecord {
            budget: ev,
            tick_ns,
            events_by_kind,
        });
    }
    Ok(out)
}

/// p 分位数(nearest-rank;输入任意序,内部排序)。
pub fn percentile_ns(samples: &[u64], p: f64) -> u64 {
    assert!(!samples.is_empty(), "p 分位样本非空");
    let mut v = samples.to_vec();
    v.sort_unstable();
    let rank = ((p * v.len() as f64).ceil() as usize).clamp(1, v.len());
    v[rank - 1]
}

/// 逐帧预算一致性机核(RXS-0363 L3/L4「不得静默超帧」硬判据):三项契约逐帧
/// 核算——本帧流送 cell 数 ≤ `MaxStreamingCellsPerFrame`、本帧 spawn actor 数
/// ≤ `MaxActorsToSpawnPerFrame`、帧末驻留内存 ≤ `MemoryBudgetMB`;任一超限即
/// typed Err(静默超帧 = RED)。harness 对每帧强制调用;RED 臂以篡改帧喂入
/// 证明本机核能红。
pub fn check_frame_budget_consistency(
    ev: &FrameBudgetEvidence,
    budget: &PartitionBudget,
) -> Result<()> {
    if ev.streaming_cells_this_frame > budget.max_streaming_cells_per_frame {
        return Err(PartitionError::SilentBudgetOverrun {
            field: "MaxStreamingCellsPerFrame",
            value: ev.streaming_cells_this_frame as u64,
            cap: budget.max_streaming_cells_per_frame as u64,
        });
    }
    if ev.actors_spawned_this_frame > budget.max_actors_to_spawn_per_frame {
        return Err(PartitionError::SilentBudgetOverrun {
            field: "MaxActorsToSpawnPerFrame",
            value: ev.actors_spawned_this_frame as u64,
            cap: budget.max_actors_to_spawn_per_frame as u64,
        });
    }
    if ev.resident_memory_bytes > budget.memory_budget_bytes() {
        return Err(PartitionError::SilentBudgetOverrun {
            field: "MemoryBudgetMB",
            value: ev.resident_memory_bytes,
            cap: budget.memory_budget_bytes(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(d: &[u8; 32]) -> String {
        d.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// RXS-0363 L1:schema encode→decode→encode 逐字节往返 + digest 双算相等。
    #[test]
    //@ spec: RXS-0363
    fn schema_roundtrip_byte_equal() {
        let world = canonical_world();
        let bytes = encode_world(&world).expect("encode");
        let back = decode_world(&bytes).expect("decode");
        assert_eq!(back, world);
        let bytes2 = encode_world(&back).expect("re-encode");
        assert_eq!(bytes, bytes2);
        assert_eq!(world_digest(&world).unwrap(), world_digest(&back).unwrap());
    }

    /// RXS-0363 L1:cell 边长为资产属性——同构世界仅 cell_size_m 不同则 digest
    /// 不同;包围盒 x/y 派生逐位核验(篡改包围盒 fail-closed)。
    #[test]
    //@ spec: RXS-0363
    fn cell_size_is_asset_property_and_bounds_derived() {
        let world = canonical_world();
        let mut resized = canonical_world();
        resized.cell_size_m = 128.0;
        let derived: Vec<([f32; 2], [f32; 2])> = resized
            .cells
            .iter()
            .map(|c| derived_cell_bounds_xy(&resized, c.coord))
            .collect();
        for (c, (lo, hi)) in resized.cells.iter_mut().zip(derived) {
            c.bounds_min[0] = lo[0];
            c.bounds_min[1] = lo[1];
            c.bounds_max[0] = hi[0];
            c.bounds_max[1] = hi[1];
        }
        assert_ne!(
            world_digest(&world).unwrap(),
            world_digest(&resized).unwrap()
        );
        // 篡改包围盒(与派生值不符)必拒。
        let mut tampered = canonical_world();
        tampered.cells[0].bounds_max[0] += 1.0;
        assert!(matches!(
            validate_world(&tampered),
            Err(PartitionError::NotCanonical(_))
        ));
    }

    /// RXS-0361 体例沿 G9 纪律:非法构造 fail-closed——magic 错/版本错/截断/
    /// 残余字节/非 canonical 序/未知 cell 引用逐一锚定。
    #[test]
    //@ spec: RXS-0363
    fn decode_fail_closed_variants() {
        let world = canonical_world();
        let bytes = encode_world(&world).unwrap();
        let mut bad_magic = bytes.clone();
        bad_magic[0] ^= 0x01;
        assert!(matches!(
            decode_world(&bad_magic),
            Err(PartitionError::BadMagic)
        ));
        let mut bad_ver = bytes.clone();
        bad_ver[4] = 9;
        assert!(matches!(
            decode_world(&bad_ver),
            Err(PartitionError::UnsupportedVersion(9))
        ));
        assert!(matches!(
            decode_world(&bytes[..bytes.len() - 3]),
            Err(PartitionError::Truncated { .. })
        ));
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(matches!(
            decode_world(&trailing),
            Err(PartitionError::TrailingBytes { extra: 1 })
        ));
        let mut bad_order = canonical_world();
        bad_order.cells.swap(0, 1);
        assert!(matches!(
            validate_world(&bad_order),
            Err(PartitionError::NotCanonical(_))
        ));
        let mut bad_ref = canonical_world();
        bad_ref.spatially_loaded[0].cell = 9999;
        assert!(matches!(
            validate_world(&bad_ref),
            Err(PartitionError::UnknownCellRef { .. })
        ));
    }

    /// RXS-0363 L1(D4 D4):Data Layer 掩码位只预留不接线——字段参与往返;激活
    /// 查询一律 typed Err;掩码非零不改变流送行为(事件 digest 逐位相等)。
    #[test]
    //@ spec: RXS-0363
    fn data_layer_reserved_not_wired() {
        let mut world = canonical_world();
        world.cells[3].data_layer_mask = 0xDEAD_BEEF;
        let bytes = encode_world(&world).unwrap();
        let back = decode_world(&bytes).unwrap();
        assert_eq!(back.cells[3].data_layer_mask, 0xDEAD_BEEF);
        assert!(matches!(
            data_layer_active(&back, 3, 0),
            Err(PartitionError::DataLayerNotWired)
        ));
        assert!(matches!(
            data_layer_active(&back, 0, 31),
            Err(PartitionError::DataLayerNotWired)
        ));
        // 掩码不消费:mask 全 0 与非零两世界事件流逐位相等。
        let path = canonical_camera_path(16);
        let budget = canonical_budget();
        let mut rt0 = PartitionRuntime::new(canonical_world(), budget).unwrap();
        let mut rt1 = PartitionRuntime::new(world, budget).unwrap();
        for (f, s) in path.iter().enumerate() {
            rt0.tick(f as u32, std::slice::from_ref(s)).unwrap();
            rt1.tick(f as u32, std::slice::from_ref(s)).unwrap();
        }
        assert_eq!(
            event_log_digest(rt0.events()),
            event_log_digest(rt1.events())
        );
    }

    /// RXS-0363 L2/L3:距离环 target 与 resident diff;三项预算计数器逐帧
    /// evidence 非空;确定性双跑位级一致。
    #[test]
    //@ spec: RXS-0363
    fn streaming_diff_and_per_frame_budget_evidence() {
        let world = canonical_world();
        let budget = canonical_budget();
        let path = canonical_camera_path(64);
        let mut rt = PartitionRuntime::new(world.clone(), budget).unwrap();
        let mut frames = Vec::new();
        for (f, s) in path.iter().enumerate() {
            frames.push(rt.tick(f as u32, std::slice::from_ref(s)).unwrap());
        }
        assert_eq!(frames.len(), 64);
        // 逐帧三项计数面在位(字段存在且语义自洽)。
        for ev in &frames {
            assert!(ev.streaming_cells_this_frame <= budget.max_streaming_cells_per_frame);
            assert!(ev.actors_spawned_this_frame <= budget.max_actors_to_spawn_per_frame);
            assert!(ev.resident_memory_bytes <= budget.memory_budget_bytes());
        }
        // 场景非平凡:有加载、有卸载、有排队滚存。
        let total_loaded: u32 = frames.iter().map(|e| e.streaming_cells_this_frame).sum();
        let total_unloaded: u32 = frames.iter().map(|e| e.cells_unloaded).sum();
        assert!(total_loaded > 0);
        assert!(total_unloaded > 0);
        assert!(rt.counters().peak_queue_depth > 0);
        // 双跑位级一致(帧 evidence 与事件 digest 全等)。
        let mut rt2 = PartitionRuntime::new(world, budget).unwrap();
        for (f, s) in path.iter().enumerate() {
            let ev2 = rt2.tick(f as u32, std::slice::from_ref(s)).unwrap();
            assert_eq!(ev2, frames[f]);
        }
        assert_eq!(
            event_log_digest(rt.events()),
            event_log_digest(rt2.events())
        );
        validate_event_log(rt.events()).expect("事件序列合法");
    }

    /// RXS-0363 L4(RED 臂):预算违约注入必排队降级——
    /// `MaxStreamingCellsPerFrame=0` 注入 ⇒ 零 cell 驻留(无静默超帧)、每帧
    /// `budget_stall` 报警、队列深度增长、降级计数器非零。
    #[test]
    //@ spec: RXS-0363
    fn budget_violation_injection_queues_and_alarms() {
        let world = canonical_world();
        let injected = PartitionBudget {
            max_streaming_cells_per_frame: 0,
            ..canonical_budget()
        };
        let path = canonical_camera_path(16);
        let mut rt = PartitionRuntime::new(world, injected).unwrap();
        let mut stall_frames = 0u64;
        for (f, s) in path.iter().enumerate() {
            let ev = rt.tick(f as u32, std::slice::from_ref(s)).unwrap();
            assert_eq!(ev.streaming_cells_this_frame, 0, "静默超帧即 RED");
            if ev.budget_stall {
                stall_frames += 1;
            }
        }
        assert_eq!(rt.resident().len(), 0);
        assert!(stall_frames > 0, "预算违约必须显式报警");
        assert_eq!(rt.counters().budget_stall_frames, stall_frames);
        assert!(rt.counters().peak_queue_depth > 0);
        assert!(rt.queue_len() > 0);
        // sabotage 探针(能红证明):充足预算下同轨迹全量即时服务、零报警——报警
        // 键于真实排队阻塞,非平凡恒置。
        let generous = PartitionBudget {
            max_streaming_cells_per_frame: 1024,
            max_actors_to_spawn_per_frame: 1 << 20,
            memory_budget_mb: 4096,
        };
        let mut ok = PartitionRuntime::new(canonical_world(), generous).unwrap();
        for (f, s) in path.iter().enumerate() {
            let ev = ok.tick(f as u32, std::slice::from_ref(s)).unwrap();
            assert!(!ev.budget_stall, "充足预算下不应有排队报警");
        }
        assert!(ok.counters().total_cells_streamed > 0);
        assert_eq!(ok.counters().budget_stall_frames, 0);
    }

    /// RXS-0363 L4:内存/actor 预算违约同律——MemoryBudgetMB 压到装不下任一
    /// cell ⇒ 排队降级报警;预算内 cell 照常流送。
    #[test]
    //@ spec: RXS-0363
    fn memory_budget_violation_queues() {
        let world = canonical_world();
        let min_cell_mem = (0..world.cells.len())
            .map(|i| {
                world
                    .spatially_loaded
                    .iter()
                    .filter(|s| s.cell == i as u32)
                    .map(|s| s.object.mem_bytes)
                    .sum::<u64>()
            })
            .min()
            .unwrap();
        let injected = PartitionBudget {
            memory_budget_mb: 0, // 0 字节 = 任何 cell 装不下
            ..canonical_budget()
        };
        assert!(min_cell_mem > 0);
        let path = canonical_camera_path(8);
        let mut rt = PartitionRuntime::new(world, injected).unwrap();
        for (f, s) in path.iter().enumerate() {
            let ev = rt.tick(f as u32, std::slice::from_ref(s)).unwrap();
            assert_eq!(ev.streaming_cells_this_frame, 0);
            assert!(ev.budget_stall);
        }
        assert!(rt.counters().budget_stall_frames >= 8);
    }

    /// RXS-0363 L5:四事件序列状态机——乱序注入(Resident 先于 LoadBegin /
    /// Unload 与 Evicted 颠倒)必拒;合法序列接受(校验器非平凡恒拒)。
    #[test]
    //@ spec: RXS-0363
    fn event_order_validator_rejects_out_of_order() {
        let ev = |f, c, k| CellEvent {
            frame: f,
            cell: c,
            kind: k,
        };
        let good = [
            ev(0, 1, CellEventKind::CellLoadBegin),
            ev(0, 1, CellEventKind::CellResident),
            ev(1, 1, CellEventKind::CellUnloadBegin),
            ev(1, 1, CellEventKind::CellEvicted),
            ev(2, 1, CellEventKind::CellLoadBegin),
            ev(2, 1, CellEventKind::CellResident),
        ];
        validate_event_log(&good).expect("合法序列");
        // Resident 先于 LoadBegin。
        let mut bad1 = good;
        bad1.swap(0, 1);
        assert!(matches!(
            validate_event_log(&bad1),
            Err(PartitionError::EventOutOfOrder { cell: 1, .. })
        ));
        // UnloadBegin 与 Evicted 颠倒。
        let mut bad2 = good;
        bad2.swap(2, 3);
        assert!(matches!(
            validate_event_log(&bad2),
            Err(PartitionError::EventOutOfOrder { cell: 1, .. })
        ));
        // 帧号回退(第 4 条帧号前进到 5,第 5 条回退到 2 ⇒ 非单调)。
        let mut bad4 = good;
        bad4[3].frame = 5;
        bad4[4].frame = 2;
        assert!(matches!(
            validate_event_log(&bad4),
            Err(PartitionError::FrameNonMonotonic { .. })
        ));
    }

    /// RXS-0363 L5:事件 digest 对乱序敏感(任意两条交换 ⇒ digest 分叉)。
    #[test]
    //@ spec: RXS-0363
    fn event_log_digest_order_sensitive() {
        let world = canonical_world();
        let budget = canonical_budget();
        let path = canonical_camera_path(32);
        let mut rt = PartitionRuntime::new(world, budget).unwrap();
        for (f, s) in path.iter().enumerate() {
            rt.tick(f as u32, std::slice::from_ref(s)).unwrap();
        }
        let events = rt.events().to_vec();
        assert!(events.len() >= 8);
        let d0 = event_log_digest(&events);
        let mut swapped = events.clone();
        swapped.swap(2, 5);
        assert_ne!(d0, event_log_digest(&swapped), "乱序必须分叉: {}", hex(&d0));
        assert!(validate_event_log(&events).is_ok());
        assert!(validate_event_log(&swapped).is_err() || event_log_digest(&swapped) != d0);
    }

    /// RXS-0363 L3/L4:逐帧预算一致性机核——三项契约逐帧核算;篡改帧(静默
    /// 超任一契约上限)必 typed Err(能红证明)。
    #[test]
    //@ spec: RXS-0363
    fn frame_budget_consistency_checker_catches_silent_overrun() {
        let budget = canonical_budget();
        let ok = FrameBudgetEvidence {
            frame: 0,
            streaming_cells_this_frame: budget.max_streaming_cells_per_frame,
            actors_spawned_this_frame: budget.max_actors_to_spawn_per_frame,
            resident_memory_bytes: budget.memory_budget_bytes(),
            ..FrameBudgetEvidence::default()
        };
        check_frame_budget_consistency(&ok, &budget).expect("恰在界内合法");
        let mut over_cells = ok;
        over_cells.streaming_cells_this_frame += 1;
        assert!(matches!(
            check_frame_budget_consistency(&over_cells, &budget),
            Err(PartitionError::SilentBudgetOverrun {
                field: "MaxStreamingCellsPerFrame",
                ..
            })
        ));
        let mut over_actors = ok;
        over_actors.actors_spawned_this_frame += 1;
        assert!(matches!(
            check_frame_budget_consistency(&over_actors, &budget),
            Err(PartitionError::SilentBudgetOverrun {
                field: "MaxActorsToSpawnPerFrame",
                ..
            })
        ));
        let mut over_mem = ok;
        over_mem.resident_memory_bytes += 1;
        assert!(matches!(
            check_frame_budget_consistency(&over_mem, &budget),
            Err(PartitionError::SilentBudgetOverrun {
                field: "MemoryBudgetMB",
                ..
            })
        ));
    }

    /// RXS-0363 L2:距离环 target 求解(bbox 直取)与暴力全扫逐集合等价——单源
    /// 与多源合并双臂(距离²取最近源、内环取并集)。
    #[test]
    //@ spec: RXS-0363
    fn target_cells_matches_brute_force() {
        fn brute(world: &PersistentWorld, sources: &[StreamingSource]) -> Vec<(u32, f64, bool)> {
            let mut out = Vec::new();
            for (i, c) in world.cells.iter().enumerate() {
                let cx = (c.coord.x as f64 + 0.5) * world.cell_size_m;
                let cy = (c.coord.y as f64 + 0.5) * world.cell_size_m;
                let mut best = f64::INFINITY;
                let mut inner = false;
                for s in sources {
                    let dx = cx - s.position_m[0] as f64;
                    let dy = cy - s.position_m[1] as f64;
                    let d2 = dx * dx + dy * dy;
                    let r = s.loading_radius_m as f64;
                    if d2 <= r * r {
                        best = best.min(d2);
                    }
                    let ri = s.inner_radius_m as f64;
                    if d2 <= ri * ri {
                        inner = true;
                    }
                }
                if best.is_finite() {
                    out.push((i as u32, best, inner));
                }
            }
            out.sort_by(|a, b| {
                b.2.cmp(&a.2)
                    .then(a.1.partial_cmp(&b.1).unwrap())
                    .then(a.0.cmp(&b.0))
            });
            out
        }
        let world = canonical_world();
        let s1 = StreamingSource {
            kind: SourceKind::Camera,
            position_m: [300.0, 300.0],
            loading_radius_m: 160.0,
            inner_radius_m: 64.0,
        };
        let s2 = StreamingSource {
            kind: SourceKind::Probe,
            position_m: [330.0, 310.0],
            loading_radius_m: 200.0,
            inner_radius_m: 100.0,
        };
        assert_eq!(
            target_cells(&world, &[s1]),
            brute(&world, &[s1]),
            "单源等价"
        );
        assert_eq!(
            target_cells(&world, &[s1, s2]),
            brute(&world, &[s1, s2]),
            "多源合并等价"
        );
        // 环外/边界:半径小于半 cell 时 target 可为空或恰含脚下 cell。
        let s3 = StreamingSource {
            loading_radius_m: 10.0,
            inner_radius_m: 0.0,
            ..s1
        };
        assert_eq!(
            target_cells(&world, &[s3]),
            brute(&world, &[s3]),
            "微半径等价"
        );
    }

    /// RXS-0363 L6 前置:soak 合成场景规模与帧数阈值断言(≥10000 帧声明口径)。
    #[test]
    //@ spec: RXS-0363
    fn soak_world_scale_and_path_deterministic() {
        let world = soak_world();
        validate_world(&world).unwrap();
        assert_eq!(world.cells.len(), 512 * 512);
        assert!(world.spatially_loaded.len() > 100_000);
        let path = soak_camera_path(M110_SOAK_DEFAULT_FRAMES);
        assert_eq!(path.len(), M110_SOAK_DEFAULT_FRAMES as usize);
        assert!(path.len() >= M110_SOAK_MIN_FRAMES as usize);
        // 路径位级确定(双跑逐点相等)。
        let path2 = soak_camera_path(M110_SOAK_DEFAULT_FRAMES);
        assert_eq!(path, path2);
        // percentile 语义锚定。
        assert_eq!(percentile_ns(&[1, 2, 3, 4, 5], 0.5), 3);
        assert_eq!(percentile_ns(&[5, 1, 4, 2, 3], 0.99), 5);
        assert_eq!(percentile_ns(&[7, 7, 7], 0.99), 7);
    }
}
